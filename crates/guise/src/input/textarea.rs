//! `TextArea` — a multiline text field (gpui entity).
//!
//! Reuses the [`TextEdit`] char model (newline-aware), renders line-by-line with
//! a caret on the active line, and emits [`TextAreaEvent`] on edit. Enter inserts
//! a newline; up/down move between lines keeping the column.

use gpui::prelude::*;
use gpui::{
    div, px, Context, EventEmitter, FocusHandle, IntoElement, KeyDownEvent, MouseButton,
    SharedString, Window,
};

use super::{control_metrics, Field, TextEdit};
use crate::theme::{theme, ColorName, Size};

/// Emitted as the user edits the field. Carries the full new value.
#[derive(Debug, Clone)]
pub struct TextAreaEvent(pub String);

/// A multiline text field. Create with `cx.new(|cx| TextArea::new(cx))`.
pub struct TextArea {
    edit: TextEdit,
    focus: FocusHandle,
    placeholder: SharedString,
    label: Option<SharedString>,
    description: Option<SharedString>,
    error: Option<SharedString>,
    rows: usize,
    size: Size,
    disabled: bool,
}

impl EventEmitter<TextAreaEvent> for TextArea {}

/// A line that renders with height even when empty.
fn line(text: &str) -> SharedString {
    if text.is_empty() {
        SharedString::new_static(" ")
    } else {
        SharedString::from(text.to_string())
    }
}

impl TextArea {
    pub fn new(cx: &mut Context<Self>) -> Self {
        TextArea {
            edit: TextEdit::new(""),
            focus: cx.focus_handle(),
            placeholder: SharedString::default(),
            label: None,
            description: None,
            error: None,
            rows: 3,
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

    pub fn set_text(&mut self, value: &str, cx: &mut Context<Self>) {
        self.edit = TextEdit::new(value);
        cx.notify();
    }

    fn on_key(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        let ks = &event.keystroke;
        let m = &ks.modifiers;
        // Tab and unconsumed shortcuts bubble so the host can act; Escape too.
        // (Enter inserts a newline here — this is a multi-line field.)
        if matches!(ks.key.as_str(), "escape" | "tab") {
            return;
        }
        let edited = match ks.key.as_str() {
            "enter" => {
                self.edit.insert("\n");
                true
            }
            "left" => {
                if m.platform {
                    self.edit.home();
                } else if m.alt {
                    self.edit.word_left();
                } else {
                    self.edit.left();
                }
                true
            }
            "right" => {
                if m.platform {
                    self.edit.end();
                } else if m.alt {
                    self.edit.word_right();
                } else {
                    self.edit.right();
                }
                true
            }
            "up" => {
                self.edit.up();
                true
            }
            "down" => {
                self.edit.down();
                true
            }
            "home" => {
                self.edit.home();
                true
            }
            "end" => {
                self.edit.end();
                true
            }
            "backspace" => {
                if m.platform {
                    self.edit.delete_to_start();
                } else if m.alt {
                    self.edit.delete_word_back();
                } else {
                    self.edit.backspace();
                }
                true
            }
            "delete" => {
                if m.platform {
                    self.edit.delete_to_end();
                } else if m.alt {
                    self.edit.delete_word_forward();
                } else {
                    self.edit.delete();
                }
                true
            }
            "k" if m.control => {
                self.edit.delete_to_end();
                true
            }
            "a" if m.control => {
                self.edit.home();
                true
            }
            "e" if m.control => {
                self.edit.end();
                true
            }
            _ => {
                if !m.platform && !m.control {
                    if let Some(text) = ks.key_char.as_deref().filter(|t| !t.is_empty()) {
                        self.edit.insert(text);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
        };
        if edited {
            cx.emit(TextAreaEvent(self.edit.text()));
            cx.notify();
            cx.stop_propagation();
        }
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
        let caret = t.primary().hsla();

        let mut body = div().flex().flex_col().text_color(text_color);
        if focused {
            let (before, after) = self.edit.split();
            let before_lines: Vec<&str> = before.split('\n').collect();
            let after_lines: Vec<&str> = after.split('\n').collect();
            let last = before_lines.len() - 1;
            for l in &before_lines[..last] {
                body = body.child(div().h(px(line_h)).child(line(l)));
            }
            // Caret line: tail of `before` + caret + head of `after`.
            body = body.child(
                div()
                    .flex()
                    .items_center()
                    .h(px(line_h))
                    .child(SharedString::from(before_lines[last].to_string()))
                    .child(div().w(px(1.0)).h(px(font * 1.15)).bg(caret))
                    .child(SharedString::from(after_lines[0].to_string())),
            );
            for l in &after_lines[1..] {
                body = body.child(div().h(px(line_h)).child(line(l)));
            }
        } else if self.edit.is_empty() {
            body = body
                .text_color(dimmed)
                .child(div().h(px(line_h)).child(self.placeholder.clone()));
        } else {
            for l in self.edit.text().split('\n') {
                body = body.child(div().h(px(line_h)).child(line(l)));
            }
        }

        let field = div()
            .id("guise-textarea")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _ev, window, cx| {
                    window.focus(&this.focus);
                    cx.notify();
                }),
            )
            .flex()
            .items_start()
            .min_h(px(min_h))
            .w_full()
            .px(px(pad_x))
            .py(px(pad_y))
            .rounded(px(radius))
            .border_1()
            .border_color(border)
            .bg(surface)
            .text_size(px(font))
            .child(body);

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
        chrome
    }
}
