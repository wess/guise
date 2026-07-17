//! `BarChart` — vertical bars over a value series, with optional category
//! labels below the bars.
//!
//! ```ignore
//! use guise::chart::BarChart;
//!
//! BarChart::entries([("Mon", 12.0), ("Tue", 9.0), ("Wed", 15.0)]).gap(0.3)
//! ```

use gpui::prelude::*;
use gpui::{
    canvas, div, fill, point, px, size, App, Bounds, Hsla, IntoElement, SharedString, Window,
};

use crate::style::ColorValue;
use crate::theme::theme;

use super::axis::nice_ticks;
use super::frame::{hover_slots, x_label_row, y_axis_column};
use super::{bar_heights, bar_slot, series_color, tick_label};

/// A vertical bar chart. Bars scale against the largest value with the
/// baseline at zero; negative values clamp to zero (no downward bars in v1).
/// Bar colors rotate through the theme palette by default.
#[derive(IntoElement)]
pub struct BarChart {
    values: Vec<f32>,
    labels: Vec<SharedString>,
    colors: Vec<ColorValue>,
    gap: f32,
    axis: bool,
    hover: bool,
    width: Option<f32>,
    height: f32,
}

impl BarChart {
    pub fn new(values: impl IntoIterator<Item = f32>) -> Self {
        BarChart {
            values: values.into_iter().collect(),
            labels: Vec::new(),
            colors: Vec::new(),
            gap: 0.2,
            axis: false,
            hover: false,
            width: None,
            height: 140.0,
        }
    }

    /// Build from `(label, value)` pairs; labels render below the bars.
    pub fn entries(entries: impl IntoIterator<Item = (impl Into<SharedString>, f32)>) -> Self {
        let (labels, values): (Vec<SharedString>, Vec<f32>) = entries
            .into_iter()
            .map(|(label, value)| (label.into(), value))
            .unzip();
        BarChart {
            labels,
            ..BarChart::new(values)
        }
    }

    /// One color for every bar (disables the palette rotation).
    pub fn color(mut self, color: impl Into<ColorValue>) -> Self {
        self.colors = vec![color.into()];
        self
    }

    /// Per-bar colors, cycled when shorter than the series.
    pub fn colors(mut self, colors: impl IntoIterator<Item = impl Into<ColorValue>>) -> Self {
        self.colors = colors.into_iter().map(Into::into).collect();
        self
    }

    /// Fraction of each bar slot left empty, `0.0..=0.9` (default 0.2).
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap.clamp(0.0, 0.9);
        self
    }

    /// Show a y-axis: nice-number ticks from zero, gridlines aligned to
    /// them, bars scaled against the top tick.
    pub fn axis(mut self) -> Self {
        self.axis = true;
        self
    }

    /// Show each bar's value in a tooltip on hover.
    pub fn hover(mut self) -> Self {
        self.hover = true;
        self
    }

    /// Fixed width in px. Defaults to the parent's full width.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Plot height in px (default 140), excluding the label row.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }
}

impl RenderOnce for BarChart {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let n = self.values.len();
        let bar_colors: Vec<Hsla> = (0..n).map(|i| series_color(t, &self.colors, i)).collect();
        let grid = t.border().alpha(0.5);
        let gap = self.gap;

        // Axis mode scales bars against the top nice tick (so the tallest
        // bar lines up with a labeled gridline); otherwise against the max.
        let ticks = if self.axis {
            let max = self.values.iter().copied().fold(0.0_f32, f32::max);
            nice_ticks(0.0, max.max(1.0), 4)
        } else {
            Vec::new()
        };
        let fracs: Vec<f32> = match ticks.last() {
            Some(&top) if top > 0.0 => self
                .values
                .iter()
                .map(|&v| {
                    if v.is_finite() {
                        (v.max(0.0) / top).min(1.0)
                    } else {
                        0.0
                    }
                })
                .collect(),
            _ => bar_heights(&self.values),
        };
        let gridline_count = ticks.len();

        let plot = canvas(
            |_, _, _| (),
            move |bounds, _, window, _cx| {
                let w = f32::from(bounds.size.width);
                let h = f32::from(bounds.size.height);
                if w <= 0.0 || h <= 0.0 {
                    return;
                }
                for i in 0..gridline_count {
                    let y = (h - 1.0) * (i as f32 / (gridline_count - 1).max(1) as f32);
                    window.paint_quad(fill(
                        Bounds::new(bounds.origin + point(px(0.0), px(y)), size(px(w), px(1.0))),
                        grid,
                    ));
                }
                for (i, frac) in fracs.iter().enumerate() {
                    let bh = h * frac;
                    if bh <= 0.0 {
                        continue;
                    }
                    let (x, bw) = bar_slot(i, fracs.len(), w, gap);
                    window.paint_quad(fill(
                        Bounds::new(
                            bounds.origin + point(px(x), px(h - bh)),
                            size(px(bw), px(bh)),
                        ),
                        bar_colors[i],
                    ));
                }
            },
        )
        .w_full()
        .h(px(self.height));

        // Per-bar hover readouts layered over the plot.
        let mut plot_wrap = div().relative().w_full().child(plot);
        if self.hover && n > 0 {
            let texts: Vec<SharedString> = (0..n)
                .map(|i| {
                    let value = tick_label(self.values[i]);
                    match self.labels.get(i) {
                        Some(label) => format!("{label}: {value}").into(),
                        None => value.into(),
                    }
                })
                .collect();
            plot_wrap = plot_wrap.child(hover_slots("guise-barchart-hover".into(), texts));
        }

        let mut plot_column = div()
            .flex_1()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .child(plot_wrap);
        // Category labels: one equal-width cell per bar slot, so the row lines
        // up with the bars painted above. Plain divs — no canvas text in v1.
        if !self.labels.is_empty() && n > 0 {
            plot_column = plot_column.child(x_label_row(t, &self.labels));
        }

        let mut body = div().flex().flex_row();
        body = match self.width {
            Some(w) => body.w(px(w)),
            None => body.w_full(),
        };
        if self.axis {
            body = body.child(y_axis_column(t, &ticks, self.height));
        }
        body.child(plot_column)
    }
}
