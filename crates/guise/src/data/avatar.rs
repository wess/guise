//! `Avatar` — a circular initials/placeholder badge.

use gpui::prelude::*;
use gpui::{div, px, App, FontWeight, IntoElement, SharedString, Window};

use crate::style::{surface, Variant};
use crate::theme::{theme, ColorName, Size};

/// A user avatar showing initials. The Mantine `Avatar` (image variants aside).
#[derive(IntoElement)]
pub struct Avatar {
    initials: SharedString,
    color: ColorName,
    variant: Variant,
    size: Size,
    /// `None` renders a full circle; `Some` sets a square corner radius.
    radius: Option<Size>,
}

impl Avatar {
    pub fn new(initials: impl Into<SharedString>) -> Self {
        Avatar {
            initials: initials.into(),
            color: ColorName::Gray,
            variant: Variant::Light,
            size: Size::Md,
            radius: None,
        }
    }

    pub fn color(mut self, color: ColorName) -> Self {
        self.color = color;
        self
    }

    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
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
}

/// Avatar diameter (px) across the size scale. Shared with `AvatarGroup`.
pub(crate) fn avatar_size(size: Size) -> f32 {
    match size {
        Size::Xs => 16.0,
        Size::Sm => 26.0,
        Size::Md => 38.0,
        Size::Lg => 56.0,
        Size::Xl => 84.0,
    }
}

impl RenderOnce for Avatar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let s = surface(t, self.color, self.variant);
        let dim = avatar_size(self.size);
        let radius = match self.radius {
            Some(r) => t.radius(r),
            None => dim, // full circle
        };

        let mut el = div()
            .w(px(dim))
            .h(px(dim))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(radius))
            .bg(s.bg)
            .text_color(s.fg)
            .text_size(px(dim * 0.4))
            .font_weight(FontWeight::SEMIBOLD)
            .child(self.initials);
        if let Some(border) = s.border {
            el = el.border_1().border_color(border);
        }
        el
    }
}
