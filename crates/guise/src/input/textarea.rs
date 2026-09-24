//! `TextArea` — a multiline text field (gpui entity).
//!
//! Reuses the [`TextEdit`] char model (newline-aware) and emits
//! [`TextAreaEvent`] on edit. Enter inserts a newline; up/down move between
//! lines keeping the column. The text itself is drawn by
//! [`area`](super::area), which wraps it at the field's width and keeps the
//! layout for the mouse (click, drag, double-click a word, triple-click a
//! line) and the platform's input handler (IME, dead keys, press-and-hold).
//! So, as in a single-line field, typing arrives through
//! `replace_text_in_range` rather than the key handler.

use crate::chord::Chord;
use std::ops::Range;

use gpui::prelude::*;
use gpui::{
  div, px, App, Bounds, ClipboardItem, Context, Entity, EventEmitter, FocusHandle, IntoElement,
  KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point,
  ScrollHandle, SharedString, UTF16Selection, Window,
};

use super::area::{AreaState, AreaText};
use super::editmenu::{self, EditMenu};
use super::line::{from_utf16, slice, to_utf16, utf16_selection};
use super::{control_metrics, Field, TextEdit};
use crate::actions;
use crate::devtools::ProbedAny;
use crate::overlay::ContextMenu;
use crate::reactive::Signal;
use crate::theme::{theme, ColorName, Size};

/// Emitted as the user edits the field. Carries the full new value.
#[derive(Debug, Clone)]
pub struct TextAreaEvent(pub String);

/// Emitted when Enter commits the field, which only happens under
/// [`TextArea::submit_on_enter`]. Carries the value. It is a separate event
/// type rather than a variant so that subscribers to [`TextAreaEvent`] keep
/// working unchanged.
#[derive(Debug, Clone)]
pub struct TextAreaSubmit(pub String);

/// A multiline text field. Create with `cx.new(|cx| TextArea::new(cx))`.
pub struct TextArea {
  pub(crate) edit: TextEdit,
  pub(super) focus: FocusHandle,
  pub(crate) area: AreaState,
  menu: Option<Entity<ContextMenu>>,
  scroll: ScrollHandle,
  placeholder: SharedString,
  label: Option<SharedString>,
  description: Option<SharedString>,
  error: Option<SharedString>,
  rows: usize,
  max_rows: Option<usize>,
  submit_on_enter: bool,
  size: Size,
  pub(super) disabled: bool,
}

impl EventEmitter<TextAreaEvent> for TextArea {}
impl EventEmitter<TextAreaSubmit> for TextArea {}

/// What a text area accepts from a paste, a drop, or the IME: line breaks
/// normalised to `\n`, and no control characters other than those and tabs,
/// which would otherwise shape as nothing useful.
fn normalize(text: &str) -> String {
  text
    .replace("\r\n", "\n")
    .replace('\r', "\n")
    .chars()
    .filter(|c| *c == '\n' || *c == '\t' || !c.is_control())
    .collect()
}

impl TextArea {
  pub fn new(cx: &mut Context<Self>) -> Self {
    TextArea {
      edit: TextEdit::new(""),
      focus: cx.focus_handle().tab_stop(true),
      area: AreaState::default(),
      menu: None,
      scroll: ScrollHandle::new(),
      placeholder: SharedString::default(),
      label: None,
      description: None,
      error: None,
      rows: 3,
      max_rows: None,
      submit_on_enter: false,
      size: Size::Sm,
      disabled: false,
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

  /// Replace the placeholder after construction, for a field that is built
  /// once and re-labelled later.
  pub fn set_placeholder(&mut self, placeholder: impl Into<SharedString>, cx: &mut Context<Self>) {
    self.placeholder = placeholder.into();
    cx.notify();
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

  /// Stop growing past `rows` and scroll instead. Without this a field that
  /// grows with its content has no ceiling, which is wrong for a composer
  /// pinned to the bottom of a window.
  pub fn max_rows(mut self, rows: usize) -> Self {
    self.max_rows = Some(rows.max(1));
    self
  }

  /// Make Enter commit the value (emitting [`TextAreaSubmit`]) and
  /// Shift+Enter insert the newline — the convention a chat composer uses.
  pub fn submit_on_enter(mut self, submit: bool) -> Self {
    self.submit_on_enter = submit;
    self
  }

  /// Minimum visible rows (sets the field's minimum height).
  pub fn rows(mut self, rows: usize) -> Self {
    self.rows = rows.max(1);
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

  /// The field's focus handle, so a host can focus it on open.
  pub fn focus_handle(&self) -> FocusHandle {
    self.focus.clone()
  }

  pub fn text(&self) -> String {
    self.edit.text()
  }

  /// Whether the field holds nothing but whitespace. Cheaper than
  /// `text().trim().is_empty()`, which builds and throws away a copy of the
  /// whole value — and callers ask this every frame to enable a send button.
  pub fn is_blank(&self) -> bool {
    self.edit.chars().iter().all(|c| c.is_whitespace())
  }

  pub fn set_text(&mut self, value: &str, cx: &mut Context<Self>) {
    self.edit = TextEdit::new(value);
    self.area.marked = None;
    cx.notify();
  }

  /// Two-way bind this field's text to a `Signal<String>`. The signal is
  /// the source of truth: the field adopts its value now, edits write back
  /// through [`Signal::set_if_changed`], and signal writes replace the text.
  /// Equality guards on both directions prevent update loops.
  pub fn bind(entity: &Entity<TextArea>, signal: &Signal<String>, cx: &mut App) {
    let initial = signal.get(cx);
    entity.update(cx, |this, cx| {
      if this.text() != initial {
        this.set_text(&initial, cx);
      }
    });
    let sink = signal.clone();
    cx.subscribe(entity, move |_area, event: &TextAreaEvent, cx| {
      sink.set_if_changed(cx, event.0.clone());
    })
    .detach();
    let area = entity.downgrade();
    cx.observe(signal.entity(), move |observed, cx| {
      let value = observed.read(cx).clone();
      area
        .update(cx, |this, cx| {
          if this.text() != value {
            this.set_text(&value, cx);
          }
        })
        .ok();
    })
    .detach();
  }

  fn changed(&mut self, cx: &mut Context<Self>) {
    self.area.reveal = true;
    self.area.goal_x = None;
    cx.emit(TextAreaEvent(self.edit.text()));
    cx.notify();
  }

  fn copy(&mut self, cx: &mut Context<Self>) {
    if let Some(text) = self.edit.selected_text() {
      cx.write_to_clipboard(ClipboardItem::new_string(text));
    }
    cx.stop_propagation();
  }

  fn cut(&mut self, cx: &mut Context<Self>) {
    cx.stop_propagation();
    if self.disabled {
      return;
    }
    if let Some(text) = self.edit.selected_text() {
      cx.write_to_clipboard(ClipboardItem::new_string(text));
      self.edit.delete_selection();
      self.changed(cx);
    }
  }

  fn paste(&mut self, cx: &mut Context<Self>) {
    cx.stop_propagation();
    if self.disabled {
      return;
    }
    if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
      // A paste is one undo step of its own, not merged into the typing
      // around it.
      self.edit.break_undo();
      self.edit.insert(&normalize(&text));
      self.edit.break_undo();
      self.changed(cx);
    }
  }

  fn select_all(&mut self, cx: &mut Context<Self>) {
    self.edit.select_all();
    cx.notify();
    cx.stop_propagation();
  }

  fn history(&mut self, undo: bool, cx: &mut Context<Self>) {
    cx.stop_propagation();
    if self.disabled {
      return;
    }
    let changed = if undo {
      self.edit.undo()
    } else {
      self.edit.redo()
    };
    if changed {
      self.changed(cx);
    } else {
      cx.notify();
    }
  }

  /// The logical line (between line breaks) around a char index, for a
  /// triple-click.
  fn line_at(&self, index: usize) -> (usize, usize) {
    let chars = self.edit.chars();
    let index = index.min(chars.len());
    let start = chars[..index]
      .iter()
      .rposition(|c| *c == '\n')
      .map_or(0, |at| at + 1);
    let end = chars[index..]
      .iter()
      .position(|c| *c == '\n')
      .map_or(chars.len(), |at| index + at);
    (start, end)
  }

  /// Move one visual row, keeping the goal column. Before the first paint
  /// there is no layout to ask, so it falls back to moving by line break.
  fn vertical(&mut self, down: bool, extend: bool) {
    let byte = self.edit.byte_of(self.edit.cursor());
    self.edit.pre_move(extend);
    match self.area.vertical(byte, self.area.goal_x, down) {
      Some((target, x)) => {
        let index = self.edit.char_of(target);
        if extend {
          self.edit.extend_to(index);
        } else {
          self.edit.set_cursor(index);
        }
        self.area.goal_x = Some(x);
      }
      None if down => self.edit.down(),
      None => self.edit.up(),
    }
  }

  /// Right-click: move the caret there unless the click is inside the
  /// selection, then offer Cut / Copy / Paste / Select All.
  fn on_right_mouse_down(
    &mut self,
    event: &MouseDownEvent,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    window.focus(&self.focus, cx);
    if let Some(index) = self.index_at(event.position) {
      let inside = self
        .edit
        .selection()
        .is_some_and(|(start, end)| index >= start && index <= end);
      if !inside {
        self.edit.set_cursor(index);
      }
    }
    let menu = EditMenu::new(self.edit.has_selection(), false, self.disabled);
    let mut slot = self.menu.take();
    editmenu::open(&mut slot, menu, &self.focus, event.position, window, cx);
    self.menu = slot;
    cx.stop_propagation();
    cx.notify();
  }

  pub(super) fn menu_open(&self, cx: &App) -> bool {
    editmenu::is_open(&self.menu, cx)
  }

  fn index_at(&self, position: Point<Pixels>) -> Option<usize> {
    let byte = self.area.byte_at(position)?;
    Some(self.edit.char_of(byte))
  }

  /// Click counts follow the platform convention: one places the caret
  /// (shift extends the selection to it), two takes the word, three the
  /// line.
  fn on_mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
    window.focus(&self.focus, cx);
    if self.disabled {
      return;
    }
    self.area.goal_x = None;
    let Some(index) = self.index_at(event.position) else {
      cx.notify();
      return;
    };
    match event.click_count {
      1 if event.modifiers.shift => self.edit.extend_to(index),
      1 => self.edit.set_cursor(index),
      2 => {
        let (start, end) = self.edit.word_at(index);
        self.edit.set_selection(start, end);
      }
      _ => {
        let (start, end) = self.line_at(index);
        self.edit.set_selection(start, end);
      }
    }
    self.area.selecting = true;
    cx.notify();
  }

  fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
    if !self.area.selecting {
      return;
    }
    if let Some(index) = self.index_at(event.position) {
      self.edit.extend_to(index);
      self.area.reveal = true;
      cx.notify();
    }
  }

  fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
    if self.area.selecting {
      self.area.selecting = false;
      cx.notify();
    }
  }

  fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
    if self.disabled {
      return;
    }
    if editmenu::is_open(&self.menu, cx) {
      return;
    }
    let ks = &event.keystroke;
    let m = &ks.modifiers;
    if !matches!(ks.key.as_str(), "up" | "down") {
      self.area.goal_x = None;
    }
    if m.shortcut() {
      match ks.key.as_str() {
        "a" => return self.select_all(cx),
        "c" => return self.copy(cx),
        "x" => return self.cut(cx),
        "v" => return self.paste(cx),
        "z" => return self.history(!m.shift, cx),
        "y" => return self.history(false, cx),
        _ => {}
      }
    }
    // Tab moves to the next field, as it does in a `<textarea>` — a text
    // area is still a form control, not a code editor. Escape bubbles so
    // the host can dismiss. (Enter inserts a newline: this is multi-line.)
    if ks.key == "tab" && !m.platform && !m.control {
      if m.shift {
        window.focus_prev(cx);
      } else {
        window.focus_next(cx);
      }
      cx.notify();
      cx.stop_propagation();
      return;
    }
    if ks.key == "escape" {
      return;
    }
    let edited = match ks.key.as_str() {
      "enter" if self.submit_on_enter && !m.shift => {
        cx.emit(TextAreaSubmit(self.edit.text()));
        cx.notify();
        cx.stop_propagation();
        return;
      }
      "enter" => {
        self.edit.insert("\n");
        true
      }
      "left" => {
        if !m.shift && !m.line() && !m.word() && self.edit.collapse_selection_start() {
          true
        } else {
          self.edit.pre_move(m.shift);
          if m.line() {
            self.edit.line_home();
          } else if m.word() {
            self.edit.word_left();
          } else {
            self.edit.left();
          }
          true
        }
      }
      "right" => {
        if !m.shift && !m.line() && !m.word() && self.edit.collapse_selection_end() {
          true
        } else {
          self.edit.pre_move(m.shift);
          if m.line() {
            self.edit.line_end();
          } else if m.word() {
            self.edit.word_right();
          } else {
            self.edit.right();
          }
          true
        }
      }
      "up" | "down" if m.line() => {
        self.edit.pre_move(m.shift);
        if ks.key == "up" {
          self.edit.home();
        } else {
          self.edit.end();
        }
        true
      }
      "up" | "down" => {
        self.vertical(ks.key == "down", m.shift);
        true
      }
      "home" => {
        self.edit.pre_move(m.shift);
        self.edit.line_home();
        true
      }
      "end" => {
        self.edit.pre_move(m.shift);
        self.edit.line_end();
        true
      }
      "backspace" => {
        if m.line() {
          self.edit.delete_to_start();
        } else if m.word() {
          self.edit.delete_word_back();
        } else {
          self.edit.backspace();
        }
        true
      }
      "delete" => {
        if m.line() {
          self.edit.delete_to_end();
        } else if m.word() {
          self.edit.delete_word_forward();
        } else {
          self.edit.delete();
        }
        true
      }
      "k" if m.emacs() => {
        self.edit.delete_to_end();
        true
      }
      "a" if m.emacs() => {
        self.edit.home();
        true
      }
      "e" if m.emacs() => {
        self.edit.end();
        true
      }
      // Printable keys pass: the platform hands them to the input
      // handler, which is what makes IME and dead keys work.
      _ => false,
    };
    if edited {
      self.changed(cx);
      cx.stop_propagation();
    }
  }
}

/// The platform's side of text entry: IME composition, dead keys,
/// press-and-hold, and plain typing all arrive here. The same translation
/// between UTF-16 offsets and chars the single-line fields use.
impl gpui::EntityInputHandler for TextArea {
  fn text_for_range(
    &mut self,
    range_utf16: Range<usize>,
    actual: &mut Option<Range<usize>>,
    _window: &mut Window,
    _cx: &mut Context<Self>,
  ) -> Option<String> {
    let range = from_utf16(&self.edit, &range_utf16);
    actual.replace(to_utf16(&self.edit, &range));
    Some(slice(&self.edit, &range))
  }

  fn selected_text_range(
    &mut self,
    _ignore_disabled: bool,
    _window: &mut Window,
    _cx: &mut Context<Self>,
  ) -> Option<UTF16Selection> {
    Some(utf16_selection(&self.edit))
  }

  fn marked_text_range(
    &self,
    _window: &mut Window,
    _cx: &mut Context<Self>,
  ) -> Option<Range<usize>> {
    let marked = self.area.marked.clone()?;
    Some(to_utf16(&self.edit, &marked))
  }

  fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
    self.area.marked = None;
  }

  fn replace_text_in_range(
    &mut self,
    range_utf16: Option<Range<usize>>,
    text: &str,
    _window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    self.replace(range_utf16, text, None, cx);
  }

  fn replace_and_mark_text_in_range(
    &mut self,
    range_utf16: Option<Range<usize>>,
    text: &str,
    selected_utf16: Option<Range<usize>>,
    _window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    self.replace(range_utf16, text, Some(selected_utf16), cx);
  }

  fn bounds_for_range(
    &mut self,
    range_utf16: Range<usize>,
    _bounds: Bounds<Pixels>,
    _window: &mut Window,
    _cx: &mut Context<Self>,
  ) -> Option<Bounds<Pixels>> {
    let range = from_utf16(&self.edit, &range_utf16);
    let start = self.area.point_for(self.edit.byte_of(range.start))?;
    let end = self.area.point_for(self.edit.byte_of(range.end))?;
    let lh = self.area.line_height();
    Some(Bounds::from_corners(
      start,
      gpui::point(end.x.max(start.x), end.y.max(start.y) + lh),
    ))
  }

  fn character_index_for_point(
    &mut self,
    point: Point<Pixels>,
    _window: &mut Window,
    _cx: &mut Context<Self>,
  ) -> Option<usize> {
    let index = self.index_at(point)?;
    Some(to_utf16(&self.edit, &(0..index)).end)
  }
}

impl TextArea {
  /// The shared body of `replace_text_in_range` and its marked-text sibling.
  /// `marking` is `Some(selection)` while the IME is mid-composition.
  fn replace(
    &mut self,
    range_utf16: Option<Range<usize>>,
    text: &str,
    marking: Option<Option<Range<usize>>>,
    cx: &mut Context<Self>,
  ) {
    if self.disabled {
      return;
    }
    let text = normalize(text);
    let range = range_utf16
      .map(|r| from_utf16(&self.edit, &r))
      .or_else(|| self.area.marked.clone())
      .unwrap_or_else(|| {
        self
          .edit
          .selection()
          .map(|(s, e)| s..e)
          .unwrap_or_else(|| self.edit.cursor()..self.edit.cursor())
      });
    let start = range.start;
    self.edit.replace_range(range, &text);
    match marking {
      Some(selection) => {
        let end = start + text.chars().count();
        self.area.marked = (!text.is_empty()).then_some(start..end);
        if let Some(selection) = selection {
          let selection = from_utf16(&self.edit, &selection);
          self
            .edit
            .set_selection(start + selection.start, start + selection.end);
        }
      }
      None => self.area.marked = None,
    }
    self.changed(cx);
  }
}

impl Render for TextArea {
  fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let t = theme(cx);
    let (_, pad_x, font) = control_metrics(self.size);
    let radius = t.radius(t.default_radius);
    let focused = self.focus.is_focused(window) && !self.disabled;
    let line_h = font * 1.5;
    let pad_y = 8.0;
    let min_h = self.rows as f32 * line_h + pad_y * 2.0;
    let max_h = self
      .max_rows
      .map(|rows| rows as f32 * line_h + pad_y * 2.0)
      .filter(|max| *max >= min_h);

    let border = if self.error.is_some() {
      t.color(ColorName::Red, 6)
    } else if focused {
      t.primary()
    } else {
      t.border()
    }
    .hsla();
    let text_color = t.text().hsla();
    let surface = t.surface().hsla();

    let body = AreaText::new(cx.entity(), self.placeholder.clone())
      .scroll(max_h.map(|_| self.scroll.clone()));

    let mut field = div()
      .id("guise-textarea")
      .track_focus(&self.focus)
      .on_key_down(cx.listener(Self::on_key))
      .cursor(gpui::CursorStyle::IBeam)
      .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
      .on_mouse_move(cx.listener(Self::on_mouse_move))
      .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
      .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
      .on_mouse_down(MouseButton::Right, cx.listener(Self::on_right_mouse_down))
      .children(editmenu::slot(&self.menu))
      .on_action(cx.listener(|this, _: &actions::Copy, _, cx| this.copy(cx)))
      .on_action(cx.listener(|this, _: &actions::Cut, _, cx| this.cut(cx)))
      .on_action(cx.listener(|this, _: &actions::Paste, _, cx| this.paste(cx)))
      .on_action(cx.listener(|this, _: &actions::SelectAll, _, cx| this.select_all(cx)))
      .on_action(cx.listener(|this, _: &actions::Undo, _, cx| this.history(true, cx)))
      .on_action(cx.listener(|this, _: &actions::Redo, _, cx| this.history(false, cx)))
      .flex()
      .items_start()
      .overflow_x_hidden()
      .min_h(px(min_h))
      .w_full()
      .px(px(pad_x))
      .py(px(pad_y))
      .rounded(px(radius))
      .border_1()
      .border_color(border)
      .bg(surface)
      .text_size(px(font))
      .line_height(px(line_h))
      .text_color(text_color)
      .child(div().w_full().min_w(px(0.0)).child(body));

    if let Some(max) = max_h {
      field = field
        .max_h(px(max))
        .overflow_y_scroll()
        .track_scroll(&self.scroll);
    }

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
    chrome.probe_any("TextArea")
  }
}
