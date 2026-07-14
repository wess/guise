//! `OverlayHost` — window-level overlay services (gpui entity).
//!
//! One host per window owns the modal stack and the toast queue, so opening
//! a dialog is a call, not render-flag plumbing: any handler with the host's
//! entity can `open_modal`/`toast` from anywhere. The host also restores
//! focus to whatever was focused before a modal opened, and closes the top
//! modal on Escape.
//!
//! ```ignore
//! // At the root view: create once, render last (so overlays paint above).
//! let overlays = cx.new(OverlayHost::new);
//! div().child(page_content).child(overlays.clone())
//!
//! // From any handler:
//! overlays.update(cx, |host, cx| {
//!     host.toast("Saved", cx);
//!     host.open_modal(window, cx, |close, _window, _cx| {
//!         Modal::new()
//!             .title("Settings")
//!             .on_close(move |_ev, window, cx| close(window, cx))
//!             .child(Text::new("..."))
//!             .into_any_element()
//!     });
//! });
//! ```

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    div, AnyElement, App, Context, Entity, FocusHandle, IntoElement, KeyDownEvent, SharedString,
    Window,
};

use crate::feedback::ToastStack;
use crate::theme::ColorName;

/// Closes the modal it was handed to. Wire it to your modal's close
/// button/backdrop (`Modal::on_close`).
pub type ModalCloser = Rc<dyn Fn(&mut Window, &mut App) + 'static>;

type ModalBuilder = Rc<dyn Fn(ModalCloser, &mut Window, &mut App) -> AnyElement + 'static>;

struct ModalEntry {
    id: usize,
    builder: ModalBuilder,
    previous_focus: Option<FocusHandle>,
}

/// Window-level modal stack + toast queue. Create with
/// `cx.new(OverlayHost::new)` and render it as the last child of the root.
pub struct OverlayHost {
    modals: Vec<ModalEntry>,
    toasts: Entity<ToastStack>,
    next_id: usize,
}

impl OverlayHost {
    pub fn new(cx: &mut Context<Self>) -> Self {
        OverlayHost {
            modals: Vec::new(),
            toasts: cx.new(|_| ToastStack::new()),
            next_id: 0,
        }
    }

    /// The inner [`ToastStack`], for `duration`/`clear`/`remove` control.
    pub fn toast_stack(&self) -> Entity<ToastStack> {
        self.toasts.clone()
    }

    /// Push a plain toast.
    pub fn toast(&mut self, message: impl Into<SharedString>, cx: &mut Context<Self>) {
        let message = message.into();
        self.toasts.update(cx, |toasts, cx| {
            toasts.push(message, cx);
        });
    }

    /// Push a titled, colored toast.
    pub fn toast_titled(
        &mut self,
        title: impl Into<SharedString>,
        message: impl Into<SharedString>,
        color: ColorName,
        cx: &mut Context<Self>,
    ) {
        let (title, message) = (title.into(), message.into());
        self.toasts.update(cx, |toasts, cx| {
            toasts.push_titled(title, message, color, cx);
        });
    }

    /// Open a modal above everything (stacked above any already open). The
    /// builder is re-invoked every frame (live content) and receives a
    /// [`ModalCloser`] to wire to its close affordances. Returns the modal's
    /// id for [`close_modal`](Self::close_modal).
    ///
    /// Whatever was focused when the modal opened is refocused when it
    /// closes; Escape (with focus anywhere inside the modal) closes it.
    pub fn open_modal<E>(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        builder: impl Fn(ModalCloser, &mut Window, &mut App) -> E + 'static,
    ) -> usize
    where
        E: IntoElement,
    {
        let id = self.next_id;
        self.next_id += 1;
        self.modals.push(ModalEntry {
            id,
            builder: Rc::new(move |close, window, cx| {
                builder(close, window, cx).into_any_element()
            }),
            previous_focus: window.focused(cx),
        });
        cx.notify();
        id
    }

    /// Close a modal by id, restoring the focus it captured on open.
    pub fn close_modal(&mut self, id: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.modals.iter().position(|m| m.id == id) else {
            return;
        };
        let entry = self.modals.remove(index);
        if let Some(focus) = entry.previous_focus {
            window.focus(&focus, cx);
        }
        cx.notify();
    }

    /// Close the top-most modal, if any.
    pub fn close_top(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(id) = self.modals.last().map(|m| m.id) {
            self.close_modal(id, window, cx);
        }
    }

    pub fn modal_count(&self) -> usize {
        self.modals.len()
    }

    fn closer(&self, id: usize, cx: &mut Context<Self>) -> ModalCloser {
        let host = cx.entity().downgrade();
        Rc::new(move |window, cx| {
            host.update(cx, |host, cx| host.close_modal(id, window, cx))
                .ok();
        })
    }
}

impl Render for OverlayHost {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut root = div().child(self.toasts.clone());

        let top_id = self.modals.last().map(|m| m.id);
        let entries: Vec<(usize, ModalBuilder)> = self
            .modals
            .iter()
            .map(|m| (m.id, m.builder.clone()))
            .collect();

        for (id, builder) in entries {
            let close = self.closer(id, cx);
            let content = builder(close, window, cx);
            let is_top = top_id == Some(id);
            root = root.child(
                div()
                    .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                        if is_top && event.keystroke.key == "escape" {
                            this.close_modal(id, window, cx);
                            cx.stop_propagation();
                        }
                    }))
                    .child(content),
            );
        }
        root
    }
}
