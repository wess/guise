//! `Popover` — the reusable anchored-floating primitive.
//!
//! A trigger plus a deferred panel positioned relative to it. This is the
//! shared mechanism behind dropdown-style UI (`Menu`/`Select` predate it and
//! still hand-roll their own); build `Drawer`, combobox dropdowns, and custom
//! flyouts on top of it.
//!
//! Both the trigger and the content are **builder closures**, re-invoked each
//! render so they can show live data. The popover closes on Escape or a second
//! trigger click; call [`Popover::close`] from a content action to dismiss it.

use gpui::prelude::*;
use gpui::{
    deferred, div, px, relative, AnyElement, App, Context, FocusHandle, IntoElement, KeyDownEvent,
    Window,
};

use crate::theme::theme;

type Builder = Box<dyn Fn(&mut Window, &mut App) -> AnyElement + 'static>;

/// Where the panel sits relative to its trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// Below, left-aligned (the default).
    Bottom,
    /// Below, right-aligned.
    BottomEnd,
    /// Above, left-aligned.
    Top,
    /// Above, right-aligned.
    TopEnd,
}

/// An anchored floating panel. Create with `cx.new(|cx| Popover::new(cx, ..))`.
pub struct Popover {
    open: bool,
    focus: FocusHandle,
    trigger: Builder,
    content: Builder,
    placement: Placement,
    width: Option<f32>,
}

impl Popover {
    pub fn new(
        cx: &mut Context<Self>,
        trigger: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
        content: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        Popover {
            open: false,
            focus: cx.focus_handle(),
            trigger: Box::new(trigger),
            content: Box::new(content),
            placement: Placement::Bottom,
            width: None,
        }
    }

    pub fn placement(mut self, placement: Placement) -> Self {
        self.placement = placement;
        self
    }

    /// Fix the panel width (otherwise it sizes to content, min 180px).
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn open(&mut self, cx: &mut Context<Self>) {
        self.open = true;
        cx.notify();
    }

    pub fn close(&mut self, cx: &mut Context<Self>) {
        self.open = false;
        cx.notify();
    }

    pub fn toggle(&mut self, cx: &mut Context<Self>) {
        self.open = !self.open;
        cx.notify();
    }
}

impl Render for Popover {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        let radius = t.radius(t.default_radius);
        let surface = t.surface().hsla();
        let border = t.border().hsla();

        let trigger_el = (self.trigger)(window, cx);
        let mut wrap = div().relative().child(
            div()
                .id("guise-popover-trigger")
                .track_focus(&self.focus)
                .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _window, cx| {
                    if ev.keystroke.key.as_str() == "escape" {
                        this.open = false;
                        cx.notify();
                    }
                }))
                .on_click(cx.listener(|this, _ev, window, cx| {
                    this.open = !this.open;
                    window.focus(&this.focus, cx);
                    cx.notify();
                }))
                .child(trigger_el),
        );

        if self.open {
            let content_el = (self.content)(window, cx);
            let base = div()
                .absolute()
                .flex()
                .flex_col()
                .p(px(4.0))
                .rounded(px(radius))
                .border_1()
                .border_color(border)
                .bg(surface)
                .shadow_md()
                .child(content_el);
            let placed = match self.placement {
                Placement::Bottom => base.top(relative(1.0)).mt(px(6.0)).left(px(0.0)),
                Placement::BottomEnd => base.top(relative(1.0)).mt(px(6.0)).right(px(0.0)),
                Placement::Top => base.bottom(relative(1.0)).mb(px(6.0)).left(px(0.0)),
                Placement::TopEnd => base.bottom(relative(1.0)).mb(px(6.0)).right(px(0.0)),
            };
            let placed = match self.width {
                Some(w) => placed.w(px(w)),
                None => placed.min_w(px(180.0)),
            };
            wrap = wrap.child(deferred(placed));
        }

        wrap
    }
}
