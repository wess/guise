//! `LineChart` — a sparkline grown up: light horizontal gridlines and an
//! optional area fill, still axis-free.
//!
//! ```ignore
//! use guise::chart::LineChart;
//!
//! LineChart::new([12.0, 18.0, 9.0, 24.0, 20.0, 31.0]).fill().height(180.0)
//! ```

use gpui::prelude::*;
use gpui::{canvas, fill, point, px, size, App, Bounds, Hsla, IntoElement, Window};

use crate::style::ColorValue;
use crate::theme::theme;

use super::{paint_polyline, resolve_color};

/// How many horizontal gridlines a `LineChart` paints (top and bottom included).
const GRIDLINES: usize = 4;

/// A single-series line chart. Values are min/max normalized so the line spans
/// the full height; fewer than two values paint only the gridlines.
#[derive(IntoElement)]
pub struct LineChart {
    values: Vec<f32>,
    color: Option<ColorValue>,
    stroke: f32,
    fill: bool,
    width: Option<f32>,
    height: f32,
}

impl LineChart {
    pub fn new(values: impl IntoIterator<Item = f32>) -> Self {
        LineChart {
            values: values.into_iter().collect(),
            color: None,
            stroke: 2.0,
            fill: false,
            width: None,
            height: 140.0,
        }
    }

    /// Line color. Defaults to the theme primary.
    pub fn color(mut self, color: impl Into<ColorValue>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Stroke width in px (default 2).
    pub fn stroke(mut self, width: f32) -> Self {
        self.stroke = width.max(0.5);
        self
    }

    /// Fill the area between the line and the baseline (line color at 0.15 alpha).
    pub fn fill(mut self) -> Self {
        self.fill = true;
        self
    }

    /// Fixed width in px. Defaults to the parent's full width.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Height in px (default 140).
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }
}

impl RenderOnce for LineChart {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let line = self
            .color
            .map(|c| resolve_color(t, c))
            .unwrap_or_else(|| t.primary().hsla());
        let area = self.fill.then_some(Hsla { a: 0.15, ..line });
        let grid = t.border().alpha(0.5);
        let stroke = self.stroke;
        let values = self.values;

        let plot = canvas(
            |_, _, _| (),
            move |bounds, _, window, _cx| {
                let w = f32::from(bounds.size.width);
                let h = f32::from(bounds.size.height);
                if w <= 0.0 || h <= 0.0 {
                    return;
                }
                // Evenly spaced 1px gridlines, top edge through bottom edge.
                for i in 0..GRIDLINES {
                    let y = (h - 1.0) * (i as f32 / (GRIDLINES - 1) as f32);
                    window.paint_quad(fill(
                        Bounds::new(bounds.origin + point(px(0.0), px(y)), size(px(w), px(1.0))),
                        grid,
                    ));
                }
                paint_polyline(window, bounds, &values, stroke, line, area);
            },
        )
        .h(px(self.height));

        match self.width {
            Some(w) => plot.w(px(w)),
            None => plot.w_full(),
        }
    }
}
