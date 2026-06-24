//! `Text` — themed inline text with size, weight, and color controls.

use gpui::prelude::*;
use gpui::{div, px, App, FontWeight, IntoElement, SharedString, Window};

use crate::theme::{theme, Color, Size};

/// Themed body text. The Mantine `Text`.
#[derive(IntoElement)]
pub struct Text {
    content: SharedString,
    size: Size,
    weight: FontWeight,
    color: Option<Color>,
    dimmed: bool,
}

impl Text {
    pub fn new(content: impl Into<SharedString>) -> Self {
        Text {
            content: content.into(),
            size: Size::Md,
            weight: FontWeight::NORMAL,
            color: None,
            dimmed: false,
        }
    }

    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    pub fn weight(mut self, weight: FontWeight) -> Self {
        self.weight = weight;
        self
    }

    /// Render at the medium font weight (500).
    pub fn medium(self) -> Self {
        self.weight(FontWeight::MEDIUM)
    }

    /// Render at the bold font weight (700).
    pub fn bold(self) -> Self {
        self.weight(FontWeight::BOLD)
    }

    /// Override the text color.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Use the muted/secondary text color.
    pub fn dimmed(mut self) -> Self {
        self.dimmed = true;
        self
    }
}

impl RenderOnce for Text {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let color = match (self.color, self.dimmed) {
            (Some(c), _) => c,
            (None, true) => t.dimmed(),
            (None, false) => t.text(),
        };
        div()
            .text_size(px(t.font_size(self.size)))
            .font_weight(self.weight)
            .text_color(color.hsla())
            .child(self.content)
    }
}
