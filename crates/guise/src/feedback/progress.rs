//! `Progress` — a horizontal completion bar.

use gpui::prelude::*;
use gpui::{div, px, relative, App, IntoElement, Window};

use crate::theme::{theme, ColorName, Size};

/// A determinate progress bar. The Mantine `Progress`. `value` is a percentage
/// in `0.0..=100.0`.
#[derive(IntoElement)]
pub struct Progress {
    value: f32,
    color: ColorName,
    size: Size,
    radius: Option<Size>,
}

impl Progress {
    pub fn new(value: f32) -> Self {
        Progress {
            value: value.clamp(0.0, 100.0),
            color: ColorName::Blue,
            size: Size::Md,
            radius: None,
        }
    }

    pub fn color(mut self, color: ColorName) -> Self {
        self.color = color;
        self
    }

    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    pub fn radius(mut self, radius: Size) -> Self {
        self.radius = Some(radius);
        self
    }

    fn height(&self) -> f32 {
        match self.size {
            Size::Xs => 4.0,
            Size::Sm => 6.0,
            Size::Md => 8.0,
            Size::Lg => 12.0,
            Size::Xl => 16.0,
        }
    }
}

impl RenderOnce for Progress {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let height = self.height();
        let radius = self
            .radius
            .map(|r| t.radius(r))
            .unwrap_or(height / 2.0);
        let track = t
            .color(ColorName::Gray, if t.scheme.is_dark() { 7 } else { 2 })
            .hsla();
        let fill = t.color(self.color, t.primary_shade()).hsla();
        let fraction = (self.value / 100.0).clamp(0.0, 1.0);

        div()
            .w_full()
            .h(px(height))
            .rounded(px(radius))
            .bg(track)
            .child(
                div()
                    .h_full()
                    .w(relative(fraction))
                    .rounded(px(radius))
                    .bg(fill),
            )
    }
}
