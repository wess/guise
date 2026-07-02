//! `NumberInput` — a numeric text field with stepper buttons (gpui entity).
//!
//! Owns an editable buffer (reusing [`TextEdit`]) constrained to numeric input,
//! plus optional min/max/step. Emits [`NumberInputEvent`] with the parsed value
//! whenever it changes.

use gpui::prelude::*;
use gpui::{
    div, px, App, Context, Entity, EventEmitter, FocusHandle, IntoElement, KeyDownEvent,
    MouseButton, SharedString, Window,
};

use super::{control_metrics, Field, TextEdit};
use crate::icon::{Icon, IconName};
use crate::reactive::Signal;
use crate::theme::{theme, Size};

/// Emitted when the numeric value changes. Carries the parsed value.
#[derive(Debug, Clone, Copy)]
pub struct NumberInputEvent(pub f64);

/// A numeric input. Create with `cx.new(|cx| NumberInput::new(cx))`.
pub struct NumberInput {
    edit: TextEdit,
    focus: FocusHandle,
    min: Option<f64>,
    max: Option<f64>,
    step: f64,
    label: Option<SharedString>,
    description: Option<SharedString>,
    error: Option<SharedString>,
    size: Size,
    disabled: bool,
}

impl EventEmitter<NumberInputEvent> for NumberInput {}

/// Parse a numeric buffer, tolerating surrounding whitespace and a lone `-`.
fn parse_number(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty() || t == "-" {
        return None;
    }
    t.parse::<f64>().ok()
}

fn clamp(v: f64, min: Option<f64>, max: Option<f64>) -> f64 {
    let v = min.map_or(v, |m| v.max(m));
    max.map_or(v, |m| v.min(m))
}

/// Format without a trailing `.0` for whole numbers.
fn format_number(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

impl NumberInput {
    pub fn new(cx: &mut Context<Self>) -> Self {
        NumberInput {
            edit: TextEdit::new(""),
            focus: cx.focus_handle(),
            min: None,
            max: None,
            step: 1.0,
            label: None,
            description: None,
            error: None,
            size: Size::Sm,
            disabled: false,
        }
    }

    pub fn value(mut self, value: f64) -> Self {
        let value = clamp(value, self.min, self.max);
        self.edit = TextEdit::new(&format_number(value));
        self
    }

    pub fn min(mut self, min: f64) -> Self {
        self.min = Some(min);
        self
    }

    pub fn max(mut self, max: f64) -> Self {
        self.max = Some(max);
        self
    }

    pub fn step(mut self, step: f64) -> Self {
        self.step = step;
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

    /// The current parsed value, or `None` if the buffer isn't a number.
    pub fn value_f64(&self) -> Option<f64> {
        parse_number(&self.edit.text())
    }

    /// Two-way bind this input's value to a `Signal<f64>`. The signal is the
    /// source of truth: the input adopts its value now (clamped to min/max),
    /// edits write back through [`Signal::set_if_changed`], and signal writes
    /// replace the buffer without emitting [`NumberInputEvent`]. Equality
    /// guards on both directions prevent update loops.
    pub fn bind(entity: &Entity<NumberInput>, signal: &Signal<f64>, cx: &mut App) {
        let initial = signal.get(cx);
        entity.update(cx, |this, cx| this.sync_value(initial, cx));
        let sink = signal.clone();
        cx.subscribe(entity, move |_input, event: &NumberInputEvent, cx| {
            sink.set_if_changed(cx, event.0);
        })
        .detach();
        let input = entity.downgrade();
        cx.observe(signal.entity(), move |observed, cx| {
            let value = *observed.read(cx);
            input.update(cx, |this, cx| this.sync_value(value, cx)).ok();
        })
        .detach();
    }

    /// Programmatic set: clamp and repaint without emitting an event.
    fn sync_value(&mut self, raw: f64, cx: &mut Context<Self>) {
        let next = clamp(raw, self.min, self.max);
        if self.value_f64() != Some(next) {
            self.edit = TextEdit::new(&format_number(next));
            cx.notify();
        }
    }

    fn nudge(&mut self, dir: f64, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        let current = parse_number(&self.edit.text()).unwrap_or(0.0);
        let next = clamp(current + dir * self.step, self.min, self.max);
        self.edit = TextEdit::new(&format_number(next));
        cx.emit(NumberInputEvent(next));
        cx.notify();
    }

    fn on_key(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        let ks = &event.keystroke;
        if ks.modifiers.platform || ks.modifiers.control {
            return;
        }
        match ks.key.as_str() {
            "up" => return self.nudge(1.0, cx),
            "down" => return self.nudge(-1.0, cx),
            "backspace" => {
                self.edit.backspace();
            }
            "delete" => {
                self.edit.delete();
            }
            "left" => self.edit.left(),
            "right" => self.edit.right(),
            "home" => self.edit.home(),
            "end" => self.edit.end(),
            _ => {
                if let Some(text) = ks.key_char.as_deref().filter(|t| {
                    !t.is_empty()
                        && !ks.modifiers.alt
                        && t.chars()
                            .all(|c| c.is_ascii_digit() || c == '.' || c == '-')
                }) {
                    self.edit.insert(text);
                }
            }
        }
        if let Some(value) = parse_number(&self.edit.text()) {
            cx.emit(NumberInputEvent(value));
        }
        cx.notify();
        cx.stop_propagation();
    }
}

impl Render for NumberInput {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        let (height, pad_x, font) = control_metrics(self.size);
        let radius = t.radius(t.default_radius);
        let focused = self.focus.is_focused(window) && !self.disabled;
        let border = if self.error.is_some() {
            t.color(crate::theme::ColorName::Red, 6)
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
                .child(SharedString::from(before))
                .child(div().w(px(1.0)).h(px(font * 1.15)).bg(caret))
                .child(SharedString::from(after))
        } else if self.edit.is_empty() {
            div()
                .text_color(dimmed)
                .child(SharedString::new_static("0"))
        } else {
            div()
                .text_color(text_color)
                .child(SharedString::from(self.edit.text()))
        };

        let stepper = |id: &'static str, icon: IconName| {
            div()
                .id(id)
                .flex()
                .items_center()
                .justify_center()
                .w(px(20.0))
                .h(px(height / 2.0 - 1.0))
                .text_color(dimmed)
                .hover(move |s| s.text_color(text_color))
                .child(Icon::new(icon).size(Size::Xs))
        };

        let steppers = div()
            .flex()
            .flex_col()
            .border_l_1()
            .border_color(border)
            .child(
                stepper("guise-number-inc", IconName::ChevronUp)
                    .on_click(cx.listener(|this, _ev, _window, cx| this.nudge(1.0, cx))),
            )
            .child(
                stepper("guise-number-dec", IconName::ChevronDown)
                    .on_click(cx.listener(|this, _ev, _window, cx| this.nudge(-1.0, cx))),
            );

        let field = div()
            .id("guise-numberinput")
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
            .h(px(height))
            .pl(px(pad_x))
            .rounded(px(radius))
            .border_1()
            .border_color(border)
            .bg(surface)
            .text_size(px(font))
            .child(interior)
            .child(steppers);

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
