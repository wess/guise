//! `PieChart` — proportional filled slices, with an optional donut hole and a
//! small legend when labels are provided.
//!
//! ```ignore
//! use guise::chart::PieChart;
//!
//! PieChart::entries([("Rust", 62.0), ("TOML", 25.0), ("Other", 13.0)]).donut(0.6)
//! ```

use gpui::prelude::*;
use gpui::{canvas, div, point, px, App, Hsla, IntoElement, PathBuilder, SharedString, Window};

use crate::style::ColorValue;
use crate::theme::{theme, Size};

use super::{arc_point, series_color, slice_spans};

/// A pie (or donut) chart. Slices are proportional to each value's share of
/// the total, starting at 12 o'clock and sweeping clockwise. Non-positive
/// values contribute nothing. Slice colors rotate through the theme palette
/// by default.
#[derive(IntoElement)]
pub struct PieChart {
    values: Vec<f32>,
    labels: Vec<SharedString>,
    colors: Vec<ColorValue>,
    size: f32,
    donut: Option<f32>,
}

impl PieChart {
    pub fn new(values: impl IntoIterator<Item = f32>) -> Self {
        PieChart {
            values: values.into_iter().collect(),
            labels: Vec::new(),
            colors: Vec::new(),
            size: 160.0,
            donut: None,
        }
    }

    /// Build from `(label, value)` pairs; labels render as a legend below.
    pub fn entries(entries: impl IntoIterator<Item = (impl Into<SharedString>, f32)>) -> Self {
        let (labels, values): (Vec<SharedString>, Vec<f32>) = entries
            .into_iter()
            .map(|(label, value)| (label.into(), value))
            .unzip();
        PieChart {
            labels,
            ..PieChart::new(values)
        }
    }

    /// One color for every slice (disables the palette rotation).
    pub fn color(mut self, color: impl Into<ColorValue>) -> Self {
        self.colors = vec![color.into()];
        self
    }

    /// Per-slice colors, cycled when shorter than the series.
    pub fn colors(mut self, colors: impl IntoIterator<Item = impl Into<ColorValue>>) -> Self {
        self.colors = colors.into_iter().map(Into::into).collect();
        self
    }

    /// Diameter in px (default 160). Pies are square.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Cut a hole in the middle: `inner_fraction` is the hole's share of the
    /// radius, clamped into `0.05..=0.95`.
    pub fn donut(mut self, inner_fraction: f32) -> Self {
        self.donut = Some(inner_fraction.clamp(0.05, 0.95));
        self
    }
}

impl RenderOnce for PieChart {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let n = self.values.len();
        let slice_colors: Vec<Hsla> = (0..n).map(|i| series_color(t, &self.colors, i)).collect();
        let legend_colors = slice_colors.clone();
        let dimmed = t.dimmed().hsla();
        let font_xs = t.font_size(Size::Xs);
        let spans = slice_spans(&self.values);
        let donut = self.donut;
        let diameter = self.size;

        let plot = canvas(
            |_, _, _| (),
            move |bounds, _, window, _cx| {
                let w = f32::from(bounds.size.width);
                let h = f32::from(bounds.size.height);
                let radius = w.min(h) / 2.0;
                if radius <= 0.0 {
                    return;
                }
                let center = (
                    f32::from(bounds.origin.x) + w / 2.0,
                    f32::from(bounds.origin.y) + h / 2.0,
                );
                for (i, &(start, end)) in spans.iter().enumerate() {
                    if end - start <= f32::EPSILON {
                        continue;
                    }
                    let color = slice_colors[i];
                    let inner = donut.map(|f| radius * f);
                    if end - start >= 0.999 {
                        // A (near-)full circle: `arc_to` from a point back to
                        // itself paints nothing, so draw it as two halves.
                        let mid = start + 0.5;
                        paint_slice(window, center, radius, inner, start, mid, color);
                        paint_slice(window, center, radius, inner, mid, start + 1.0, color);
                    } else {
                        paint_slice(window, center, radius, inner, start, end, color);
                    }
                }
            },
        )
        .w(px(diameter))
        .h(px(diameter));

        let mut root = div()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(8.0))
            .child(plot);

        // Legend: a wrapping row of color-dot + label pairs. Plain divs.
        if !self.labels.is_empty() && !legend_colors.is_empty() {
            let items = self.labels.iter().enumerate().map(|(i, label)| {
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(6.0))
                    .child(
                        div()
                            .w(px(8.0))
                            .h(px(8.0))
                            .rounded(px(4.0))
                            .bg(legend_colors[i % legend_colors.len()]),
                    )
                    .child(
                        div()
                            .text_size(px(font_xs))
                            .text_color(dimmed)
                            .child(label.clone()),
                    )
            });
            root = root.child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .justify_center()
                    .gap(px(12.0))
                    .children(items),
            );
        }
        root
    }
}

/// Fill one pie/donut slice. `start`/`end` are clockwise fractions of a full
/// turn from 12 o'clock (`end` may exceed 1.0 when a full circle wraps);
/// `inner` is the hole radius for donuts. All coordinates window-absolute.
fn paint_slice(
    window: &mut Window,
    center: (f32, f32),
    radius: f32,
    inner: Option<f32>,
    start: f32,
    end: f32,
    color: Hsla,
) {
    let large = end - start > 0.5;
    let to_pt = |(x, y): (f32, f32)| point(px(x), px(y));
    let outer_start = to_pt(arc_point(center, radius, start));
    let outer_end = to_pt(arc_point(center, radius, end));
    let radii = point(px(radius), px(radius));

    let mut pb = PathBuilder::fill();
    match inner {
        // Donut: outer arc out, straight edge in, inner arc back (an annular
        // sector traced as one closed contour).
        Some(hole) if hole > 0.0 => {
            let inner_radii = point(px(hole), px(hole));
            pb.move_to(outer_start);
            pb.arc_to(radii, px(0.0), large, true, outer_end);
            pb.line_to(to_pt(arc_point(center, hole, end)));
            pb.arc_to(
                inner_radii,
                px(0.0),
                large,
                false,
                to_pt(arc_point(center, hole, start)),
            );
            pb.close();
        }
        // Pie: center, out to the rim, arc, back to center.
        _ => {
            pb.move_to(to_pt(center));
            pb.line_to(outer_start);
            pb.arc_to(radii, px(0.0), large, true, outer_end);
            pb.close();
        }
    }
    if let Ok(path) = pb.build() {
        window.paint_path(path, color);
    }
}
