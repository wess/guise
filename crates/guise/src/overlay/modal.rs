//! `Modal` — a centered dialog over a dimming backdrop.
//!
//! Controlled: the parent owns `opened` and renders the `Modal` only while it
//! is true, passing an `on_close` handler. Place it as a child of a full-size
//! root so the backdrop covers the window. Clicking the backdrop or the close
//! button invokes `on_close`.

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    deferred, div, px, AnyElement, App, ClickEvent, FontWeight, IntoElement, SharedString, Window,
};

use crate::theme::{theme, Size};

type CloseHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// A modal dialog. The Mantine `Modal`.
#[derive(IntoElement)]
pub struct Modal {
    title: Option<SharedString>,
    children: Vec<AnyElement>,
    width: f32,
    padding: Size,
    radius: Option<Size>,
    on_close: Option<CloseHandler>,
}

impl Modal {
    pub fn new() -> Self {
        Modal {
            title: None,
            children: Vec::new(),
            width: 440.0,
            padding: Size::Lg,
            radius: Some(Size::Md),
            on_close: None,
        }
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn padding(mut self, padding: Size) -> Self {
        self.padding = padding;
        self
    }

    pub fn radius(mut self, radius: Size) -> Self {
        self.radius = Some(radius);
        self
    }

    /// Called when the backdrop or close button is clicked. Wire it with
    /// `cx.listener(...)` to flip the parent's `opened` flag.
    pub fn on_close(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_close = Some(Rc::new(handler));
        self
    }
}

impl Default for Modal {
    fn default() -> Self {
        Modal::new()
    }
}

impl ParentElement for Modal {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Modal {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let radius = t.radius(self.radius.unwrap_or(Size::Md));
        let padding = t.spacing(self.padding);
        let gap = t.spacing(Size::Md);
        let surface = t.surface().hsla();
        let text = t.text().hsla();
        let dimmed = t.dimmed().hsla();
        let scrim = t.black.alpha(0.55);

        let viewport = window.viewport_size();
        let close = self.on_close.clone();

        let mut dialog = div()
            .id("guise-modal-dialog")
            .occlude()
            .flex()
            .flex_col()
            .gap(px(gap))
            .w(px(self.width))
            .bg(surface)
            .rounded(px(radius))
            .p(px(padding))
            .shadow_xl()
            .on_click(|_ev, _window, cx| cx.stop_propagation());

        if self.title.is_some() || close.is_some() {
            let mut header = div().flex().items_center().justify_between();
            header = header.child(match self.title {
                Some(title) => div()
                    .text_size(px(18.0))
                    .font_weight(FontWeight::BOLD)
                    .text_color(text)
                    .child(title),
                None => div(),
            });
            if let Some(handler) = close.clone() {
                header = header.child(
                    div()
                        .id("guise-modal-close")
                        .px(px(6.0))
                        .rounded(px(4.0))
                        .text_size(px(18.0))
                        .text_color(dimmed)
                        .hover(move |s| s.text_color(text))
                        .child(SharedString::new_static("\u{00d7}"))
                        .on_click(move |ev, window, cx| {
                            handler(ev, window, cx);
                            cx.stop_propagation();
                        }),
                );
            }
            dialog = dialog.child(header);
        }

        dialog = dialog.children(self.children);

        let mut backdrop = div()
            .id("guise-modal-backdrop")
            .occlude()
            .absolute()
            .top(px(0.0))
            .left(px(0.0))
            .w(viewport.width)
            .h(viewport.height)
            .flex()
            .items_center()
            .justify_center()
            .bg(scrim)
            .child(dialog);

        if let Some(handler) = close {
            backdrop = backdrop.on_click(move |ev, window, cx| handler(ev, window, cx));
        }

        deferred(backdrop)
    }
}
