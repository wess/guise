//! `Stack` and `Positioned` — Flutter's overlap layout.

use gpui::prelude::*;
use gpui::{div, px, AnyElement, App, IntoElement, Window};

/// Overlays children on top of one another. Flutter's `Stack`.
///
/// The first child sits in normal flow and defines the size; wrap later
/// children in [`Positioned`] to place them as overlays.
#[derive(IntoElement)]
pub struct Stack {
    children: Vec<AnyElement>,
}

impl Stack {
    pub fn new() -> Self {
        Stack {
            children: Vec::new(),
        }
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
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div().relative().children(self.children)
    }
}

/// Positions a child within a [`Stack`] by edge offsets. Flutter's `Positioned`.
#[derive(IntoElement)]
pub struct Positioned {
    child: AnyElement,
    top: Option<f32>,
    right: Option<f32>,
    bottom: Option<f32>,
    left: Option<f32>,
    width: Option<f32>,
    height: Option<f32>,
}

impl Positioned {
    pub fn new(child: impl IntoElement) -> Self {
        Positioned {
            child: child.into_any_element(),
            top: None,
            right: None,
            bottom: None,
            left: None,
            width: None,
            height: None,
        }
    }

    /// Pins to all four edges (fills the stack).
    pub fn fill(child: impl IntoElement) -> Self {
        Positioned::new(child)
            .top(0.0)
            .right(0.0)
            .bottom(0.0)
            .left(0.0)
    }

    pub fn top(mut self, value: f32) -> Self {
        self.top = Some(value);
        self
    }

    pub fn right(mut self, value: f32) -> Self {
        self.right = Some(value);
        self
    }

    pub fn bottom(mut self, value: f32) -> Self {
        self.bottom = Some(value);
        self
    }

    pub fn left(mut self, value: f32) -> Self {
        self.left = Some(value);
        self
    }

    pub fn width(mut self, value: f32) -> Self {
        self.width = Some(value);
        self
    }

    pub fn height(mut self, value: f32) -> Self {
        self.height = Some(value);
        self
    }
}

impl RenderOnce for Positioned {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut el = div().absolute();
        if let Some(v) = self.top {
            el = el.top(px(v));
        }
        if let Some(v) = self.right {
            el = el.right(px(v));
        }
        if let Some(v) = self.bottom {
            el = el.bottom(px(v));
        }
        if let Some(v) = self.left {
            el = el.left(px(v));
        }
        if let Some(v) = self.width {
            el = el.w(px(v));
        }
        if let Some(v) = self.height {
            el = el.h(px(v));
        }
        el.child(self.child)
    }
}
