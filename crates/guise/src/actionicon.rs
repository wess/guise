//! `ActionIcon` — a square icon-only button.

use gpui::prelude::*;
use gpui::{div, px, App, ClickEvent, ElementId, IntoElement, SharedString, Window};

use crate::input::ClickHandler;
use crate::style::{icon_size, surface, ColorValue, Variant};
use crate::theme::{theme, ColorName, Size};

/// A compact, square icon button. The Mantine `ActionIcon`.
#[derive(IntoElement)]
pub struct ActionIcon {
    id: ElementId,
    icon: SharedString,
    variant: Variant,
    color: ColorValue,
    size: Size,
    radius: Option<Size>,
    disabled: bool,
    on_click: Option<ClickHandler>,
}

impl ActionIcon {
    pub fn new(id: impl Into<ElementId>, icon: impl Into<SharedString>) -> Self {
        ActionIcon {
            id: id.into(),
            icon: icon.into(),
            variant: Variant::Subtle,
            color: ColorValue::Named(ColorName::Gray),
            size: Size::Md,
            radius: None,
            disabled: false,
            on_click: None,
        }
    }

    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    pub fn color(mut self, color: impl Into<ColorValue>) -> Self {
        self.color = color.into();
        self
    }

    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    pub fn radius(mut self, radius: Size) -> Self {
        self.radius = Some(radius);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for ActionIcon {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let s = surface(t, self.color, self.variant);
        let dim = icon_size(self.size);
        let radius = t.radius(self.radius.unwrap_or(t.default_radius));

        let mut el = div()
            .id(self.id)
            .w(px(dim))
            .h(px(dim))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(radius))
            .bg(s.bg)
            .text_color(s.fg)
            .text_size(px(dim * 0.5))
            .child(self.icon);
        if let Some(border) = s.border {
            el = el.border_1().border_color(border);
        }

        if self.disabled {
            el.opacity(0.5)
        } else {
            let hover_bg = s.bg_hover;
            el = el.hover(move |st| st.bg(hover_bg));
            if let Some(handler) = self.on_click {
                el = el.on_click(handler);
            }
            el
        }
    }
}
