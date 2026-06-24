//! `Radio` — a controlled single-choice button with an optional label.

use gpui::prelude::*;
use gpui::{div, px, App, ClickEvent, ElementId, IntoElement, SharedString, Window};

use super::{control_box_size, ClickHandler};
use crate::theme::{theme, ColorName, Size};

/// A controlled radio button. The Mantine `Radio`. Grouping/exclusivity is the
/// parent view's responsibility — give each a `checked` and a change handler.
#[derive(IntoElement)]
pub struct Radio {
    id: ElementId,
    checked: bool,
    label: Option<SharedString>,
    size: Size,
    color: ColorName,
    disabled: bool,
    on_change: Option<ClickHandler>,
}

impl Radio {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Radio {
            id: id.into(),
            checked: false,
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

impl RenderOnce for Radio {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let outer = control_box_size(self.size);
        let accent = t.color(self.color, t.primary_shade());

        let mut ring = div()
            .w(px(outer))
            .h(px(outer))
            .rounded(px(outer))
            .flex()
            .items_center()
            .justify_center();
        if self.checked {
            ring = ring.bg(accent.hsla()).child(
                div()
                    .w(px(outer * 0.36))
                    .h(px(outer * 0.36))
                    .rounded(px(outer))
                    .bg(t.white.hsla()),
            );
        } else {
            ring = ring
                .bg(t.surface().hsla())
                .border_1()
                .border_color(t.border().hsla());
        }

        let mut row = div().id(self.id).flex().items_center().gap(px(8.0)).child(ring);
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
