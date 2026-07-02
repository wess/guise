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
    MouseButton, SharedString, Window,
};

use super::{apply_key, control_metrics, edit::TextEdit, Field, KeyOutcome};
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
    focus: FocusHandle,
    visible: bool,
    placeholder: SharedString,
    label: Option<SharedString>,
    description: Option<SharedString>,
    error: Option<SharedString>,
    size: Size,
    disabled: bool,
}

impl EventEmitter<PasswordInputEvent> for PasswordInput {}

impl PasswordInput {
    pub fn new(cx: &mut Context<Self>) -> Self {
        PasswordInput {
            edit: TextEdit::new(""),
            focus: cx.focus_handle(),
            visible: false,
            placeholder: SharedString::default(),
            label: None,
            description: None,
            error: None,
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

    fn on_key(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        match apply_key(&mut self.edit, &event.keystroke) {
            KeyOutcome::Submit => {
                cx.emit(PasswordInputEvent::Submit(self.edit.text()));
                cx.notify();
                cx.stop_propagation();
            }
            KeyOutcome::Edited => {
                cx.emit(PasswordInputEvent::Change(self.edit.text()));
                cx.notify();
                cx.stop_propagation();
            }
            // Escape and unhandled keys (Tab, Cmd+W, …) bubble to the host.
            KeyOutcome::Cancel | KeyOutcome::Pass => {}
        }
    }

    fn mask(&self, s: String) -> String {
        if self.visible {
            s
        } else {
            "\u{2022}".repeat(s.chars().count())
        }
    }
}

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
        let caret = t.primary().hsla();

        let interior = if focused {
            let (before, after) = self.edit.split();
            div()
                .flex()
                .items_center()
                .text_color(text_color)
                .child(SharedString::from(self.mask(before)))
                .child(div().w(px(1.0)).h(px(font * 1.15)).bg(caret))
                .child(SharedString::from(self.mask(after)))
        } else if self.edit.is_empty() {
            div().text_color(dimmed).child(self.placeholder.clone())
        } else {
            div()
                .text_color(text_color)
                .child(SharedString::from(self.mask(self.edit.text())))
        };

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

        let field = div()
            .id("guise-passwordinput")
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
            .justify_between()
            .gap(px(8.0))
            .h(px(height))
            .px(px(pad_x))
            .rounded(px(radius))
            .border_1()
            .border_color(border)
            .bg(surface)
            .text_size(px(font))
            .child(interior)
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
        chrome
    }
}
