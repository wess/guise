//! Shared chart chrome: the y-axis label column, x-label row, legend row,
//! and per-slot hover targets. Every axis-capable chart composes these so
//! the frames look identical across chart types.

use gpui::prelude::*;
use gpui::{div, px, Div, ElementId, Hsla, SharedString, Stateful};

use crate::overlay::tooltip;
use crate::theme::{Size, Theme};

use super::axis::tick_label;

/// Width (px) reserved for the y-axis label column.
pub(crate) const Y_AXIS_WIDTH: f32 = 36.0;

/// The y-axis labels, top (max) to bottom (min), spaced to line up with
/// gridlines painted at even tick fractions.
pub(crate) fn y_axis_column(t: &Theme, ticks: &[f32], height: f32) -> Div {
    let dimmed = t.dimmed().hsla();
    let font_xs = t.font_size(Size::Xs);
    let mut column = div()
        .flex()
        .flex_col()
        .justify_between()
        .items_end()
        .w(px(Y_AXIS_WIDTH))
        .h(px(height))
        .pr(px(6.0));
    for tick in ticks.iter().rev() {
        column = column.child(
            div()
                .text_size(px(font_xs))
                .text_color(dimmed)
                .child(SharedString::from(tick_label(*tick))),
        );
    }
    column
}

/// Equal-width category labels under a plot (one cell per slot).
pub(crate) fn x_label_row(t: &Theme, labels: &[SharedString]) -> Div {
    let dimmed = t.dimmed().hsla();
    let font_xs = t.font_size(Size::Xs);
    let cells = labels.iter().map(|label| {
        div()
            .flex_1()
            .flex()
            .justify_center()
            .overflow_hidden()
            .text_size(px(font_xs))
            .text_color(dimmed)
            .child(label.clone())
    });
    div().flex().flex_row().w_full().children(cells)
}

/// A legend row: colored dot + series label per entry.
pub(crate) fn legend_row(t: &Theme, entries: &[(SharedString, Hsla)]) -> Div {
    let text = t.text().hsla();
    let font_xs = t.font_size(Size::Xs);
    let mut row = div().flex().flex_wrap().gap(px(12.0)).pt(px(4.0));
    for (label, color) in entries {
        row = row.child(
            div()
                .flex()
                .items_center()
                .gap(px(5.0))
                .child(div().w(px(8.0)).h(px(8.0)).rounded_full().bg(*color))
                .child(
                    div()
                        .text_size(px(font_xs))
                        .text_color(text)
                        .child(label.clone()),
                ),
        );
    }
    row
}

/// An invisible overlay of equal-width hover targets across a plot, one per
/// data slot, each showing a tooltip. Absolutely positioned to cover the
/// canvas it sits after (parent must be `relative()`).
pub(crate) fn hover_slots(id: ElementId, texts: Vec<SharedString>) -> Stateful<Div> {
    let mut row = div().id(id).absolute().inset_0().flex().flex_row();
    for (i, text) in texts.into_iter().enumerate() {
        row = row.child(
            div()
                .id(("guise-chart-slot", i))
                .flex_1()
                .h_full()
                .tooltip(tooltip(text)),
        );
    }
    row
}
