//! `RingProgress` — a circular determinate progress indicator.
//!
//! gpui has no arc/conic primitive, so the fill is rendered as a clipped column
//! rising from the bottom of a circle (a gauge), with the percentage centered on
//! top. A true stroked ring would need a custom `canvas` paint pass.

use gpui::prelude::*;
use gpui::{div, px, relative, App, FontWeight, IntoElement, SharedString, Window};

use crate::theme::{theme, ColorName};

/// A circular progress gauge. `RingProgress::new(72.0).label("72%")`.
#[derive(IntoElement)]
pub struct RingProgress {
    value: f32,
    size: f32,
    color: ColorName,
    label: Option<SharedString>,
}

impl RingProgress {
    pub fn new(value: f32) -> Self {
        RingProgress {
            value: value.clamp(0.0, 100.0),
            size: 80.0,
            color: ColorName::Blue,
            label: None,
        }
    }

    /// Diameter in px.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn color(mut self, color: ColorName) -> Self {
        self.color = color;
        self
    }

    /// Centered label (defaults to the rounded percentage).
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }
}

impl RenderOnce for RingProgress {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let accent = t.color(self.color, t.primary_shade()).alpha(0.85);
        let track = if t.scheme.is_dark() {
            t.color(ColorName::Dark, 4)
        } else {
            t.color(ColorName::Gray, 2)
        }
        .hsla();
        let text = t.text().hsla();
        let frac = (self.value / 100.0).clamp(0.0, 1.0);
        let label = self
            .label
            .unwrap_or_else(|| SharedString::from(format!("{}%", self.value.round() as i64)));

        div()
            .relative()
            .w(px(self.size))
            .h(px(self.size))
            .rounded(px(self.size / 2.0))
            .overflow_hidden()
            .bg(track)
            .flex()
            .items_center()
            .justify_center()
            // Fill rising from the bottom.
            .child(
                div()
                    .absolute()
                    .bottom(px(0.0))
                    .left(px(0.0))
                    .right(px(0.0))
                    .h(relative(frac))
                    .bg(accent),
            )
            // Centered label, painted over the fill.
            .child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_size(px(self.size * 0.22))
                    .text_color(text)
                    .child(label),
            )
    }
}
