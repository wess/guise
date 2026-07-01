//! `Anchor` — a clickable, colored text link.

use gpui::prelude::*;
use gpui::{div, px, App, ClickEvent, ElementId, IntoElement, SharedString, Window};

use crate::input::ClickHandler;
use crate::theme::{theme, ColorName, Size};

/// A text link. The Mantine `Anchor`.
#[derive(IntoElement)]
pub struct Anchor {
    id: ElementId,
    label: SharedString,
    color: ColorName,
    size: Size,
    on_click: Option<ClickHandler>,
}

impl Anchor {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Anchor {
            id: id.into(),
            label: label.into(),
            color: ColorName::Blue,
            size: Size::Md,
            on_click: None,
        }
    }

    pub fn color(mut self, color: ColorName) -> Self {
        self.color = color;
        self
    }

    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for Anchor {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let color = t
            .color(self.color, if t.scheme.is_dark() { 4 } else { 6 })
            .hsla();
        let hover = t
            .color(self.color, if t.scheme.is_dark() { 3 } else { 7 })
            .hsla();

        let mut el = div()
            .id(self.id)
            .text_size(px(t.font_size(self.size)))
            .text_color(color)
            .hover(move |s| s.text_color(hover))
            .child(self.label);
        if let Some(handler) = self.on_click {
            el = el.on_click(handler);
        }
        el
    }
}
