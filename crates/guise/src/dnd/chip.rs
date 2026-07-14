//! The floating chip shown under the pointer while dragging. Wraps the
//! caller's payload so [`DropTarget`](super::DropTarget) can match on the
//! payload type it expects.

use gpui::prelude::*;
use gpui::{div, px, Context, IntoElement, SharedString, Window};

use crate::theme::{theme, Size};

/// The typed drag value: the caller's payload plus the chip label.
pub(crate) struct DragChip<T: 'static> {
    pub value: T,
    pub label: SharedString,
}

impl<T: Clone + 'static> Clone for DragChip<T> {
    fn clone(&self) -> Self {
        DragChip {
            value: self.value.clone(),
            label: self.label.clone(),
        }
    }
}

impl<T: 'static> Render for DragChip<T> {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        div()
            .px(px(10.0))
            .py(px(5.0))
            .rounded(px(t.radius(t.default_radius)))
            .bg(t.surface().hsla())
            .border_1()
            .border_color(t.primary().hsla())
            .shadow_md()
            .text_size(px(t.font_size(Size::Sm)))
            .text_color(t.text().hsla())
            .child(self.label.clone())
    }
}
