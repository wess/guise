//! `AreaChart` — filled line series, stacked by default.
//!
//! Stacking accumulates the layers (`stack_layers`), so each band shows its
//! own contribution and the top edge is the total. `.overlaid()` draws the
//! raw series over each other instead.
//!
//! ```ignore
//! use guise::chart::AreaChart;
//!
//! AreaChart::series("Free", [40.0, 42.0, 45.0])
//!     .add_series("Pro", [12.0, 15.0, 21.0])
//!     .axis()
//!     .labels(["May", "Jun", "Jul"])
//! ```

use gpui::prelude::*;
use gpui::{canvas, div, fill, point, px, size, App, Bounds, Hsla, IntoElement, SharedString, Window};

use crate::style::ColorValue;
use crate::theme::theme;

use super::axis::nice_ticks;
use super::frame::{legend_row, x_label_row, y_axis_column};
use super::{
    min_max, normalize_between, paint_band, paint_polyline_ys, series_color, stack_layers,
};

/// A stacked (or overlaid) area chart over one or more series.
#[derive(IntoElement)]
pub struct AreaChart {
    series: Vec<(Option<SharedString>, Vec<f32>)>,
    colors: Vec<ColorValue>,
    stacked: bool,
    axis: bool,
    labels: Vec<SharedString>,
    stroke: f32,
    width: Option<f32>,
    height: f32,
}

impl AreaChart {
    pub fn new(values: impl IntoIterator<Item = f32>) -> Self {
        AreaChart {
            series: vec![(None, values.into_iter().collect())],
            colors: Vec::new(),
            stacked: true,
            axis: false,
            labels: Vec::new(),
            stroke: 1.5,
            width: None,
            height: 140.0,
        }
    }

    /// Start a named multi-series chart; extend with [`add_series`](Self::add_series).
    pub fn series(
        label: impl Into<SharedString>,
        values: impl IntoIterator<Item = f32>,
    ) -> Self {
        let mut chart = AreaChart::new(values);
        chart.series[0].0 = Some(label.into());
        chart
    }

    /// Add another named series (stacked on top by default).
    pub fn add_series(
        mut self,
        label: impl Into<SharedString>,
        values: impl IntoIterator<Item = f32>,
    ) -> Self {
        self.series
            .push((Some(label.into()), values.into_iter().collect()));
        self
    }

    /// Draw the raw series over each other instead of stacking.
    pub fn overlaid(mut self) -> Self {
        self.stacked = false;
        self
    }

    /// Per-series colors, cycled when shorter than the series list.
    pub fn colors(mut self, colors: impl IntoIterator<Item = impl Into<ColorValue>>) -> Self {
        self.colors = colors.into_iter().map(Into::into).collect();
        self
    }

    /// Show a y-axis with nice-number ticks and aligned gridlines.
    pub fn axis(mut self) -> Self {
        self.axis = true;
        self
    }

    /// Category labels under the plot, one per data point.
    pub fn labels(mut self, labels: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.labels = labels.into_iter().map(Into::into).collect();
        self
    }

    /// Fixed width in px. Defaults to the parent's full width.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Plot height in px (default 140), excluding labels and legend.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }
}

impl RenderOnce for AreaChart {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let colors: Vec<Hsla> = (0..self.series.len())
            .map(|i| series_color(t, &self.colors, i))
            .collect();
        let grid = t.border().alpha(0.5);
        let stroke = self.stroke;
        let stacked = self.stacked;

        let raw: Vec<Vec<f32>> = self.series.iter().map(|(_, v)| v.clone()).collect();
        let layers = if stacked { stack_layers(&raw) } else { raw.clone() };

        // Scale: stacked charts run 0..=top-layer max; overlaid use the
        // combined data range.
        let flat: Vec<f32> = layers.iter().flatten().copied().collect();
        let (lo, hi) = if stacked {
            (0.0, min_max(&flat).map(|(_, hi)| hi).unwrap_or(1.0))
        } else {
            min_max(&flat).unwrap_or((0.0, 1.0))
        };
        let ticks = nice_ticks(lo, hi, 4);
        let (lo, hi) = (*ticks.first().unwrap(), *ticks.last().unwrap());
        let gridline_count = ticks.len().max(2);

        let paint_colors = colors.clone();
        let plot = canvas(
            |_, _, _| (),
            move |bounds, _, window, _cx| {
                let w = f32::from(bounds.size.width);
                let h = f32::from(bounds.size.height);
                if w <= 0.0 || h <= 0.0 {
                    return;
                }
                for i in 0..gridline_count {
                    let y = (h - 1.0) * (i as f32 / (gridline_count - 1) as f32);
                    window.paint_quad(fill(
                        Bounds::new(bounds.origin + point(px(0.0), px(y)), size(px(w), px(1.0))),
                        grid,
                    ));
                }
                let zero = normalize_between(&vec![0.0; 2], lo, hi)[0];
                let mut below: Option<Vec<f32>> = None;
                for (i, layer) in layers.iter().enumerate() {
                    let ys = normalize_between(layer, lo, hi);
                    let color = paint_colors[i];
                    let band = Hsla { a: 0.25, ..color };
                    if stacked {
                        let lower = below
                            .clone()
                            .unwrap_or_else(|| vec![zero; ys.len()]);
                        paint_band(window, bounds, &ys, &lower, band);
                        below = Some(ys.clone());
                    }
                    paint_polyline_ys(
                        window,
                        bounds,
                        &ys,
                        stroke,
                        color,
                        (!stacked).then_some(band),
                    );
                }
            },
        )
        .w_full()
        .h(px(self.height));

        let mut body = div().flex().flex_row().w_full();
        if self.axis {
            body = body.child(y_axis_column(t, &ticks, self.height));
        }
        let mut plot_column = div().flex_1().flex().flex_col().gap(px(4.0)).child(plot);
        if !self.labels.is_empty() {
            plot_column = plot_column.child(x_label_row(t, &self.labels));
        }
        body = body.child(plot_column);

        let legend: Vec<(SharedString, Hsla)> = self
            .series
            .iter()
            .enumerate()
            .filter_map(|(i, (label, _))| label.clone().map(|l| (l, colors[i])))
            .collect();

        let mut root = div().flex().flex_col();
        root = match self.width {
            Some(w) => root.w(px(w)),
            None => root.w_full(),
        };
        root = root.child(body);
        if !legend.is_empty() {
            root = root.child(legend_row(t, &legend));
        }
        root
    }
}
