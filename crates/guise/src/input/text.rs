//! `TextInput` — a stateful single-line text field (gpui entity).
//!
//! Owns its buffer and focus; renders Mantine chrome (label, field,
//! description/error) and emits [`TextInputEvent`] on edit and submit.

use gpui::prelude::*;
use gpui::{
    div, px, ClipboardItem, Context, EventEmitter, FocusHandle, IntoElement, KeyDownEvent,
    MouseButton, SharedString, Window,
};

use super::{apply_key, control_metrics, edit::TextEdit, KeyOutcome};
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
    focus: FocusHandle,
    placeholder: SharedString,
    label: Option<SharedString>,
    description: Option<SharedString>,
    error: Option<SharedString>,
    size: Size,
    radius: Option<Size>,
    disabled: bool,
    password: bool,
}

impl EventEmitter<TextInputEvent> for TextInput {}

impl TextInput {
    pub fn new(cx: &mut Context<Self>) -> Self {
        TextInput {
            edit: TextEdit::new(""),
            focus: cx.focus_handle(),
            placeholder: SharedString::default(),
            label: None,
            description: None,
            error: None,
            size: Size::Sm,
            radius: None,
            disabled: false,
            password: false,
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

    pub fn password(mut self, password: bool) -> Self {
        self.password = password;
        self
    }

    /// The field's focus handle, so a host can focus it on open.
    pub fn focus_handle(&self) -> FocusHandle {
        self.focus.clone()
    }

    /// The current text.
    pub fn text(&self) -> String {
        self.edit.text()
    }

    /// Replace the text programmatically.
    pub fn set_text(&mut self, value: &str, cx: &mut Context<Self>) {
        self.edit = TextEdit::new(value);
        cx.notify();
    }

    /// Copy the selection to the clipboard (never from a password field).
    fn copy(&self, cx: &mut Context<Self>) {
        if !self.password {
            if let Some(text) = self.edit.selected_text() {
                cx.write_to_clipboard(ClipboardItem::new_string(text));
            }
        }
        cx.stop_propagation();
    }

    /// Cut the selection to the clipboard, removing it from the field.
    fn cut(&mut self, cx: &mut Context<Self>) {
        if !self.password {
            if let Some(text) = self.edit.selected_text() {
                cx.write_to_clipboard(ClipboardItem::new_string(text));
                self.edit.delete_selection();
                cx.emit(TextInputEvent::Change(self.edit.text()));
                cx.notify();
            }
        }
        cx.stop_propagation();
    }

    /// Paste clipboard text at the cursor, replacing any selection. Newlines are
    /// flattened to spaces for this single-line field.
    fn paste(&mut self, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|i| i.text()) {
            self.edit.insert(&text.replace(['\n', '\r'], " "));
            cx.emit(TextInputEvent::Change(self.edit.text()));
            cx.notify();
        }
        cx.stop_propagation();
    }

    fn on_key(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        // Clipboard chords need clipboard (App) access, so handle them here
        // rather than in the pure `apply_key`.
        let m = &event.keystroke.modifiers;
        if m.platform && !m.alt && !m.control {
            match event.keystroke.key.as_str() {
                "c" => return self.copy(cx),
                "x" => return self.cut(cx),
                "v" => return self.paste(cx),
                _ => {}
            }
        }
        match apply_key(&mut self.edit, &event.keystroke) {
            KeyOutcome::Submit => {
                cx.emit(TextInputEvent::Submit(self.edit.text()));
                cx.notify();
                cx.stop_propagation();
            }
            KeyOutcome::Edited => {
                cx.emit(TextInputEvent::Change(self.edit.text()));
                cx.notify();
                cx.stop_propagation();
            }
            // Escape (Cancel) and unhandled keys (Tab, Cmd+W, …) bubble to the
            // host: dialogs cancel on Escape, forms move focus on Tab.
            KeyOutcome::Cancel | KeyOutcome::Pass => {}
        }
    }
}

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
        };
        let text_color = t.text().hsla();
        let dimmed = t.dimmed().hsla();
        let surface = t.surface().hsla();
        let caret_color = t.primary().hsla();
        let error_color = t.color(ColorName::Red, if t.scheme.is_dark() { 5 } else { 7 }).hsla();
        let border = border.hsla();
        let font_sm = t.font_size(Size::Sm);
        let font_xs = t.font_size(Size::Xs);

        let mask = |s: String| {
            if self.password {
                "\u{2022}".repeat(s.chars().count())
            } else {
                s
            }
        };

        let mut selection_bg = t.primary().hsla();
        selection_bg.a = 0.30;

        // The interior: a highlighted selection or a caret when focused, else
        // the value or the placeholder.
        let interior = if focused {
            if let Some((before, selected, after)) = self.edit.split_selection() {
                div()
                    .flex()
                    .items_center()
                    .text_color(text_color)
                    .child(SharedString::from(mask(before)))
                    .child(
                        div()
                            .bg(selection_bg)
                            .rounded(px(2.0))
                            .child(SharedString::from(mask(selected))),
                    )
                    .child(SharedString::from(mask(after)))
            } else {
                let (before, after) = self.edit.split();
                div()
                    .flex()
                    .items_center()
                    .text_color(text_color)
                    .child(SharedString::from(mask(before)))
                    .child(div().w(px(1.0)).h(px(font * 1.15)).bg(caret_color))
                    .child(SharedString::from(mask(after)))
            }
        } else if self.edit.is_empty() {
            div()
                .text_color(dimmed)
                .child(self.placeholder.clone())
        } else {
            div()
                .text_color(text_color)
                .child(SharedString::from(mask(self.edit.text())))
        };

        let field = div()
            .id("guise-textinput")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _ev, window, cx| {
                    window.focus(&this.focus, cx);
                    cx.notify();
                }),
            )
            .flex()
            .items_center()
            .h(px(height))
            .px(px(pad_x))
            .rounded(px(radius))
            .border_1()
            .border_color(border)
            .bg(surface)
            .text_size(px(font))
            .child(interior);

        let mut column = div().flex().flex_col().gap(px(4.0));
        if let Some(label) = self.label.clone() {
            column = column.child(
                div()
                    .text_size(px(font_sm))
                    .text_color(text_color)
                    .child(label),
            );
        }
        column = column.child(field);
        if let Some(error) = self.error.clone() {
            column = column.child(
                div()
                    .text_size(px(font_xs))
                    .text_color(error_color)
                    .child(error),
            );
        } else if let Some(description) = self.description.clone() {
            column = column.child(
                div()
                    .text_size(px(font_xs))
                    .text_color(dimmed)
                    .child(description),
            );
        }

        if self.disabled {
            column.opacity(0.6)
        } else {
            column
        }
    }
}
