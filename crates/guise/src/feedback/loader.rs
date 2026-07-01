//! `Loader` — an animated busy indicator (pulsing dots or bars).

use std::time::Duration;

use gpui::prelude::*;
use gpui::{div, pulsating_between, px, Animation, AnimationExt, App, IntoElement, Window};

use crate::theme::{theme, ColorName, Size};

/// The loader's visual style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoaderVariant {
    /// Three pulsing dots (the default).
    Dots,
    /// Three pulsing vertical bars.
    Bars,
}

/// An animated loading indicator. The Mantine `Loader`.
#[derive(IntoElement)]
pub struct Loader {
    variant: LoaderVariant,
    size: Size,
    color: ColorName,
}

impl Loader {
    pub fn new() -> Self {
        Loader {
            variant: LoaderVariant::Dots,
            size: Size::Md,
            color: ColorName::Blue,
        }
    }

    pub fn variant(mut self, variant: LoaderVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    pub fn color(mut self, color: ColorName) -> Self {
        self.color = color;
        self
    }

    fn unit(&self) -> f32 {
        match self.size {
            Size::Xs => 6.0,
            Size::Sm => 8.0,
            Size::Md => 10.0,
            Size::Lg => 13.0,
            Size::Xl => 16.0,
        }
    }
}

impl Default for Loader {
    fn default() -> Self {
        Loader::new()
    }
}

impl RenderOnce for Loader {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let color = t.color(self.color, t.primary_shade()).hsla();
        let unit = self.unit();
        let bars = self.variant == LoaderVariant::Bars;

        let dots = (0..3usize).map(move |i| {
            let phase = i as f32 / 3.0;
            let pulse = pulsating_between(0.25, 1.0);
            let animation = Animation::new(Duration::from_millis(900))
                .repeat()
                .with_easing(move |delta| pulse((delta + phase) % 1.0));

            let dot = if bars {
                div()
                    .w(px(unit * 0.6))
                    .h(px(unit * 2.4))
                    .rounded(px(unit * 0.3))
            } else {
                div().w(px(unit)).h(px(unit)).rounded(px(unit))
            }
            .bg(color);

            dot.with_animation(("guise-loader-unit", i), animation, |dot, delta| {
                dot.opacity(delta)
            })
        });

        div()
            .flex()
            .items_center()
            .gap(px(unit * 0.6))
            .children(dots)
    }
}
