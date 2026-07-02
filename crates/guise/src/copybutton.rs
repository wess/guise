//! `CopyButton` — copies text to the clipboard, with transient feedback.

use std::time::Duration;

use gpui::prelude::*;
use gpui::{div, px, ClickEvent, ClipboardItem, Context, IntoElement, SharedString, Window};

use crate::theme::{theme, ColorName, Size};

/// A small button that writes `text` to the system clipboard when clicked, then
/// shows a "Copied" state for a moment. A gpui entity — create with
/// `cx.new(|_| CopyButton::new("…"))`.
pub struct CopyButton {
    text: SharedString,
    label: SharedString,
    copied_label: SharedString,
    copied: bool,
}

impl CopyButton {
    pub fn new(text: impl Into<SharedString>) -> Self {
        CopyButton {
            text: text.into(),
            label: SharedString::new_static("Copy"),
            copied_label: SharedString::new_static("\u{2713} Copied"),
            copied: false,
        }
    }

    /// Override the idle label (default "Copy").
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }

    /// The text this button copies.
    pub fn text(&self) -> SharedString {
        self.text.clone()
    }

    /// Replace the text to copy.
    pub fn set_text(&mut self, text: impl Into<SharedString>) {
        self.text = text.into();
    }

    fn copy(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(self.text.to_string()));
        self.copied = true;
        cx.notify();

        // Revert the "Copied" state after a beat.
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(1200))
                .await;
            this.update(cx, |this, cx| {
                this.copied = false;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}

impl Render for CopyButton {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        let dark = t.scheme.is_dark();
        let copied = self.copied;

        let (bg, fg) = if copied {
            (
                t.color(ColorName::Teal, if dark { 8 } else { 0 }).hsla(),
                t.color(ColorName::Teal, if dark { 3 } else { 7 }).hsla(),
            )
        } else {
            (t.surface_hover().hsla(), t.dimmed().hsla())
        };
        let hover_bg = t.color(ColorName::Gray, if dark { 6 } else { 2 }).hsla();
        let label = if copied {
            self.copied_label.clone()
        } else {
            self.label.clone()
        };

        let mut el = div()
            .id("guise-copy-button")
            .flex()
            .items_center()
            .h(px(24.0))
            .px(px(8.0))
            .rounded(px(t.radius(Size::Sm)))
            .bg(bg)
            .text_color(fg)
            .text_size(px(12.0))
            .child(label)
            .on_click(cx.listener(Self::copy));
        if !copied {
            el = el.hover(move |s| s.bg(hover_bg));
        }
        el
    }
}
