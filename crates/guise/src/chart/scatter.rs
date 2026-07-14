//! `ScatterChart` — (x, y) points with axes and per-point hover readouts.
//!
//! ```ignore
//! use guise::chart::ScatterChart;
//!
//! ScatterChart::series("Trial A", [(1.0, 3.2), (2.0, 4.1), (3.5, 2.8)])
//!     .add_series("Trial B", [(1.5, 2.0), (2.5, 5.5)])
//!     .hover()
//! ```

use gpui::prelude::*;
use gpui::{
    canvas, div, fill, point, px, relative, size, App, Bounds, Hsla, IntoElement, SharedString,
    Window,
};

use crate::overlay::tooltip;
use crate::style::ColorValue;
use crate::theme::theme;

use super::axis::nice_ticks;
use super::frame::{legend_row, y_axis_column};
use super::{min_max, series_color, tick_label};

/// Side (px) of a painted point marker.
const MARKER: f32 = 6.0;

/// A scatter plot over `(x, y)` pairs, one or more series. Axes are always
/// on (a scatter without a scale reads as noise).
#[derive(IntoElement)]
pub struct ScatterChart {
    series: Vec<(Option<SharedString>, Vec<(f32, f32)>)>,
    colors: Vec<ColorValue>,
    hover: bool,
    width: Option<f32>,
    height: f32,
}

impl ScatterChart {
    pub fn new(points: impl IntoIterator<Item = (f32, f32)>) -> Self {
        ScatterChart {
            series: vec![(None, points.into_iter().collect())],
            colors: Vec::new(),
            hover: false,
            width: None,
            height: 180.0,
        }
    }

    /// Start a named multi-series plot; extend with [`add_series`](Self::add_series).
    pub fn series(
        label: impl Into<SharedString>,
        points: impl IntoIterator<Item = (f32, f32)>,
    ) -> Self {
        let mut chart = ScatterChart::new(points);
        chart.series[0].0 = Some(label.into());
        chart
    }

    /// Add another named series.
    pub fn add_series(
        mut self,
        label: impl Into<SharedString>,
        points: impl IntoIterator<Item = (f32, f32)>,
    ) -> Self {
        self.series
            .push((Some(label.into()), points.into_iter().collect()));
        self
    }

    /// Per-series colors, cycled when shorter than the series list.
    pub fn colors(mut self, colors: impl IntoIterator<Item = impl Into<ColorValue>>) -> Self {
        self.colors = colors.into_iter().map(Into::into).collect();
        self
    }

    /// Show each point's `(x, y)` in a tooltip on hover.
    pub fn hover(mut self) -> Self {
        self.hover = true;
        self
    }

    /// Fixed width in px. Defaults to the parent's full width.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Plot height in px (default 180), excluding the x-label row and legend.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }
}

impl RenderOnce for ScatterChart {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let colors: Vec<Hsla> = (0..self.series.len())
            .map(|i| series_color(t, &self.colors, i))
            .collect();
        let grid = t.border().alpha(0.5);
        let dimmed = t.dimmed().hsla();
        let font_xs = t.font_size(crate::theme::Size::Xs);

        let xs: Vec<f32> = self
            .series
            .iter()
            .flat_map(|(_, pts)| pts.iter().map(|p| p.0))
            .collect();
        let ys: Vec<f32> = self
            .series
            .iter()
            .flat_map(|(_, pts)| pts.iter().map(|p| p.1))
            .collect();
        let (x_lo, x_hi) = min_max(&xs).unwrap_or((0.0, 1.0));
        let (y_lo, y_hi) = min_max(&ys).unwrap_or((0.0, 1.0));
        let x_ticks = nice_ticks(x_lo, x_hi, 5);
        let y_ticks = nice_ticks(y_lo, y_hi, 4);
        let (x_lo, x_hi) = (*x_ticks.first().unwrap(), *x_ticks.last().unwrap());
        let (y_lo, y_hi) = (*y_ticks.first().unwrap(), *y_ticks.last().unwrap());

        // Normalized 0..=1 position of a data point (y up).
        let fx = move |x: f32| ((x - x_lo) / (x_hi - x_lo)).clamp(0.0, 1.0);
        let fy = move |y: f32| ((y - y_lo) / (y_hi - y_lo)).clamp(0.0, 1.0);

        let series = self.series.clone();
        let paint_colors = colors.clone();
        let x_grid = x_ticks.len().max(2);
        let y_grid = y_ticks.len().max(2);
        let plot = canvas(
            |_, _, _| (),
            move |bounds, _, window, _cx| {
                let w = f32::from(bounds.size.width);
                let h = f32::from(bounds.size.height);
                if w <= 0.0 || h <= 0.0 {
                    return;
                }
                for i in 0..y_grid {
                    let y = (h - 1.0) * (i as f32 / (y_grid - 1) as f32);
                    window.paint_quad(fill(
                        Bounds::new(bounds.origin + point(px(0.0), px(y)), size(px(w), px(1.0))),
                        grid,
                    ));
                }
                for i in 0..x_grid {
                    let x = (w - 1.0) * (i as f32 / (x_grid - 1) as f32);
                    window.paint_quad(fill(
                        Bounds::new(bounds.origin + point(px(x), px(0.0)), size(px(1.0), px(h))),
                        grid,
                    ));
                }
                for (s, (_, pts)) in series.iter().enumerate() {
                    for &(x, y) in pts {
                        if !x.is_finite() || !y.is_finite() {
                            continue;
                        }
                        let cx0 = w * fx(x) - MARKER / 2.0;
                        let cy0 = h * (1.0 - fy(y)) - MARKER / 2.0;
                        window.paint_quad(
                            fill(
                                Bounds::new(
                                    bounds.origin + point(px(cx0), px(cy0)),
                                    size(px(MARKER), px(MARKER)),
                                ),
                                paint_colors[s],
                            )
                            .corner_radii(px(MARKER / 2.0)),
                        );
                    }
                }
            },
        )
        .w_full()
        .h(px(self.height));

        // Hover targets: small absolutely-positioned cells over each point.
        let mut plot_wrap = div().relative().flex_1().child(plot);
        if self.hover {
            let mut n = 0usize;
            for (label, pts) in &self.series {
                for &(x, y) in pts {
                    if !x.is_finite() || !y.is_finite() {
                        continue;
                    }
                    let text = match label {
                        Some(name) => format!("{name}: ({}, {})", tick_label(x), tick_label(y)),
                        None => format!("({}, {})", tick_label(x), tick_label(y)),
                    };
                    plot_wrap = plot_wrap.child(
                        div()
                            .id(("guise-scatter-pt", n))
                            .absolute()
                            .left(relative(fx(x)))
                            .top(relative(1.0 - fy(y)))
                            .ml(px(-8.0))
                            .mt(px(-8.0))
                            .w(px(16.0))
                            .h(px(16.0))
                            .tooltip(tooltip(text)),
                    );
                    n += 1;
                }
            }
        }

        let x_labels = div()
            .flex()
            .justify_between()
            .w_full()
            .children(x_ticks.iter().map(|tick| {
                div()
                    .text_size(px(font_xs))
                    .text_color(dimmed)
                    .child(SharedString::from(tick_label(*tick)))
            }));

        let legend: Vec<(SharedString, Hsla)> = self
            .series
            .iter()
            .enumerate()
            .filter_map(|(i, (label, _))| label.clone().map(|l| (l, colors[i])))
            .collect();

        let body = div()
            .flex()
            .flex_row()
            .w_full()
            .child(y_axis_column(t, &y_ticks, self.height))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(plot_wrap)
                    .child(x_labels),
            );

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
