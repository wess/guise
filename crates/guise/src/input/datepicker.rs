//! `DatePicker` — a stateful date field (gpui entity).
//!
//! Owns its open state, the visible month, and the selection; renders a
//! trigger plus a deferred [`Calendar`] dropdown and emits
//! [`DatePickerEvent`] on selection. Supports single dates and ranges.

use gpui::prelude::*;
use gpui::{
    deferred, div, px, App, Context, Entity, EventEmitter, FocusHandle, IntoElement, SharedString,
    Window,
};

use super::calendar::Calendar;
use super::control_metrics;
use super::date::{Date, Weekday};
use crate::icon::{Icon, IconName};
use crate::reactive::Signal;
use crate::theme::{theme, Size};

/// Emitted when the user completes a pick.
#[derive(Debug, Clone)]
pub enum DatePickerEvent {
    /// Single-date mode: the chosen date.
    Selected(Date),
    /// Range mode: both endpoints chosen (start ≤ end).
    Range(Date, Date),
}

/// A dropdown date field. Create with `cx.new(|cx| DatePicker::new(cx))`.
pub struct DatePicker {
    open: bool,
    focus: FocusHandle,
    view_year: i32,
    view_month: u32,
    selected: Option<Date>,
    range_mode: bool,
    range_start: Option<Date>,
    range_end: Option<Date>,
    min: Option<Date>,
    max: Option<Date>,
    week_start: Weekday,
    format: SharedString,
    placeholder: SharedString,
    label: Option<SharedString>,
    size: Size,
    disabled: bool,
}

impl EventEmitter<DatePickerEvent> for DatePicker {}

impl DatePicker {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let today = Date::today();
        DatePicker {
            open: false,
            focus: cx.focus_handle(),
            view_year: today.year(),
            view_month: today.month(),
            selected: None,
            range_mode: false,
            range_start: None,
            range_end: None,
            min: None,
            max: None,
            week_start: Weekday::Sunday,
            format: SharedString::new_static("MMM D, YYYY"),
            placeholder: SharedString::new_static("Pick a date"),
            label: None,
            size: Size::Sm,
            disabled: false,
        }
    }

    /// Pick a start and an end instead of a single date.
    pub fn range_mode(mut self) -> Self {
        self.range_mode = true;
        self.placeholder = SharedString::new_static("Pick a range");
        self
    }

    pub fn value(mut self, value: Date) -> Self {
        self.selected = Some(value);
        self.view_year = value.year();
        self.view_month = value.month();
        self
    }

    pub fn min(mut self, min: Date) -> Self {
        self.min = Some(min);
        self
    }

    pub fn max(mut self, max: Date) -> Self {
        self.max = Some(max);
        self
    }

    pub fn week_start(mut self, week_start: Weekday) -> Self {
        self.week_start = week_start;
        self
    }

    /// Display pattern for the trigger (see [`Date::format`]).
    pub fn format(mut self, pattern: impl Into<SharedString>) -> Self {
        self.format = pattern.into();
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

    pub fn selected_date(&self) -> Option<Date> {
        self.selected
    }

    pub fn selected_range(&self) -> Option<(Date, Date)> {
        Some((self.range_start?, self.range_end?))
    }

    /// Two-way bind the single-date selection to a `Signal<Option<Date>>`.
    /// The signal is the source of truth; equality guards on both directions
    /// prevent update loops.
    pub fn bind(entity: &Entity<DatePicker>, signal: &Signal<Option<Date>>, cx: &mut App) {
        let initial = signal.get(cx);
        entity.update(cx, |this, cx| this.sync_selected(initial, cx));
        let sink = signal.clone();
        cx.subscribe(entity, move |_picker, event: &DatePickerEvent, cx| {
            if let DatePickerEvent::Selected(date) = event {
                sink.set_if_changed(cx, Some(*date));
            }
        })
        .detach();
        let picker = entity.downgrade();
        cx.observe(signal.entity(), move |observed, cx| {
            let date = *observed.read(cx);
            picker
                .update(cx, |this, cx| this.sync_selected(date, cx))
                .ok();
        })
        .detach();
    }

    /// Programmatic set: repaint without emitting an event.
    fn sync_selected(&mut self, date: Option<Date>, cx: &mut Context<Self>) {
        if self.selected != date {
            self.selected = date;
            if let Some(date) = date {
                self.view_year = date.year();
                self.view_month = date.month();
            }
            cx.notify();
        }
    }

    fn pick(&mut self, date: Date, cx: &mut Context<Self>) {
        if self.range_mode {
            match (self.range_start, self.range_end) {
                (Some(start), None) => {
                    let (lo, hi) = if date < start {
                        (date, start)
                    } else {
                        (start, date)
                    };
                    self.range_start = Some(lo);
                    self.range_end = Some(hi);
                    self.open = false;
                    cx.emit(DatePickerEvent::Range(lo, hi));
                }
                _ => {
                    self.range_start = Some(date);
                    self.range_end = None;
                }
            }
        } else {
            self.selected = Some(date);
            self.open = false;
            cx.emit(DatePickerEvent::Selected(date));
        }
        cx.notify();
    }

    fn show_month(&mut self, year: i32, month: u32, cx: &mut Context<Self>) {
        self.view_year = year;
        self.view_month = month;
        cx.notify();
    }

    fn trigger_text(&self) -> Option<SharedString> {
        if self.range_mode {
            let start = self.range_start?;
            let text = match self.range_end {
                Some(end) => format!(
                    "{} – {}",
                    start.format(&self.format),
                    end.format(&self.format)
                ),
                None => format!("{} – …", start.format(&self.format)),
            };
            Some(text.into())
        } else {
            Some(self.selected?.format(&self.format).into())
        }
    }
}

impl Render for DatePicker {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        let (height, pad_x, font) = control_metrics(self.size);
        let radius = t.radius(t.default_radius);
        let surface = t.surface().hsla();
        let border = t.border().hsla();
        let text_color = t.text().hsla();
        let dimmed = t.dimmed().hsla();
        let font_sm = t.font_size(Size::Sm);

        let value_text = self.trigger_text();
        let has_value = value_text.is_some();
        let shown: SharedString = value_text.unwrap_or_else(|| self.placeholder.clone());

        let trigger = div()
            .id("guise-datepicker-trigger")
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
                    .child(Icon::new(IconName::CalendarDays).size(Size::Sm)),
            )
            .on_click(cx.listener(|this, _ev, _window, cx| {
                if !this.disabled {
                    this.open = !this.open;
                    cx.notify();
                }
            }));

        let mut wrap = div().relative().child(trigger);

        if self.open && !self.disabled {
            let this = cx.entity().downgrade();
            let month_target = this.clone();
            let mut calendar = Calendar::new("guise-datepicker-calendar")
                .month(self.view_year, self.view_month)
                .week_start(self.week_start)
                .size(self.size)
                .on_select(move |date, _window, cx| {
                    this.update(cx, |picker, cx| picker.pick(date, cx)).ok();
                })
                .on_month_change(move |year, month, _window, cx| {
                    month_target
                        .update(cx, |picker, cx| picker.show_month(year, month, cx))
                        .ok();
                });
            if self.range_mode {
                if let Some(start) = self.range_start {
                    calendar = calendar.range(start, self.range_end);
                }
            } else {
                calendar = calendar.value(self.selected);
            }
            if let Some(min) = self.min {
                calendar = calendar.min(min);
            }
            if let Some(max) = self.max {
                calendar = calendar.max(max);
            }

            let panel = div()
                .absolute()
                .top(px(height + 6.0))
                .left(px(0.0))
                .p(px(10.0))
                .rounded(px(radius))
                .border_1()
                .border_color(border)
                .bg(surface)
                .shadow_md()
                .occlude()
                .child(calendar);
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
