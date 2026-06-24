//! `Indicator` — a dot or count badge overlaid on a child element.

use gpui::prelude::*;
use gpui::{div, px, AnyElement, App, FontWeight, IntoElement, SharedString, Window};

use crate::theme::{theme, ColorName};

/// A corner indicator over any child. The Mantine `Indicator`.
#[derive(IntoElement)]
pub struct Indicator {
    child: AnyElement,
    label: Option<SharedString>,
    color: ColorName,
    disabled: bool,
}

impl Indicator {
    pub fn new(child: impl IntoElement) -> Self {
        Indicator {
            child: child.into_any_element(),
            label: None,
            color: ColorName::Red,
            disabled: false,
        }
    }

    /// Show a count/text instead of a plain dot.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn color(mut self, color: ColorName) -> Self {
        self.color = color;
        self
    }

    /// Hide the indicator (but keep the child).
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for Indicator {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let accent = t.color(self.color, t.primary_shade());
        let fg = accent.contrasting().hsla();
        let bg = accent.hsla();

        let mut root = div().relative().child(self.child);
        if !self.disabled {
            let dot = match self.label {
                Some(label) => div()
                    .absolute()
                    .top(px(-4.0))
                    .right(px(-4.0))
                    .h(px(16.0))
                    .min_w(px(16.0))
                    .px(px(4.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(16.0))
                    .bg(bg)
                    .text_color(fg)
                    .text_size(px(10.0))
                    .font_weight(FontWeight::BOLD)
                    .child(label),
                None => div()
                    .absolute()
                    .top(px(-2.0))
                    .right(px(-2.0))
                    .w(px(10.0))
                    .h(px(10.0))
                    .rounded(px(10.0))
                    .bg(bg),
            };
            root = root.child(dot);
        }
        root
    }
}
