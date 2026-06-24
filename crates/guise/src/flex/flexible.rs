//! Flex helpers: `Expanded`, `Flexible`, `Spacer`, and `SizedBox`.

use gpui::prelude::*;
use gpui::{div, px, relative, App, IntoElement, Window};

/// Fills the available main-axis space inside a Row/Column, by `flex` weight.
/// Flutter's `Expanded`.
#[derive(IntoElement)]
pub struct Expanded {
    child: gpui::AnyElement,
    flex: f32,
}

impl Expanded {
    pub fn new(child: impl IntoElement) -> Self {
        Expanded {
            child: child.into_any_element(),
            flex: 1.0,
        }
    }

    /// The flex weight relative to sibling `Expanded`/`Flexible` (default 1).
    pub fn flex(mut self, flex: f32) -> Self {
        self.flex = flex;
        self
    }
}

impl RenderOnce for Expanded {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        // grow by weight, shrink, zero basis — the child takes its flex share.
        div()
            .flex_grow(self.flex)
            .flex_shrink(1.0)
            .flex_basis(relative(0.0))
            .child(self.child)
    }
}

/// Takes available space up to its content, by `flex` weight. Flutter's
/// `Flexible`.
#[derive(IntoElement)]
pub struct Flexible {
    child: gpui::AnyElement,
    flex: f32,
}

impl Flexible {
    pub fn new(child: impl IntoElement) -> Self {
        Flexible {
            child: child.into_any_element(),
            flex: 1.0,
        }
    }

    pub fn flex(mut self, flex: f32) -> Self {
        self.flex = flex;
        self
    }
}

impl RenderOnce for Flexible {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div().flex_grow(self.flex).child(self.child)
    }
}

/// An empty element that expands to push siblings apart. Flutter's `Spacer`.
#[derive(IntoElement)]
pub struct Spacer {
    flex: f32,
}

impl Spacer {
    pub fn new() -> Self {
        Spacer { flex: 1.0 }
    }

    pub fn flex(mut self, flex: f32) -> Self {
        self.flex = flex;
        self
    }
}

impl Default for Spacer {
    fn default() -> Self {
        Spacer::new()
    }
}

impl RenderOnce for Spacer {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div().flex_grow(self.flex).flex_basis(relative(0.0))
    }
}

/// A fixed-size box, optionally wrapping a child. Flutter's `SizedBox`.
#[derive(IntoElement)]
pub struct SizedBox {
    width: Option<f32>,
    height: Option<f32>,
    expand: bool,
    child: Option<gpui::AnyElement>,
}

impl SizedBox {
    pub fn new() -> Self {
        SizedBox {
            width: None,
            height: None,
            expand: false,
            child: None,
        }
    }

    pub fn width(width: f32) -> Self {
        SizedBox::new().with_width(width)
    }

    pub fn height(height: f32) -> Self {
        SizedBox::new().with_height(height)
    }

    pub fn square(size: f32) -> Self {
        SizedBox::new().with_width(size).with_height(size)
    }

    /// Fills its parent on both axes.
    pub fn expand() -> Self {
        SizedBox {
            expand: true,
            ..SizedBox::new()
        }
    }

    pub fn with_width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn with_height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.child = Some(child.into_any_element());
        self
    }
}

impl Default for SizedBox {
    fn default() -> Self {
        SizedBox::new()
    }
}

impl RenderOnce for SizedBox {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut el = div();
        if self.expand {
            el = el.size_full();
        }
        if let Some(w) = self.width {
            el = el.w(px(w));
        }
        if let Some(h) = self.height {
            el = el.h(px(h));
        }
        if let Some(child) = self.child {
            el = el.child(child);
        }
        el
    }
}
