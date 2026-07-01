//! `Kbd` — a keyboard-key cap.

use gpui::prelude::*;
use gpui::{div, px, App, FontWeight, IntoElement, SharedString, Window};

use crate::theme::{theme, ColorName, Size};

/// A keyboard key. The Mantine `Kbd`.
#[derive(IntoElement)]
pub struct Kbd {
    key: SharedString,
}

impl Kbd {
    pub fn new(key: impl Into<SharedString>) -> Self {
        Kbd { key: key.into() }
    }
}

impl RenderOnce for Kbd {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let bg = t
            .color(ColorName::Gray, if t.scheme.is_dark() { 7 } else { 0 })
            .hsla();
        let border = t
            .color(ColorName::Gray, if t.scheme.is_dark() { 6 } else { 3 })
            .hsla();

        div()
            .px(px(7.0))
            .py(px(2.0))
            .rounded(px(t.radius(Size::Sm)))
            .border_1()
            .border_color(border)
            .bg(bg)
            .text_color(t.text().hsla())
            .text_size(px(t.font_size(Size::Xs)))
            .font_weight(FontWeight::SEMIBOLD)
            .child(self.key)
    }
}
