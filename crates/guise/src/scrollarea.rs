//! `ScrollArea` — a bounded, scrollable container.
//!
//! Desktop UIs scroll; most builders assume their content fits. Wrap an
//! overflowing column (or row) in a `ScrollArea` with a `max_height` to clip and
//! scroll it. Each instance needs a unique id so gpui can track its scroll
//! offset.

use gpui::prelude::*;
use gpui::{div, px, AnyElement, App, ElementId, IntoElement, Window};

/// A scrollable region. `ScrollArea::new("id").max_height(240.0)`.
#[derive(IntoElement)]
pub struct ScrollArea {
    id: ElementId,
    children: Vec<AnyElement>,
    max_height: Option<f32>,
    horizontal: bool,
}

impl ScrollArea {
    pub fn new(id: impl Into<ElementId>) -> Self {
        ScrollArea {
            id: id.into(),
            children: Vec::new(),
            max_height: None,
            horizontal: false,
        }
    }

    /// Clip to this height (px) and scroll past it.
    pub fn max_height(mut self, height: f32) -> Self {
        self.max_height = Some(height);
        self
    }

    /// Scroll horizontally instead of vertically.
    pub fn horizontal(mut self, horizontal: bool) -> Self {
        self.horizontal = horizontal;
        self
    }
}

impl ParentElement for ScrollArea {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for ScrollArea {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut el = div().id(self.id).flex();
        el = if self.horizontal {
            el.flex_row().overflow_x_scroll()
        } else {
            el.flex_col().overflow_y_scroll()
        };
        if let Some(height) = self.max_height {
            el = el.max_h(px(height));
        }
        el.children(self.children)
    }
}
