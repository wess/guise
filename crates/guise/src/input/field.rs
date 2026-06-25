//! `Field` — the shared label / description / error chrome that wraps a form
//! control. Extracted so every input draws its surrounding text the same way;
//! `NumberInput`, `TextArea`, and `Combobox` all compose it.

use gpui::prelude::*;
use gpui::{div, px, AnyElement, App, IntoElement, SharedString, Window};

use crate::theme::{theme, ColorName, Size};

/// Label / description / error wrapper around a single control.
#[derive(IntoElement)]
pub struct Field {
    label: Option<SharedString>,
    description: Option<SharedString>,
    error: Option<SharedString>,
    child: Option<AnyElement>,
}

impl Field {
    pub fn new() -> Self {
        Field {
            label: None,
            description: None,
            error: None,
            child: None,
        }
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn error(mut self, error: impl Into<SharedString>) -> Self {
        self.error = Some(error.into());
        self
    }

    /// The wrapped control.
    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.child = Some(child.into_any_element());
        self
    }
}

impl Default for Field {
    fn default() -> Self {
        Field::new()
    }
}

impl RenderOnce for Field {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let text = t.text().hsla();
        let dimmed = t.dimmed().hsla();
        let error_color = t
            .color(ColorName::Red, if t.scheme.is_dark() { 5 } else { 7 })
            .hsla();
        let font_sm = t.font_size(Size::Sm);
        let font_xs = t.font_size(Size::Xs);

        let mut column = div().flex().flex_col().gap(px(4.0));
        if let Some(label) = self.label {
            column = column.child(
                div()
                    .text_size(px(font_sm))
                    .text_color(text)
                    .child(label),
            );
        }
        if let Some(child) = self.child {
            column = column.child(child);
        }
        if let Some(error) = self.error {
            column = column.child(
                div()
                    .text_size(px(font_xs))
                    .text_color(error_color)
                    .child(error),
            );
        } else if let Some(description) = self.description {
            column = column.child(
                div()
                    .text_size(px(font_xs))
                    .text_color(dimmed)
                    .child(description),
            );
        }
        column
    }
}
