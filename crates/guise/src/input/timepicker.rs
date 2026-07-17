//! `TimePicker` — a stateful time-of-day field (gpui entity).
//!
//! A trigger plus a deferred dropdown of hour/minute (and AM/PM) columns.
//! Emits [`TimePickerEvent`] whenever the value changes; picking a minute
//! closes the dropdown.

use gpui::prelude::*;
use gpui::{
    deferred, div, px, App, Context, Div, Entity, EventEmitter, FocusHandle, IntoElement,
    SharedString, Stateful, Window,
};

use super::control_metrics;
use super::time::Time;
use crate::icon::{Icon, IconName};
use crate::reactive::Signal;
use crate::theme::{theme, Size};

/// Emitted whenever the picked time changes. Carries the new value.
#[derive(Debug, Clone)]
pub struct TimePickerEvent(pub Time);

/// A dropdown time field. Create with `cx.new(|cx| TimePicker::new(cx))`.
pub struct TimePicker {
    open: bool,
    focus: FocusHandle,
    value: Option<Time>,
    twelve_hour: bool,
    minute_step: u32,
    placeholder: SharedString,
    label: Option<SharedString>,
    size: Size,
    disabled: bool,
}

impl EventEmitter<TimePickerEvent> for TimePicker {}

impl TimePicker {
    pub fn new(cx: &mut Context<Self>) -> Self {
        TimePicker {
            open: false,
            focus: cx.focus_handle(),
            value: None,
            twelve_hour: true,
            minute_step: 5,
            placeholder: SharedString::new_static("Pick a time"),
            label: None,
            size: Size::Sm,
            disabled: false,
        }
    }

    pub fn value(mut self, value: Time) -> Self {
        self.value = Some(value);
        self
    }

    /// Show a 24-hour clock (default is 12-hour with AM/PM).
    pub fn twenty_four_hour(mut self) -> Self {
        self.twelve_hour = false;
        self
    }

    /// Minute list granularity (default 5; use 1, 5, 10, 15, 30…).
    pub fn minute_step(mut self, step: u32) -> Self {
        self.minute_step = step.clamp(1, 30);
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

    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn time(&self) -> Option<Time> {
        self.value
    }

    /// Two-way bind the value to a `Signal<Option<Time>>`. The signal is the
    /// source of truth; equality guards on both directions prevent loops.
    pub fn bind(entity: &Entity<TimePicker>, signal: &Signal<Option<Time>>, cx: &mut App) {
        let initial = signal.get(cx);
        entity.update(cx, |this, cx| this.sync_value(initial, cx));
        let sink = signal.clone();
        cx.subscribe(entity, move |_picker, event: &TimePickerEvent, cx| {
            sink.set_if_changed(cx, Some(event.0));
        })
        .detach();
        let picker = entity.downgrade();
        cx.observe(signal.entity(), move |observed, cx| {
            let time = *observed.read(cx);
            picker.update(cx, |this, cx| this.sync_value(time, cx)).ok();
        })
        .detach();
    }

    fn sync_value(&mut self, time: Option<Time>, cx: &mut Context<Self>) {
        if self.value != time {
            self.value = time;
            cx.notify();
        }
    }

    fn base(&self) -> Time {
        self.value
            .unwrap_or_else(|| Time::new(12, 0).expect("noon is valid"))
    }

    fn set_value(&mut self, time: Time, close: bool, cx: &mut Context<Self>) {
        self.value = Some(time);
        if close {
            self.open = false;
        }
        cx.emit(TimePickerEvent(time));
        cx.notify();
    }

    /// One scrollable option column of the dropdown.
    fn column(
        &self,
        id: &'static str,
        entries: Vec<(SharedString, Time, bool, bool)>,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let t = theme(cx);
        let font = t.font_size(self.size);
        let text_color = t.text().hsla();
        let surface_hover = t.surface_hover().hsla();
        let accent = t.primary();
        let accent_bg = accent.hsla();
        let accent_fg = accent.contrasting().hsla();

        let mut column = div()
            .id(id)
            .flex()
            .flex_col()
            .gap(px(2.0))
            .max_h(px(200.0))
            .overflow_y_scroll()
            .pr(px(2.0));
        for (i, (text, next, is_selected, closes)) in entries.into_iter().enumerate() {
            let mut option = div()
                .id((id, i))
                .px(px(10.0))
                .py(px(4.0))
                .rounded(px(4.0))
                .text_size(px(font))
                .text_color(text_color)
                .child(text)
                .on_click(cx.listener(move |this, _ev, _window, cx| {
                    this.set_value(next, closes, cx);
                }));
            if is_selected {
                option = option.bg(accent_bg).text_color(accent_fg);
            } else {
                option = option.hover(move |s| s.bg(surface_hover));
            }
            column = column.child(option);
        }
        column
    }
}

impl Render for TimePicker {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        let (height, pad_x, font) = control_metrics(self.size);
        let radius = t.radius(t.default_radius);
        let surface = t.surface().hsla();
        let border = t.border().hsla();
        let text_color = t.text().hsla();
        let dimmed = t.dimmed().hsla();
        let font_sm = t.font_size(Size::Sm);

        let has_value = self.value.is_some();
        let shown: SharedString = match self.value {
            Some(time) if self.twelve_hour => time.format_12().into(),
            Some(time) => time.format_24().into(),
            None => self.placeholder.clone(),
        };

        let trigger = div()
            .id("guise-timepicker-trigger")
            .track_focus(&self.focus)
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
            .text_color(if has_value { text_color } else { dimmed })
            .child(shown)
            .child(
                div()
                    .text_color(dimmed)
                    .child(Icon::new(IconName::Clock).size(Size::Sm)),
            )
            .on_click(cx.listener(|this, _ev, _window, cx| {
                if !this.disabled {
                    this.open = !this.open;
                    cx.notify();
                }
            }));

        let mut wrap = div().relative().child(trigger);

        if self.open && !self.disabled {
            let base = self.base();
            let selected = self.value;

            let hours: Vec<(SharedString, Time, bool, bool)> = if self.twelve_hour {
                let (sel_hour, sel_pm) = base.hour_12();
                (0..12)
                    .map(|i| {
                        let display = if i == 0 { 12 } else { i };
                        let next = base.with_hour_12(display, sel_pm);
                        (
                            SharedString::from(display.to_string()),
                            next,
                            selected.is_some() && display == sel_hour,
                            false,
                        )
                    })
                    .collect()
            } else {
                (0..24)
                    .map(|hour| {
                        (
                            SharedString::from(format!("{hour:02}")),
                            base.with_hour(hour),
                            selected.is_some() && hour == base.hour(),
                            false,
                        )
                    })
                    .collect()
            };

            let minutes: Vec<(SharedString, Time, bool, bool)> = (0..60)
                .step_by(self.minute_step as usize)
                .map(|minute| {
                    (
                        SharedString::from(format!("{minute:02}")),
                        base.with_minute(minute),
                        selected.is_some() && minute == base.minute(),
                        true,
                    )
                })
                .collect();

            let mut panel = div()
                .absolute()
                .top(px(height + 6.0))
                .left(px(0.0))
                .flex()
                .gap(px(6.0))
                .p(px(8.0))
                .rounded(px(radius))
                .border_1()
                .border_color(border)
                .bg(surface)
                .shadow_md()
                .occlude()
                .child(self.column("guise-timepicker-hours", hours, cx))
                .child(self.column("guise-timepicker-minutes", minutes, cx));

            if self.twelve_hour {
                let (_, pm) = base.hour_12();
                let meridiem: Vec<(SharedString, Time, bool, bool)> = [(false, "AM"), (true, "PM")]
                    .into_iter()
                    .map(|(is_pm, label)| {
                        let (hour, _) = base.hour_12();
                        (
                            SharedString::new_static(label),
                            base.with_hour_12(hour, is_pm),
                            selected.is_some() && pm == is_pm,
                            false,
                        )
                    })
                    .collect();
                panel = panel.child(self.column("guise-timepicker-meridiem", meridiem, cx));
            }

            wrap = wrap.child(deferred(panel));
        }

        let mut column = div().flex().flex_col().gap(px(4.0));
        if let Some(label) = self.label.clone() {
            column = column.child(
                div()
                    .text_size(px(font_sm))
                    .text_color(text_color)
                    .child(label),
            );
        }
        column = column.child(wrap);

        if self.disabled {
            column.opacity(0.6)
        } else {
            column
        }
    }
}
