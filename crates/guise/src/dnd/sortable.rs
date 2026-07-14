//! `SortableList` — drag rows to reorder.
//!
//! Stateless: the parent owns the items; the list reports `(from, to)` and
//! the parent applies it (usually with [`apply_reorder`](super::apply_reorder)).

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{div, px, AnyElement, App, ElementId, IntoElement, SharedString, Window};

use super::chip::DragChip;
use crate::theme::theme;

type ItemBuilder = Rc<dyn Fn(usize, &mut Window, &mut App) -> AnyElement + 'static>;
type ReorderHandler = Rc<dyn Fn(usize, usize, &mut Window, &mut App) + 'static>;
type Labeler = Rc<dyn Fn(usize) -> SharedString + 'static>;

/// The payload a sortable row drags: its list group + index.
#[derive(Clone)]
struct SortDrag {
    group: SharedString,
    index: usize,
}

/// A vertical list whose rows drag to reorder. Dropping row `from` on row
/// `to` calls `on_reorder(from, to)` — "place it where I dropped it".
///
/// ```ignore
/// let view = cx.entity().downgrade();
/// SortableList::new("queue", self.tracks.len(), {
///     let tracks = self.tracks.clone();
///     move |i, _w, _cx| Text::new(tracks[i].clone()).into_any_element()
/// })
/// .on_reorder(move |from, to, _window, cx| {
///     view.update(cx, |this, cx| {
///         guise::dnd::apply_reorder(&mut this.tracks, from, to);
///         cx.notify();
///     })
///     .ok();
/// })
/// ```
#[derive(IntoElement)]
pub struct SortableList {
    id: ElementId,
    group: SharedString,
    count: usize,
    item: ItemBuilder,
    labeler: Option<Labeler>,
    gap: f32,
    on_reorder: Option<ReorderHandler>,
}

impl SortableList {
    /// `group` (from `id`) guards drops: rows only accept drags from the
    /// same list. `item` builds each row's content, re-invoked every frame.
    pub fn new<E>(
        id: impl Into<SharedString>,
        count: usize,
        item: impl Fn(usize, &mut Window, &mut App) -> E + 'static,
    ) -> Self
    where
        E: IntoElement,
    {
        let group: SharedString = id.into();
        SortableList {
            id: ElementId::Name(group.clone()),
            group,
            count,
            item: Rc::new(move |i, window, cx| item(i, window, cx).into_any_element()),
            labeler: None,
            gap: 4.0,
            on_reorder: None,
        }
    }

    /// Chip label for the dragged row (default "Item N").
    pub fn label_of(mut self, labeler: impl Fn(usize) -> SharedString + 'static) -> Self {
        self.labeler = Some(Rc::new(labeler));
        self
    }

    /// Vertical gap between rows in px (default 4).
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap.max(0.0);
        self
    }

    /// Called with `(from, to)` when a row is dropped on another.
    pub fn on_reorder(
        mut self,
        handler: impl Fn(usize, usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_reorder = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for SortableList {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let accent = t.primary().hsla();

        let mut root = div().id(self.id).flex().flex_col().gap(px(self.gap));
        for i in 0..self.count {
            let content = (self.item)(i, window, cx);
            let label = match &self.labeler {
                Some(labeler) => labeler(i),
                None => SharedString::from(format!("Item {}", i + 1)),
            };
            let chip = DragChip {
                value: SortDrag {
                    group: self.group.clone(),
                    index: i,
                },
                label,
            };

            let mut row = div()
                .id(("guise-sortable-row", i))
                .cursor_grab()
                // A constant transparent top border keeps layout stable while
                // the drag-over highlight recolors it as the insert marker.
                .border_t_2()
                .border_color(gpui::transparent_black())
                .on_drag(chip, |dragged: &DragChip<SortDrag>, _off, _w, cx| {
                    cx.new(|_| dragged.clone())
                })
                .drag_over::<DragChip<SortDrag>>(move |style, _drag, _window, _cx| {
                    style.border_color(accent)
                })
                .child(content);

            if let Some(handler) = self.on_reorder.clone() {
                let group = self.group.clone();
                row = row.on_drop(move |dragged: &DragChip<SortDrag>, window, cx| {
                    if dragged.value.group == group && dragged.value.index != i {
                        handler(dragged.value.index, i, window, cx);
                    }
                });
            }
            root = root.child(row);
        }
        root
    }
}
