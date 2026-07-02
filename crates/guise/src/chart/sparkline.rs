//! `Sparkline` — a tiny inline trend line with an optional area fill.
//!
//! ```ignore
//! use guise::chart::Sparkline;
//!
//! Sparkline::new([3.0, 5.0, 2.0, 8.0, 6.0]).fill()
//! ```

use gpui::prelude::*;
use gpui::{canvas, px, App, Hsla, IntoElement, Window};

use crate::style::ColorValue;
use crate::theme::theme;

use super::{paint_polyline, resolve_color};

/// A minimal, axis-free polyline over a value series. Values are min/max
/// normalized to the chart height; fewer than two values paint nothing.
#[derive(IntoElement)]
pub struct Sparkline {
    values: Vec<f32>,
    color: Option<ColorValue>,
    stroke: f32,
    fill: bool,
    width: Option<f32>,
    full_width: bool,
    height: f32,
}

impl Sparkline {
    pub fn new(values: impl IntoIterator<Item = f32>) -> Self {
        Sparkline {
            values: values.into_iter().collect(),
            color: None,
            stroke: 2.0,
            fill: false,
            width: None,
            full_width: false,
            height: 32.0,
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

    /// Fixed width in px (default 120).
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Stretch to the parent's width instead of a fixed one.
    pub fn full_width(mut self) -> Self {
        self.full_width = true;
        self
    }

    /// Height in px (default 32).
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }
}

impl RenderOnce for Sparkline {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let line = self
            .color
            .map(|c| resolve_color(t, c))
            .unwrap_or_else(|| t.primary().hsla());
        let area = self.fill.then_some(Hsla { a: 0.15, ..line });
        let stroke = self.stroke;
        let values = self.values;

        let plot = canvas(
            |_, _, _| (),
            move |bounds, _, window, _cx| {
                paint_polyline(window, bounds, &values, stroke, line, area);
            },
        )
        .h(px(self.height));

        if self.full_width {
            plot.w_full()
        } else {
            plot.w(px(self.width.unwrap_or(120.0)))
        }
    }
}
