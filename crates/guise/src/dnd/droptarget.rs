//! `DropTarget` — receive typed drags with a built-in hover highlight.

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{div, AnyElement, App, ElementId, IntoElement, Window};

use super::chip::DragChip;
use crate::theme::theme;

type DropHandler<T> = Rc<dyn Fn(&T, &mut Window, &mut App) + 'static>;

/// A region that accepts drags from [`Draggable`](super::Draggable)s carrying
/// the same payload type. While a matching drag hovers, the target shows a
/// primary border + tint (disable with `.plain()`).
///
/// ```ignore
/// DropTarget::<CardId>::new("done-lane")
///     .on_drop(|card, _window, _cx| move_to_done(*card))
///     .child(lane_content)
/// ```
#[derive(IntoElement)]
pub struct DropTarget<T: Clone + 'static> {
    id: ElementId,
    child: Option<AnyElement>,
    highlight: bool,
    on_drop: Option<DropHandler<T>>,
}

impl<T: Clone + 'static> DropTarget<T> {
    pub fn new(id: impl Into<ElementId>) -> Self {
        DropTarget {
            id: id.into(),
            child: None,
            highlight: true,
            on_drop: None,
        }
    }

    /// Disable the built-in drag-over highlight.
    pub fn plain(mut self) -> Self {
        self.highlight = false;
        self
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.child = Some(child.into_any_element());
        self
    }

    /// Receives the dropped payload.
    pub fn on_drop(mut self, handler: impl Fn(&T, &mut Window, &mut App) + 'static) -> Self {
        self.on_drop = Some(Rc::new(handler));
        self
    }
}

impl<T: Clone + 'static> RenderOnce for DropTarget<T> {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let accent = t.primary();
        let accent_border = accent.hsla();
        let accent_tint = accent.alpha(0.08);

        let mut root = div().id(self.id);
        if self.highlight {
            root = root.drag_over::<DragChip<T>>(move |style, _drag, _window, _cx| {
                style.border_color(accent_border).bg(accent_tint)
            });
        }
        if let Some(handler) = self.on_drop {
            root = root.on_drop(move |dragged: &DragChip<T>, window, cx| {
                handler(&dragged.value, window, cx);
            });
        }
        if let Some(child) = self.child {
            root = root.child(child);
        }
        root
    }
}
