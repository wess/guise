//! `Drawer` — a panel that slides in from a window edge over a scrim.
//!
//! Controlled like [`Modal`](super::Modal): the parent owns `opened` and renders
//! the `Drawer` only while true, passing an `on_close` handler. Place it as a
//! child of a full-size root. Clicking the scrim or close button invokes
//! `on_close`.

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    deferred, div, px, AnyElement, App, ClickEvent, FontWeight, IntoElement, SharedString, Window,
};

use crate::theme::{theme, Size};

type CloseHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// Which edge the drawer slides from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
    Top,
    Bottom,
}

/// A slide-in panel. The Mantine `Drawer`.
#[derive(IntoElement)]
pub struct Drawer {
    title: Option<SharedString>,
    children: Vec<AnyElement>,
    side: Side,
    size: f32,
    padding: Size,
    on_close: Option<CloseHandler>,
}

impl Drawer {
    pub fn new() -> Self {
        Drawer {
            title: None,
            children: Vec::new(),
            side: Side::Right,
            size: 360.0,
            padding: Size::Lg,
            on_close: None,
        }
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn side(mut self, side: Side) -> Self {
        self.side = side;
        self
    }

    /// Width for left/right drawers, height for top/bottom (px).
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn padding(mut self, padding: Size) -> Self {
        self.padding = padding;
        self
    }

    pub fn on_close(mut self, handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(handler));
        self
    }
}

impl Default for Drawer {
    fn default() -> Self {
        Drawer::new()
    }
}

impl ParentElement for Drawer {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Drawer {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let surface = t.surface().hsla();
        let text = t.text().hsla();
        let dimmed = t.dimmed().hsla();
        let scrim = t.black.alpha(0.55);
        let padding = t.spacing(self.padding);
        let gap = t.spacing(Size::Md);
        let viewport = window.viewport_size();
        let close = self.on_close.clone();

        let mut panel = div()
            .id("guise-drawer-panel")
            .occlude()
            .flex()
            .flex_col()
            .gap(px(gap))
            .bg(surface)
            .p(px(padding))
            .shadow_xl()
            .on_click(|_ev, _window, cx| cx.stop_propagation());
        panel = match self.side {
            Side::Left | Side::Right => panel.h(viewport.height).w(px(self.size)),
            Side::Top | Side::Bottom => panel.w(viewport.width).h(px(self.size)),
        };

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
                        .id("guise-drawer-close")
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
            panel = panel.child(header);
        }
        panel = panel.children(self.children);

        let mut backdrop = div()
            .id("guise-drawer-backdrop")
            .occlude()
            .absolute()
            .top(px(0.0))
            .left(px(0.0))
            .w(viewport.width)
            .h(viewport.height)
            .flex()
            .bg(scrim);
        backdrop = match self.side {
            Side::Left => backdrop.flex_row().justify_start(),
            Side::Right => backdrop.flex_row().justify_end(),
            Side::Top => backdrop.flex_col().justify_start(),
            Side::Bottom => backdrop.flex_col().justify_end(),
        };
        backdrop = backdrop.child(panel);

        if let Some(handler) = close {
            backdrop = backdrop.on_click(move |ev, window, cx| handler(ev, window, cx));
        }

        deferred(backdrop)
    }
}
