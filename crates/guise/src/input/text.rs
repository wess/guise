//! `TextInput` — a stateful single-line text field (gpui entity).
//!
//! Owns its buffer and focus; renders the shared field chrome (label, box,
//! description/error) and emits [`TextInputEvent`] on edit and submit.
//!
//! The editing surface itself lives in [`line`](super::line), which is what
//! gives the field the behaviour an `<input>` has: click and drag to select,
//! double-click a word, Tab to the next field, the clipboard, undo, IME, and
//! horizontal scrolling for values wider than the box.

use gpui::prelude::*;
use gpui::{
  div, px, App, Context, Entity, EventEmitter, FocusHandle, IntoElement, KeyDownEvent,
  SharedString, Window,
};

use super::line::{self, Line, LineEditor, LineState};
use super::{control_metrics, edit::TextEdit, Field, KeyOutcome};
use crate::devtools::ProbedAny;
use crate::reactive::Signal;
use crate::theme::{theme, ColorName, Size};

/// Emitted as the user edits or submits the field.
#[derive(Debug, Clone)]
pub enum TextInputEvent {
  /// The text changed. Carries the full new value.
  Change(String),
  /// The user pressed Enter. Carries the current value.
  Submit(String),
}

/// A single-line text field. Create with `cx.new(|cx| TextInput::new(cx))`.
pub struct TextInput {
  edit: TextEdit,
  state: LineState,
  focus: FocusHandle,
  placeholder: SharedString,
  label: Option<SharedString>,
  description: Option<SharedString>,
  error: Option<SharedString>,
  size: Size,
  radius: Option<Size>,
  disabled: bool,
  read_only: bool,
  password: bool,
  max_length: Option<usize>,
}

impl EventEmitter<TextInputEvent> for TextInput {}

impl TextInput {
  pub fn new(cx: &mut Context<Self>) -> Self {
    TextInput {
      edit: TextEdit::new(""),
      state: LineState::new(),
      // Every field is a tab stop by default, the way a form control is
      // in a browser. Ordering falls out of render order unless a host
      // sets `tab_index`.
      focus: cx.focus_handle().tab_stop(true),
      placeholder: SharedString::default(),
      label: None,
      description: None,
      error: None,
      size: Size::Sm,
      radius: None,
      disabled: false,
      read_only: false,
      password: false,
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

  pub fn radius(mut self, radius: Size) -> Self {
    self.radius = Some(radius);
    self
  }

  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Selectable and copyable, but not editable — an `<input readonly>`.
  /// Unlike [`disabled`](Self::disabled) the field still takes focus.
  pub fn read_only(mut self, read_only: bool) -> Self {
    self.read_only = read_only;
    self
  }

  pub fn password(mut self, password: bool) -> Self {
    self.password = password;
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

  /// Select the whole value, as focusing a field with `<input autofocus>`
  /// and a preset value tends to.
  pub fn select_all(&mut self, cx: &mut Context<Self>) {
    self.edit.select_all();
    cx.notify();
  }

  /// Two-way bind the text to a `Signal<String>`. The signal is the source
  /// of truth: the field adopts its value now, edits write back through
  /// [`Signal::set_if_changed`], and signal writes replace the text without
  /// emitting. Equality guards on both directions prevent update loops.
  pub fn bind(entity: &Entity<TextInput>, signal: &Signal<String>, cx: &mut App) {
    let initial = signal.get(cx);
    entity.update(cx, |this, cx| this.sync_text(initial, cx));
    let sink = signal.clone();
    cx.subscribe(entity, move |_input, event: &TextInputEvent, cx| {
      if let TextInputEvent::Change(text) = event {
        sink.set_if_changed(cx, text.clone());
      }
    })
    .detach();
    let field = entity.downgrade();
    cx.observe(signal.entity(), move |observed, cx| {
      let text = observed.read(cx).clone();
      field.update(cx, |this, cx| this.sync_text(text, cx)).ok();
    })
    .detach();
  }

  /// Programmatic set: repaint without emitting an event.
  fn sync_text(&mut self, text: String, cx: &mut Context<Self>) {
    if self.edit.text() != text {
      self.edit.set_text(&text);
      cx.notify();
    }
  }

  fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
    if self.disabled {
      return;
    }
    match line::keys(self, event, window, cx) {
      KeyOutcome::Submit => {
        cx.emit(TextInputEvent::Submit(self.edit.text()));
        cx.notify();
        cx.stop_propagation();
      }
      KeyOutcome::Edited => {
        self.line_changed(cx);
        cx.stop_propagation();
      }
      // Escape (Cancel) bubbles so dialogs can close on it. Printable
      // keys pass too: the platform hands them to the input handler,
      // which is what makes IME and dead keys work.
      KeyOutcome::Cancel | KeyOutcome::Pass => {}
    }
  }
}

impl LineEditor for TextInput {
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

  fn line_masked(&self) -> bool {
    self.password
  }

  fn line_read_only(&self) -> bool {
    self.read_only || self.disabled
  }

  fn line_max_length(&self) -> Option<usize> {
    self.max_length
  }

  fn line_changed(&mut self, cx: &mut Context<Self>) {
    cx.emit(TextInputEvent::Change(self.edit.text()));
    cx.notify();
  }
}

line::line_input_handler!(TextInput);
line::line_focus_builders!(TextInput);

impl Render for TextInput {
  fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let t = theme(cx);
    let (height, pad_x, font) = control_metrics(self.size);
    let radius = t.radius(self.radius.unwrap_or(t.default_radius));
    let focused = self.focus.is_focused(window) && !self.disabled;
    let has_error = self.error.is_some();

    let border = if has_error {
      t.color(ColorName::Red, 6)
    } else if focused {
      t.primary()
    } else {
      t.border()
    }
    .hsla();
    let dimmed = t.dimmed().hsla();
    let surface = t.surface().hsla();

    let line = Line::new(cx.entity()).placeholder(self.placeholder.clone(), dimmed);

    let field = line::wire(div().id("guise-textinput"), &self.focus, &self.state, cx)
      .on_key_down(cx.listener(Self::on_key))
      .flex()
      .items_center()
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
      .child(div().flex_1().min_w(px(0.0)).child(line));

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
    chrome.probe_any("TextInput")
  }
}
