//! `Skeleton` — an animated loading placeholder.

use std::time::Duration;

use gpui::prelude::*;
use gpui::{div, px, pulsating_between, Animation, AnimationExt, App, IntoElement, Window};

use crate::theme::{theme, ColorName, Size};

/// A pulsing placeholder block. The Mantine `Skeleton`.
#[derive(IntoElement)]
pub struct Skeleton {
    width: Option<f32>,
    height: f32,
    radius: Size,
    circle: bool,
}

impl Skeleton {
    pub fn new() -> Self {
        Skeleton {
            width: None,
            height: 16.0,
            radius: Size::Sm,
            circle: false,
        }
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub fn radius(mut self, radius: Size) -> Self {
        self.radius = radius;
        self
    }

    /// Render a circle of `size` (overrides width/height/radius).
    pub fn circle(mut self, size: f32) -> Self {
        self.circle = true;
        self.width = Some(size);
        self.height = size;
        self
    }
}

impl Default for Skeleton {
    fn default() -> Self {
        Skeleton::new()
    }
}

impl RenderOnce for Skeleton {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let color = t
            .color(ColorName::Gray, if t.scheme.is_dark() { 7 } else { 2 })
            .hsla();
        let radius = if self.circle {
            self.height
        } else {
            t.radius(self.radius)
        };

        let mut block = div().h(px(self.height)).rounded(px(radius)).bg(color);
        block = match self.width {
            Some(w) => block.w(px(w)),
            None => block.w_full(),
        };

        let pulse = pulsating_between(0.4, 1.0);
        block.with_animation(
            "guise-skeleton",
            Animation::new(Duration::from_millis(1100)).repeat().with_easing(pulse),
            |block, delta| block.opacity(delta),
        )
    }
}
