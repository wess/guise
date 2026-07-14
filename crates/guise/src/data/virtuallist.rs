//! `VirtualList` — windowed rendering for large flat collections.
//!
//! Wraps gpui's `uniform_list`: only the rows in view are built each frame,
//! so a 100k-item list renders as cheaply as a 20-item one. Items come from
//! a factory closure (not pre-built children) and must share one height —
//! that uniformity is what makes the scroll math O(1).

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{px, uniform_list, AnyElement, App, ElementId, IntoElement, Window};

type ItemBuilder = Rc<dyn Fn(usize, &mut Window, &mut App) -> AnyElement + 'static>;

/// A virtualized list. `VirtualList::new("log", 100_000, |i, _, _| row(i)).height(400.0)`.
#[derive(IntoElement)]
pub struct VirtualList {
    id: ElementId,
    count: usize,
    height: f32,
    item: ItemBuilder,
}

impl VirtualList {
    /// `item` is invoked per visible index, every frame — keep it cheap and
    /// return rows of equal height.
    pub fn new<E>(
        id: impl Into<ElementId>,
        count: usize,
        item: impl Fn(usize, &mut Window, &mut App) -> E + 'static,
    ) -> Self
    where
        E: IntoElement,
    {
        VirtualList {
            id: id.into(),
            count,
            height: 240.0,
            item: Rc::new(move |ix, window, cx| item(ix, window, cx).into_any_element()),
        }
    }

    /// Viewport height in px (default `240.0`).
    pub fn height(mut self, height: f32) -> Self {
        self.height = height.max(0.0);
        self
    }
}

impl RenderOnce for VirtualList {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let item = self.item;
        uniform_list(self.id, self.count, move |range, window, cx| {
            range.map(|ix| item(ix, window, cx)).collect::<Vec<_>>()
        })
        .h(px(self.height))
        .w_full()
    }
}
