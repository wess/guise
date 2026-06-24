//! `Stack` — children laid out in a column with consistent spacing.

use gpui::prelude::*;
use gpui::{div, px, AnyElement, App, IntoElement, Window};

use super::{apply_align, apply_justify, Align, Justify};
use crate::theme::{theme, Size};

/// A vertical flex container. The Mantine `Stack`.
#[derive(IntoElement)]
pub struct Stack {
    children: Vec<AnyElement>,
    gap: Size,
    align: Align,
    justify: Justify,
}

impl Stack {
    pub fn new() -> Self {
        Stack {
            children: Vec::new(),
            gap: Size::Md,
            align: Align::Stretch,
            justify: Justify::Start,
        }
    }

    /// Spacing between children.
    pub fn gap(mut self, gap: Size) -> Self {
        self.gap = gap;
        self
    }

    /// Cross-axis (horizontal) alignment.
    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    /// Main-axis (vertical) distribution.
    pub fn justify(mut self, justify: Justify) -> Self {
        self.justify = justify;
        self
    }
}

impl Default for Stack {
    fn default() -> Self {
        Stack::new()
    }
}

impl ParentElement for Stack {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Stack {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let gap = theme(cx).spacing(self.gap);
        let base = div().flex().flex_col().gap(px(gap));
        apply_justify(apply_align(base, self.align), self.justify).children(self.children)
    }
}
