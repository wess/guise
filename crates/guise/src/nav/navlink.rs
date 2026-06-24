//! `NavLink` — a sidebar navigation row with active state.

use gpui::prelude::*;
use gpui::{div, px, App, ClickEvent, ElementId, FontWeight, IntoElement, SharedString, Window};

use crate::input::ClickHandler;
use crate::theme::{theme, ColorName, Size};

/// A navigation link. The Mantine `NavLink`.
#[derive(IntoElement)]
pub struct NavLink {
    id: ElementId,
    label: SharedString,
    description: Option<SharedString>,
    icon: Option<SharedString>,
    color: ColorName,
    active: bool,
    on_click: Option<ClickHandler>,
}

impl NavLink {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        NavLink {
            id: id.into(),
            label: label.into(),
            description: None,
            icon: None,
            color: ColorName::Blue,
            active: false,
            on_click: None,
        }
    }

    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn color(mut self, color: ColorName) -> Self {
        self.color = color;
        self
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
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

impl RenderOnce for NavLink {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let radius = t.radius(Size::Sm);
        let font = t.font_size(Size::Sm);
        let dimmed = t.dimmed().hsla();
        let hover_bg = t.surface_hover().hsla();
        let (bg, label_color) = if self.active {
            let accent = t.color(self.color, if t.scheme.is_dark() { 4 } else { 7 });
            (
                t.color(self.color, if t.scheme.is_dark() { 8 } else { 0 }).hsla(),
                accent.hsla(),
            )
        } else {
            (gpui::transparent_black(), t.text().hsla())
        };

        let mut text_col = div().flex().flex_col().gap(px(1.0)).child(
            div()
                .text_size(px(font))
                .font_weight(FontWeight::MEDIUM)
                .text_color(label_color)
                .child(self.label),
        );
        if let Some(description) = self.description {
            text_col = text_col.child(
                div()
                    .text_size(px(t.font_size(Size::Xs)))
                    .text_color(dimmed)
                    .child(description),
            );
        }

        let mut row = div()
            .id(self.id)
            .flex()
            .items_center()
            .gap(px(10.0))
            .px(px(10.0))
            .py(px(8.0))
            .rounded(px(radius))
            .bg(bg)
            .hover(move |s| s.bg(hover_bg));
        if let Some(icon) = self.icon {
            row = row.child(
                div()
                    .text_size(px(t.font_size(Size::Md)))
                    .text_color(label_color)
                    .child(icon),
            );
        }
        row = row.child(text_col);
        if let Some(handler) = self.on_click {
            row = row.on_click(handler);
        }
        row
    }
}
