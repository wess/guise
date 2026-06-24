//! `Breadcrumbs` — a trail of locations separated by a glyph.

use gpui::prelude::*;
use gpui::{div, px, App, FontWeight, IntoElement, SharedString, Window};

use crate::theme::{theme, Size};

/// A breadcrumb trail. The Mantine `Breadcrumbs`. The last item is rendered as
/// the current location.
#[derive(IntoElement)]
pub struct Breadcrumbs {
    items: Vec<SharedString>,
    separator: SharedString,
}

impl Breadcrumbs {
    pub fn new() -> Self {
        Breadcrumbs {
            items: Vec::new(),
            separator: SharedString::new_static("/"),
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

    pub fn separator(mut self, separator: impl Into<SharedString>) -> Self {
        self.separator = separator.into();
        self
    }
}

impl Default for Breadcrumbs {
    fn default() -> Self {
        Breadcrumbs::new()
    }
}

impl RenderOnce for Breadcrumbs {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let font = t.font_size(Size::Sm);
        let current = t.text().hsla();
        let muted = t.dimmed().hsla();
        let separator = self.separator;
        let last = self.items.len().saturating_sub(1);

        let mut row = div().flex().items_center().gap(px(8.0)).text_size(px(font));
        for (i, item) in self.items.into_iter().enumerate() {
            if i > 0 {
                row = row.child(div().text_color(muted).child(separator.clone()));
            }
            let is_last = i == last;
            row = row.child(
                div()
                    .text_color(if is_last { current } else { muted })
                    .font_weight(if is_last {
                        FontWeight::SEMIBOLD
                    } else {
                        FontWeight::NORMAL
                    })
                    .child(item),
            );
        }
        row
    }
}
