//! `Code` — inline monospace-style code text.

use gpui::prelude::*;
use gpui::{div, px, App, IntoElement, SharedString, Window};

use crate::theme::{theme, ColorName, Size};

/// Inline code. The Mantine `Code`. (Uses the window font; gpui has no generic
/// monospace fallback, so style a real mono family at the app level if needed.)
#[derive(IntoElement)]
pub struct Code {
    content: SharedString,
    color: Option<ColorName>,
}

impl Code {
    pub fn new(content: impl Into<SharedString>) -> Self {
        Code {
            content: content.into(),
            color: None,
        }
    }

    /// Tint the code chip with a named color instead of the neutral surface.
    pub fn color(mut self, color: ColorName) -> Self {
        self.color = Some(color);
        self
    }
}

impl RenderOnce for Code {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let (bg, fg) = match self.color {
            Some(name) => (
                t.color(name, if t.scheme.is_dark() { 8 } else { 0 }).hsla(),
                t.color(name, if t.scheme.is_dark() { 2 } else { 8 }).hsla(),
            ),
            None => (
                t.color(ColorName::Gray, if t.scheme.is_dark() { 8 } else { 1 }).hsla(),
                t.text().hsla(),
            ),
        };

        div()
            .px(px(6.0))
            .py(px(2.0))
            .rounded(px(t.radius(Size::Sm)))
            .bg(bg)
            .text_color(fg)
            .text_size(px(t.font_size(Size::Sm)))
            .child(self.content)
    }
}
