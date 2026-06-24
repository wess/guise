//! `Center` — centers its content on both axes.

use gpui::prelude::*;
use gpui::{div, AnyElement, App, IntoElement, Window};

/// A flex container that centers its children. The Mantine `Center`.
#[derive(IntoElement)]
pub struct Center {
    children: Vec<AnyElement>,
    inline: bool,
}

impl Center {
    pub fn new() -> Self {
        Center {
            children: Vec::new(),
            inline: false,
        }
    }

    /// Lay out inline (shrink to content) instead of filling the parent.
    pub fn inline(mut self, inline: bool) -> Self {
        self.inline = inline;
        self
    }
}

impl Default for Center {
    fn default() -> Self {
        Center::new()
    }
}

impl ParentElement for Center {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Center {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut base = div().flex().items_center().justify_center();
        if !self.inline {
            base = base.size_full();
        }
        base.children(self.children)
    }
}
