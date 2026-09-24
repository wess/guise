//! The app-level plumbing: the launch-and-hourly poller, the manual "Check for
//! Updates…" entry point, and the standalone update windows they open.
//!
//! Everything here is optional sugar over the components. An app that wants the
//! update UI inside a window it already owns can render
//! [`UpdatePrompt`](super::UpdatePrompt) directly and drive it from its own
//! [`UpdateConfig::check`](super::UpdateConfig::check).

use gpui::{
  point, px, size, App, AppContext as _, Bounds, SharedString, TitlebarOptions, WindowBounds,
  WindowHandle, WindowOptions,
};

use super::{
  is_installing, Release, UpdateCheck, UpdateNotice, UpdateNoticeEvent, UpdateOutcome,
  UpdatePrompt, UpdatePromptEvent, Updater,
};

/// The update windows are fixed-size: the prompt's copy is short and known, and a
/// resizable window whose content can't use the room reads as an oversight.
const WIDTH: f32 = 460.0;
const PROMPT_HEIGHT: f32 = 300.0;
/// The notice has no progress area or release notes, so it is shorter.
const NOTICE_HEIGHT: f32 = 180.0;

/// Marker so the background poller is started at most once.
struct Started;
impl gpui::Global for Started {}

/// The version the prompt was last opened for, so re-checks don't reopen a window
/// the user already dismissed.
#[derive(Default)]
struct Notified(String);
impl gpui::Global for Notified {}

/// Start the launch + interval update check (once). On finding a newer release it
/// opens the prompt once per version. Call it behind whatever "check
/// automatically" preference the app offers.
pub fn start(updater: Updater, cx: &mut App) {
  if cx.try_global::<Started>().is_some() {
    return;
  }
  cx.set_global(Started);
  let every = updater.poll();
  let executor = cx.background_executor().clone();
  cx.spawn(async move |cx| loop {
    let config = updater.config().clone();
    let found = executor.spawn(async move { config.check() }).await;
    cx.update(|cx| apply(&updater, found, false, cx));
    executor.timer(every).await;
  })
  .detach();
}

/// Run a check now — the "Check for Updates…" menu item. Opens the prompt if
/// there is an update, and a short notice saying why not if there isn't.
pub fn check_now(updater: Updater, cx: &mut App) {
  let executor = cx.background_executor().clone();
  cx.spawn(async move |cx| {
    let config = updater.config().clone();
    let found = executor.spawn(async move { config.check() }).await;
    cx.update(|cx| apply(&updater, found, true, cx));
  })
  .detach();
}

/// Apply a check result: open the prompt for an installable release (once per
/// version, or always when the user asked), and otherwise say why not.
fn apply(updater: &Updater, found: Result<UpdateCheck, String>, manual: bool, cx: &mut App) {
  match found {
    // Never stack a second prompt on top of a running install: the new
    // prompt's action button would start a concurrent one.
    Ok(UpdateCheck::Ready(_)) if is_installing(cx) => {}
    Ok(UpdateCheck::Ready(release)) => {
      let seen = cx
        .try_global::<Notified>()
        .is_some_and(|n| n.0 == release.version);
      if manual || !seen {
        cx.set_global(Notified(release.version.clone()));
        open(updater.clone(), release, cx);
      }
    }
    // A newer version exists but its build hasn't finished uploading. Say so
    // rather than claiming you're up to date; the next check picks it up.
    Ok(UpdateCheck::Pending(version)) => {
      if manual {
        open_notice(updater.clone(), UpdateOutcome::Pending(version), cx);
      }
    }
    Ok(UpdateCheck::UpToDate) => {
      if manual {
        open_notice(updater.clone(), UpdateOutcome::UpToDate, cx);
      }
    }
    Err(e) => {
      if manual {
        open_notice(updater.clone(), UpdateOutcome::Failed(e), cx);
      }
    }
  }
}

/// Open the update prompt in its own window, centered on the primary display.
/// The window closes itself when the prompt is dismissed.
pub fn open(
  updater: Updater,
  release: Release,
  cx: &mut App,
) -> Option<WindowHandle<UpdatePrompt>> {
  let title = updater.title().clone();
  let prompt = cx.new(|cx| UpdatePrompt::new(updater, release, cx).window_root(true));
  let handle = window(WIDTH, PROMPT_HEIGHT, title, prompt.clone(), cx)?;
  // The component emits rather than closing a window it may not own, so the
  // opener is what turns "Dismissed" into a closed window.
  cx.subscribe(&prompt, move |_prompt, event: &UpdatePromptEvent, cx| {
    if matches!(event, UpdatePromptEvent::Dismissed) {
      let _ = handle.update(cx, |_view, window, _cx| window.remove_window());
    }
  })
  .detach();
  Some(handle)
}

/// Open the short answer to a check that had nothing to install, in its own
/// window. The window closes itself when the notice is acknowledged.
pub fn open_notice(
  updater: Updater,
  outcome: UpdateOutcome,
  cx: &mut App,
) -> Option<WindowHandle<UpdateNotice>> {
  let title = updater.title().clone();
  let notice = cx.new(|cx| UpdateNotice::new(updater, outcome, cx).window_root(true));
  let handle = window(WIDTH, NOTICE_HEIGHT, title, notice.clone(), cx)?;
  cx.subscribe(&notice, move |_notice, event: &UpdateNoticeEvent, cx| {
    let UpdateNoticeEvent::Dismissed = event;
    let _ = handle.update(cx, |_view, window, _cx| window.remove_window());
  })
  .detach();
  Some(handle)
}

/// A centered, non-resizable window with a transparent titlebar, rooted at
/// `view`. Both update windows are the same shape.
fn window<V: gpui::Render>(
  width: f32,
  height: f32,
  title: SharedString,
  view: gpui::Entity<V>,
  cx: &mut App,
) -> Option<WindowHandle<V>> {
  let bounds = Bounds::centered(None, size(px(width), px(height)), cx);
  let handle = cx
    .open_window(
      WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        is_resizable: false,
        titlebar: Some(TitlebarOptions {
          title: Some(title.clone()),
          appears_transparent: true,
          traffic_light_position: Some(point(px(12.0), px(12.0))),
        }),
        ..Default::default()
      },
      |window, _cx| {
        window.set_window_title(&title);
        view
      },
    )
    .ok()?;
  handle
    .update(cx, |_view, window, _cx| window.activate_window())
    .ok();
  Some(handle)
}
