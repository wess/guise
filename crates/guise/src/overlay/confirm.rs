//! `ConfirmModal` — a yes/no dialog built on [`Modal`](super::Modal).
//!
//! Controlled exactly like `Modal`: the parent owns an `opened` flag, renders
//! the `ConfirmModal` only while it is true, and flips the flag from
//! `on_confirm` / `on_cancel`. The backdrop and the header `×` also run
//! `on_cancel`. A message string covers the common case; extra body content
//! can be added as children (`ParentElement`).
//!
//! ```ignore
//! if self.confirm_open {
//!     root = root.child(
//!         ConfirmModal::new()
//!             .title("Delete file?")
//!             .message("del.rs will be moved to the Trash.")
//!             .confirm_label("Delete")
//!             .danger()
//!             .on_confirm(cx.listener(|this, _ev, _w, cx| { this.confirm_open = false; cx.notify(); }))
//!             .on_cancel(cx.listener(|this, _ev, _w, cx| { this.confirm_open = false; cx.notify(); })),
//!     );
//! }
//! ```

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{div, px, AnyElement, App, ClickEvent, IntoElement, SharedString, Window};

use super::Modal;
use crate::button::Button;
use crate::style::Variant;
use crate::text::Text;
use crate::theme::{theme, ColorName, Size};

type Handler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// A confirm/cancel dialog. The Mantine `modals.openConfirmModal`, as a
/// controlled component.
#[derive(IntoElement)]
pub struct ConfirmModal {
    title: Option<SharedString>,
    message: Option<SharedString>,
    children: Vec<AnyElement>,
    confirm_label: SharedString,
    cancel_label: SharedString,
    danger: bool,
    width: Option<f32>,
    on_confirm: Option<Handler>,
    on_cancel: Option<Handler>,
}

impl ConfirmModal {
    pub fn new() -> Self {
        ConfirmModal {
            title: None,
            message: None,
            children: Vec::new(),
            confirm_label: SharedString::new_static("Confirm"),
            cancel_label: SharedString::new_static("Cancel"),
            danger: false,
            width: None,
            on_confirm: None,
            on_cancel: None,
        }
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// The dimmed body text. For richer content add children instead (or too).
    pub fn message(mut self, message: impl Into<SharedString>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Label of the confirming button (default `"Confirm"`).
    pub fn confirm_label(mut self, label: impl Into<SharedString>) -> Self {
        self.confirm_label = label.into();
        self
    }

    /// Label of the cancelling button (default `"Cancel"`).
    pub fn cancel_label(mut self, label: impl Into<SharedString>) -> Self {
        self.cancel_label = label.into();
        self
    }

    /// Render the confirm button in red for destructive actions.
    pub fn danger(mut self) -> Self {
        self.danger = true;
        self
    }

    /// Dialog width in pixels (defaults to `Modal`'s 440).
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Called when the confirm button is clicked. Close the dialog here.
    pub fn on_confirm(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_confirm = Some(Rc::new(handler));
        self
    }

    /// Called on cancel — the cancel button, the backdrop, and the header `×`.
    pub fn on_cancel(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_cancel = Some(Rc::new(handler));
        self
    }
}

impl Default for ConfirmModal {
    fn default() -> Self {
        ConfirmModal::new()
    }
}

impl ParentElement for ConfirmModal {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for ConfirmModal {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let gap = t.spacing(Size::Sm);

        let mut modal = Modal::new();
        if let Some(width) = self.width {
            modal = modal.width(width);
        }
        if let Some(title) = self.title {
            modal = modal.title(title);
        }
        if let Some(cancel) = self.on_cancel.clone() {
            modal = modal.on_close(move |ev, window, cx| cancel(ev, window, cx));
        }
        if let Some(message) = self.message {
            modal = modal.child(Text::new(message).dimmed().size(Size::Sm));
        }
        modal = modal.children(self.children);

        let mut cancel_button =
            Button::new("guise-confirm-cancel", self.cancel_label).variant(Variant::Default);
        if let Some(handler) = self.on_cancel {
            cancel_button = cancel_button.on_click(move |ev, window, cx| handler(ev, window, cx));
        }

        let mut confirm_button = Button::new("guise-confirm-accept", self.confirm_label);
        if self.danger {
            confirm_button = confirm_button.color(ColorName::Red);
        }
        if let Some(handler) = self.on_confirm {
            confirm_button = confirm_button.on_click(move |ev, window, cx| handler(ev, window, cx));
        }

        modal.child(
            div()
                .flex()
                .justify_end()
                .gap(px(gap))
                .child(cancel_button)
                .child(confirm_button),
        )
    }
}
