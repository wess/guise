//! `Calendar` — a controlled month grid.
//!
//! The parent owns the displayed month and the selection; the calendar just
//! draws and reports clicks. [`DatePicker`](super::DatePicker) wraps one in a
//! dropdown, but it also works standalone (inline pickers, booking ranges).

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{div, px, App, ElementId, FontWeight, IntoElement, SharedString, Window};

use super::date::{month_grid, Date, Weekday};
use crate::icon::{Icon, IconName};
use crate::theme::{theme, Size};

type SelectHandler = Rc<dyn Fn(Date, &mut Window, &mut App) + 'static>;
type MonthHandler = Rc<dyn Fn(i32, u32, &mut Window, &mut App) + 'static>;

/// Day-cell edge (px) per size token.
fn cell_size(size: Size) -> f32 {
    match size {
        Size::Xs => 26.0,
        Size::Sm => 30.0,
        Size::Md => 34.0,
        Size::Lg => 40.0,
        Size::Xl => 46.0,
    }
}

/// A controlled month-view calendar. Pass the visible `month`, the current
/// selection (`value` or `range`), and handlers; the parent owns all state.
#[derive(IntoElement)]
pub struct Calendar {
    id: ElementId,
    year: i32,
    month: u32,
    value: Option<Date>,
    range: Option<(Date, Option<Date>)>,
    min: Option<Date>,
    max: Option<Date>,
    week_start: Weekday,
    size: Size,
    on_select: Option<SelectHandler>,
    on_month_change: Option<MonthHandler>,
}

impl Calendar {
    /// A calendar showing the month containing `Date::today()`.
    pub fn new(id: impl Into<ElementId>) -> Self {
        let today = Date::today();
        Calendar {
            id: id.into(),
            year: today.year(),
            month: today.month(),
            value: None,
            range: None,
            min: None,
            max: None,
            week_start: Weekday::Sunday,
            size: Size::Sm,
            on_select: None,
            on_month_change: None,
        }
    }

    /// The month to display (`month` is 1–12).
    pub fn month(mut self, year: i32, month: u32) -> Self {
        self.year = year;
        self.month = month.clamp(1, 12);
        self
    }

    /// Single selected date.
    pub fn value(mut self, value: Option<Date>) -> Self {
        self.value = value;
        self
    }

    /// Selected range: a start and an optional end (while picking).
    pub fn range(mut self, start: Date, end: Option<Date>) -> Self {
        self.range = Some((start, end));
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

    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// A day cell was clicked (never fires for min/max-disabled days).
    pub fn on_select(mut self, handler: impl Fn(Date, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    /// Prev/next was clicked; receives the new (year, month) to display.
    pub fn on_month_change(
        mut self,
        handler: impl Fn(i32, u32, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_month_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Calendar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let cell = cell_size(self.size);
        let font = t.font_size(self.size);
        let radius = t.radius(Size::Xs) + 2.0;
        let text_color = t.text().hsla();
        let dimmed = t.dimmed().hsla();
        let surface_hover = t.surface_hover().hsla();
        let accent = t.primary();
        let accent_bg = accent.hsla();
        let accent_fg = accent.contrasting().hsla();
        let in_range_bg = accent.alpha(0.12);

        let today = Date::today();
        let first = Date::new(self.year, self.month, 1).unwrap_or(today);
        let title: SharedString = format!("{} {}", first.month_name(), self.year).into();

        let mut header = div().flex().items_center().justify_between().child(
            div()
                .text_size(px(font))
                .font_weight(FontWeight::MEDIUM)
                .text_color(text_color)
                .child(title),
        );
        let mut nav = div().flex().items_center().gap(px(4.0));
        for (key, icon, delta) in [
            ("guise-cal-prev", IconName::ChevronLeft, -1),
            ("guise-cal-next", IconName::ChevronRight, 1),
        ] {
            let target = first.add_months(delta);
            let handler = self.on_month_change.clone();
            nav = nav.child(
                div()
                    .id(key)
                    .flex()
                    .items_center()
                    .justify_center()
                    .w(px(cell * 0.8))
                    .h(px(cell * 0.8))
                    .rounded(px(radius))
                    .text_color(dimmed)
                    .hover(move |s| s.bg(surface_hover))
                    .child(Icon::new(icon).size(Size::Sm))
                    .on_click(move |_ev, window, cx| {
                        if let Some(handler) = &handler {
                            handler(target.year(), target.month(), window, cx);
                        }
                    }),
            );
        }
        header = header.child(nav);

        let mut weekdays = div().flex();
        for i in 0..7 {
            let day = Weekday::from_index(self.week_start.index() + i);
            weekdays = weekdays.child(
                div()
                    .w(px(cell))
                    .h(px(cell * 0.8))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(font - 3.0))
                    .text_color(dimmed)
                    .child(SharedString::new_static(day.short())),
            );
        }

        let grid = month_grid(self.year, self.month, self.week_start);
        let (range_start, range_end) = match self.range {
            Some((start, end)) => (Some(start), end),
            None => (None, None),
        };

        let mut days = div().flex().flex_col();
        for week in grid.chunks(7) {
            let mut row = div().flex();
            for date in week {
                let date = *date;
                let outside = date.month() != self.month;
                let disabled = self.min.is_some_and(|min| date < min)
                    || self.max.is_some_and(|max| date > max);
                let selected = self.value == Some(date)
                    || range_start == Some(date)
                    || range_end == Some(date);
                let in_range = match (range_start, range_end) {
                    (Some(start), Some(end)) => date > start && date < end,
                    _ => false,
                };

                let mut day = div()
                    .id(("guise-cal-day", date.to_days() as usize))
                    .w(px(cell))
                    .h(px(cell))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(radius))
                    .text_size(px(font - 1.0))
                    .text_color(if outside { dimmed } else { text_color })
                    .child(SharedString::from(date.day().to_string()));

                if selected {
                    day = day
                        .bg(accent_bg)
                        .text_color(accent_fg)
                        .font_weight(FontWeight::MEDIUM);
                } else if in_range {
                    day = day.bg(in_range_bg);
                } else if date == today {
                    day = day.border_1().border_color(accent_bg);
                }

                if disabled {
                    day = day.opacity(0.35);
                } else {
                    if !selected {
                        day = day.hover(move |s| s.bg(surface_hover));
                    }
                    let handler = self.on_select.clone();
                    day = day.on_click(move |_ev, window, cx| {
                        if let Some(handler) = &handler {
                            handler(date, window, cx);
                        }
                    });
                }
                row = row.child(day);
            }
            days = days.child(row);
        }

        div()
            .id(self.id)
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(header)
            .child(weekdays)
            .child(days)
    }
}
