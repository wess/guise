//! `Alert` — an inline callout for info, success, warning, and error states.

use gpui::prelude::*;
use gpui::{div, px, App, ClickEvent, FontWeight, IntoElement, SharedString, Window};

use crate::icon::Glyph;
use crate::input::ClickHandler;
use crate::style::{surface, ColorValue, Variant};
use crate::theme::{theme, Size};

/// A colored message callout. The Mantine `Alert`.
#[derive(IntoElement)]
pub struct Alert {
    title: Option<SharedString>,
    message: SharedString,
    variant: Variant,
    color: ColorValue,
    icon: Option<Glyph>,
    on_close: Option<ClickHandler>,
}

impl Alert {
    pub fn new(message: impl Into<SharedString>) -> Self {
        Alert {
            title: None,
            message: message.into(),
            variant: Variant::Light,
            color: ColorValue::default(),
            icon: None,
            on_close: None,
        }
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    pub fn color(mut self, color: impl Into<ColorValue>) -> Self {
        self.color = color.into();
        self
    }

    /// A leading glyph (a Lucide [`IconName`](crate::IconName) or a character).
    pub fn icon(mut self, icon: impl Into<Glyph>) -> Self {
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

impl RenderOnce for Alert {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let s = surface(t, self.color, self.variant);
        let filled = self.variant == Variant::Filled;
        let body_color = if filled { s.fg } else { t.text().hsla() };
        let radius = t.radius(Size::Md);
        let pad = t.spacing(Size::Md);

        let mut content = div().flex().flex_col().gap(px(2.0)).flex_1();
        if let Some(title) = self.title {
            content = content.child(
                div()
                    .font_weight(FontWeight::BOLD)
                    .text_size(px(t.font_size(Size::Sm)))
                    .text_color(s.fg)
                    .child(title),
            );
        }
        content = content.child(
            div()
                .text_size(px(t.font_size(Size::Sm)))
                .text_color(body_color)
                .child(self.message),
        );

        let mut row = div()
            .flex()
            .items_start()
            .gap(px(10.0))
            .p(px(pad))
            .rounded(px(radius))
            .bg(s.bg);
        if let Some(border) = s.border {
            row = row.border_1().border_color(border);
        }
        if let Some(icon) = self.icon {
            row = row.child(
                div()
                    .text_size(px(t.font_size(Size::Md)))
                    .text_color(s.fg)
                    .child(icon),
            );
        }
        row = row.child(content);
        if let Some(handler) = self.on_close {
            let close_color = if filled { s.fg } else { t.dimmed().hsla() };
            row = row.child(
                div()
                    .id("guise-alert-close")
                    .text_color(close_color)
                    .child(SharedString::new_static("\u{00d7}"))
                    .on_click(handler),
            );
        }
        row
    }
}
