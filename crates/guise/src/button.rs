//! `Button` — the flagship interactive control, with Mantine's variants,
//! colors, and sizes.

use gpui::prelude::*;
use gpui::{
    div, px, App, ClickEvent, ElementId, FontWeight, IntoElement, SharedString, Window,
};

use crate::input::ClickHandler;
use crate::style::{surface, Variant};
use crate::theme::{theme, ColorName, Size};

/// A clickable button. The Mantine `Button`.
#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    label: SharedString,
    variant: Variant,
    color: ColorName,
    size: Size,
    radius: Option<Size>,
    full_width: bool,
    disabled: bool,
    left_section: Option<gpui::AnyElement>,
    right_section: Option<gpui::AnyElement>,
    on_click: Option<ClickHandler>,
}

impl Button {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Button {
            id: id.into(),
            label: label.into(),
            variant: Variant::Filled,
            color: ColorName::Blue,
            size: Size::Sm,
            radius: None,
            full_width: false,
            disabled: false,
            left_section: None,
            right_section: None,
            on_click: None,
        }
    }

    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    /// Override the button color (defaults to the theme's primary color usage).
    pub fn color(mut self, color: ColorName) -> Self {
        self.color = color;
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

    pub fn full_width(mut self, full_width: bool) -> Self {
        self.full_width = full_width;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Content shown before the label (e.g. an icon).
    pub fn left_section(mut self, section: impl IntoElement) -> Self {
        self.left_section = Some(section.into_any_element());
        self
    }

    /// Content shown after the label.
    pub fn right_section(mut self, section: impl IntoElement) -> Self {
        self.right_section = Some(section.into_any_element());
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }

    /// (height, horizontal padding, font size) for the size scale.
    fn metrics(&self) -> (f32, f32, f32) {
        match self.size {
            Size::Xs => (30.0, 14.0, 12.0),
            Size::Sm => (36.0, 18.0, 14.0),
            Size::Md => (42.0, 22.0, 16.0),
            Size::Lg => (50.0, 26.0, 18.0),
            Size::Xl => (60.0, 32.0, 20.0),
        }
    }
}

impl RenderOnce for Button {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let s = surface(t, self.color, self.variant);
        let (height, pad_x, font) = self.metrics();
        let radius = t.radius(self.radius.unwrap_or(t.default_radius));

        let mut el = div()
            .id(self.id)
            .flex()
            .items_center()
            .justify_center()
            .gap(px(8.0))
            .h(px(height))
            .px(px(pad_x))
            .rounded(px(radius))
            .bg(s.bg)
            .text_color(s.fg)
            .text_size(px(font))
            .font_weight(FontWeight::SEMIBOLD);

        if let Some(border) = s.border {
            el = el.border_1().border_color(border);
        }
        if self.full_width {
            el = el.w_full();
        }
        if let Some(left) = self.left_section {
            el = el.child(left);
        }
        el = el.child(self.label);
        if let Some(right) = self.right_section {
            el = el.child(right);
        }

        if self.disabled {
            el.opacity(0.6)
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
