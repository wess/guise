//! `LineChart` — one or more line series with optional axis, legend, area
//! fill, and hover value readouts.
//!
//! ```ignore
//! use guise::chart::LineChart;
//!
//! LineChart::new([12.0, 18.0, 9.0, 24.0]).fill().height(180.0)
//!
//! LineChart::series("Revenue", [12.0, 18.0, 24.0])
//!     .add_series("Costs", [8.0, 11.0, 13.0])
//!     .axis()
//!     .labels(["Q1", "Q2", "Q3"])
//!     .hover()
//! ```

use gpui::prelude::*;
use gpui::{
    canvas, div, fill, point, px, size, App, Bounds, Hsla, IntoElement, SharedString, Window,
};

use crate::style::ColorValue;
use crate::theme::theme;

use super::axis::nice_ticks;
use super::frame::{hover_slots, legend_row, x_label_row, y_axis_column};
use super::{
    min_max, normalize_between, paint_polyline, paint_polyline_ys, resolve_color, series_color,
    tick_label,
};

/// How many horizontal gridlines an axis-free `LineChart` paints.
const GRIDLINES: usize = 4;

/// A line chart. One series (`new`) or several (`series`, chained); values
/// are min/max normalized, or scaled to nice axis ticks with [`LineChart::axis`].
#[derive(IntoElement)]
pub struct LineChart {
    series: Vec<(Option<SharedString>, Vec<f32>)>,
    colors: Vec<ColorValue>,
    stroke: f32,
    fill: bool,
    axis: bool,
    hover: bool,
    labels: Vec<SharedString>,
    width: Option<f32>,
    height: f32,
}

impl LineChart {
    pub fn new(values: impl IntoIterator<Item = f32>) -> Self {
        LineChart {
            series: vec![(None, values.into_iter().collect())],
            colors: Vec::new(),
            stroke: 2.0,
            fill: false,
            axis: false,
            hover: false,
            labels: Vec::new(),
            width: None,
            height: 140.0,
        }
    }

    /// Start a named multi-series chart; extend with
    /// [`add_series`](Self::add_series). Named series show in the legend and
    /// every series shares the y scale.
    pub fn series(label: impl Into<SharedString>, values: impl IntoIterator<Item = f32>) -> Self {
        let mut chart = LineChart::new(values);
        chart.series[0].0 = Some(label.into());
        chart
    }

    /// Add another named series.
    pub fn add_series(
        mut self,
        label: impl Into<SharedString>,
        values: impl IntoIterator<Item = f32>,
    ) -> Self {
        self.series
            .push((Some(label.into()), values.into_iter().collect()));
        self
    }

    /// One color for every line (single-series) — defaults to theme primary.
    pub fn color(mut self, color: impl Into<ColorValue>) -> Self {
        self.colors = vec![color.into()];
        self
    }

    /// Per-series colors, cycled when shorter than the series list.
    pub fn colors(mut self, colors: impl IntoIterator<Item = impl Into<ColorValue>>) -> Self {
        self.colors = colors.into_iter().map(Into::into).collect();
        self
    }

    /// Stroke width in px (default 2).
    pub fn stroke(mut self, width: f32) -> Self {
        self.stroke = width.max(0.5);
        self
    }

    /// Fill the area under each line (line color at 0.15 alpha).
    pub fn fill(mut self) -> Self {
        self.fill = true;
        self
    }

    /// Show a y-axis: nice-number tick labels on the left, gridlines aligned
    /// to them, and the lines scaled against the tick range.
    pub fn axis(mut self) -> Self {
        self.axis = true;
        self
    }

    /// Show per-point values in a tooltip as the pointer moves across.
    pub fn hover(mut self) -> Self {
        self.hover = true;
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

impl RenderOnce for LineChart {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let multi = self.series.len() > 1;
        let line_colors: Vec<Hsla> = (0..self.series.len())
            .map(|i| {
                if !multi && self.colors.is_empty() {
                    t.primary().hsla()
                } else if self.colors.len() == 1 && !multi {
                    resolve_color(t, self.colors[0])
                } else {
                    series_color(t, &self.colors, i)
                }
            })
            .collect();
        let grid = t.border().alpha(0.5);
        let stroke = self.stroke;
        let filled = self.fill;

        // Shared scale across every series.
        let all: Vec<f32> = self
            .series
            .iter()
            .flat_map(|(_, values)| values.iter().copied())
            .collect();
        let ticks = if self.axis {
            let (lo, hi) = min_max(&all).unwrap_or((0.0, 1.0));
            nice_ticks(lo, hi, 4)
        } else {
            Vec::new()
        };
        let scale = (!ticks.is_empty()).then(|| (*ticks.first().unwrap(), *ticks.last().unwrap()));

        let point_count = self.series.iter().map(|(_, v)| v.len()).max().unwrap_or(0);
        let gridline_count = if self.axis {
            ticks.len().max(2)
        } else {
            GRIDLINES
        };

        let series = self.series.clone();
        let paint_colors = line_colors.clone();
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
                for (i, (_, values)) in series.iter().enumerate() {
                    let color = paint_colors[i];
                    let area = filled.then_some(Hsla { a: 0.15, ..color });
                    match scale {
                        Some((lo, hi)) => paint_polyline_ys(
                            window,
                            bounds,
                            &normalize_between(values, lo, hi),
                            stroke,
                            color,
                            area,
                        ),
                        None => paint_polyline(window, bounds, values, stroke, color, area),
                    }
                }
            },
        )
        .w_full()
        .h(px(self.height));

        // Plot area, with optional hover readout slots layered above.
        let mut plot_wrap = div().relative().flex_1().child(plot);
        if self.hover && point_count > 0 {
            let texts: Vec<SharedString> = (0..point_count)
                .map(|i| {
                    let parts: Vec<String> = self
                        .series
                        .iter()
                        .map(|(label, values)| {
                            let value = values
                                .get(i)
                                .map(|v| tick_label(*v))
                                .unwrap_or_else(|| "–".into());
                            match label {
                                Some(name) => format!("{name}: {value}"),
                                None => value,
                            }
                        })
                        .collect();
                    let text = match self.labels.get(i) {
                        Some(cat) => format!("{cat} — {}", parts.join("  ")),
                        None => parts.join("  "),
                    };
                    text.into()
                })
                .collect();
            plot_wrap = plot_wrap.child(hover_slots("guise-linechart-hover".into(), texts));
        }

        let mut body = div().flex().flex_row().w_full();
        if self.axis {
            body = body.child(y_axis_column(t, &ticks, self.height));
        }
        let mut plot_column = div()
            .flex_1()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .child(plot_wrap);
        if !self.labels.is_empty() {
            plot_column = plot_column.child(x_label_row(t, &self.labels));
        }
        body = body.child(plot_column);

        let legend: Vec<(SharedString, Hsla)> = self
            .series
            .iter()
            .enumerate()
            .filter_map(|(i, (label, _))| label.clone().map(|l| (l, line_colors[i])))
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
