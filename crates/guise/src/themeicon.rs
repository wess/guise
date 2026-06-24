//! `ThemeIcon` — a colored, rounded container for a single icon.

use gpui::prelude::*;
use gpui::{div, px, App, IntoElement, SharedString, Window};

use crate::style::{surface, Variant};
use crate::theme::{theme, ColorName, Size};

/// A decorative colored icon chip. The Mantine `ThemeIcon`.
#[derive(IntoElement)]
pub struct ThemeIcon {
    icon: SharedString,
    variant: Variant,
    color: ColorName,
    size: Size,
    radius: Option<Size>,
}

impl ThemeIcon {
    pub fn new(icon: impl Into<SharedString>) -> Self {
        ThemeIcon {
            icon: icon.into(),
            variant: Variant::Filled,
            color: ColorName::Blue,
            size: Size::Md,
            radius: None,
        }
    }

    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
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

    fn dimension(&self) -> f32 {
        match self.size {
            Size::Xs => 16.0,
            Size::Sm => 22.0,
            Size::Md => 28.0,
            Size::Lg => 38.0,
            Size::Xl => 52.0,
        }
    }
}

impl RenderOnce for ThemeIcon {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let s = surface(t, self.color, self.variant);
        let dim = self.dimension();
        let radius = t.radius(self.radius.unwrap_or(t.default_radius));

        let mut el = div()
            .w(px(dim))
            .h(px(dim))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(radius))
            .bg(s.bg)
            .text_color(s.fg)
            .text_size(px(dim * 0.52))
            .child(self.icon);
        if let Some(border) = s.border {
            el = el.border_1().border_color(border);
        }
        el
    }
}
