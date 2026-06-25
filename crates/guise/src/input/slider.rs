//! `Slider` — a draggable value track (gpui entity).
//!
//! Holds a continuous value in `min..=max` snapped to `step`. The track paints a
//! filled portion and a knob; an overlaid row of invisible segment cells turns
//! clicks into values (gpui doesn't hand elements their own bounds, so position
//! is derived from discrete cells rather than the raw pointer x). Arrow keys
//! nudge by one step. Emits [`SliderEvent`] on change.

use gpui::prelude::*;
use gpui::{
    div, px, relative, Context, EventEmitter, FocusHandle, IntoElement, KeyDownEvent, SharedString,
    Window,
};

use crate::theme::{theme, ColorName, Size};

/// Emitted when the slider value changes.
#[derive(Debug, Clone, Copy)]
pub struct SliderEvent(pub f64);

/// A horizontal slider. Create with `cx.new(|cx| Slider::new(cx))`.
pub struct Slider {
    value: f64,
    min: f64,
    max: f64,
    step: f64,
    color: ColorName,
    focus: FocusHandle,
    disabled: bool,
}

impl EventEmitter<SliderEvent> for Slider {}

impl Slider {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Slider {
            value: 0.0,
            min: 0.0,
            max: 100.0,
            step: 1.0,
            color: ColorName::Blue,
            focus: cx.focus_handle(),
            disabled: false,
        }
    }

    pub fn value(mut self, value: f64) -> Self {
        self.value = value;
        self
    }

    pub fn min(mut self, min: f64) -> Self {
        self.min = min;
        self
    }

    pub fn max(mut self, max: f64) -> Self {
        self.max = max;
        self
    }

    pub fn step(mut self, step: f64) -> Self {
        self.step = step.max(f64::EPSILON);
        self
    }

    pub fn color(mut self, color: ColorName) -> Self {
        self.color = color;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn value_f64(&self) -> f64 {
        self.value
    }

    fn fraction(&self) -> f32 {
        if self.max <= self.min {
            0.0
        } else {
            (((self.value - self.min) / (self.max - self.min)) as f32).clamp(0.0, 1.0)
        }
    }

    fn snap(&self, raw: f64) -> f64 {
        let stepped = (raw / self.step).round() * self.step;
        stepped.clamp(self.min, self.max)
    }

    fn set_value(&mut self, raw: f64, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        let next = self.snap(raw);
        if next != self.value {
            self.value = next;
            cx.emit(SliderEvent(next));
            cx.notify();
        }
    }

    fn segment_count(&self) -> usize {
        (((self.max - self.min) / self.step).round() as usize).clamp(1, 200)
    }

    fn on_key(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "left" | "down" => self.set_value(self.value - self.step, cx),
            "right" | "up" => self.set_value(self.value + self.step, cx),
            "home" => self.set_value(self.min, cx),
            "end" => self.set_value(self.max, cx),
            _ => return,
        }
        cx.stop_propagation();
    }
}

impl Render for Slider {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        let accent = t.color(self.color, t.primary_shade()).hsla();
        let track_color = if t.scheme.is_dark() {
            t.color(ColorName::Dark, 4)
        } else {
            t.color(ColorName::Gray, 2)
        }
        .hsla();
        let knob_bg = t.surface().hsla();
        let frac = self.fraction();

        let knob = div()
            .w(px(16.0))
            .h(px(16.0))
            .rounded(px(8.0))
            .bg(knob_bg)
            .border_2()
            .border_color(accent);

        let fill = div()
            .h_full()
            .w(relative(frac))
            .rounded(px(3.0))
            .bg(accent)
            .flex()
            .items_center()
            .justify_end()
            .child(knob);

        let track = div()
            .relative()
            .w_full()
            .h(px(6.0))
            .rounded(px(3.0))
            .bg(track_color)
            .flex()
            .items_center()
            .child(fill);

        let count = self.segment_count();
        let min = self.min;
        let span = self.max - self.min;
        let mut overlay = div()
            .absolute()
            .top(px(0.0))
            .left(px(0.0))
            .right(px(0.0))
            .bottom(px(0.0))
            .flex()
            .flex_row()
            .items_center();
        for i in 0..count {
            let raw = min + (i as f64) / ((count - 1).max(1) as f64) * span;
            overlay = overlay.child(
                div()
                    .id(("guise-slider-seg", i))
                    .flex_grow()
                    .flex_basis(relative(0.0))
                    .h_full()
                    .on_click(cx.listener(move |this, _ev, _window, cx| this.set_value(raw, cx))),
            );
        }

        let slider = div()
            .id("guise-slider")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key))
            .relative()
            .w_full()
            .h(px(20.0))
            .flex()
            .items_center()
            .child(track)
            .child(overlay);

        // A label keeps the value visible while dragging via clicks.
        let value_label = div()
            .text_size(px(t.font_size(Size::Xs)))
            .text_color(t.dimmed().hsla())
            .child(SharedString::from(format!("{}", self.value)));

        let column = div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .child(slider)
            .child(value_label);

        if self.disabled {
            column.opacity(0.5)
        } else {
            column
        }
    }
}
