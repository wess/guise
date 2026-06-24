//! `Chip` — a selectable pill (controlled).

use gpui::prelude::*;
use gpui::{div, px, App, ClickEvent, ElementId, FontWeight, IntoElement, SharedString, Window};

use crate::input::ClickHandler;
use crate::theme::{theme, ColorName, Size};

/// A selectable chip. The Mantine `Chip`. Controlled: pass `checked` and a
/// change handler via `cx.listener`.
#[derive(IntoElement)]
pub struct Chip {
    id: ElementId,
    label: SharedString,
    checked: bool,
    color: ColorName,
    size: Size,
    on_change: Option<ClickHandler>,
}

impl Chip {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Chip {
            id: id.into(),
            label: label.into(),
            checked: false,
            color: ColorName::Blue,
            size: Size::Md,
            on_change: None,
        }
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn color(mut self, color: ColorName) -> Self {
        self.color = color;
        self
    }

    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }

    fn metrics(&self) -> (f32, f32, f32) {
        match self.size {
            Size::Xs => (24.0, 10.0, 11.0),
            Size::Sm => (28.0, 12.0, 12.0),
            Size::Md => (32.0, 16.0, 14.0),
            Size::Lg => (38.0, 20.0, 16.0),
            Size::Xl => (44.0, 24.0, 18.0),
        }
    }
}

impl RenderOnce for Chip {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let (height, pad_x, font) = self.metrics();
        let accent = t.color(self.color, t.primary_shade());

        let (bg, fg, border) = if self.checked {
            let light = if t.scheme.is_dark() {
                accent.alpha(0.20)
            } else {
                t.color(self.color, 0).hsla()
            };
            (light, accent.hsla(), accent.hsla())
        } else {
            (t.surface().hsla(), t.text().hsla(), t.border().hsla())
        };
        let hover_bg = t.surface_hover().hsla();

        let mut el = div()
            .id(self.id)
            .flex()
            .items_center()
            .gap(px(6.0))
            .h(px(height))
            .px(px(pad_x))
            .rounded(px(height))
            .border_1()
            .border_color(border)
            .bg(bg)
            .text_color(fg)
            .text_size(px(font))
            .font_weight(FontWeight::MEDIUM);
        if self.checked {
            el = el.child(SharedString::new_static("\u{2713}"));
        } else {
            el = el.hover(move |s| s.bg(hover_bg));
        }
        el = el.child(self.label);
        if let Some(handler) = self.on_change {
            el = el.on_click(handler);
        }
        el
    }
}
