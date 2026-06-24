//! `Group` — children laid out in a row with consistent spacing.

use gpui::prelude::*;
use gpui::{div, px, AnyElement, App, IntoElement, Window};

use super::{apply_align, apply_justify, Align, Justify};
use crate::theme::{theme, Size};

/// A horizontal flex container. The Mantine `Group`.
#[derive(IntoElement)]
pub struct Group {
    children: Vec<AnyElement>,
    gap: Size,
    align: Align,
    justify: Justify,
    wrap: bool,
    grow: bool,
}

impl Group {
    pub fn new() -> Self {
        Group {
            children: Vec::new(),
            gap: Size::Md,
            align: Align::Center,
            justify: Justify::Start,
            wrap: true,
            grow: false,
        }
    }

    pub fn gap(mut self, gap: Size) -> Self {
        self.gap = gap;
        self
    }

    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    pub fn justify(mut self, justify: Justify) -> Self {
        self.justify = justify;
        self
    }

    /// Allow children to wrap onto multiple lines (default true).
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    /// Stretch children to share the available width equally.
    pub fn grow(mut self, grow: bool) -> Self {
        self.grow = grow;
        self
    }
}

impl Default for Group {
    fn default() -> Self {
        Group::new()
    }
}

impl ParentElement for Group {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Group {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let gap = theme(cx).spacing(self.gap);
        let mut base = div().flex().flex_row().gap(px(gap));
        if self.wrap {
            base = base.flex_wrap();
        }
        let grow = self.grow;
        apply_justify(apply_align(base, self.align), self.justify).children(
            self.children.into_iter().map(move |c| {
                if grow {
                    div().flex_1().child(c).into_any_element()
                } else {
                    c
                }
            }),
        )
    }
}
