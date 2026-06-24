//! `Divider` — a thin separating line, optionally with a centered label.

use gpui::prelude::*;
use gpui::{div, px, App, IntoElement, SharedString, Window};

use crate::theme::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

/// A separator line. The Mantine `Divider`.
#[derive(IntoElement)]
pub struct Divider {
    orientation: Orientation,
    label: Option<SharedString>,
}

impl Divider {
    pub fn new() -> Self {
        Divider {
            orientation: Orientation::Horizontal,
            label: None,
        }
    }

    pub fn vertical() -> Self {
        Divider {
            orientation: Orientation::Vertical,
            label: None,
        }
    }

    /// A label rendered centered on a horizontal divider.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }
}

impl Default for Divider {
    fn default() -> Self {
        Divider::new()
    }
}

impl RenderOnce for Divider {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let line_color = t.border().hsla();

        if self.orientation == Orientation::Vertical {
            return div().w(px(1.0)).h_full().bg(line_color);
        }

        match self.label {
            None => div().w_full().h(px(1.0)).bg(line_color),
            Some(label) => div()
                .flex()
                .items_center()
                .gap(px(t.spacing(crate::theme::Size::Sm)))
                .w_full()
                .child(div().flex_1().h(px(1.0)).bg(line_color))
                .child(
                    div()
                        .text_size(px(t.font_size(crate::theme::Size::Sm)))
                        .text_color(t.dimmed().hsla())
                        .child(label),
                )
                .child(div().flex_1().h(px(1.0)).bg(line_color)),
        }
    }
}
