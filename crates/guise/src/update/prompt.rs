//! `UpdatePrompt` — the "an update is available" panel (gpui entity).
//!
//! A state machine with exactly one action in flight. Once an install starts it
//! reports every stage it moves through, and it stays put until it either
//! restarts the app or fails with the reason on screen — never a button whose
//! only feedback is a notification you might not see.
//!
//! ```ignore
//! let prompt = cx.new(|cx| UpdatePrompt::new(updater, release, cx));
//! cx.subscribe(&prompt, |_this, _prompt, event: &UpdatePromptEvent, _cx| {
//!     if let UpdatePromptEvent::Failed(why) = event { /* log it */ }
//! })
//! .detach();
//! ```
//!
//! [`super::open`] wraps one in its own window; rendered directly it is an
//! ordinary panel, so it can live in a modal or a settings pane instead.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gpui::prelude::*;
use gpui::{
  div, px, App, Context, EventEmitter, FocusHandle, Focusable, FontWeight, IntoElement,
  KeyDownEvent, MouseButton, SharedString, Window, WindowControlArea,
};

use super::{InstallKind, Relaunch, Release, UpdateStage, Updater};
use crate::devtools::Probed;
use crate::theme::{theme, ColorName, Size};
use crate::{Alert, Button, Progress, Variant};

/// Height of the strip that drags the window, and the padding that clears a
/// transparent titlebar. A platform metric, not a themed one.
const TITLEBAR: f32 = 34.0;

/// How often the foreground task drains installer progress into the view.
const PROGRESS_TICK: Duration = Duration::from_millis(80);

/// Emitted as the prompt works.
#[derive(Debug, Clone)]
pub enum UpdatePromptEvent {
  /// The user accepted and the install began.
  Started,
  /// The installer moved to a new stage.
  Stage(UpdateStage),
  /// The new version is on disk. Carries how to relaunch into it; the prompt
  /// restarts the app itself unless [`UpdatePrompt::auto_restart`] is off.
  Installed(Relaunch),
  /// The install failed, with the reason already on screen and the action
  /// button offering to retry.
  Failed(String),
  /// The user is done with the prompt — "Later", Escape, or the download page
  /// having opened for an install that can't be rewritten. Whoever owns the
  /// window closes it; the prompt never closes a window it doesn't own.
  Dismissed,
}

/// Where the prompt is in its one-shot lifecycle. Only `Idle` and `Failed`
/// accept the action, which is what keeps the action button from starting a
/// second install over the first.
enum Phase {
  Idle,
  Working(UpdateStage),
  Failed(String),
}

/// Set while an install is in flight, anywhere in the process.
///
/// `Phase::Working` only serializes installs within a single prompt, and there
/// can be more than one: a manual "Check for Updates…" opens a prompt
/// unconditionally. Two installs would race two downloads over the same staging
/// path, two `hdiutil attach` calls on the same mountpoint, and two rsyncs into
/// the live bundle — each one's unmount tearing down the other's mount mid-copy.
#[derive(Default)]
struct Installing(bool);
impl gpui::Global for Installing {}

/// Whether an update is installing right now. Worth checking before offering a
/// "Check for Updates…" menu item that would open a second prompt.
pub fn is_installing(cx: &App) -> bool {
  cx.try_global::<Installing>().is_some_and(|i| i.0)
}

fn set_installing(active: bool, cx: &mut App) {
  cx.set_global(Installing(active));
}

/// How far along the bar each stage sits, as the percentage [`Progress`] wants
/// (0..=100, *not* a 0..1 fraction). The download dominates the wall clock, so it
/// owns most of the bar and the later stages are checkpoints past it.
fn percent(stage: &UpdateStage) -> f32 {
  match stage {
    UpdateStage::Downloading { done, total } if *total > 0 => {
      85.0 * (*done as f32 / *total as f32).clamp(0.0, 1.0)
    }
    // Total unknown: hold at the start rather than pretending to advance.
    UpdateStage::Downloading { .. } => 0.0,
    UpdateStage::Preparing => 88.0,
    UpdateStage::Installing => 94.0,
    UpdateStage::Verifying => 98.0,
  }
}

/// Whether an install is in flight in this prompt, and so whether the action
/// button and Escape are inert. Free of the view so it can be tested directly.
fn busy(phase: &Phase) -> bool {
  matches!(phase, Phase::Working(_))
}

/// The action button's label for a given phase. `installable` is false when this
/// install can't be rewritten, or when the release hasn't published the asset to
/// do it with — either way the button must not promise an install.
fn action_label(phase: &Phase, installable: bool) -> &'static str {
  match phase {
    Phase::Working(_) => "Updating…",
    Phase::Failed(_) => "Try Again",
    Phase::Idle if installable => "Update & Restart",
    Phase::Idle => "Open Download",
  }
}

/// `Downloading` renders its byte counts; the rest are just their label.
fn detail(stage: &UpdateStage) -> String {
  match stage {
    UpdateStage::Downloading { done, total } if *total > 0 => {
      format!("{} of {}", megabytes(*done), megabytes(*total))
    }
    _ => String::new(),
  }
}

/// Decimal megabytes, matching what a release page and the OS file browser report
/// for the same file. Dividing by 1 MiB instead would render an 87.4 MB download
/// as "83.3 MB" and read as a stalled or wrong transfer.
fn megabytes(bytes: u64) -> String {
  format!("{:.1} MB", bytes as f64 / 1_000_000.0)
}

/// The update prompt: what is available, what will happen, and one action.
pub struct UpdatePrompt {
  updater: Updater,
  release: Release,
  kind: InstallKind,
  /// Whether this release can be rewritten into this install — see
  /// [`super::UpdateConfig::can_install`]. Resolved once at construction so the
  /// button's promise can't drift from what the installer will do.
  installable: bool,
  phase: Phase,
  auto_restart: bool,
  window_root: bool,
  focus: FocusHandle,
}

impl UpdatePrompt {
  /// A prompt offering `release`.
  pub fn new(updater: Updater, release: Release, cx: &mut Context<Self>) -> Self {
    let kind = updater.config().install_kind();
    let installable = updater.config().can_install(&release, &kind);
    UpdatePrompt {
      updater,
      release,
      kind,
      installable,
      phase: Phase::Idle,
      auto_restart: true,
      window_root: false,
      focus: cx.focus_handle(),
    }
  }

  /// Whether a successful install restarts the app itself (default `true`).
  /// Turn it off to handle [`UpdatePromptEvent::Installed`] yourself — the
  /// prompt then stays in its installing state, since only the host knows what
  /// comes next.
  pub fn auto_restart(mut self, auto_restart: bool) -> Self {
    self.auto_restart = auto_restart;
    self
  }

  /// Whether this prompt is the root view of its own update window: draws the
  /// titlebar drag strip and pads for a transparent titlebar. [`super::open`]
  /// sets it; leave it off when embedding the prompt in a window of your own.
  pub fn window_root(mut self, window_root: bool) -> Self {
    self.window_root = window_root;
    self
  }

  /// The release being offered.
  pub fn release(&self) -> &Release {
    &self.release
  }

  /// Whether an install is in flight in this prompt.
  pub fn busy(&self) -> bool {
    busy(&self.phase)
  }

  /// The stage the installer is on, if one is running.
  pub fn stage(&self) -> Option<&UpdateStage> {
    match &self.phase {
      Phase::Working(stage) => Some(stage),
      _ => None,
    }
  }

  /// Why the last attempt failed, if it did.
  pub fn error(&self) -> Option<&str> {
    match &self.phase {
      Phase::Failed(reason) => Some(reason),
      _ => None,
    }
  }

  /// Take the action the button offers: install in place and restart, or — when
  /// this install can't be rewritten — open the release page and dismiss.
  pub fn accept(&mut self, cx: &mut Context<Self>) {
    // `busy` covers this prompt; the global covers a second prompt whose
    // install is already running.
    if self.busy() || is_installing(cx) {
      return;
    }
    if self.installable {
      self.phase = Phase::Working(UpdateStage::Downloading { done: 0, total: 0 });
      set_installing(true, cx);
      cx.emit(UpdatePromptEvent::Started);
      cx.notify();
      self.install(cx);
    } else {
      cx.open_url(&self.release.url);
      cx.emit(UpdatePromptEvent::Dismissed);
    }
  }

  /// Give up on the prompt (the "Later" button and Escape). Inert while an
  /// install is running, so the progress can't be dismissed out from under
  /// itself.
  pub fn dismiss(&mut self, cx: &mut Context<Self>) {
    if self.busy() {
      return;
    }
    cx.emit(UpdatePromptEvent::Dismissed);
  }

  /// Show a stage without running the installer — for a host driving its own
  /// install, and for previewing the states.
  pub fn set_stage(&mut self, stage: UpdateStage, cx: &mut Context<Self>) {
    self.phase = Phase::Working(stage);
    cx.notify();
  }

  /// Show a failure without running the installer. The action button becomes
  /// "Try Again".
  pub fn set_failed(&mut self, reason: impl Into<String>, cx: &mut Context<Self>) {
    self.phase = Phase::Failed(reason.into());
    cx.notify();
  }

  /// Return to the offer, whatever state the prompt was in.
  pub fn reset(&mut self, cx: &mut Context<Self>) {
    self.phase = Phase::Idle;
    cx.notify();
  }

  /// Download and install off the UI thread, then relaunch into it.
  ///
  /// The installer reports its stages from a background thread, and gpui's
  /// background executor only carries `Send` work — so progress crosses back
  /// through a shared cell that a foreground task drains into the view.
  /// Without that, the window shows nothing at all until the whole install
  /// resolves.
  fn install(&mut self, cx: &mut Context<Self>) {
    self.updater.notify(
      self.updater.app(),
      &format!(
        "Downloading {} {}…",
        self.updater.app(),
        self.release.version
      ),
    );
    let updater = self.updater.clone();
    let config = self.updater.config().clone();
    let release = self.release.clone();
    let kind = self.kind.clone();
    let executor = cx.background_executor().clone();
    let latest: Arc<Mutex<Option<UpdateStage>>> = Arc::new(Mutex::new(None));
    // The installer finishing is its own signal. Watching `busy()` alone
    // would leave this loop ticking forever on the one path that finishes
    // without changing the phase: a success under `auto_restart(false)`,
    // where the host — not the prompt — decides what happens next.
    let finished = Arc::new(AtomicBool::new(false));

    let drained = latest.clone();
    let done = finished.clone();
    let ticker = executor.clone();
    cx.spawn(async move |this, cx| loop {
      let stage = drained.lock().ok().and_then(|mut slot| slot.take());
      // Stop as soon as the prompt is no longer installing (or is gone):
      // the install task owns the terminal states.
      let running = this.update(cx, |view, cx| {
        let running = view.busy();
        if running {
          if let Some(stage) = stage {
            view.phase = Phase::Working(stage.clone());
            cx.emit(UpdatePromptEvent::Stage(stage));
            cx.notify();
          }
        }
        running
      });
      // Checked after the drain, so the last stage reported before the
      // installer returned still reaches the view.
      if !matches!(running, Ok(true)) || done.load(Ordering::Relaxed) {
        break;
      }
      ticker.timer(PROGRESS_TICK).await;
    })
    .detach();

    let reported = latest.clone();
    cx.spawn(async move |this, cx| {
      let staged = executor
        .spawn(async move {
          config.install(&release, &kind, &|stage| {
            if let Ok(mut slot) = reported.lock() {
              *slot = Some(stage);
            }
          })
        })
        .await;
      finished.store(true, Ordering::Relaxed);
      match staged {
        Ok(relaunch) => {
          // A prompt in its own window disables its buttons while
          // installing, but the titlebar's close control stays live.
          // Closing it withdraws consent to be restarted, so leave the
          // new version on disk for the next launch instead. A dead
          // entity is how that close reaches us here.
          let dismissed = this.update(cx, |_, _| ()).is_err();
          cx.update(|cx| set_installing(false, cx));
          if dismissed {
            updater.notify(
              "Update installed",
              &format!(
                "{} will finish updating the next time you open it.",
                updater.app()
              ),
            );
            return;
          }
          let restart = this
            .update(cx, |view, cx| {
              cx.emit(UpdatePromptEvent::Installed(relaunch.clone()));
              view.auto_restart
            })
            .unwrap_or(false);
          if restart {
            cx.update(|cx| {
              updater.run_before_restart(cx);
              // `Relaunch::Current` restarts with no explicit path
              // on purpose: gpui reopens the running bundle via
              // NSBundle. Handing `open` an explicit path right
              // after an in-place install is what relaunches the
              // bare Mach-O in a terminal.
              if let Relaunch::Binary(path) = relaunch {
                cx.set_restart_path(path);
              }
              cx.restart();
            });
          }
        }
        Err(e) => {
          cx.update(|cx| set_installing(false, cx));
          updater.notify("Update failed", &e);
          // Show the reason in the prompt and let the user retry it. A
          // failure that lands only in a notification leaves the prompt
          // looking like the click did nothing.
          let _ = this.update(cx, |view, cx| {
            view.phase = Phase::Failed(e.clone());
            cx.emit(UpdatePromptEvent::Failed(e));
            cx.notify();
          });
        }
      }
    })
    .detach();
  }

  fn key_down(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
    if event.keystroke.key == "escape" {
      self.dismiss(cx);
    }
  }

  /// The status area: a live progress bar while installing, the reason when it
  /// failed, and what the button will do when idle.
  fn status(&self, dim: gpui::Hsla, small: f32, gap: f32) -> gpui::AnyElement {
    match &self.phase {
      Phase::Working(stage) => div()
        .flex()
        .flex_col()
        .gap(px(gap))
        .child(Progress::new(percent(stage)).size(Size::Sm))
        .child(
          div()
            .flex()
            .justify_between()
            .text_size(px(small))
            .text_color(dim)
            .child(SharedString::from(stage.label()))
            .child(SharedString::from(detail(stage))),
        )
        .into_any_element(),
      Phase::Failed(reason) => Alert::new(SharedString::from(reason.clone()))
        .title("Update failed")
        .variant(Variant::Light)
        .color(ColorName::Red)
        .into_any_element(),
      Phase::Idle => div()
        .text_size(px(small))
        .child(SharedString::from(if self.installable {
          format!(
            "{} will download the update, install it, and restart.",
            self.updater.app()
          )
        } else {
          "Open the download page to update.".to_string()
        }))
        .into_any_element(),
    }
  }
}

impl Focusable for UpdatePrompt {
  fn focus_handle(&self, _cx: &App) -> FocusHandle {
    self.focus.clone()
  }
}

impl EventEmitter<UpdatePromptEvent> for UpdatePrompt {}

impl Render for UpdatePrompt {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let t = theme(cx);
    let bg = t.body().hsla();
    let text = t.text().hsla();
    let dim = t.dimmed().hsla();
    let pad = t.spacing(Size::Lg);
    let gap = t.spacing(Size::Xs);
    let headline = t.font_size(Size::Md);
    let body = t.font_size(Size::Sm);
    let small = t.font_size(Size::Xs);
    // Reserve the status area so the panel doesn't jump between states; the
    // progress bar over its byte counts is the tallest arrangement.
    let status = t.spacing(Size::Xl) + small * 2.0;
    let busy = self.busy();
    let label = action_label(&self.phase, self.installable);
    let title = format!(
      "{} {} is available",
      self.updater.app(),
      self.release.version
    );
    let have = format!("You have {}.", self.updater.version());
    let notes = self.release.url.clone();

    div()
      .size_full()
      .flex()
      .flex_col()
      .track_focus(&self.focus)
      .on_key_down(cx.listener(Self::key_down))
      .bg(bg)
      .text_color(text)
      .pt(px(if self.window_root { TITLEBAR } else { pad }))
      .px(px(pad))
      .pb(px(pad))
      .gap(px(gap))
      .when(self.window_root, |this| this.child(drag_strip()))
      .child(
        div()
          .text_size(px(headline))
          .font_weight(FontWeight::BOLD)
          .child(SharedString::from(title)),
      )
      .child(
        div()
          .text_size(px(small))
          .text_color(dim)
          .child(SharedString::from(have)),
      )
      .child(
        div()
          .min_h(px(status))
          .text_size(px(body))
          .child(self.status(dim, small, gap)),
      )
      .child(div().flex_1())
      .child(
        div()
          .flex()
          .items_center()
          .justify_end()
          .gap(px(gap))
          .child(
            Button::new("guise-update-notes", "Release Notes")
              .variant(Variant::Subtle)
              .disabled(busy || notes.is_empty())
              .on_click(move |_, _, cx| cx.open_url(&notes)),
          )
          .child(
            Button::new("guise-update-later", "Later")
              .variant(Variant::Default)
              .disabled(busy)
              .on_click(cx.listener(|this, _, _, cx| this.dismiss(cx))),
          )
          .child(
            Button::new("guise-update-go", label)
              .variant(Variant::Filled)
              .disabled(busy)
              .on_click(cx.listener(|this, _, _, cx| this.accept(cx))),
          ),
      )
      .probe("UpdatePrompt")
  }
}

/// The strip along the top of an update window that drags it, kept clear of the
/// macOS traffic lights.
fn drag_strip() -> impl IntoElement {
  let lead = if cfg!(target_os = "macos") { 70.0 } else { 0.0 };
  div()
    .absolute()
    .top_0()
    .left(px(lead))
    .right_0()
    .h(px(TITLEBAR - 6.0))
    .window_control_area(WindowControlArea::Drag)
    .on_mouse_down(MouseButton::Left, |_, window, _| window.start_window_move())
}

#[cfg(test)]
mod tests {
  use super::*;

  /// [`Progress`] takes a percentage, not a 0..1 fraction — feeding it a
  /// fraction renders a bar that never visibly leaves the left edge.
  #[test]
  fn download_progress_is_a_percentage_across_most_of_the_bar() {
    assert_eq!(
      percent(&UpdateStage::Downloading {
        done: 0,
        total: 100
      }),
      0.0
    );
    assert!(
      (percent(&UpdateStage::Downloading {
        done: 50,
        total: 100
      }) - 42.5)
        .abs()
        < 0.01
    );
    assert!(
      (percent(&UpdateStage::Downloading {
        done: 100,
        total: 100
      }) - 85.0)
        .abs()
        < 0.01
    );
  }

  #[test]
  fn every_stage_stays_in_percentage_range() {
    for stage in [
      UpdateStage::Downloading { done: 1, total: 2 },
      UpdateStage::Preparing,
      UpdateStage::Installing,
      UpdateStage::Verifying,
    ] {
      let value = percent(&stage);
      assert!((0.0..=100.0).contains(&value), "{stage:?} -> {value}");
    }
  }

  #[test]
  fn stages_after_the_download_only_move_forward() {
    let done = percent(&UpdateStage::Downloading {
      done: 100,
      total: 100,
    });
    assert!(done < percent(&UpdateStage::Preparing));
    assert!(percent(&UpdateStage::Preparing) < percent(&UpdateStage::Installing));
    assert!(percent(&UpdateStage::Installing) < percent(&UpdateStage::Verifying));
  }

  #[test]
  fn an_unknown_total_holds_the_bar_at_zero() {
    // Better a bar that hasn't moved than one inventing progress it can't know.
    assert_eq!(
      percent(&UpdateStage::Downloading {
        done: 900,
        total: 0
      }),
      0.0
    );
  }

  #[test]
  fn overlong_downloads_cannot_overflow_the_bar() {
    assert!(
      percent(&UpdateStage::Downloading {
        done: 500,
        total: 100
      }) <= 85.0
    );
  }

  #[test]
  fn only_the_download_reports_byte_counts() {
    assert_eq!(
      detail(&UpdateStage::Downloading {
        done: 5_000_000,
        total: 20_000_000
      }),
      "5.0 MB of 20.0 MB"
    );
    assert_eq!(detail(&UpdateStage::Downloading { done: 5, total: 0 }), "");
    assert_eq!(detail(&UpdateStage::Preparing), "");
    assert_eq!(detail(&UpdateStage::Verifying), "");
  }

  #[test]
  fn sizes_are_decimal_mb_to_match_what_release_pages_report() {
    // A real 20,314,688-byte dmg: a release page and the OS file browser both
    // call this 20.3 MB. Dividing by 1 MiB would render "19.4 MB" for the same
    // file and read as a stalled or mismatched download.
    assert_eq!(
      detail(&UpdateStage::Downloading {
        done: 0,
        total: 20_314_688
      }),
      "0.0 MB of 20.3 MB"
    );
  }

  #[test]
  fn only_the_working_phase_is_busy() {
    // What stops the action button from starting a second install over the
    // first: every other phase accepts the click, Working never does. Calls
    // the real predicate — asserting on `Phase` literals instead would pass
    // even with `busy` inverted.
    assert!(busy(&Phase::Working(UpdateStage::Preparing)));
    assert!(!busy(&Phase::Idle));
    assert!(!busy(&Phase::Failed("boom".into())));
  }

  #[test]
  fn the_action_label_tracks_phase_and_installability() {
    assert_eq!(action_label(&Phase::Idle, true), "Update & Restart");
    // No installable asset: the button must not promise an install it can't do.
    assert_eq!(action_label(&Phase::Idle, false), "Open Download");
    assert_eq!(
      action_label(&Phase::Working(UpdateStage::Installing), true),
      "Updating…"
    );
    // A failure has to stay retryable rather than dead-ending the prompt.
    assert_eq!(
      action_label(&Phase::Failed("boom".into()), true),
      "Try Again"
    );
  }
}
