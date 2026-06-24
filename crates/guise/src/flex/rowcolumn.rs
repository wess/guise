//! `Row` and `Column` — Flutter's primary flex containers.

use gpui::prelude::*;
use gpui::{div, px, AnyElement, App, IntoElement, Window};

use super::{
    apply_cross, apply_main, CrossAxisAlignment, MainAxisAlignment, MainAxisSize,
};

/// A horizontal flex container. Flutter's `Row`.
#[derive(IntoElement)]
pub struct Row {
    children: Vec<AnyElement>,
    main: MainAxisAlignment,
    cross: CrossAxisAlignment,
    size: MainAxisSize,
    gap: f32,
}

impl Row {
    pub fn new() -> Self {
        Row {
            children: Vec::new(),
            main: MainAxisAlignment::Start,
            cross: CrossAxisAlignment::Center,
            size: MainAxisSize::Max,
            gap: 0.0,
        }
    }

    pub fn main_axis_alignment(mut self, main: MainAxisAlignment) -> Self {
        self.main = main;
        self
    }

    pub fn cross_axis_alignment(mut self, cross: CrossAxisAlignment) -> Self {
        self.cross = cross;
        self
    }

    pub fn main_axis_size(mut self, size: MainAxisSize) -> Self {
        self.size = size;
        self
    }

    /// Convenience spacing between children (Flutter uses `SizedBox`).
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }
}

impl Default for Row {
    fn default() -> Self {
        Row::new()
    }
}

impl ParentElement for Row {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Row {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut base = div().flex().flex_row();
        if self.gap > 0.0 {
            base = base.gap(px(self.gap));
        }
        if self.size == MainAxisSize::Max {
            base = base.w_full();
        }
        let base = apply_cross(apply_main(base, self.main), self.cross);
        base.children(self.children)
    }
}

/// A vertical flex container. Flutter's `Column`.
#[derive(IntoElement)]
pub struct Column {
    children: Vec<AnyElement>,
    main: MainAxisAlignment,
    cross: CrossAxisAlignment,
    size: MainAxisSize,
    gap: f32,
}

impl Column {
    pub fn new() -> Self {
        Column {
            children: Vec::new(),
            main: MainAxisAlignment::Start,
            cross: CrossAxisAlignment::Center,
            // Min by default (shrink to content) — more predictable than
            // Flutter's Max inside a scrolling page.
            size: MainAxisSize::Min,
            gap: 0.0,
        }
    }

    pub fn main_axis_alignment(mut self, main: MainAxisAlignment) -> Self {
        self.main = main;
        self
    }

    pub fn cross_axis_alignment(mut self, cross: CrossAxisAlignment) -> Self {
        self.cross = cross;
        self
    }

    pub fn main_axis_size(mut self, size: MainAxisSize) -> Self {
        self.size = size;
        self
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }
}

impl Default for Column {
    fn default() -> Self {
        Column::new()
    }
}

impl ParentElement for Column {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Column {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut base = div().flex().flex_col();
        if self.gap > 0.0 {
            base = base.gap(px(self.gap));
        }
        if self.size == MainAxisSize::Max {
            base = base.h_full();
        }
        let base = apply_cross(apply_main(base, self.main), self.cross);
        base.children(self.children)
    }
}
