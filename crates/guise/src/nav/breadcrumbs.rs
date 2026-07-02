//! `Breadcrumbs` — a trail of locations separated by a glyph.

use gpui::prelude::*;
use gpui::{div, px, App, ClickEvent, FontWeight, IntoElement, SharedString, Window};

use crate::input::ClickHandler;
use crate::theme::{theme, Size};

struct Crumb {
    label: SharedString,
    on_click: Option<ClickHandler>,
}

/// A breadcrumb trail. The Mantine `Breadcrumbs`. The last item is rendered as
/// the current location and is never clickable, even when built with [`link`].
///
/// [`link`]: Breadcrumbs::link
#[derive(IntoElement)]
pub struct Breadcrumbs {
    items: Vec<Crumb>,
    separator: SharedString,
}

impl Breadcrumbs {
    pub fn new() -> Self {
        Breadcrumbs {
            items: Vec::new(),
            separator: SharedString::new_static("/"),
        }
    }

    /// Add a plain, non-clickable item.
    pub fn item(mut self, item: impl Into<SharedString>) -> Self {
        self.items.push(Crumb {
            label: item.into(),
            on_click: None,
        });
        self
    }

    /// Add a clickable item that navigates via `handler`. Handlers on the
    /// last item are ignored — the trail's tail is the current page.
    pub fn link(
        mut self,
        item: impl Into<SharedString>,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.items.push(Crumb {
            label: item.into(),
            on_click: Some(Box::new(handler)),
        });
        self
    }

    pub fn items<I, S>(mut self, items: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.items.extend(items.into_iter().map(|item| Crumb {
            label: item.into(),
            on_click: None,
        }));
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
        for (i, crumb) in self.items.into_iter().enumerate() {
            if i > 0 {
                row = row.child(div().text_color(muted).child(separator.clone()));
            }
            let is_last = i == last;
            let item = div()
                .text_color(if is_last { current } else { muted })
                .font_weight(if is_last {
                    FontWeight::SEMIBOLD
                } else {
                    FontWeight::NORMAL
                })
                .child(crumb.label);
            match crumb.on_click.filter(|_| !is_last) {
                Some(handler) => {
                    row = row.child(
                        item.id(("guise-breadcrumb", i))
                            .hover(move |s| s.text_color(current))
                            .on_click(handler),
                    );
                }
                None => row = row.child(item),
            }
        }
        row
    }
}
