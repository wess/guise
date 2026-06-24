//! `Wrap` — a flex row that wraps onto multiple lines. Flutter's `Wrap`.

use gpui::prelude::*;
use gpui::{div, px, AnyElement, App, IntoElement, Window};

/// Lays children out in a row, wrapping to new lines as needed.
#[derive(IntoElement)]
pub struct Wrap {
    children: Vec<AnyElement>,
    spacing: f32,
}

impl Wrap {
    pub fn new() -> Self {
        Wrap {
            children: Vec::new(),
            spacing: 8.0,
        }
    }

    /// Gap between children, on both axes.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }
}

impl Default for Wrap {
    fn default() -> Self {
        Wrap::new()
    }
}

impl ParentElement for Wrap {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Wrap {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .flex_wrap()
            .gap(px(self.spacing))
            .children(self.children)
    }
}
