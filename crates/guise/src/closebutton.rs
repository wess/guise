//! `CloseButton` — a subtle square button with an `×` glyph.

use gpui::prelude::*;
use gpui::{div, px, App, ClickEvent, ElementId, IntoElement, SharedString, Window};

use crate::input::ClickHandler;
use crate::style::icon_size;
use crate::theme::{theme, Size};

/// A dismiss button. The Mantine `CloseButton`.
#[derive(IntoElement)]
pub struct CloseButton {
    id: ElementId,
    size: Size,
    on_click: Option<ClickHandler>,
}

impl CloseButton {
    pub fn new(id: impl Into<ElementId>) -> Self {
        CloseButton {
            id: id.into(),
            size: Size::Md,
            on_click: None,
        }
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

impl RenderOnce for CloseButton {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let dim = icon_size(self.size);
        let hover_bg = t.surface_hover().hsla();
        let hover_fg = t.text().hsla();

        let mut el = div()
            .id(self.id)
            .w(px(dim))
            .h(px(dim))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(t.radius(Size::Sm)))
            .text_color(t.dimmed().hsla())
            .text_size(px(dim * 0.5))
            .hover(move |s| s.bg(hover_bg).text_color(hover_fg))
            .child(SharedString::new_static("\u{00d7}"));
        if let Some(handler) = self.on_click {
            el = el.on_click(handler);
        }
        el
    }
}
