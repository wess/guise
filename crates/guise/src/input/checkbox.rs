//! `Checkbox` — a controlled boolean toggle with an optional label.

use gpui::prelude::*;
use gpui::{div, px, App, ClickEvent, ElementId, FontWeight, IntoElement, SharedString, Window};

use super::{control_box_size, ClickHandler};
use crate::theme::{theme, ColorName, Size};

/// A controlled checkbox. The Mantine `Checkbox`. Pass `checked` and a change
/// handler (via `cx.listener`); the parent view owns the value.
#[derive(IntoElement)]
pub struct Checkbox {
    id: ElementId,
    checked: bool,
    indeterminate: bool,
    label: Option<SharedString>,
    size: Size,
    color: ColorName,
    disabled: bool,
    on_change: Option<ClickHandler>,
}

impl Checkbox {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Checkbox {
            id: id.into(),
            checked: false,
            indeterminate: false,
            label: None,
            size: Size::Sm,
            color: ColorName::Blue,
            disabled: false,
            on_change: None,
        }
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn indeterminate(mut self, indeterminate: bool) -> Self {
        self.indeterminate = indeterminate;
        self
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    pub fn color(mut self, color: ColorName) -> Self {
        self.color = color;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for Checkbox {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let on = self.checked || self.indeterminate;
        let accent = t.color(self.color, t.primary_shade());
        let box_size = control_box_size(self.size);

        let mut check = div()
            .w(px(box_size))
            .h(px(box_size))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(t.radius(Size::Xs) + 2.0))
            .text_size(px(box_size * 0.7))
            .font_weight(FontWeight::BOLD);
        if on {
            check = check
                .bg(accent.hsla())
                .text_color(accent.contrasting().hsla())
                .child(SharedString::new_static(if self.indeterminate {
                    "\u{2212}"
                } else {
                    "\u{2713}"
                }));
        } else {
            check = check
                .bg(t.surface().hsla())
                .border_1()
                .border_color(t.border().hsla());
        }

        let mut row = div().id(self.id).flex().items_center().gap(px(8.0)).child(check);
        if let Some(label) = self.label {
            row = row.child(
                div()
                    .text_size(px(t.font_size(self.size)))
                    .text_color(t.text().hsla())
                    .child(label),
            );
        }

        if self.disabled {
            row.opacity(0.5)
        } else {
            if let Some(handler) = self.on_change {
                row = row.on_click(handler);
            }
            row
        }
    }
}
