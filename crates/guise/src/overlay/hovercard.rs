//! `HoverCard` — an anchored panel that opens on hover (gpui entity).
//!
//! The pointer-driven sibling of [`Popover`](super::Popover): a trigger plus a
//! deferred card, both **builder closures** re-invoked every render. Hovering
//! the trigger opens the card after a short delay; leaving both the trigger
//! and the card closes it after another. Moving the pointer onto the card
//! keeps it open, so its content can be interacted with.
//!
//! ```ignore
//! let card = cx.new(|cx| {
//!     HoverCard::new(
//!         cx,
//!         |_w, _app| Badge::new("@ada").into_any_element(),
//!         |_w, _app| Text::new("Ada Lovelace — wrote the first program.").into_any_element(),
//!     )
//!     .placement(Placement::Bottom)
//!     .width(260.0)
//! });
//! ```

use std::time::Duration;

use gpui::prelude::*;
use gpui::{deferred, div, px, relative, AnyElement, App, Context, IntoElement, Task, Window};

use super::Placement;
use crate::theme::theme;

type Builder = Box<dyn Fn(&mut Window, &mut App) -> AnyElement + 'static>;

/// An on-hover floating card. Create with `cx.new(|cx| HoverCard::new(cx, ..))`.
pub struct HoverCard {
    open: bool,
    trigger: Builder,
    content: Builder,
    placement: Placement,
    width: Option<f32>,
    open_delay: Duration,
    close_delay: Duration,
    over_trigger: bool,
    over_card: bool,
    /// The one in-flight open/close timer. Replacing it drops (cancels) the
    /// previous task, so at most one delayed transition is ever pending.
    pending: Option<Task<()>>,
}

impl HoverCard {
    pub fn new(
        _cx: &mut Context<Self>,
        trigger: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
        content: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        HoverCard {
            open: false,
            trigger: Box::new(trigger),
            content: Box::new(content),
            placement: Placement::Bottom,
            width: None,
            open_delay: Duration::from_millis(300),
            close_delay: Duration::from_millis(150),
            over_trigger: false,
            over_card: false,
            pending: None,
        }
    }

    pub fn placement(mut self, placement: Placement) -> Self {
        self.placement = placement;
        self
    }

    /// Fix the card width (otherwise it sizes to content, min 180px).
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Hover time before the card opens (default 300ms).
    pub fn open_delay(mut self, delay: Duration) -> Self {
        self.open_delay = delay;
        self
    }

    /// Grace period after the pointer leaves before the card closes
    /// (default 150ms) — long enough to travel from trigger to card.
    pub fn close_delay(mut self, delay: Duration) -> Self {
        self.close_delay = delay;
        self
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Close immediately (e.g. from an action inside the card).
    pub fn close(&mut self, cx: &mut Context<Self>) {
        self.pending = None;
        self.over_trigger = false;
        self.over_card = false;
        self.open = false;
        cx.notify();
    }

    fn hover_trigger(&mut self, hovered: bool, cx: &mut Context<Self>) {
        self.over_trigger = hovered;
        if hovered {
            self.schedule_open(cx);
        } else {
            self.schedule_close(cx);
        }
    }

    fn hover_card(&mut self, hovered: bool, cx: &mut Context<Self>) {
        self.over_card = hovered;
        if hovered {
            // Cancel any pending close: the pointer arrived on the card.
            self.pending = None;
        } else {
            self.schedule_close(cx);
        }
    }

    fn schedule_open(&mut self, cx: &mut Context<Self>) {
        if self.open {
            // Already open — just cancel a pending close.
            self.pending = None;
            return;
        }
        let delay = self.open_delay;
        self.pending = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(delay).await;
            this.update(cx, |this, cx| {
                if this.over_trigger && !this.open {
                    this.open = true;
                    cx.notify();
                }
            })
            .ok();
        }));
    }

    fn schedule_close(&mut self, cx: &mut Context<Self>) {
        if !self.open {
            // Not open yet — dropping the pending task cancels a scheduled open.
            self.pending = None;
            return;
        }
        let delay = self.close_delay;
        self.pending = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(delay).await;
            this.update(cx, |this, cx| {
                if !this.over_trigger && !this.over_card && this.open {
                    this.open = false;
                    cx.notify();
                }
            })
            .ok();
        }));
    }
}

impl Render for HoverCard {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        let radius = t.radius(t.default_radius);
        let surface = t.surface().hsla();
        let border = t.border().hsla();

        let trigger_el = (self.trigger)(window, cx);
        let mut wrap = div().relative().child(
            div()
                .id("guise-hovercard-trigger")
                .on_hover(cx.listener(|this, hovered: &bool, _window, cx| {
                    this.hover_trigger(*hovered, cx);
                }))
                .child(trigger_el),
        );

        if self.open {
            let content_el = (self.content)(window, cx);
            let base = div()
                .id("guise-hovercard-card")
                .on_hover(cx.listener(|this, hovered: &bool, _window, cx| {
                    this.hover_card(*hovered, cx);
                }))
                .absolute()
                .flex()
                .flex_col()
                .p(px(12.0))
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
