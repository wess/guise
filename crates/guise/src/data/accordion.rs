//! `Accordion` — collapsible sections (gpui entity).

use gpui::prelude::*;
use gpui::{div, px, App, Context, IntoElement, SharedString, Window};

use super::Content;
use crate::theme::{theme, Size};

struct AccItem {
    label: SharedString,
    content: Content,
}

/// A set of collapsible panels. Create with
/// `cx.new(|cx| Accordion::new(cx).item("Title", |_, _| ...))`.
pub struct Accordion {
    items: Vec<AccItem>,
    open: Vec<bool>,
    multiple: bool,
}

impl Accordion {
    pub fn new(_cx: &mut Context<Self>) -> Self {
        Accordion {
            items: Vec::new(),
            open: Vec::new(),
            multiple: false,
        }
    }

    /// Add a panel. `content` is rebuilt each render.
    pub fn item<E>(
        mut self,
        label: impl Into<SharedString>,
        content: impl Fn(&mut Window, &mut App) -> E + 'static,
    ) -> Self
    where
        E: IntoElement,
    {
        self.items.push(AccItem {
            label: label.into(),
            content: Box::new(move |window, cx| content(window, cx).into_any_element()),
        });
        self.open.push(false);
        self
    }

    /// Allow multiple panels to be open at once (default: only one).
    pub fn multiple(mut self, multiple: bool) -> Self {
        self.multiple = multiple;
        self
    }

    /// Open the panel at `index` initially.
    pub fn default_open(mut self, index: usize) -> Self {
        if let Some(slot) = self.open.get_mut(index) {
            *slot = true;
        }
        self
    }
}

impl Render for Accordion {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        let text = t.text().hsla();
        let dimmed = t.dimmed().hsla();
        let line = t.border().hsla();
        let surface_hover = t.surface_hover().hsla();
        let radius = t.radius(Size::Md);
        let font = t.font_size(Size::Md);
        let font_sm = t.font_size(Size::Sm);

        let mut root = div().flex().flex_col().gap(px(8.0));

        for (i, item) in self.items.iter().enumerate() {
            let is_open = self.open.get(i).copied().unwrap_or(false);

            let header = div()
                .id(("guise-accordion-header", i))
                .flex()
                .items_center()
                .justify_between()
                .px(px(14.0))
                .py(px(12.0))
                .text_size(px(font))
                .text_color(text)
                .hover(move |s| s.bg(surface_hover))
                .child(item.label.clone())
                .child(
                    div()
                        .text_color(dimmed)
                        .child(SharedString::new_static(if is_open {
                            "\u{25be}"
                        } else {
                            "\u{25b8}"
                        })),
                )
                .on_click(cx.listener(move |this, _ev, _window, cx| {
                    let was = this.open.get(i).copied().unwrap_or(false);
                    if !this.multiple {
                        this.open.iter_mut().for_each(|o| *o = false);
                    }
                    if let Some(slot) = this.open.get_mut(i) {
                        *slot = !was;
                    }
                    cx.notify();
                }));

            let mut panel = div()
                .flex()
                .flex_col()
                .rounded(px(radius))
                .border_1()
                .border_color(line)
                .child(header);

            if is_open {
                let body = (item.content)(window, cx);
                panel = panel.child(
                    div()
                        .px(px(14.0))
                        .pb(px(12.0))
                        .text_size(px(font_sm))
                        .text_color(dimmed)
                        .child(body),
                );
            }

            root = root.child(panel);
        }

        root
    }
}
