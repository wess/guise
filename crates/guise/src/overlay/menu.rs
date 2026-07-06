//! `Menu` — a stateful dropdown of actions (gpui entity).
//!
//! A trigger button toggles a deferred list of items, section labels, and
//! dividers. Each item carries its own handler, run on click.

use gpui::prelude::*;
use gpui::{
    deferred, div, px, App, Context, FocusHandle, IntoElement, KeyDownEvent, SharedString, Window,
};

use crate::input::control_metrics;
use crate::style::{surface, Variant};
use crate::theme::{theme, ColorName, Size};

type ItemHandler = Box<dyn Fn(&mut Window, &mut App) + 'static>;

enum Entry {
    Item {
        label: SharedString,
        danger: bool,
        handler: Option<ItemHandler>,
    },
    Section(SharedString),
    Divider,
}

/// A dropdown action menu. Create with `cx.new(|cx| Menu::new(cx, "Actions"))`.
pub struct Menu {
    trigger: SharedString,
    entries: Vec<Entry>,
    open: bool,
    focus: FocusHandle,
    size: Size,
    /// Entry index of the keyboard-highlighted item.
    highlight: usize,
}

impl Menu {
    pub fn new(cx: &mut Context<Self>, trigger: impl Into<SharedString>) -> Self {
        Menu {
            trigger: trigger.into(),
            entries: Vec::new(),
            open: false,
            focus: cx.focus_handle(),
            size: Size::Sm,
            highlight: 0,
        }
    }

    /// Entry indices that are actionable items (skipping sections/dividers).
    fn item_indices(&self) -> Vec<usize> {
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, e)| matches!(e, Entry::Item { .. }))
            .map(|(i, _)| i)
            .collect()
    }

    fn move_highlight(&mut self, delta: isize) {
        let items = self.item_indices();
        if items.is_empty() {
            return;
        }
        let pos = items.iter().position(|&i| i == self.highlight).unwrap_or(0);
        let len = items.len() as isize;
        let next = (((pos as isize + delta) % len) + len) % len;
        self.highlight = items[next as usize];
    }

    fn on_key(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            return;
        }
        match event.keystroke.key.as_str() {
            "escape" => self.open = false,
            "down" => self.move_highlight(1),
            "up" => self.move_highlight(-1),
            "enter" => {
                self.open = false;
                if let Some(Entry::Item {
                    handler: Some(handler),
                    ..
                }) = self.entries.get(self.highlight)
                {
                    handler(_window, cx);
                }
            }
            _ => return,
        }
        cx.notify();
        cx.stop_propagation();
    }

    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Add an action item.
    pub fn item(
        mut self,
        label: impl Into<SharedString>,
        handler: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.entries.push(Entry::Item {
            label: label.into(),
            danger: false,
            handler: Some(Box::new(handler)),
        });
        self
    }

    /// Add a destructive action item (rendered in red).
    pub fn danger_item(
        mut self,
        label: impl Into<SharedString>,
        handler: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.entries.push(Entry::Item {
            label: label.into(),
            danger: true,
            handler: Some(Box::new(handler)),
        });
        self
    }

    /// Add a non-interactive section label.
    pub fn section(mut self, label: impl Into<SharedString>) -> Self {
        self.entries.push(Entry::Section(label.into()));
        self
    }

    /// Add a separating divider.
    pub fn divider(mut self) -> Self {
        self.entries.push(Entry::Divider);
        self
    }
}

impl Render for Menu {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        let (height, pad_x, font) = control_metrics(self.size);
        let radius = t.radius(t.default_radius);
        let s = surface(t, ColorName::Gray, Variant::Default);
        let surface_color = t.surface().hsla();
        let surface_hover = t.surface_hover().hsla();
        let border = t.border().hsla();
        let text = t.text().hsla();
        let dimmed = t.dimmed().hsla();
        let danger = t
            .color(ColorName::Red, if t.scheme.is_dark() { 5 } else { 6 })
            .hsla();
        let font_xs = t.font_size(Size::Xs);
        let trigger_hover = s.bg_hover;

        let mut trigger = div()
            .id("guise-menu-trigger")
            .track_focus(&self.focus)
            .flex()
            .items_center()
            .gap(px(6.0))
            .h(px(height))
            .px(px(pad_x))
            .rounded(px(radius))
            .bg(s.bg)
            .text_color(s.fg)
            .text_size(px(font))
            .hover(move |st| st.bg(trigger_hover))
            .child(self.trigger.clone())
            .child(
                div()
                    .text_color(dimmed)
                    .child(SharedString::new_static("\u{25be}")),
            )
            .on_key_down(cx.listener(Self::on_key))
            .on_click(cx.listener(|this, _ev, window, cx| {
                this.open = !this.open;
                if this.open {
                    this.highlight = this.item_indices().first().copied().unwrap_or(0);
                    window.focus(&this.focus, cx);
                }
                cx.notify();
            }));
        if let Some(b) = s.border {
            trigger = trigger.border_1().border_color(b);
        }

        let mut wrap = div().relative().child(trigger);

        if self.open {
            let mut menu = div()
                .absolute()
                .top(px(height + 6.0))
                .left(px(0.0))
                .min_w(px(180.0))
                .flex()
                .flex_col()
                .gap(px(2.0))
                .p(px(4.0))
                .rounded(px(radius))
                .border_1()
                .border_color(border)
                .bg(surface_color)
                .shadow_md();

            for (i, entry) in self.entries.iter().enumerate() {
                match entry {
                    Entry::Item {
                        label,
                        danger: is_danger,
                        ..
                    } => {
                        let mut item = div()
                            .id(("guise-menu-item", i))
                            .px(px(10.0))
                            .py(px(6.0))
                            .rounded(px(4.0))
                            .text_size(px(font))
                            .text_color(if *is_danger { danger } else { text })
                            .hover(move |s| s.bg(surface_hover))
                            .child(label.clone());
                        if i == self.highlight {
                            item = item.bg(surface_hover);
                        }
                        menu =
                            menu.child(item.on_click(cx.listener(move |this, _ev, window, cx| {
                                this.open = false;
                                if let Entry::Item {
                                    handler: Some(handler),
                                    ..
                                } = &this.entries[i]
                                {
                                    handler(window, cx);
                                }
                                cx.notify();
                            })));
                    }
                    Entry::Section(label) => {
                        menu = menu.child(
                            div()
                                .px(px(10.0))
                                .pt(px(6.0))
                                .pb(px(2.0))
                                .text_size(px(font_xs))
                                .text_color(dimmed)
                                .child(label.clone()),
                        );
                    }
                    Entry::Divider => {
                        menu = menu.child(div().my(px(4.0)).h(px(1.0)).bg(border));
                    }
                }
            }

            wrap = wrap.child(deferred(menu));
        }

        wrap
    }
}
