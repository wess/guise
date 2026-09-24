//! `TagsInput` — a pill list with an inline editor (gpui entity).
//!
//! Type and press Enter (or comma) to commit a tag; committed tags render as
//! removable pills. Tags are trimmed, non-empty and unique; Backspace in an
//! empty query pops the last pill. Emits [`TagsInputEvent`] with the full tag
//! list on every change.
//!
//! ```ignore
//! let topics = cx.new(|cx| {
//!     TagsInput::new(cx)
//!         .label("Topics")
//!         .placeholder("Add a topic…")
//!         .max_tags(5)
//! });
//! cx.subscribe(&topics, |_this, _input, event: &TagsInputEvent, _cx| {
//!     let tags: &Vec<String> = &event.0;
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
use crate::reactive::Signal;
use crate::theme::{theme, ColorName, Size};

/// Emitted whenever the tag list changes. Carries the full list.
#[derive(Debug, Clone)]
pub struct TagsInputEvent(pub Vec<String>);

/// What committing a query did to the tag list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Commit {
  /// The tag was added; clear the query and emit.
  Added,
  /// Already present; clear the query, nothing to emit.
  Duplicate,
  /// Empty after trimming, or the list is full; leave the query alone.
  Rejected,
}

/// Trim `raw` and append it if non-empty, unique, and under `max_tags`.
fn commit_tag(tags: &mut Vec<String>, raw: &str, max_tags: Option<usize>) -> Commit {
  let tag = raw.trim();
  if tag.is_empty() {
    return Commit::Rejected;
  }
  if tags.iter().any(|t| t == tag) {
    return Commit::Duplicate;
  }
  if max_tags.is_some_and(|m| tags.len() >= m) {
    return Commit::Rejected;
  }
  tags.push(tag.to_string());
  Commit::Added
}

/// A tag list editor. Create with `cx.new(|cx| TagsInput::new(cx))`.
pub struct TagsInput {
  tags: Vec<String>,
  query: TextEdit,
  state: LineState,
  focus: FocusHandle,
  placeholder: SharedString,
  label: Option<SharedString>,
  description: Option<SharedString>,
  error: Option<SharedString>,
  max_tags: Option<usize>,
  size: Size,
  disabled: bool,
}

impl EventEmitter<TagsInputEvent> for TagsInput {}

impl TagsInput {
  pub fn new(cx: &mut Context<Self>) -> Self {
    TagsInput {
      tags: Vec::new(),
      query: TextEdit::new(""),
      state: LineState::new(),
      focus: cx.focus_handle().tab_stop(true),
      placeholder: SharedString::default(),
      label: None,
      description: None,
      error: None,
      max_tags: None,
      size: Size::Sm,
      disabled: false,
    }
  }

  /// The initial tags.
  pub fn tags<I, S>(mut self, tags: I) -> Self
  where
    I: IntoIterator<Item = S>,
    S: Into<String>,
  {
    self.tags = tags.into_iter().map(Into::into).collect();
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

  /// Cap the number of tags; commits beyond it are ignored.
  pub fn max_tags(mut self, max_tags: usize) -> Self {
    self.max_tags = Some(max_tags);
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

  /// The current tags.
  pub fn tag_values(&self) -> &[String] {
    &self.tags
  }

  /// Replace the tags programmatically (emits no event).
  pub fn set_tags(&mut self, tags: Vec<String>, cx: &mut Context<Self>) {
    if self.tags != tags {
      self.tags = tags;
      cx.notify();
    }
  }

  /// Two-way bind this input's tags to a `Signal<Vec<String>>`. The signal
  /// is the source of truth: the input adopts its value now, commits and
  /// removals write back through [`Signal::set_if_changed`], and signal
  /// writes replace the pills without emitting [`TagsInputEvent`]. Equality
  /// guards on both directions prevent update loops.
  pub fn bind(entity: &Entity<TagsInput>, signal: &Signal<Vec<String>>, cx: &mut App) {
    let initial = signal.get(cx);
    entity.update(cx, |this, cx| this.set_tags(initial, cx));
    let sink = signal.clone();
    cx.subscribe(entity, move |_input, event: &TagsInputEvent, cx| {
      sink.set_if_changed(cx, event.0.clone());
    })
    .detach();
    let input = entity.downgrade();
    cx.observe(signal.entity(), move |observed, cx| {
      let value = observed.read(cx).clone();
      input.update(cx, |this, cx| this.set_tags(value, cx)).ok();
    })
    .detach();
  }

  fn remove(&mut self, index: usize, cx: &mut Context<Self>) {
    if self.disabled || index >= self.tags.len() {
      return;
    }
    self.tags.remove(index);
    cx.emit(TagsInputEvent(self.tags.clone()));
    cx.notify();
  }

  fn commit(&mut self, cx: &mut Context<Self>) {
    match commit_tag(&mut self.tags, &self.query.text(), self.max_tags) {
      Commit::Added => {
        self.query.set_text("");
        cx.emit(TagsInputEvent(self.tags.clone()));
      }
      Commit::Duplicate => self.query.set_text(""),
      Commit::Rejected => {}
    }
  }

  fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
    if self.disabled {
      return;
    }
    let ks = &event.keystroke;
    // Enter and comma commit instead of editing the query. Catching the
    // comma here, before the key is offered to the platform's input
    // handler, is what keeps it from being typed as a character.
    if ks.key.as_str() == "enter" || ks.key_char.as_deref() == Some(",") {
      self.commit(cx);
      cx.notify();
      cx.stop_propagation();
      return;
    }
    // Backspace in an empty query pops the last pill.
    if ks.key.as_str() == "backspace" && self.query.is_empty() {
      if self.tags.pop().is_some() {
        cx.emit(TagsInputEvent(self.tags.clone()));
      }
      cx.notify();
      cx.stop_propagation();
      return;
    }
    match line::keys(self, event, window, cx) {
      KeyOutcome::Edited => {
        cx.notify();
        cx.stop_propagation();
      }
      // Submit is handled above; Escape and the rest bubble to the host.
      KeyOutcome::Submit | KeyOutcome::Cancel | KeyOutcome::Pass => {}
    }
  }
}

impl LineEditor for TagsInput {
  fn edit(&self) -> &TextEdit {
    &self.query
  }

  fn edit_mut(&mut self) -> &mut TextEdit {
    &mut self.query
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

  fn line_read_only(&self) -> bool {
    self.disabled
  }

  /// A comma pasted mid-string is still a separator, so it commits rather
  /// than landing in the query.
  fn line_filter(&self, text: String) -> String {
    text.replace(',', " ")
  }

  fn line_changed(&mut self, cx: &mut Context<Self>) {
    cx.notify();
  }
}

line::line_input_handler!(TagsInput);
line::line_focus_builders!(TagsInput);

impl Render for TagsInput {
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
    let pill_bg = t.surface_hover().hsla();
    let pill_h = height - 14.0;
    let pill_font = (font - 2.0).max(10.0);

    let mut field = line::wire(div().id("guise-tagsinput"), &self.focus, &self.state, cx)
      .on_key_down(cx.listener(Self::on_key))
      .flex()
      .flex_row()
      .flex_wrap()
      .items_center()
      .gap(px(6.0))
      .min_h(px(height))
      .px(px(pad_x))
      .py(px(5.0))
      .rounded(px(radius))
      .border_1()
      .border_color(border)
      .bg(surface)
      .text_size(px(font));

    for (i, tag) in self.tags.iter().enumerate() {
      let remove = div()
        .id(("guise-tag-remove", i))
        .cursor_pointer()
        .text_color(dimmed)
        .hover(move |s| s.text_color(text_color))
        .child(SharedString::new_static("\u{00d7}"))
        .on_click(cx.listener(move |this, _ev, _window, cx| this.remove(i, cx)));
      field = field.child(
        div()
          .id(("guise-tag", i))
          .flex()
          .items_center()
          .gap(px(4.0))
          .h(px(pill_h))
          .px(px(8.0))
          .rounded(px(pill_h / 2.0))
          .bg(pill_bg)
          .text_size(px(pill_font))
          .text_color(text_color)
          .child(SharedString::from(tag.clone()))
          .child(remove),
      );
    }

    let mut interior = Line::new(cx.entity());
    // Only offer the placeholder while the field is genuinely empty; with
    // pills present it would read as another tag.
    if self.tags.is_empty() {
      interior = interior.placeholder(self.placeholder.clone(), dimmed);
    }
    field = field.child(
      div()
        .flex_1()
        .min_w(px(80.0))
        .line_height(px(font * 1.3))
        .child(interior),
    );

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
    chrome.probe_any("TagsInput")
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn commits_trimmed_unique_tags() {
    let mut tags = Vec::new();
    assert_eq!(commit_tag(&mut tags, "  rust  ", None), Commit::Added);
    assert_eq!(commit_tag(&mut tags, "gpui", None), Commit::Added);
    assert_eq!(tags, vec!["rust".to_string(), "gpui".to_string()]);
  }

  #[test]
  fn rejects_empty_and_whitespace() {
    let mut tags = Vec::new();
    assert_eq!(commit_tag(&mut tags, "", None), Commit::Rejected);
    assert_eq!(commit_tag(&mut tags, "   ", None), Commit::Rejected);
    assert!(tags.is_empty());
  }

  #[test]
  fn detects_duplicates_after_trimming() {
    let mut tags = vec!["rust".to_string()];
    assert_eq!(commit_tag(&mut tags, " rust ", None), Commit::Duplicate);
    assert_eq!(tags.len(), 1);
  }

  #[test]
  fn respects_max_tags() {
    let mut tags = vec!["a".to_string(), "b".to_string()];
    assert_eq!(commit_tag(&mut tags, "c", Some(2)), Commit::Rejected);
    assert_eq!(tags.len(), 2);
    // A duplicate at the cap still reports Duplicate (clears the query).
    assert_eq!(commit_tag(&mut tags, "a", Some(2)), Commit::Duplicate);
  }
}
