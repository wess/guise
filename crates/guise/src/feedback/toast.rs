//! `ToastStack` — a positioned, stacking toast manager (gpui entity).
//!
//! Holds a list of live toasts and paints them as a deferred, top-right stack
//! above the page. Push from anywhere you hold the entity handle; each card has
//! a close button. Pushed toasts auto-dismiss after four seconds by default;
//! set [`ToastStack::duration`] to change the delay or pass `None` to keep
//! toasts until closed.

use std::time::Duration;

use gpui::prelude::*;
use gpui::{deferred, div, px, Context, FontWeight, IntoElement, SharedString, Window};

use crate::theme::{theme, ColorName, Size};

struct Toast {
    id: usize,
    title: Option<SharedString>,
    message: SharedString,
    color: ColorName,
}

/// A stack of toasts. Create with `cx.new(|_| ToastStack::new())` and render it
/// inside a full-size root.
pub struct ToastStack {
    toasts: Vec<Toast>,
    next_id: usize,
    duration: Option<Duration>,
}

impl ToastStack {
    pub fn new() -> Self {
        ToastStack {
            toasts: Vec::new(),
            next_id: 0,
            duration: Some(Duration::from_secs(4)),
        }
    }

    /// Set the auto-dismiss delay for subsequently pushed toasts. `None`
    /// keeps toasts until closed. Chainable, so it slots into construction:
    /// `cx.new(|_| ToastStack::new().duration(None))`.
    pub fn duration(mut self, duration: Option<Duration>) -> Self {
        self.set_duration(duration);
        self
    }

    /// [`duration`](ToastStack::duration) for an already-built stack, e.g.
    /// inside `entity.update(cx, ...)` right before a sticky push.
    pub fn set_duration(&mut self, duration: Option<Duration>) {
        self.duration = duration;
    }

    /// Push a plain message toast. Returns its id (pass to [`remove`]).
    ///
    /// [`remove`]: ToastStack::remove
    pub fn push(&mut self, message: impl Into<SharedString>, cx: &mut Context<Self>) -> usize {
        self.push_toast(None, message.into(), ColorName::Blue, cx)
    }

    /// Push a titled, colored toast.
    pub fn push_titled(
        &mut self,
        title: impl Into<SharedString>,
        message: impl Into<SharedString>,
        color: ColorName,
        cx: &mut Context<Self>,
    ) -> usize {
        self.push_toast(Some(title.into()), message.into(), color, cx)
    }

    fn push_toast(
        &mut self,
        title: Option<SharedString>,
        message: SharedString,
        color: ColorName,
        cx: &mut Context<Self>,
    ) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.toasts.push(Toast {
            id,
            title,
            message,
            color,
        });
        cx.notify();

        if let Some(delay) = self.duration {
            // Ids are never reused, so this removes exactly this toast (or
            // nothing, if it was closed by hand first).
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(delay).await;
                this.update(cx, |this, cx| this.remove(id, cx)).ok();
            })
            .detach();
        }

        id
    }

    /// Remove a toast by id.
    pub fn remove(&mut self, id: usize, cx: &mut Context<Self>) {
        self.toasts.retain(|t| t.id != id);
        cx.notify();
    }

    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.toasts.clear();
        cx.notify();
    }

    pub fn len(&self) -> usize {
        self.toasts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.toasts.is_empty()
    }
}

impl Default for ToastStack {
    fn default() -> Self {
        ToastStack::new()
    }
}

impl Render for ToastStack {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut root = div();
        if self.toasts.is_empty() {
            return root;
        }

        let t = theme(cx);
        let surface = t.surface().hsla();
        let border = t.border().hsla();
        let text = t.text().hsla();
        let dimmed = t.dimmed().hsla();
        let radius = t.radius(Size::Md);
        let font_sm = t.font_size(Size::Sm);

        let mut stack = div()
            .absolute()
            .top(px(16.0))
            .right(px(16.0))
            .flex()
            .flex_col()
            .gap(px(10.0));

        for toast in &self.toasts {
            let id = toast.id;
            let accent = t.color(toast.color, t.primary_shade()).hsla();

            let mut content = div().flex().flex_col().gap(px(2.0)).flex_1();
            if let Some(title) = toast.title.clone() {
                content = content.child(
                    div()
                        .font_weight(FontWeight::BOLD)
                        .text_size(px(font_sm))
                        .text_color(text)
                        .child(title),
                );
            }
            content = content.child(
                div()
                    .text_size(px(font_sm))
                    .text_color(dimmed)
                    .child(toast.message.clone()),
            );

            let card = div()
                .flex()
                .items_start()
                .gap(px(12.0))
                .w(px(320.0))
                .p(px(t.spacing(Size::Md)))
                .rounded(px(radius))
                .bg(surface)
                .border_1()
                .border_color(border)
                .shadow_md()
                .child(div().w(px(4.0)).h(px(38.0)).rounded(px(4.0)).bg(accent))
                .child(content)
                .child(
                    div()
                        .id(("guise-toast-close", id))
                        .text_color(dimmed)
                        .hover(move |s| s.text_color(text))
                        .child(SharedString::new_static("\u{00d7}"))
                        .on_click(cx.listener(move |this, _ev, _window, cx| this.remove(id, cx))),
                );

            stack = stack.child(card);
        }

        root = root.child(deferred(stack));
        root
    }
}
