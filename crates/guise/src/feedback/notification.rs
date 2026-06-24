//! `Notification` — an elevated toast card with an accent bar.

use gpui::prelude::*;
use gpui::{div, px, App, ClickEvent, FontWeight, IntoElement, SharedString, Window};

use crate::input::ClickHandler;
use crate::theme::{theme, ColorName, Size};

/// A toast-style notification card. The Mantine `Notification`. Positioning and
/// stacking are the host's responsibility; this is the visual card.
#[derive(IntoElement)]
pub struct Notification {
    title: Option<SharedString>,
    message: SharedString,
    color: ColorName,
    icon: Option<SharedString>,
    on_close: Option<ClickHandler>,
}

impl Notification {
    pub fn new(message: impl Into<SharedString>) -> Self {
        Notification {
            title: None,
            message: message.into(),
            color: ColorName::Blue,
            icon: None,
            on_close: None,
        }
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn color(mut self, color: ColorName) -> Self {
        self.color = color;
        self
    }

    pub fn icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn on_close(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_close = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for Notification {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let accent = t.color(self.color, t.primary_shade()).hsla();
        let surface = t.surface().hsla();
        let text = t.text().hsla();
        let dimmed = t.dimmed().hsla();
        let radius = t.radius(Size::Md);

        let mut content = div().flex().flex_col().gap(px(2.0)).flex_1();
        if let Some(title) = self.title {
            content = content.child(
                div()
                    .font_weight(FontWeight::BOLD)
                    .text_size(px(t.font_size(Size::Sm)))
                    .text_color(text)
                    .child(title),
            );
        }
        content = content.child(
            div()
                .text_size(px(t.font_size(Size::Sm)))
                .text_color(dimmed)
                .child(self.message),
        );

        let mut row = div()
            .flex()
            .items_start()
            .gap(px(12.0))
            .w(px(360.0))
            .p(px(t.spacing(Size::Md)))
            .rounded(px(radius))
            .bg(surface)
            .border_1()
            .border_color(t.border().hsla())
            .shadow_md()
            // Leading accent bar.
            .child(div().w(px(4.0)).h(px(38.0)).rounded(px(4.0)).bg(accent));

        if let Some(icon) = self.icon {
            row = row.child(div().text_size(px(t.font_size(Size::Md))).text_color(accent).child(icon));
        }
        row = row.child(content);
        if let Some(handler) = self.on_close {
            row = row.child(
                div()
                    .id("guise-notification-close")
                    .text_color(dimmed)
                    .child(SharedString::new_static("\u{00d7}"))
                    .on_click(handler),
            );
        }
        row
    }
}
