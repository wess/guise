//! `Spoiler` — clips tall content to a max height behind a "Show more" toggle.
//!
//! Controlled: the parent owns `expanded` and flips it in `on_toggle`, exactly
//! like `Modal`'s `opened`/`on_close` pair.
//!
//! ```ignore
//! Spoiler::new("bio-spoiler")
//!     .max_height(60.0)
//!     .expanded(self.bio_open)
//!     .on_toggle(cx.listener(|this, _, _, cx| {
//!         this.bio_open = !this.bio_open;
//!         cx.notify();
//!     }))
//!     .child(Text::new(LONG_BIO).size(Size::Sm))
//! ```

use gpui::prelude::*;
use gpui::{div, px, AnyElement, App, ClickEvent, ElementId, IntoElement, SharedString, Window};

use crate::input::ClickHandler;
use crate::theme::{theme, ColorName, Size};

/// A collapsible content clip. The Mantine `Spoiler`.
///
/// While collapsed the children render inside an `overflow-hidden` box capped
/// at `max_height`; the toggle below is styled like an `Anchor` link.
#[derive(IntoElement)]
pub struct Spoiler {
    id: ElementId,
    children: Vec<AnyElement>,
    max_height: f32,
    expanded: bool,
    show_label: SharedString,
    hide_label: SharedString,
    color: ColorName,
    size: Size,
    on_toggle: Option<ClickHandler>,
}

impl Spoiler {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Spoiler {
            id: id.into(),
            children: Vec::new(),
            max_height: 100.0,
            expanded: false,
            show_label: SharedString::new_static("Show more"),
            hide_label: SharedString::new_static("Hide"),
            color: ColorName::Blue,
            size: Size::Sm,
            on_toggle: None,
        }
    }

    /// Visible height in px while collapsed (default 100).
    pub fn max_height(mut self, max_height: f32) -> Self {
        self.max_height = max_height;
        self
    }

    /// Whether the full content is shown. The parent owns this flag.
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }

    /// Toggle label while collapsed (default "Show more").
    pub fn show_label(mut self, label: impl Into<SharedString>) -> Self {
        self.show_label = label.into();
        self
    }

    /// Toggle label while expanded (default "Hide").
    pub fn hide_label(mut self, label: impl Into<SharedString>) -> Self {
        self.hide_label = label.into();
        self
    }

    /// The toggle link color (default `Blue`).
    pub fn color(mut self, color: ColorName) -> Self {
        self.color = color;
        self
    }

    /// The toggle label font size (default `Sm`).
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Called when the toggle is clicked. Wire with `cx.listener(...)` to
    /// flip the parent's `expanded` flag.
    pub fn on_toggle(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_toggle = Some(Box::new(handler));
        self
    }
}

impl ParentElement for Spoiler {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Spoiler {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let dark = t.scheme.is_dark();
        let link = t.color(self.color, if dark { 4 } else { 6 }).hsla();
        let link_hover = t.color(self.color, if dark { 3 } else { 7 }).hsla();
        let font = t.font_size(self.size);
        let gap = t.spacing(Size::Xs);

        let mut content = div().w_full().children(self.children);
        if !self.expanded {
            content = content.max_h(px(self.max_height)).overflow_hidden();
        }

        let label = if self.expanded {
            self.hide_label
        } else {
            self.show_label
        };
        let mut toggle = div()
            .id(self.id)
            .cursor_pointer()
            .text_size(px(font))
            .text_color(link)
            .hover(move |s| s.text_color(link_hover))
            .child(label);
        if let Some(handler) = self.on_toggle {
            toggle = toggle.on_click(handler);
        }

        div()
            .flex()
            .flex_col()
            .items_start()
            .gap(px(gap))
            .child(content)
            .child(toggle)
    }
}
