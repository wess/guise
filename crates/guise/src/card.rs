//! `Card` — a `Paper` preset: bordered, padded, lightly raised surface.

use gpui::prelude::*;
use gpui::{div, px, AnyElement, App, IntoElement, Window};

use crate::paper::apply_shadow;
use crate::theme::{theme, Size};

/// A content card. The Mantine `Card` — a `Paper` with sensible defaults.
#[derive(IntoElement)]
pub struct Card {
    children: Vec<AnyElement>,
    padding: Size,
    radius: Option<Size>,
    with_border: bool,
    shadow: Option<Size>,
}

impl Card {
    pub fn new() -> Self {
        Card {
            children: Vec::new(),
            padding: Size::Lg,
            radius: Some(Size::Md),
            with_border: true,
            shadow: Some(Size::Sm),
        }
    }

    pub fn padding(mut self, padding: Size) -> Self {
        self.padding = padding;
        self
    }

    pub fn radius(mut self, radius: Size) -> Self {
        self.radius = Some(radius);
        self
    }

    pub fn with_border(mut self, with_border: bool) -> Self {
        self.with_border = with_border;
        self
    }

    pub fn shadow(mut self, shadow: Size) -> Self {
        self.shadow = Some(shadow);
        self
    }
}

impl Default for Card {
    fn default() -> Self {
        Card::new()
    }
}

impl ParentElement for Card {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Card {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let radius = t.radius(self.radius.unwrap_or(Size::Md));
        let mut el = div()
            .flex()
            .flex_col()
            .bg(t.surface().hsla())
            .rounded(px(radius))
            .p(px(t.spacing(self.padding)));
        if self.with_border {
            el = el.border_1().border_color(t.border().hsla());
        }
        el = apply_shadow(el, self.shadow);
        el.children(self.children)
    }
}
