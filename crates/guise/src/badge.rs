//! `Badge` — a small pill for labels, counts, and statuses.

use gpui::prelude::*;
use gpui::{div, px, App, FontWeight, IntoElement, SharedString, Window};

use crate::style::{surface, ColorValue, Variant};
use crate::theme::{theme, Size};

/// A compact status pill. The Mantine `Badge`.
#[derive(IntoElement)]
pub struct Badge {
    label: SharedString,
    variant: Variant,
    color: ColorValue,
    size: Size,
}

impl Badge {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Badge {
            label: label.into(),
            variant: Variant::Light,
            color: ColorValue::default(),
            size: Size::Md,
        }
    }

    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    pub fn color(mut self, color: impl Into<ColorValue>) -> Self {
        self.color = color.into();
        self
    }

    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// (height, horizontal padding, font size).
    fn metrics(&self) -> (f32, f32, f32) {
        match self.size {
            Size::Xs => (16.0, 6.0, 9.0),
            Size::Sm => (18.0, 8.0, 10.0),
            Size::Md => (20.0, 10.0, 11.0),
            Size::Lg => (26.0, 12.0, 13.0),
            Size::Xl => (32.0, 16.0, 16.0),
        }
    }
}

impl RenderOnce for Badge {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let s = surface(t, self.color, self.variant);
        let (height, pad_x, font) = self.metrics();
        let mut el = div()
            .flex()
            .items_center()
            .justify_center()
            .h(px(height))
            .px(px(pad_x))
            .rounded(px(height))
            .bg(s.bg)
            .text_color(s.fg)
            .text_size(px(font))
            .font_weight(FontWeight::BOLD)
            .child(self.label);
        if let Some(border) = s.border {
            el = el.border_1().border_color(border);
        }
        el
    }
}
