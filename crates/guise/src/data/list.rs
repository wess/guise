//! `List` — an ordered or unordered list of text items.

use gpui::prelude::*;
use gpui::{div, px, App, IntoElement, SharedString, Window};

use crate::theme::{theme, Size};

/// A bulleted or numbered list. The Mantine `List`.
#[derive(IntoElement)]
pub struct List {
    items: Vec<SharedString>,
    ordered: bool,
    size: Size,
    spacing: Size,
    /// Custom bullet glyph for unordered lists (defaults to a dot).
    icon: Option<SharedString>,
}

impl List {
    pub fn new() -> Self {
        List {
            items: Vec::new(),
            ordered: false,
            size: Size::Md,
            spacing: Size::Xs,
            icon: None,
        }
    }

    pub fn item(mut self, item: impl Into<SharedString>) -> Self {
        self.items.push(item.into());
        self
    }

    pub fn items<I, S>(mut self, items: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.items.extend(items.into_iter().map(Into::into));
        self
    }

    pub fn ordered(mut self, ordered: bool) -> Self {
        self.ordered = ordered;
        self
    }

    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    pub fn spacing(mut self, spacing: Size) -> Self {
        self.spacing = spacing;
        self
    }

    pub fn icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }
}

impl Default for List {
    fn default() -> Self {
        List::new()
    }
}

impl RenderOnce for List {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let font = t.font_size(self.size);
        let gap = t.spacing(self.spacing);
        let text = t.text().hsla();
        let marker = t.dimmed().hsla();
        let ordered = self.ordered;
        let icon = self.icon.clone();

        let rows = self.items.into_iter().enumerate().map(move |(i, item)| {
            let bullet: SharedString = if ordered {
                SharedString::from(format!("{}.", i + 1))
            } else if let Some(glyph) = icon.clone() {
                glyph
            } else {
                SharedString::new_static("\u{2022}")
            };
            div()
                .flex()
                .items_start()
                .gap(px(8.0))
                .text_size(px(font))
                .child(div().min_w(px(font * 1.2)).text_color(marker).child(bullet))
                .child(div().text_color(text).child(item))
        });

        div().flex().flex_col().gap(px(gap)).children(rows)
    }
}
