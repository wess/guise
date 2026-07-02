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
use crate::theme::{theme, Size};

use super::{bar_heights, bar_slot, series_color};

/// A vertical bar chart. Bars scale against the largest value with the
/// baseline at zero; negative values clamp to zero (no downward bars in v1).
/// Bar colors rotate through the theme palette by default.
#[derive(IntoElement)]
pub struct BarChart {
    values: Vec<f32>,
    labels: Vec<SharedString>,
    colors: Vec<ColorValue>,
    gap: f32,
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
        let dimmed = t.dimmed().hsla();
        let font_xs = t.font_size(Size::Xs);
        let fracs = bar_heights(&self.values);
        let gap = self.gap;

        let plot = canvas(
            |_, _, _| (),
            move |bounds, _, window, _cx| {
                let w = f32::from(bounds.size.width);
                let h = f32::from(bounds.size.height);
                if w <= 0.0 || h <= 0.0 {
                    return;
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

        let mut root = div().flex().flex_col().gap(px(4.0));
        root = match self.width {
            Some(w) => root.w(px(w)),
            None => root.w_full(),
        };
        root = root.child(plot);

        // Category labels: one equal-width cell per bar slot, so the row lines
        // up with the bars painted above. Plain divs — no canvas text in v1.
        if !self.labels.is_empty() && n > 0 {
            let cells = (0..n).map(|i| {
                div()
                    .flex_1()
                    .flex()
                    .justify_center()
                    .overflow_hidden()
                    .text_size(px(font_xs))
                    .text_color(dimmed)
                    .child(self.labels.get(i).cloned().unwrap_or_default())
            });
            root = root.child(div().flex().flex_row().w_full().children(cells));
        }
        root
    }
}
