//! `StatusBar` — a thin app status bar with left, center, and right sections.

use gpui::prelude::*;
use gpui::{div, px, AnyElement, App, IntoElement, Window};

use crate::theme::{theme, Size};

/// A bottom/top status bar with three slots. Not a Mantine component, but the
/// shell most desktop apps need; styled to match the theme.
#[derive(IntoElement)]
pub struct StatusBar {
    left: Vec<AnyElement>,
    center: Vec<AnyElement>,
    right: Vec<AnyElement>,
    height: f32,
}

impl StatusBar {
    pub fn new() -> Self {
        StatusBar {
            left: Vec::new(),
            center: Vec::new(),
            right: Vec::new(),
            height: 28.0,
        }
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub fn left(mut self, child: impl IntoElement) -> Self {
        self.left.push(child.into_any_element());
        self
    }

    pub fn center(mut self, child: impl IntoElement) -> Self {
        self.center.push(child.into_any_element());
        self
    }

    pub fn right(mut self, child: impl IntoElement) -> Self {
        self.right.push(child.into_any_element());
        self
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        StatusBar::new()
    }
}

impl RenderOnce for StatusBar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let font = t.font_size(Size::Xs);

        div()
            .flex()
            .items_center()
            .w_full()
            .h(px(self.height))
            .px(px(12.0))
            .gap(px(12.0))
            .border_t_1()
            .border_color(t.border().hsla())
            .bg(t.surface().hsla())
            .text_size(px(font))
            .text_color(t.dimmed().hsla())
            .child(div().flex().items_center().gap(px(12.0)).children(self.left))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(12.0))
                    .children(self.center),
            )
            .child(div().flex().items_center().gap(px(12.0)).children(self.right))
    }
}
