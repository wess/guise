//! `Draggable` — make any element the source of a typed drag.

use gpui::prelude::*;
use gpui::{div, AnyElement, App, ElementId, IntoElement, SharedString, Window};

use super::chip::DragChip;

/// Wraps a child so dragging it carries `payload`; pair with a
/// [`DropTarget`](super::DropTarget) of the same payload type.
///
/// ```ignore
/// Draggable::new("card-3", CardId(3))
///     .label("Q3 report")
///     .child(Card::new().child(summary))
/// ```
#[derive(IntoElement)]
pub struct Draggable<T: Clone + 'static> {
    id: ElementId,
    payload: T,
    label: SharedString,
    child: Option<AnyElement>,
}

impl<T: Clone + 'static> Draggable<T> {
    pub fn new(id: impl Into<ElementId>, payload: T) -> Self {
        Draggable {
            id: id.into(),
            payload,
            label: SharedString::new_static("…"),
            child: None,
        }
    }

    /// Text on the chip that follows the pointer (default "…").
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.child = Some(child.into_any_element());
        self
    }
}

impl<T: Clone + 'static> RenderOnce for Draggable<T> {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let chip = DragChip {
            value: self.payload,
            label: self.label,
        };
        let mut root = div().id(self.id).cursor_grab().on_drag(
            chip,
            |dragged: &DragChip<T>, _offset, _window, cx| cx.new(|_| dragged.clone()),
        );
        if let Some(child) = self.child {
            root = root.child(child);
        }
        root
    }
}
