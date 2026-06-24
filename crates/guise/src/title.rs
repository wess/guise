//! `Title` — headings `h1..h6` by `order`.

use gpui::prelude::*;
use gpui::{div, px, App, FontWeight, IntoElement, SharedString, Window};

use crate::theme::{theme, Color};

/// A heading. The Mantine `Title`; `order` 1..=6 selects the heading level.
#[derive(IntoElement)]
pub struct Title {
    content: SharedString,
    order: u8,
    color: Option<Color>,
}

impl Title {
    pub fn new(content: impl Into<SharedString>) -> Self {
        Title {
            content: content.into(),
            order: 1,
            color: None,
        }
    }

    /// Heading level 1..=6 (clamped). 1 is the largest.
    pub fn order(mut self, order: u8) -> Self {
        self.order = order.clamp(1, 6);
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    fn font_size(&self) -> f32 {
        match self.order {
            1 => 34.0,
            2 => 26.0,
            3 => 22.0,
            4 => 18.0,
            5 => 16.0,
            _ => 14.0,
        }
    }
}

impl RenderOnce for Title {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let color = self.color.unwrap_or_else(|| t.text());
        let size = self.font_size();
        div()
            .text_size(px(size))
            .font_weight(FontWeight::BOLD)
            .line_height(px(size * 1.3))
            .text_color(color.hsla())
            .child(self.content)
    }
}
