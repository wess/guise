//! `Tabs` — a tab bar with switchable panels (gpui entity).

use gpui::prelude::*;
use gpui::{div, px, transparent_black, App, Context, IntoElement, SharedString, Window};

use super::Content;
use crate::theme::{theme, Size};

struct TabItem {
    label: SharedString,
    content: Content,
}

/// A tabbed view. Create with `cx.new(|cx| Tabs::new(cx).tab("One", |_, _| ...))`.
pub struct Tabs {
    tabs: Vec<TabItem>,
    active: usize,
}

impl Tabs {
    pub fn new(_cx: &mut Context<Self>) -> Self {
        Tabs {
            tabs: Vec::new(),
            active: 0,
        }
    }

    /// Add a tab. `content` is rebuilt each render so it can show live data.
    pub fn tab<E>(
        mut self,
        label: impl Into<SharedString>,
        content: impl Fn(&mut Window, &mut App) -> E + 'static,
    ) -> Self
    where
        E: IntoElement,
    {
        self.tabs.push(TabItem {
            label: label.into(),
            content: Box::new(move |window, cx| content(window, cx).into_any_element()),
        });
        self
    }

    pub fn active(mut self, index: usize) -> Self {
        self.active = index;
        self
    }

    /// The index of the active tab.
    pub fn active_index(&self) -> usize {
        self.active
    }
}

impl Render for Tabs {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        let accent = t.primary().hsla();
        let dimmed = t.dimmed().hsla();
        let text = t.text().hsla();
        let line = t.border().hsla();
        let font = t.font_size(Size::Sm);

        let count = self.tabs.len();
        let active = if count == 0 { 0 } else { self.active.min(count - 1) };

        let mut bar = div().flex().border_b_1().border_color(line);
        for (i, tab) in self.tabs.iter().enumerate() {
            let is_active = i == active;
            bar = bar.child(
                div()
                    .id(("guise-tab", i))
                    .px(px(16.0))
                    .py(px(8.0))
                    .border_b_2()
                    .border_color(if is_active { accent } else { transparent_black() })
                    .text_size(px(font))
                    .text_color(if is_active { accent } else { dimmed })
                    .hover(move |s| s.text_color(text))
                    .child(tab.label.clone())
                    .on_click(cx.listener(move |this, _ev, _window, cx| {
                        this.active = i;
                        cx.notify();
                    })),
            );
        }

        let mut root = div().flex().flex_col().gap(px(12.0)).child(bar);
        if count > 0 {
            let panel = (self.tabs[active].content)(window, cx);
            root = root.child(panel);
        }
        root
    }
}
