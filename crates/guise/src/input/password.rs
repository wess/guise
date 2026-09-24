//! `PasswordInput` — a masked text field with a visibility toggle (gpui entity).
//!
//! Owns its buffer and focus like [`TextInput`](super::TextInput) in password
//! mode, plus an eye button that reveals the plain text while toggled. Emits
//! [`PasswordInputEvent`] on edit and submit.
//!
//! ```ignore
//! let secret = cx.new(|cx| {
//!     PasswordInput::new(cx)
//!         .label("Password")
//!         .placeholder("At least 8 characters")
//! });
//! cx.subscribe(&secret, |_this, _input, event: &PasswordInputEvent, _cx| {
//!     if let PasswordInputEvent::Submit(value) = event { /* log in */ }
//! })
//! .detach();
//! ```

use gpui::prelude::*;
use gpui::{
  div, px, App, Context, Entity, EventEmitter, FocusHandle, IntoElement, KeyDownEvent,
  SharedString, Window,
};

use super::line::{self, Line, LineEditor, LineState};
use super::{control_metrics, edit::TextEdit, Field, KeyOutcome};
use crate::devtools::ProbedAny;
use crate::icon::{Icon, IconName};
use crate::reactive::Signal;
use crate::theme::{theme, ColorName, Size};

/// Emitted as the user edits or submits the field.
#[derive(Debug, Clone)]
pub enum PasswordInputEvent {
  /// The text changed. Carries the full new value.
  Change(String),
  /// The user pressed Enter. Carries the current value.
  Submit(String),
}

/// A password field with an eye toggle. Create with
/// `cx.new(|cx| PasswordInput::new(cx))`.
pub struct PasswordInput {
  edit: TextEdit,
  state: LineState,
  focus: FocusHandle,
  visible: bool,
  placeholder: SharedString,
  label: Option<SharedString>,
  description: Option<SharedString>,
  error: Option<SharedString>,
  size: Size,
  disabled: bool,
  read_only: bool,
  max_length: Option<usize>,
}

impl EventEmitter<PasswordInputEvent> for PasswordInput {}

impl PasswordInput {
  pub fn new(cx: &mut Context<Self>) -> Self {
    PasswordInput {
      edit: TextEdit::new(""),
      state: LineState::new(),
      focus: cx.focus_handle().tab_stop(true),
      visible: false,
      placeholder: SharedString::default(),
      label: None,
      description: None,
      error: None,
      size: Size::Sm,
      disabled: false,
      read_only: false,
      max_length: None,
    }
  }

  pub fn value(mut self, value: &str) -> Self {
    self.edit = TextEdit::new(value);
    self
  }

  pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
    self.placeholder = placeholder.into();
    self
  }

  pub fn label(mut self, label: impl Into<SharedString>) -> Self {
    self.label = Some(label.into());
    self
  }

  pub fn description(mut self, description: impl Into<SharedString>) -> Self {
    self.description = Some(description.into());
    self
  }

  pub fn error(mut self, error: impl Into<SharedString>) -> Self {
    self.error = Some(error.into());
    self
  }

  pub fn size(mut self, size: Size) -> Self {
    self.size = size;
    self
  }

  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Start with the text revealed (the eye still toggles it).
  pub fn visible(mut self, visible: bool) -> Self {
    self.visible = visible;
    self
  }

  /// Selectable but not editable, like an `<input readonly>`.
  pub fn read_only(mut self, read_only: bool) -> Self {
    self.read_only = read_only;
    self
  }

  /// Cap the value's length in characters, like `<input maxlength>`.
  pub fn max_length(mut self, max: usize) -> Self {
    self.max_length = Some(max);
    self
  }

  /// The current text.
  pub fn text(&self) -> String {
    self.edit.text()
  }

  /// Replace the text programmatically.
  pub fn set_text(&mut self, value: &str, cx: &mut Context<Self>) {
    self.edit.set_text(value);
    cx.notify();
  }

  /// Two-way bind this input's text to a `Signal<String>`. The signal is
  /// the source of truth: the field adopts its value now, edits write back
  /// through [`Signal::set_if_changed`], and signal writes replace the text.
  /// Equality guards on both directions prevent update loops.
  pub fn bind(entity: &Entity<PasswordInput>, signal: &Signal<String>, cx: &mut App) {
    let initial = signal.get(cx);
    entity.update(cx, |this, cx| {
      if this.text() != initial {
        this.set_text(&initial, cx);
      }
    });
    let sink = signal.clone();
    cx.subscribe(entity, move |_input, event: &PasswordInputEvent, cx| {
      if let PasswordInputEvent::Change(text) = event {
        sink.set_if_changed(cx, text.clone());
      }
    })
    .detach();
    let input = entity.downgrade();
    cx.observe(signal.entity(), move |observed, cx| {
      let value = observed.read(cx).clone();
      input
        .update(cx, |this, cx| {
          if this.text() != value {
            this.set_text(&value, cx);
          }
        })
        .ok();
    })
    .detach();
  }

  fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
    if self.disabled {
      return;
    }
    match line::keys(self, event, window, cx) {
      KeyOutcome::Submit => {
        cx.emit(PasswordInputEvent::Submit(self.edit.text()));
        cx.notify();
        cx.stop_propagation();
      }
      KeyOutcome::Edited => {
        self.line_changed(cx);
        cx.stop_propagation();
      }
      // Escape and unhandled keys bubble to the host.
      KeyOutcome::Cancel | KeyOutcome::Pass => {}
    }
  }
}

impl LineEditor for PasswordInput {
  fn edit(&self) -> &TextEdit {
    &self.edit
  }

  fn edit_mut(&mut self) -> &mut TextEdit {
    &mut self.edit
  }

  fn line(&self) -> &LineState {
    &self.state
  }

  fn line_mut(&mut self) -> &mut LineState {
    &mut self.state
  }

  fn line_focus(&self) -> &FocusHandle {
    &self.focus
  }

  /// Revealing the field with the eye is a deliberate act, so it also lifts
  /// the copy block — otherwise the toggle would be for looking only.
  fn line_masked(&self) -> bool {
    !self.visible
  }

  fn line_read_only(&self) -> bool {
    self.read_only || self.disabled
  }

  fn line_max_length(&self) -> Option<usize> {
    self.max_length
  }

  fn line_changed(&mut self, cx: &mut Context<Self>) {
    cx.emit(PasswordInputEvent::Change(self.edit.text()));
    cx.notify();
  }
}

line::line_input_handler!(PasswordInput);
line::line_focus_builders!(PasswordInput);

impl Render for PasswordInput {
  fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let t = theme(cx);
    let (height, pad_x, font) = control_metrics(self.size);
    let radius = t.radius(t.default_radius);
    let focused = self.focus.is_focused(window) && !self.disabled;

    let border = if self.error.is_some() {
      t.color(ColorName::Red, 6)
    } else if focused {
      t.primary()
    } else {
      t.border()
    }
    .hsla();
    let text_color = t.text().hsla();
    let dimmed = t.dimmed().hsla();
    let surface = t.surface().hsla();
    let interior = Line::new(cx.entity()).placeholder(self.placeholder.clone(), dimmed);

    // While hidden the eye offers "reveal"; while revealed it offers "hide".
    let eye_icon = if self.visible {
      IconName::EyeOff
    } else {
      IconName::Eye
    };
    let eye = div()
      .id("guise-password-eye")
      .flex()
      .items_center()
      .justify_center()
      .w(px(height - 16.0))
      .h(px(height - 16.0))
      .rounded(px(4.0))
      .text_color(dimmed)
      .cursor_pointer()
      .hover(move |s| s.text_color(text_color))
      .child(Icon::new(eye_icon).size(Size::Xs))
      .on_click(cx.listener(|this, _ev, _window, cx| {
        if !this.disabled {
          this.visible = !this.visible;
          cx.notify();
        }
      }));

    let field = line::wire(
      div().id("guise-passwordinput"),
      &self.focus,
      &self.state,
      cx,
    )
    .on_key_down(cx.listener(Self::on_key))
    .flex()
    .items_center()
    .justify_between()
    .gap(px(8.0))
    .w_full()
    .overflow_hidden()
    .h(px(height))
    .px(px(pad_x))
    .rounded(px(radius))
    .border_1()
    .border_color(border)
    .bg(surface)
    .text_size(px(font))
    .line_height(px(font * 1.3))
    .child(div().flex_1().min_w(px(0.0)).child(interior))
    .child(eye);

    let mut chrome = Field::new().child(if self.disabled {
      field.opacity(0.6)
    } else {
      field
    });
    if let Some(label) = self.label.clone() {
      chrome = chrome.label(label);
    }
    if let Some(error) = self.error.clone() {
      chrome = chrome.error(error);
    } else if let Some(description) = self.description.clone() {
      chrome = chrome.description(description);
    }
    chrome.probe_any("PasswordInput")
  }
}
