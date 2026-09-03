//! Entity-level tests on gpui's test harness (`#[gpui::test]` +
//! `TestAppContext` — the same rig zed's own tests use). These cover the
//! wiring pure unit tests can't reach: signals, bindings, form observers,
//! entity events, and the theme global.
//!
//! Observer effects flush between `cx.update` blocks, so assertions that
//! depend on an observer firing sit in their own block.

use gpui::prelude::*;
use gpui::{div, Context, Entity, Modifiers, MouseButton, TestAppContext, Window};

use crate::ai::{AIChatView, AIComposer, AIComposerEvent, AITurn};
use crate::anim::{Animator, AnimatorEvent, Easing, Motion, Prop, Stagger};
use crate::devtools::{
  DevTools, DevToolsEvent, DevToolsState, DevToolsTab, LogLevel, NetworkRecord, Probed,
  RequestState, SourceRef, StorageDomain, StorageEntry,
};
use crate::input::{Date, DatePicker, LineEditor as _, Select, TextInput};
use crate::reactive::{validators, Form, Signal};
use crate::settings::{SettingsView, SettingsViewEvent};
use crate::theme::{theme, Color, ColorScheme, Theme, ThemeChoice, ThemeEntry, ThemeManager};
use crate::update::{
  is_installing, Release, UpdateNotice, UpdateNoticeEvent, UpdateOutcome, UpdatePrompt,
  UpdatePromptEvent, UpdateStage, Updater,
};
use crate::{ActionIcon, Button, Carousel, CarouselEvent, IconName, Text, Title, TransitionKind};

#[gpui::test]
fn signal_binding_and_lens_round_trip(cx: &mut TestAppContext) {
  let count = cx.update(|cx| Signal::new(cx, 5_i32));
  let binding = count.binding();
  cx.update(|cx| {
    assert_eq!(binding.get(cx), 5);
    binding.set(cx, 9);
    assert_eq!(count.get(cx), 9);
  });

  #[derive(Clone, PartialEq)]
  struct Settings {
    muted: bool,
  }
  let settings = cx.update(|cx| Signal::new(cx, Settings { muted: false }));
  let muted = settings.lens(|s| s.muted, |s, v| s.muted = v);
  cx.update(|cx| {
    muted.set(cx, true);
    assert!(settings.read(cx).muted);
    // Mapped bindings convert both ways.
    let as_text = muted.map(|b| b.to_string(), |s: String| s == "true");
    assert_eq!(as_text.get(cx), "true");
    as_text.set(cx, "false".to_string());
    assert!(!settings.read(cx).muted);
  });
}

#[gpui::test]
fn form_validates_and_revalidates_live(cx: &mut TestAppContext) {
  let form = cx.update(|cx| {
    Form::new(cx)
      .field(cx, "email", "")
      .rule("email", validators::required())
      .rule("email", validators::email())
      .field(cx, "confirm", "")
      .rule_form("confirm", validators::equals_field("email", "Must match"))
  });

  cx.update(|cx| {
    assert!(!form.validate(cx));
    assert!(form.error(cx, "email").is_some());
    assert!(!form.is_valid(cx));
  });

  // Fixing the field re-validates it live (it carried an error) — the
  // observer fires on the effect flush between these blocks.
  cx.update(|cx| form.set(cx, "email", "a@b.com"));
  cx.update(|cx| {
    assert_eq!(form.error(cx, "email"), None);
    assert!(form.touched("email"));
  });

  cx.update(|cx| form.set(cx, "confirm", "a@b.com"));
  cx.update(|cx| {
    let values = form.submit(cx).expect("form should validate");
    assert_eq!(values["email"], "a@b.com");
  });

  // Cross-field: change email, confirm no longer matches.
  cx.update(|cx| form.set(cx, "email", "other@b.com"));
  cx.update(|cx| assert!(!form.validate(cx)));
}

#[gpui::test]
fn select_bind_follows_the_signal_both_ways(cx: &mut TestAppContext) {
  let choice = cx.update(|cx| Signal::new(cx, 2_usize));
  let select = cx.update(|cx| cx.new(|cx| Select::new(cx).data(["a", "b", "c"])));
  cx.update(|cx| Select::bind(&select, &choice, cx));

  // The signal is the source of truth: the picker adopts it immediately…
  cx.update(|cx| assert_eq!(select.read(cx).selected_index(), Some(2)));

  // …and follows later writes.
  cx.update(|cx| choice.set(cx, 0));
  cx.update(|cx| assert_eq!(select.read(cx).selected_index(), Some(0)));
}

#[gpui::test]
fn datepicker_bind_adopts_signal_writes(cx: &mut TestAppContext) {
  let date = Date::new(2026, 7, 14).unwrap();
  let picked = cx.update(|cx| Signal::new(cx, None::<Date>));
  let picker = cx.update(|cx| cx.new(DatePicker::new));
  cx.update(|cx| DatePicker::bind(&picker, &picked, cx));

  cx.update(|cx| assert_eq!(picker.read(cx).selected_date(), None));
  cx.update(|cx| picked.set(cx, Some(date)));
  cx.update(|cx| assert_eq!(picker.read(cx).selected_date(), Some(date)));
}

#[gpui::test]
fn carousel_navigates_and_emits(cx: &mut TestAppContext) {
  use std::cell::RefCell;
  use std::rc::Rc;

  let deck = cx.update(|cx| {
    cx.new(|cx| {
      Carousel::new(cx)
        .slide(|_, _| gpui::Empty)
        .slide(|_, _| gpui::Empty)
        .slide(|_, _| gpui::Empty)
    })
  });
  let seen: Rc<RefCell<Vec<usize>>> = Rc::default();
  let log = seen.clone();
  cx.update(|cx| {
    cx.subscribe(&deck, move |_deck, event: &CarouselEvent, _cx| {
      log.borrow_mut().push(event.0);
    })
    .detach();
  });

  deck.update(cx, |deck, cx| {
    deck.next(cx);
    deck.next(cx);
    deck.next(cx); // wraps to 0
    deck.prev(cx); // wraps back to 2
    deck.go_to(1, cx);
    deck.go_to(1, cx); // no-op, no event
  });
  assert_eq!(*seen.borrow(), vec![1, 2, 0, 2, 1]);
  cx.update(|cx| assert_eq!(deck.read(cx).current(), 1));
}

/// The name of each event, so a subscription can be asserted on without
/// `UpdatePromptEvent` having to be `PartialEq`.
fn prompt_event_name(event: &UpdatePromptEvent) -> &'static str {
  match event {
    UpdatePromptEvent::Started => "started",
    UpdatePromptEvent::Stage(_) => "stage",
    UpdatePromptEvent::Installed(_) => "installed",
    UpdatePromptEvent::Failed(_) => "failed",
    UpdatePromptEvent::Dismissed => "dismissed",
  }
}

fn offered_release() -> Release {
  Release {
    version: "2.0.0".to_string(),
    url: "https://example.com/releases/2.0.0".to_string(),
    assets: Vec::new(),
  }
}

/// A test binary is neither an installed `.app` nor an AppImage, so `detect()`
/// reports `Unknown` — the case where the prompt must not promise an install it
/// can't perform. Accepting has to open the release page and stand down instead
/// of starting one.
#[gpui::test]
fn update_prompt_offers_the_page_when_it_cannot_install_in_place(cx: &mut TestAppContext) {
  use std::cell::RefCell;
  use std::rc::Rc;

  let updater = Updater::github("Acme", "1.0.0", "acme/acme");
  let prompt = cx.update(|cx| cx.new(|cx| UpdatePrompt::new(updater, offered_release(), cx)));
  let seen: Rc<RefCell<Vec<&'static str>>> = Rc::default();
  let log = seen.clone();
  cx.update(|cx| {
    cx.subscribe(&prompt, move |_prompt, event: &UpdatePromptEvent, _cx| {
      log.borrow_mut().push(prompt_event_name(event));
    })
    .detach();
  });

  prompt.update(cx, |prompt, cx| prompt.accept(cx));
  assert_eq!(*seen.borrow(), vec!["dismissed"]);
  cx.update(|cx| {
    assert!(!prompt.read(cx).busy());
    // Nothing was started, so no other prompt is locked out.
    assert!(!is_installing(cx));
  });
}

/// The states a host driving its own install moves the prompt through, and the
/// guarantee that a running install can't be dismissed out from under itself.
#[gpui::test]
fn update_prompt_tracks_the_stages_a_host_drives(cx: &mut TestAppContext) {
  use std::cell::RefCell;
  use std::rc::Rc;

  let updater = Updater::github("Acme", "1.0.0", "acme/acme");
  let prompt = cx.update(|cx| cx.new(|cx| UpdatePrompt::new(updater, offered_release(), cx)));
  let seen: Rc<RefCell<Vec<&'static str>>> = Rc::default();
  let log = seen.clone();
  cx.update(|cx| {
    cx.subscribe(&prompt, move |_prompt, event: &UpdatePromptEvent, _cx| {
      log.borrow_mut().push(prompt_event_name(event));
    })
    .detach();
  });

  prompt.update(cx, |prompt, cx| {
    prompt.set_stage(UpdateStage::Preparing, cx);
    // Inert while installing: the progress must not be closable.
    prompt.dismiss(cx);
  });
  cx.update(|cx| {
    let prompt = prompt.read(cx);
    assert!(prompt.busy());
    assert_eq!(prompt.stage(), Some(&UpdateStage::Preparing));
    assert_eq!(prompt.error(), None);
  });
  assert!(seen.borrow().is_empty());

  prompt.update(cx, |prompt, cx| prompt.set_failed("no disk space", cx));
  cx.update(|cx| {
    let prompt = prompt.read(cx);
    assert!(!prompt.busy());
    assert_eq!(prompt.error(), Some("no disk space"));
  });

  // A failure is retryable, and dismissable again.
  prompt.update(cx, |prompt, cx| {
    prompt.reset(cx);
    prompt.dismiss(cx);
  });
  cx.update(|cx| assert_eq!(prompt.read(cx).error(), None));
  assert_eq!(*seen.borrow(), vec!["dismissed"]);
}

#[gpui::test]
fn update_notice_answers_a_check_and_dismisses(cx: &mut TestAppContext) {
  use std::cell::RefCell;
  use std::rc::Rc;

  let updater = Updater::github("Acme", "1.31.0", "acme/acme");
  let notice = cx.update(|cx| {
    cx.new(|cx| UpdateNotice::new(updater, UpdateOutcome::Pending("1.32.0".into()), cx))
  });
  let dismissed: Rc<RefCell<usize>> = Rc::default();
  let count = dismissed.clone();
  cx.update(|cx| {
    cx.subscribe(&notice, move |_notice, event: &UpdateNoticeEvent, _cx| {
      let UpdateNoticeEvent::Dismissed = event;
      *count.borrow_mut() += 1;
    })
    .detach();
  });

  cx.update(|cx| {
    assert_eq!(
      notice.read(cx).outcome(),
      &UpdateOutcome::Pending("1.32.0".to_string())
    );
  });
  notice.update(cx, |notice, cx| notice.dismiss(cx));
  assert_eq!(*dismissed.borrow(), 1);
}

#[gpui::test]
fn theme_presets_install_and_resolve(cx: &mut TestAppContext) {
  cx.update(|cx| {
    Theme::catppuccin().init(cx);
    let t = theme(cx);
    assert!(t.scheme.is_dark());
    assert_eq!(t.primary(), Color::hex("#89b4fa"));
    assert_eq!(t.body(), Color::hex("#1e1e2e"));

    // Swapping the global restyles everything that reads theme(cx).
    Theme::solarized_light().init(cx);
    let t = theme(cx);
    assert!(!t.scheme.is_dark());
    assert_eq!(t.primary(), Color::hex("#268bd2"));
  });
}

#[gpui::test]
fn theme_manager_installs_selects_and_follows_the_system(cx: &mut TestAppContext) {
  cx.update(|cx| {
    ThemeManager::new().with_presets().install(cx);
    // Following the system, which starts light until a window says otherwise.
    assert!(!theme(cx).scheme.is_dark());

    ThemeManager::set_system_scheme(cx, ColorScheme::Dark);
    assert!(theme(cx).scheme.is_dark());
    assert_eq!(cx.global::<ThemeManager>().resolved_id().as_ref(), "dark");

    // An explicit pick stops following.
    assert!(ThemeManager::select(cx, "dracula"));
    assert_eq!(theme(cx).primary(), Color::hex("#bd93f9"));
    ThemeManager::set_system_scheme(cx, ColorScheme::Light);
    assert_eq!(theme(cx).primary(), Color::hex("#bd93f9"));

    // An id nobody registered leaves the choice alone.
    assert!(!ThemeManager::select(cx, "nope"));
    assert_eq!(
      cx.global::<ThemeManager>().selection(),
      &ThemeChoice::Fixed("dracula".into())
    );

    ThemeManager::follow_system(cx);
    assert!(!theme(cx).scheme.is_dark());
  });
}

#[gpui::test]
fn theme_manager_toggle_swaps_the_pair_and_leaves_system(cx: &mut TestAppContext) {
  cx.update(|cx| {
    ThemeManager::new()
      .with(ThemeEntry::new("night", Theme::nord()))
      .pair("light", "night")
      .install(cx);
    assert!(!theme(cx).scheme.is_dark());

    ThemeManager::toggle(cx);
    assert_eq!(
      cx.global::<ThemeManager>().selection(),
      &ThemeChoice::Fixed("night".into())
    );
    assert_eq!(theme(cx).primary(), Color::hex("#88c0d0"));

    ThemeManager::toggle(cx);
    assert_eq!(
      cx.global::<ThemeManager>().selection(),
      &ThemeChoice::Fixed("light".into())
    );
  });
}

#[gpui::test]
fn theme_manager_is_optional(cx: &mut TestAppContext) {
  cx.update(|cx| {
    Theme::light().init(cx);
    // Every mutator is a no-op without a manager, rather than a panic.
    assert!(!ThemeManager::select(cx, "dracula"));
    ThemeManager::toggle(cx);
    ThemeManager::follow_system(cx);
    ThemeManager::set_system_scheme(cx, ColorScheme::Dark);
    ThemeManager::apply(cx);
    assert!(crate::theme::manager(cx).is_none());
    assert!(!theme(cx).scheme.is_dark());
  });
}

struct KeyboardActions {
  field: Entity<TextInput>,
}

impl Render for KeyboardActions {
  fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
    div()
      .flex()
      .child(Button::new("keyboard-button", "Save"))
      .child(Button::new("keyboard-disabled", "Disabled").disabled(true))
      .child(ActionIcon::new("keyboard-icon", IconName::Pencil).label("Edit"))
      .child(self.field.clone())
  }
}

#[gpui::test]
fn buttons_move_through_the_tab_order(cx: &mut TestAppContext) {
  cx.update(|cx| Theme::light().init(cx));
  let (view, cx) = cx.add_window_view(|_window, cx| KeyboardActions {
    field: cx.new(TextInput::new),
  });
  cx.run_until_parked();

  cx.update(|window, _| window.focus_next());
  cx.run_until_parked();
  cx.simulate_keystrokes("tab tab");
  let field = view.read_with(cx, |view, _| view.field.clone());
  let handle = field.read_with(cx, |field, _| field.focus_handle());
  assert!(cx.update(|window, _| handle.is_focused(window)));

  cx.simulate_keystrokes("shift-tab shift-tab tab tab");
  cx.run_until_parked();
  assert!(cx.update(|window, _| handle.is_focused(window)));
}

// --- single-line fields -----------------------------------------------------
//
// These drive a real window: `simulate_input` dispatches the key event first
// and only then hands the character to the platform's input handler, exactly
// as macOS, X11, and Windows do. That is the path a text field now takes, so
// nothing here works unless the field is wired up the way the real thing is.

/// Two fields in a window — the smallest form that can show focus moving.
struct Pair {
  first: Entity<TextInput>,
  second: Entity<TextInput>,
}

impl Render for Pair {
  fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
    div()
      .flex()
      .flex_col()
      .size_full()
      .child(self.first.clone())
      .child(self.second.clone())
      .probe("Pair")
  }
}

fn pair(cx: &mut TestAppContext) -> (Entity<Pair>, &mut gpui::VisualTestContext) {
  cx.update(|cx| Theme::light().init(cx));
  cx.add_window_view(|_window, cx| Pair {
    first: cx.new(|cx| TextInput::new(cx).placeholder("first")),
    second: cx.new(|cx| TextInput::new(cx).placeholder("second")),
  })
}

/// Focus a field and let the window lay out again, so the tab-stop ring and
/// the field's shaped line are both current.
fn focus(field: &Entity<TextInput>, cx: &mut gpui::VisualTestContext) {
  let handle = field.read_with(cx, |field, _| field.focus_handle());
  cx.update(|window, _| window.focus(&handle));
  cx.run_until_parked();
}

#[gpui::test]
fn text_input_types_through_the_platform_input_handler(cx: &mut TestAppContext) {
  let (view, cx) = pair(cx);
  let field = view.read_with(cx, |view, _| view.first.clone());
  focus(&field, cx);

  cx.simulate_input("hello");
  assert_eq!(field.read_with(cx, |field, _| field.text()), "hello");

  // Backspace and the arrows still come from key handling.
  cx.simulate_keystrokes("backspace left left");
  cx.simulate_input("L");
  assert_eq!(field.read_with(cx, |field, _| field.text()), "heLll");
}

#[gpui::test]
fn text_input_tab_moves_focus_instead_of_typing(cx: &mut TestAppContext) {
  let (view, cx) = pair(cx);
  let (first, second) = view.read_with(cx, |view, _| (view.first.clone(), view.second.clone()));
  focus(&first, cx);
  cx.simulate_input("one");

  cx.simulate_keystrokes("tab");
  // The platform reports a `\t` for Tab. A field that typed it would be the
  // bug this replaced.
  assert_eq!(first.read_with(cx, |field, _| field.text()), "one");
  let second_handle = second.read_with(cx, |field, _| field.focus_handle());
  assert!(cx.update(|window, _| second_handle.is_focused(window)));

  cx.simulate_input("two");
  assert_eq!(second.read_with(cx, |field, _| field.text()), "two");

  cx.simulate_keystrokes("shift-tab");
  let first_handle = first.read_with(cx, |field, _| field.focus_handle());
  assert!(cx.update(|window, _| first_handle.is_focused(window)));
  assert_eq!(second.read_with(cx, |field, _| field.text()), "two");
}

#[gpui::test]
fn text_input_cuts_copies_and_pastes(cx: &mut TestAppContext) {
  let (view, cx) = pair(cx);
  let (first, second) = view.read_with(cx, |view, _| (view.first.clone(), view.second.clone()));
  focus(&first, cx);
  cx.simulate_input("copy me");

  cx.simulate_keystrokes("cmd-a cmd-c");
  focus(&second, cx);
  cx.simulate_keystrokes("cmd-v cmd-v");
  assert_eq!(
    second.read_with(cx, |field, _| field.text()),
    "copy mecopy me"
  );

  // Cut empties the source and leaves the clipboard usable again.
  focus(&first, cx);
  cx.simulate_keystrokes("cmd-a cmd-x");
  assert_eq!(first.read_with(cx, |field, _| field.text()), "");
  cx.simulate_keystrokes("cmd-v");
  assert_eq!(first.read_with(cx, |field, _| field.text()), "copy me");
}

#[gpui::test]
fn text_input_pasting_flattens_line_breaks(cx: &mut TestAppContext) {
  let (view, cx) = pair(cx);
  let field = view.read_with(cx, |view, _| view.first.clone());
  focus(&field, cx);
  cx.write_to_clipboard(gpui::ClipboardItem::new_string("one\ntwo\r\nthree".into()));
  cx.simulate_keystrokes("cmd-v");
  assert_eq!(
    field.read_with(cx, |field, _| field.text()),
    "one two  three"
  );
}

#[gpui::test]
fn text_input_undoes_by_word(cx: &mut TestAppContext) {
  let (view, cx) = pair(cx);
  let field = view.read_with(cx, |view, _| view.first.clone());
  focus(&field, cx);
  cx.simulate_input("alpha beta");

  cx.simulate_keystrokes("cmd-z");
  assert_eq!(field.read_with(cx, |field, _| field.text()), "alpha ");
  cx.simulate_keystrokes("cmd-z");
  assert_eq!(field.read_with(cx, |field, _| field.text()), "");
  cx.simulate_keystrokes("cmd-shift-z cmd-shift-z");
  assert_eq!(field.read_with(cx, |field, _| field.text()), "alpha beta");
}

#[gpui::test]
fn text_input_click_places_the_caret_and_drag_selects(cx: &mut TestAppContext) {
  let (view, cx) = pair(cx);
  let field = view.read_with(cx, |view, _| view.first.clone());
  focus(&field, cx);
  cx.simulate_input("hello world");

  // Ask the shaped line itself where a character sits, so the test doesn't
  // depend on the font's metrics.
  let (bounds, x_of) = field.read_with(cx, |field, _| {
    let state = field.line();
    let shaped = state.shaped.clone().expect("the field has been painted");
    let bounds = state.bounds.expect("the field has been painted");
    (bounds, move |index: usize| shaped.x_for_index(index))
  });
  let at = |index: usize| gpui::point(bounds.left() + x_of(index), bounds.center().y);

  cx.simulate_click(at(2), Modifiers::none());
  assert_eq!(field.read_with(cx, |field, _| field.edit().cursor()), 2);

  // Press at 2, drag to 7, release: "llo w" is selected.
  cx.simulate_mouse_down(at(2), MouseButton::Left, Modifiers::none());
  cx.simulate_mouse_move(at(7), MouseButton::Left, Modifiers::none());
  assert_eq!(
    field.read_with(cx, |field, _| field.edit().selected_text()),
    Some("llo w".to_string())
  );
  cx.simulate_mouse_up(at(7), MouseButton::Left, Modifiers::none());

  // Typing replaces the selection, as it would in a browser.
  cx.simulate_input("X");
  assert_eq!(field.read_with(cx, |field, _| field.text()), "heXorld");
}

#[gpui::test]
fn text_input_double_click_takes_a_word(cx: &mut TestAppContext) {
  let (view, cx) = pair(cx);
  let field = view.read_with(cx, |view, _| view.first.clone());
  focus(&field, cx);
  cx.simulate_input("hello world");

  let (bounds, x_of) = field.read_with(cx, |field, _| {
    let state = field.line();
    let shaped = state.shaped.clone().expect("the field has been painted");
    let bounds = state.bounds.expect("the field has been painted");
    (bounds, move |index: usize| shaped.x_for_index(index))
  });
  let position = gpui::point(bounds.left() + x_of(8), bounds.center().y);

  cx.simulate_event(gpui::MouseDownEvent {
    button: MouseButton::Left,
    position,
    modifiers: Modifiers::none(),
    click_count: 2,
    first_mouse: false,
  });
  assert_eq!(
    field.read_with(cx, |field, _| field.edit().selected_text()),
    Some("world".to_string())
  );

  cx.simulate_event(gpui::MouseDownEvent {
    button: MouseButton::Left,
    position,
    modifiers: Modifiers::none(),
    click_count: 3,
    first_mouse: false,
  });
  assert_eq!(
    field.read_with(cx, |field, _| field.edit().selected_text()),
    Some("hello world".to_string())
  );
}

#[gpui::test]
fn text_input_honours_max_length(cx: &mut TestAppContext) {
  cx.update(|cx| Theme::light().init(cx));
  let (field, cx) = cx.add_window_view(|_window, cx| TextInput::new(cx).max_length(4));
  focus(&field, cx);
  cx.simulate_input("abcdefg");
  assert_eq!(field.read_with(cx, |field, _| field.text()), "abcd");
  cx.write_to_clipboard(gpui::ClipboardItem::new_string("xyz".into()));
  cx.simulate_keystrokes("cmd-v");
  assert_eq!(field.read_with(cx, |field, _| field.text()), "abcd");
}

#[gpui::test]
fn text_input_read_only_selects_but_never_edits(cx: &mut TestAppContext) {
  cx.update(|cx| Theme::light().init(cx));
  let (field, cx) =
    cx.add_window_view(|_window, cx| TextInput::new(cx).read_only(true).value("locked"));
  focus(&field, cx);
  cx.simulate_input("nope");
  cx.simulate_keystrokes("backspace");
  assert_eq!(field.read_with(cx, |field, _| field.text()), "locked");
  // Read-only still selects and copies.
  cx.simulate_keystrokes("cmd-a cmd-c");
  assert_eq!(
    cx.read_from_clipboard().and_then(|item| item.text()),
    Some("locked".to_string())
  );
}

#[gpui::test]
fn text_input_never_copies_a_password(cx: &mut TestAppContext) {
  cx.update(|cx| Theme::light().init(cx));
  cx.write_to_clipboard(gpui::ClipboardItem::new_string("untouched".into()));
  let (field, cx) = cx.add_window_view(|_window, cx| TextInput::new(cx).password(true));
  focus(&field, cx);
  cx.simulate_input("hunter2");
  cx.simulate_keystrokes("cmd-a cmd-c");
  assert_eq!(
    cx.read_from_clipboard().and_then(|item| item.text()),
    Some("untouched".to_string())
  );
  cx.simulate_keystrokes("cmd-x");
  assert_eq!(field.read_with(cx, |field, _| field.text()), "hunter2");
}

// --- AI components ----------------------------------------------------------

#[gpui::test]
fn chat_view_streams_a_reply_and_closes_it(cx: &mut TestAppContext) {
  cx.update(|cx| Theme::light().init(cx));
  let (chat, cx) = cx.add_window_view(|_window, cx| AIChatView::new(cx));

  chat.update(cx, |chat, cx| {
    chat.push(AITurn::user("hello"), cx);
    chat.begin_reply(cx);
    chat.push_delta("Hi", cx);
    chat.push_delta(" there", cx);
    chat.push_reasoning("weighing it up", cx);
  });
  chat.read_with(cx, |chat, _| {
    assert_eq!(chat.turn_count(), 2);
    let reply = chat.turn(1).unwrap();
    assert_eq!(reply.body, "Hi there");
    assert_eq!(reply.reasoning.as_deref(), Some("weighing it up"));
    assert!(reply.streaming);
  });

  chat.update(cx, |chat, cx| chat.end_reply(cx));
  chat.read_with(cx, |chat, _| assert!(!chat.turn(1).unwrap().streaming));

  // A delta that lands after the turn closed — a cancelled request whose
  // last chunk was already in flight — must not reopen it.
  chat.update(cx, |chat, cx| chat.push_delta(" and more", cx));
  chat.read_with(cx, |chat, _| {
    assert_eq!(chat.turn(1).unwrap().body, "Hi there")
  });
}

#[gpui::test]
fn chat_view_keeps_partial_text_when_a_reply_fails(cx: &mut TestAppContext) {
  cx.update(|cx| Theme::light().init(cx));
  let (chat, cx) = cx.add_window_view(|_window, cx| AIChatView::new(cx));

  chat.update(cx, |chat, cx| {
    chat.begin_reply(cx);
    chat.push_delta("partial", cx);
    chat.fail_reply("connection reset", cx);
  });
  chat.read_with(cx, |chat, _| {
    let reply = chat.turn(0).unwrap();
    assert_eq!(reply.body, "partial");
    assert_eq!(reply.error.as_deref(), Some("connection reset"));
    assert!(!reply.streaming);
  });

  // Failing with nothing open is a no-op, not a panic.
  chat.update(cx, |chat, cx| chat.fail_reply("late", cx));
  chat.read_with(cx, |chat, _| {
    assert_eq!(
      chat.turn(0).unwrap().error.as_deref(),
      Some("connection reset")
    );
  });
}

#[gpui::test]
fn chat_view_edits_are_bounds_checked(cx: &mut TestAppContext) {
  cx.update(|cx| Theme::light().init(cx));
  let (chat, cx) = cx.add_window_view(|_window, cx| AIChatView::new(cx));
  // Editing a turn that isn't there must not panic.
  chat.update(cx, |chat, cx| {
    chat.update_turn(9, |turn| turn.body.push_str("nope"), cx);
    assert!(chat.turn(9).is_none());
  });
}

/// A long transcript must not re-parse every turn's markdown on every frame,
/// and skipping the ones off screen must not move anything that is on it.
#[gpui::test]
fn chat_view_skips_turns_that_are_far_off_screen(cx: &mut TestAppContext) {
  cx.update(|cx| Theme::light().init(cx));
  let body = "## Heading\n\nA paragraph with **bold** and `code` in it.\n\n                1. one\n2. two\n\nAnother paragraph to give the turn some height.";
  let turns: Vec<AITurn> = (0..60)
    .map(|i| {
      if i % 2 == 0 {
        AITurn::user(format!("Question {i}"))
      } else {
        AITurn::assistant(body)
      }
    })
    .collect();

  let (chat, cx) = cx.add_window_view(|_window, cx| AIChatView::new(cx).turns(turns.clone()));
  cx.run_until_parked();

  // The first frame has nothing measured, so everything is built.
  // By the second, only the turns near the viewport are.
  let drawn = chat.update(cx, |chat, _| chat.drawn_count());
  assert!(
    drawn < 60,
    "expected some turns to be skipped, drew {drawn}"
  );
  assert!(drawn > 0, "the visible turns must still be built");

  // Skipping them must leave the scroll extent exactly where it was: the
  // spacers carry each turn's measured height.
  let extent = chat.read_with(cx, |chat, _| chat.scroll_extent());
  cx.run_until_parked();
  let after = chat.read_with(cx, |chat, _| chat.scroll_extent());
  assert_eq!(
    extent, after,
    "content height moved when turns became spacers"
  );

  // A resize reflows every turn, so the measured heights are stale and all
  // of them have to be drawn once more to re-measure — clearing the heights
  // alone wouldn't do it, since an off-screen turn's bounds are its
  // spacer's and would be read straight back.
  cx.simulate_resize(gpui::size(gpui::px(500.0), gpui::px(700.0)));
  cx.run_until_parked();
  assert_eq!(
    chat.update(cx, |chat, _| chat.drawn_count()),
    60,
    "a resize must re-measure every turn"
  );

  // Turning it off builds every turn again.
  let (all, cx) =
    cx.add_window_view(|_window, cx| AIChatView::new(cx).turns(turns.clone()).virtualize(false));
  cx.run_until_parked();
  assert_eq!(all.update(cx, |chat, _| chat.drawn_count()), 60);
}

#[gpui::test]
fn composer_sends_on_enter_and_refuses_blank_drafts(cx: &mut TestAppContext) {
  cx.update(|cx| Theme::light().init(cx));
  let (composer, cx) = cx.add_window_view(|_window, cx| AIComposer::new(cx));

  let sent = std::rc::Rc::new(std::cell::RefCell::new(Vec::<String>::new()));
  let sink = sent.clone();
  cx.update(|_, cx| {
    cx.subscribe(&composer, move |_composer, event: &AIComposerEvent, _cx| {
      if let AIComposerEvent::Submit(text) = event {
        sink.borrow_mut().push(text.clone());
      }
    })
    .detach();
  });

  let input = composer.read_with(cx, |composer, _| composer.input().clone());
  let handle = input.read_with(cx, |input, _| input.focus_handle());
  cx.update(|window, _| window.focus(&handle));
  cx.run_until_parked();

  // Whitespace is not a prompt.
  cx.simulate_input("   ");
  cx.simulate_keystrokes("enter");
  assert!(sent.borrow().is_empty());

  cx.simulate_input("write a haiku");
  cx.simulate_keystrokes("enter");
  assert_eq!(sent.borrow().as_slice(), ["   write a haiku"]);
  // The box clears itself, so the next prompt starts empty.
  assert_eq!(composer.read_with(cx, |composer, cx| composer.text(cx)), "");

  // Shift+Enter is a newline, not a send.
  cx.simulate_input("one");
  cx.simulate_keystrokes("shift-enter");
  cx.simulate_input("two");
  assert_eq!(sent.borrow().len(), 1);
  assert_eq!(
    composer.read_with(cx, |composer, cx| composer.text(cx)),
    "one\ntwo"
  );

  // While a reply is streaming the composer will not send.
  composer.update(cx, |composer, cx| composer.set_busy(true, cx));
  cx.simulate_keystrokes("enter");
  assert_eq!(sent.borrow().len(), 1);
}

// --- devtools ---------------------------------------------------------------

/// A window holding the inspector next to something worth inspecting. The
/// recorder only runs while a `DevTools` is alive, so the two have to share a
/// frame for any of this to be observable.
struct Inspected {
  devtools: Entity<DevTools>,
}

impl Render for Inspected {
  fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
    div()
      .flex()
      .size_full()
      .child(
        crate::layout::Stack::new()
          .child(crate::Button::new("save", "Save"))
          .child(crate::Badge::new("new")),
      )
      .child(self.devtools.clone())
      .probe("Inspected")
  }
}

fn inspected(cx: &mut TestAppContext) -> (Entity<Inspected>, &mut gpui::VisualTestContext) {
  cx.update(|cx| {
    Theme::light().init(cx);
    DevToolsState::new().init(cx);
  });
  cx.add_window_view(|_window, cx| Inspected {
    devtools: cx.new(DevTools::new),
  })
}

#[gpui::test]
fn the_recorder_rebuilds_the_component_tree(cx: &mut TestAppContext) {
  let (view, cx) = inspected(cx);
  cx.run_until_parked();
  // The tree the panel reads is the one the *previous* frame recorded, so a
  // second frame has to land before anything is visible.
  view.update(cx, |_this, cx| cx.notify());
  cx.run_until_parked();

  let names: Vec<String> = view.read_with(cx, |this, cx| {
    this
      .devtools
      .read(cx)
      .tree()
      .nodes
      .iter()
      .map(|node| node.name.to_string())
      .collect()
  });

  assert!(names.contains(&"Inspected".to_string()), "{names:?}");
  assert!(names.contains(&"Stack".to_string()), "{names:?}");
  assert!(names.contains(&"Button".to_string()), "{names:?}");
  assert!(names.contains(&"Badge".to_string()), "{names:?}");
}

/// A second window on the same thread. The recorder is thread-local, so
/// without a claim its elements land in whatever tree is being built.
struct Neighbour;

impl Render for Neighbour {
  fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
    div()
      .size_full()
      .child(crate::Badge::new("elsewhere"))
      .probe("Neighbour")
  }
}

#[gpui::test]
fn the_recorder_skips_windows_the_inspector_did_not_claim(cx: &mut TestAppContext) {
  cx.update(|cx| {
    Theme::light().init(cx);
    DevToolsState::new().init(cx);
  });
  let neighbour = cx.add_window(|_window, _cx| Neighbour);
  let (view, cx) = cx.add_window_view(|_window, cx| Inspected {
    devtools: cx.new(DevTools::new),
  });
  cx.run_until_parked();
  // Both windows draw this frame; only one of them has an inspector in it.
  neighbour
    .update(cx, |_this, _window, cx| cx.notify())
    .expect("the neighbour window should still be open");
  view.update(cx, |_this, cx| cx.notify());
  cx.run_until_parked();

  let names: Vec<String> = view.read_with(cx, |this, cx| {
    this
      .devtools
      .read(cx)
      .tree()
      .nodes
      .iter()
      .map(|node| node.name.to_string())
      .collect()
  });

  assert!(names.contains(&"Inspected".to_string()), "{names:?}");
  assert!(!names.contains(&"Neighbour".to_string()), "{names:?}");
}

#[gpui::test]
fn a_recorded_node_carries_its_attributes_style_and_source(cx: &mut TestAppContext) {
  let (view, cx) = inspected(cx);
  cx.run_until_parked();
  view.update(cx, |_this, cx| cx.notify());
  cx.run_until_parked();

  view.read_with(cx, |this, cx| {
    let tree = this.devtools.read(cx).tree().clone();
    let button = tree
      .nodes
      .iter()
      .find(|node| node.name.as_ref() == "Button")
      .expect("the button should have reported itself");

    // Attributes come from the component's own props.
    assert!(button
      .attrs
      .iter()
      .any(|(name, value)| name.as_ref() == "variant" && value.as_ref() == "filled"));
    assert!(button.attrs.iter().any(|(name, _)| name.as_ref() == "size"));

    // The style snapshot is what fills the Styles sidebar.
    let style = button
      .style
      .as_ref()
      .expect("a styled root reports its style");
    let declarations = crate::devtools::declarations(style);
    assert!(declarations
      .iter()
      .any(|d| d.property.as_ref() == "background-color"));

    // `#[track_caller]` has to resolve to the component, not to the probe.
    let source = button.source.as_ref().expect("a probe records its caller");
    assert_eq!(source.basename(), "button.rs");

    // Bounds are captured during prepaint, so a laid-out button has some.
    assert!(f32::from(button.bounds.size.width) > 0.0);
  });
}

#[gpui::test]
fn the_recorder_is_inert_until_an_inspector_exists(cx: &mut TestAppContext) {
  assert!(!crate::devtools::is_recording());

  let (view, cx) = inspected(cx);
  cx.run_until_parked();
  assert!(crate::devtools::is_recording());

  // Dropping the inspector stops the recording, and with it the per-frame
  // cost every probe in the app would otherwise keep paying.
  view.update(cx, |this, cx| {
    this.devtools = cx.new(DevTools::new);
  });
  cx.run_until_parked();
  assert!(crate::devtools::is_recording());
}

#[gpui::test]
fn reported_records_read_back_out_of_the_store(cx: &mut TestAppContext) {
  cx.update(|cx| {
    DevToolsState::new().init(cx);

    crate::devtools::log(cx, LogLevel::Warning, "cache miss");
    crate::devtools::log(cx, LogLevel::Warning, "cache miss");
    crate::devtools::log(cx, LogLevel::Error, "boom");

    let id = crate::devtools::network_begin(
      cx,
      NetworkRecord::new("GET", "https://api.example.com/v1/items"),
    )
    .expect("the store is installed");
    crate::devtools::network_update(cx, id, |record| {
      record.state = RequestState::Finished;
      record.status = Some(200);
    });

    crate::devtools::storage_set(
      cx,
      StorageDomain::new("prefs", "app.preferences").entry(StorageEntry::new("theme", "dark")),
    );
  });

  cx.update(|cx| {
    let state = cx.global::<DevToolsState>();
    // The two identical warnings coalesced into one row with a count of 2.
    assert_eq!(state.logs().len(), 2);
    assert_eq!(state.log_issues(), (2, 1));
    // `log` is `#[track_caller]`, so the line knows where it came from.
    assert_eq!(
      state.logs()[0].source.as_ref().map(|s| s.basename()),
      Some("apptests.rs")
    );
    assert_eq!(state.network()[0].status, Some(200));
    assert_eq!(state.storage()[0].entries.len(), 1);
  });
}

#[gpui::test]
fn reporting_without_the_store_installed_is_a_no_op(cx: &mut TestAppContext) {
  // No `DevToolsState::init`, which is the state a release build is in.
  cx.update(|cx| {
    crate::devtools::log(cx, LogLevel::Error, "nobody is listening");
    assert!(crate::devtools::network_begin(cx, NetworkRecord::new("GET", "/a")).is_none());
    crate::devtools::storage_set(cx, StorageDomain::new("prefs", "Preferences"));
    crate::devtools::clear(cx);
    assert!(!cx.has_global::<DevToolsState>());
  });
}

#[gpui::test]
fn clicking_a_source_link_switches_to_sources_and_tells_the_host(cx: &mut TestAppContext) {
  let (view, cx) = inspected(cx);
  let revealed = std::rc::Rc::new(std::cell::RefCell::new(Vec::<String>::new()));
  let sink = revealed.clone();

  view.update(cx, |this, cx| {
    cx.subscribe(
      &this.devtools,
      move |_this, _devtools, event: &DevToolsEvent, _cx| {
        if let DevToolsEvent::RevealSource(source) = event {
          sink.borrow_mut().push(source.short());
        }
      },
    )
    .detach();
  });

  view.update(cx, |this, cx| {
    this.devtools.update(cx, |devtools, cx| {
      devtools.reveal_source(SourceRef::new("crates/guise/src/button.rs", 42, 9), cx);
    });
  });
  cx.run_until_parked();

  assert_eq!(revealed.borrow().as_slice(), ["button.rs:42:9"]);
  assert_eq!(
    view.read_with(cx, |this, cx| this.devtools.read(cx).active_tab()),
    DevToolsTab::Sources
  );
}

#[gpui::test]
fn picking_selects_the_deepest_node_under_the_point(cx: &mut TestAppContext) {
  let (view, cx) = inspected(cx);
  cx.run_until_parked();
  view.update(cx, |_this, cx| cx.notify());
  cx.run_until_parked();

  let button_bounds = view.read_with(cx, |this, cx| {
    this
      .devtools
      .read(cx)
      .tree()
      .nodes
      .iter()
      .find(|node| node.name.as_ref() == "Button")
      .map(|node| node.bounds)
      .expect("the button should have reported itself")
  });

  view.update(cx, |this, cx| {
    this.devtools.update(cx, |devtools, cx| {
      devtools.set_picking(true, cx);
      assert!(devtools.is_picking());
      assert!(devtools.pick_at(button_bounds.center(), cx));
      // A hit disarms the picker, the way Safari's does.
      assert!(!devtools.is_picking());
      assert_eq!(devtools.active_tab(), DevToolsTab::Elements);
      assert_eq!(devtools.selected_bounds(), Some(button_bounds));
    });
  });
}

// --- settings ---------------------------------------------------------------

fn settings_view(cx: &mut TestAppContext) -> Entity<SettingsView> {
  cx.update(|cx| Theme::light().init(cx));
  cx.update(|cx| {
    cx.new(|cx| {
      SettingsView::new(cx)
        .page("appearance", "Appearance")
        .page("editor", "Editor")
        .page("security", "Security")
        .searchable(true)
        .content(|page, query, _window, _cx| div().child(format!("{page}/{query}")))
    })
  })
}

#[gpui::test]
fn a_settings_view_opens_on_its_first_page(cx: &mut TestAppContext) {
  let view = settings_view(cx);
  assert_eq!(
    view.read_with(cx, |view, _| view.active_page().cloned()),
    Some("appearance".into())
  );
}

#[gpui::test]
fn selecting_a_page_reports_it(cx: &mut TestAppContext) {
  let view = settings_view(cx);
  let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::<String>::new()));
  let sink = seen.clone();

  cx.update(|cx| {
    cx.subscribe(&view, move |_view, event: &SettingsViewEvent, _cx| {
      if let SettingsViewEvent::PageChanged(id) = event {
        sink.borrow_mut().push(id.to_string());
      }
    })
    .detach();
  });

  view.update(cx, |view, cx| view.set_page("security", cx));
  cx.run_until_parked();

  assert_eq!(seen.borrow().as_slice(), ["security"]);
  assert_eq!(
    view.read_with(cx, |view, _| view.active_page().cloned()),
    Some("security".into())
  );

  // Selecting the page that is already active is not a change.
  view.update(cx, |view, cx| view.set_page("security", cx));
  cx.run_until_parked();
  assert_eq!(seen.borrow().len(), 1);
}

#[gpui::test]
fn an_unknown_page_id_is_ignored_rather_than_panicking(cx: &mut TestAppContext) {
  let view = settings_view(cx);
  // A stale id from a restored session must not take the window down.
  view.update(cx, |view, cx| view.set_page("does-not-exist", cx));
  assert_eq!(
    view.read_with(cx, |view, _| view.active_page().cloned()),
    Some("appearance".into())
  );
}

#[gpui::test]
fn the_active_builder_accepts_only_ids_that_exist(cx: &mut TestAppContext) {
  cx.update(|cx| Theme::light().init(cx));
  let opened = cx.update(|cx| {
    cx.new(|cx| {
      SettingsView::new(cx)
        .page("a", "A")
        .page("b", "B")
        .active("b")
    })
  });
  assert_eq!(
    opened.read_with(cx, |view, _| view.active_page().cloned()),
    Some("b".into())
  );

  let stale = cx.update(|cx| cx.new(|cx| SettingsView::new(cx).page("a", "A").active("gone")));
  assert_eq!(
    stale.read_with(cx, |view, _| view.active_page().cloned()),
    Some("a".into())
  );
}

#[gpui::test]
fn clearing_the_search_empties_the_query_and_reports_it(cx: &mut TestAppContext) {
  let view = settings_view(cx);
  let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::<String>::new()));
  let sink = seen.clone();

  cx.update(|cx| {
    cx.subscribe(&view, move |_view, event: &SettingsViewEvent, _cx| {
      if let SettingsViewEvent::Search(query) = event {
        sink.borrow_mut().push(query.to_string());
      }
    })
    .detach();
  });

  view.update(cx, |view, cx| view.clear_search(cx));
  cx.run_until_parked();

  assert_eq!(seen.borrow().as_slice(), [""]);
  assert!(view.read_with(cx, |view, _| view.query().is_empty()));
}

#[gpui::test]
fn a_view_with_no_pages_has_no_active_page(cx: &mut TestAppContext) {
  cx.update(|cx| Theme::light().init(cx));
  let empty = cx.update(|cx| cx.new(SettingsView::new));
  assert_eq!(
    empty.read_with(cx, |view, _| view.active_page().cloned()),
    None
  );
}

#[gpui::test]
fn animator_scrubs_without_a_clock(cx: &mut TestAppContext) {
  use std::time::Instant;

  let motion = Motion::new()
    .duration(1000.0)
    .ease(Easing::Linear)
    .tween(Prop::Opacity, 0.0, 1.0);
  let animator = cx.update(|cx| cx.new(|cx| Animator::new(motion, cx)));

  animator.update(cx, |animator, cx| animator.seek(500.0, cx));
  cx.update(|cx| {
    let animator = animator.read(cx);
    // Stopped: the playhead is exactly where it was put, and stays there.
    assert!(!animator.is_playing());
    let frame = animator.frame_at(Instant::now());
    assert!((frame.number(Prop::Opacity).unwrap() - 0.5).abs() < 1e-4);
    assert!((frame.progress - 0.5).abs() < 1e-4);
  });

  animator.update(cx, |animator, cx| animator.seek_progress(0.25, cx));
  cx.update(|cx| {
    assert!((animator.read(cx).time() - 250.0).abs() < 1e-4);
  });
}

#[gpui::test]
fn animator_completes_at_the_end_and_can_run_back(cx: &mut TestAppContext) {
  use std::cell::RefCell;
  use std::rc::Rc;
  use std::time::Duration;

  let motion = Motion::new()
    .duration(200.0)
    .ease(Easing::Linear)
    .tween(Prop::Opacity, 0.0, 1.0);
  let animator = cx.update(|cx| cx.new(|cx| Animator::new(motion, cx)));
  let seen: Rc<RefCell<Vec<AnimatorEvent>>> = Rc::default();
  let log = seen.clone();
  cx.update(|cx| {
    cx.subscribe(&animator, move |_animator, event: &AnimatorEvent, _cx| {
      log.borrow_mut().push(*event);
    })
    .detach();
  });

  animator.update(cx, |animator, cx| animator.play(cx));
  cx.executor().advance_clock(Duration::from_millis(500));
  cx.run_until_parked();

  cx.update(|cx| {
    let animator = animator.read(cx);
    assert!(!animator.is_playing());
    assert_eq!(animator.time(), 200.0);
  });
  assert_eq!(
    *seen.borrow(),
    vec![AnimatorEvent::Begin, AnimatorEvent::Complete]
  );

  // Reversing from the end runs the same clip the other way.
  animator.update(cx, |animator, cx| {
    animator.reverse(cx);
    animator.play(cx);
  });
  cx.executor().advance_clock(Duration::from_millis(500));
  cx.run_until_parked();
  cx.update(|cx| {
    let animator = animator.read(cx);
    assert!(animator.is_reversed());
    assert_eq!(animator.time(), 0.0);
  });
}

#[gpui::test]
fn an_endless_animator_never_completes(cx: &mut TestAppContext) {
  use std::time::Duration;

  let animator = cx.update(|cx| cx.new(|cx| Animator::new(Motion::pulse(), cx).autoplay(cx)));
  cx.executor().advance_clock(Duration::from_secs(5));
  cx.run_until_parked();
  cx.update(|cx| {
    let animator = animator.read(cx);
    assert!(animator.is_playing());
    assert!(!animator.frame_at(std::time::Instant::now()).finished);
  });
}

#[gpui::test]
fn a_staggered_entrance_holds_every_row_hidden_until_its_turn(cx: &mut TestAppContext) {
  let _ = cx;
  let stagger = Stagger::new(60.0);
  let rows = 4;
  for index in 0..rows {
    let motion = Motion::enter(TransitionKind::SlideUp).delay(stagger.at(index, rows));
    // At 100ms only the first two rows have started moving.
    let frame = motion.sample(100.0);
    let opacity = frame.number(Prop::Opacity).unwrap();
    if index < 2 {
      assert!(opacity > 0.0, "row {index} should be moving");
    } else {
      assert_eq!(opacity, 0.0, "row {index} should still be waiting");
    }
  }
  assert_eq!(stagger.span(rows), 180.0);
}

/// A clip is not a commitment to run it. Everything else about `Animator`
/// assumes a stopped start, including the "replay a finished clip" branch in
/// `play`.
#[gpui::test]
fn a_new_animator_does_not_start_itself(cx: &mut TestAppContext) {
  let animator = cx.update(|cx| cx.new(|cx| Animator::new(Motion::pulse(), cx)));
  cx.update(|cx| {
    let animator = animator.read(cx);
    assert!(!animator.is_playing());
    assert_eq!(animator.time(), 0.0);
  });
}

// --- layout -----------------------------------------------------------------

/// A window whose whole body is one filling `ScrollArea` over content far
/// taller than any window, under a parent of the caller's choosing. The probe
/// recorder is what reports the laid-out bounds, so a `DevTools` has to share
/// the frame for any of this to be observable.
struct Filled {
  devtools: Entity<DevTools>,
  header: bool,
}

impl Render for Filled {
  fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
    let area = crate::ScrollArea::new("pane")
      .fill()
      .children((0..50).map(|row| div().h(gpui::px(40.0)).child(format!("row {row}"))));
    // Two parents, because they lay out differently: a flex column where
    // the pane has to claim what a sibling left, and a plain block box
    // where nothing grows and the pane has to take the height itself.
    let body = if self.header {
      div()
        .flex()
        .flex_col()
        .size_full()
        .child(div().h(gpui::px(64.0)).child("header"))
        .child(area)
    } else {
      div().size_full().child(area)
    };
    div()
      .flex()
      .size_full()
      .child(body.w(gpui::px(400.0)))
      .child(self.devtools.clone())
      .probe("Filled")
  }
}

/// The bounds the recorder captured for the named component in the last frame.
fn probed_height(view: &Entity<Filled>, name: &str, cx: &mut gpui::VisualTestContext) -> f32 {
  view.read_with(cx, |this, cx| {
    this
      .devtools
      .read(cx)
      .tree()
      .nodes
      .iter()
      .find(|node| node.name == name)
      .unwrap_or_else(|| panic!("{name} was not recorded"))
      .bounds
      .size
      .height
      .to_f64() as f32
  })
}

fn filled(header: bool, cx: &mut TestAppContext) -> (Entity<Filled>, &mut gpui::VisualTestContext) {
  cx.update(|cx| {
    Theme::light().init(cx);
    DevToolsState::new().init(cx);
  });
  let (view, cx) = cx.add_window_view(|_window, cx| Filled {
    devtools: cx.new(DevTools::new),
    header,
  });
  cx.run_until_parked();
  // The tree the panel reads is the one the *previous* frame recorded.
  view.update(cx, |_this, cx| cx.notify());
  cx.run_until_parked();
  (view, cx)
}

/// `fill` under a flex parent: the pane takes what the header left and stops,
/// rather than growing to its 2000px of content.
#[gpui::test]
fn a_filling_scrollarea_claims_the_rest_of_a_flex_column(cx: &mut TestAppContext) {
  let (view, cx) = filled(true, cx);
  let window = probed_height(&view, "Filled", cx);
  let pane = probed_height(&view, "ScrollArea", cx);
  assert!(window > 0.0, "the window measured nothing");
  assert_eq!(
    pane,
    window - 64.0,
    "the pane should be the window less the header"
  );
}

/// `fill` under a plain block parent — gpui's default display, and the shape a
/// route body usually has. Nothing grows here, so the relative height is what
/// bounds it.
#[gpui::test]
fn a_filling_scrollarea_takes_the_height_of_a_block_parent(cx: &mut TestAppContext) {
  let (view, cx) = filled(false, cx);
  let window = probed_height(&view, "Filled", cx);
  let pane = probed_height(&view, "ScrollArea", cx);
  assert!(window > 0.0, "the window measured nothing");
  assert_eq!(pane, window, "the pane should be the full window height");
}

/// A heading and a paragraph in a column that nothing gives a width to, beside
/// a sibling that will not shrink — the shape of every page header guise draws
/// for. The probe recorder reports what they were laid out at.
struct Squeezed {
  devtools: Entity<DevTools>,
}

impl Render for Squeezed {
  fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
    div()
      .flex()
      .size_full()
      .child(
        div()
          .w(gpui::px(600.0))
          .flex()
          .justify_between()
          .gap(gpui::px(24.0))
          .child(
            div()
              .flex()
              .flex_col()
              .child(Title::new("Connect your coding tools").order(2))
              .child(Text::new("One scoped memory, available wherever you work.")),
          )
          .child(div().flex_none().child(Button::new("act", "Refresh"))),
      )
      .child(self.devtools.clone())
      .probe("Squeezed")
  }
}

/// The width the recorder captured for the named component in the last frame.
fn probed_width(view: &Entity<Squeezed>, name: &str, cx: &mut gpui::VisualTestContext) -> f32 {
  view.read_with(cx, |this, cx| {
    this
      .devtools
      .read(cx)
      .tree()
      .nodes
      .iter()
      .find(|node| node.name == name)
      .unwrap_or_else(|| panic!("{name} was not recorded"))
      .bounds
      .size
      .width
      .to_f64() as f32
  })
}

fn squeezed(cx: &mut TestAppContext) -> (Entity<Squeezed>, &mut gpui::VisualTestContext) {
  cx.update(|cx| {
    Theme::light().init(cx);
    DevToolsState::new().init(cx);
  });
  let (view, cx) = cx.add_window_view(|_window, cx| Squeezed {
    devtools: cx.new(DevTools::new),
  });
  cx.run_until_parked();
  // The tree the panel reads is the one the *previous* frame recorded.
  view.update(cx, |_this, cx| cx.notify());
  cx.run_until_parked();
  (view, cx)
}

/// Text measures its own content when the box around it has no width of its
/// own. A `min_w(0)` on the text leaf broke exactly this between 1.5.1 and
/// 1.5.3: taffy clamps the available space it hands a measured leaf by that
/// leaf's own minimum, so the min-content pass measured the string at zero and
/// gpui wrapped it after every character — a heading rendered as a vertical
/// column of letters.
#[gpui::test]
fn text_in_an_unsized_column_measures_its_content(cx: &mut TestAppContext) {
  let (view, cx) = squeezed(cx);
  let title = probed_width(&view, "Title", cx);
  let body = probed_width(&view, "Text", cx);
  assert!(
    title > 100.0,
    "the heading laid out {title}px wide, so it wrapped per character"
  );
  assert!(
    body > 100.0,
    "the paragraph laid out {body}px wide, so it wrapped per character"
  );
}
