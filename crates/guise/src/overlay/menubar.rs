//! `MenuBar` — a horizontal application menu (File / Edit / View / …).
//!
//! Each top-level label opens a dropdown of items. Once any menu is open,
//! moving the pointer onto a sibling label switches to it — the classic
//! desktop menu-bar feel. Keyboard: left/right switch menus, up/down move the
//! highlight within the open menu, enter activates, escape closes.
//!
//! Built as a gpui entity, like [`Menu`](super::Menu); drop it into a titlebar
//! strip or a [`StatusBar`](crate::nav::StatusBar) slot:
//!
//! ```ignore
//! cx.new(|cx| {
//!     MenuBar::new(cx)
//!         .menu("File", |m| {
//!             m.item_shortcut("New Tab", "⌘T", |_, cx| { /* … */ })
//!                 .item("New Window", |_, cx| { /* … */ })
//!                 .divider()
//!                 .danger_item("Quit", |_, cx| { /* … */ })
//!         })
//!         .menu("Edit", |m| {
//!             m.item_shortcut("Copy", "⌘C", |_, cx| {})
//!                 .item_shortcut("Paste", "⌘V", |_, cx| {})
//!                 .disabled_item("Redo")
//!         })
//! })
//! ```

use gpui::prelude::*;
use gpui::{
    deferred, div, px, App, Context, FocusHandle, IntoElement, KeyDownEvent, SharedString, Window,
};

use crate::input::control_metrics;
use crate::theme::{theme, ColorName, Size};

type ItemHandler = Box<dyn Fn(&mut Window, &mut App) + 'static>;

enum Entry {
    Item {
        label: SharedString,
        shortcut: Option<SharedString>,
        danger: bool,
        disabled: bool,
        handler: Option<ItemHandler>,
    },
    Section(SharedString),
    Divider,
}

/// One top-level menu in a [`MenuBar`]: a label plus its dropdown entries.
///
/// You rarely name this type directly — [`MenuBar::menu`] hands you one to
/// build inside a closure. It is exported so menus can also be assembled
/// programmatically and pushed with [`MenuBar::push`].
pub struct MenuColumn {
    label: SharedString,
    entries: Vec<Entry>,
}

impl MenuColumn {
    /// Start an empty menu with the given top-level label.
    pub fn new(label: impl Into<SharedString>) -> Self {
        MenuColumn {
            label: label.into(),
            entries: Vec::new(),
        }
    }

    /// Add an action item.
    pub fn item(
        self,
        label: impl Into<SharedString>,
        handler: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.entry(label, None, false, false, Some(Box::new(handler)))
    }

    /// Add an action item with a right-aligned shortcut hint (e.g. `"⌘T"`).
    pub fn item_shortcut(
        self,
        label: impl Into<SharedString>,
        shortcut: impl Into<SharedString>,
        handler: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.entry(label, Some(shortcut.into()), false, false, Some(Box::new(handler)))
    }

    /// Add a destructive action item, rendered in red.
    pub fn danger_item(
        self,
        label: impl Into<SharedString>,
        handler: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.entry(label, None, true, false, Some(Box::new(handler)))
    }

    /// Add a disabled item: greyed out, no shortcut, not clickable or
    /// keyboard-selectable.
    pub fn disabled_item(self, label: impl Into<SharedString>) -> Self {
        self.entry(label, None, false, true, None)
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

    fn entry(
        mut self,
        label: impl Into<SharedString>,
        shortcut: Option<SharedString>,
        danger: bool,
        disabled: bool,
        handler: Option<ItemHandler>,
    ) -> Self {
        self.entries.push(Entry::Item {
            label: label.into(),
            shortcut,
            danger,
            disabled,
            handler,
        });
        self
    }

    /// Entry indices that are actionable (enabled items with a handler).
    fn actionable(&self) -> Vec<usize> {
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                matches!(
                    e,
                    Entry::Item {
                        disabled: false,
                        handler: Some(_),
                        ..
                    }
                )
            })
            .map(|(i, _)| i)
            .collect()
    }
}

/// A horizontal strip of dropdown menus — an application menu bar.
///
/// Create with `cx.new(|cx| MenuBar::new(cx))`, then add menus with
/// [`menu`](Self::menu).
pub struct MenuBar {
    menus: Vec<MenuColumn>,
    /// Index of the open top-level menu, if any.
    open: Option<usize>,
    focus: FocusHandle,
    size: Size,
    /// Entry index of the keyboard-highlighted item within the open menu.
    highlight: usize,
}

impl MenuBar {
    pub fn new(cx: &mut Context<Self>) -> Self {
        MenuBar {
            menus: Vec::new(),
            open: None,
            focus: cx.focus_handle(),
            size: Size::Sm,
            highlight: 0,
        }
    }

    /// Sizing token for the top-level labels.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Add a top-level menu, building its entries in the closure.
    pub fn menu(
        mut self,
        label: impl Into<SharedString>,
        build: impl FnOnce(MenuColumn) -> MenuColumn,
    ) -> Self {
        self.menus.push(build(MenuColumn::new(label)));
        self
    }

    /// Add a pre-built [`MenuColumn`] (for menus assembled programmatically).
    pub fn push(mut self, menu: MenuColumn) -> Self {
        self.menus.push(menu);
        self
    }

    /// Open a menu and highlight its first actionable item.
    fn open_menu(&mut self, idx: usize) {
        self.open = Some(idx);
        self.highlight = self
            .menus
            .get(idx)
            .and_then(|m| m.actionable().first().copied())
            .unwrap_or(0);
    }

    fn move_menu(&mut self, delta: isize) {
        if self.menus.is_empty() {
            return;
        }
        let cur = self.open.unwrap_or(0) as isize;
        let len = self.menus.len() as isize;
        let next = (((cur + delta) % len) + len) % len;
        self.open_menu(next as usize);
    }

    fn move_highlight(&mut self, delta: isize) {
        let Some(open) = self.open else { return };
        let Some(menu) = self.menus.get(open) else { return };
        let items = menu.actionable();
        if items.is_empty() {
            return;
        }
        let pos = items.iter().position(|&i| i == self.highlight).unwrap_or(0);
        let len = items.len() as isize;
        let next = (((pos as isize + delta) % len) + len) % len;
        self.highlight = items[next as usize];
    }

    fn activate(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(open) = self.open else { return };
        self.open = None;
        if let Some(Entry::Item {
            handler: Some(handler),
            ..
        }) = self.menus.get(open).and_then(|m| m.entries.get(self.highlight))
        {
            handler(window, cx);
        }
    }

    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.open.is_none() {
            return;
        }
        match event.keystroke.key.as_str() {
            "escape" => self.open = None,
            "left" => self.move_menu(-1),
            "right" => self.move_menu(1),
            "down" => self.move_highlight(1),
            "up" => self.move_highlight(-1),
            "enter" => self.activate(window, cx),
            _ => return,
        }
        cx.notify();
        cx.stop_propagation();
    }
}

impl Render for MenuBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        let (height, pad_x, font) = control_metrics(self.size);
        let radius = t.radius(t.default_radius);
        let surface_color = t.surface().hsla();
        let surface_hover = t.surface_hover().hsla();
        let border = t.border().hsla();
        let text = t.text().hsla();
        let dimmed = t.dimmed().hsla();
        let danger = t
            .color(ColorName::Red, if t.scheme.is_dark() { 5 } else { 6 })
            .hsla();
        let font_xs = t.font_size(Size::Xs);

        let mut bar = div()
            .id("guise-menubar")
            .track_focus(&self.focus)
            .flex()
            .flex_row()
            .items_center()
            .gap(px(2.0))
            .text_size(px(font))
            .on_key_down(cx.listener(Self::on_key));

        for (mi, menu) in self.menus.iter().enumerate() {
            let is_open = self.open == Some(mi);

            let mut label = div()
                .id(("guise-menubar-label", mi))
                .flex()
                .items_center()
                .h(px(height))
                .px(px(pad_x))
                .rounded(px(radius))
                .text_color(text)
                .hover(move |s| s.bg(surface_hover))
                .child(menu.label.clone())
                .on_click(cx.listener(move |this, _ev, window, cx| {
                    if this.open == Some(mi) {
                        this.open = None;
                    } else {
                        this.open_menu(mi);
                        window.focus(&this.focus, cx);
                    }
                    cx.notify();
                }))
                // Once a menu is open, hovering a sibling label switches to it.
                .on_hover(cx.listener(move |this, hovered: &bool, _window, cx| {
                    if *hovered && this.open.is_some() && this.open != Some(mi) {
                        this.open_menu(mi);
                        cx.notify();
                    }
                }));
            if is_open {
                label = label.bg(surface_hover);
            }

            let mut wrap = div().relative().child(label);

            if is_open {
                let mut dropdown = div()
                    .absolute()
                    .top(px(height + 4.0))
                    .left(px(0.0))
                    .min_w(px(200.0))
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .p(px(4.0))
                    .rounded(px(radius))
                    .border_1()
                    .border_color(border)
                    .bg(surface_color)
                    .shadow_md();

                for (ei, entry) in menu.entries.iter().enumerate() {
                    match entry {
                        Entry::Item {
                            label,
                            shortcut,
                            danger: is_danger,
                            disabled,
                            ..
                        } => {
                            let color = if *disabled {
                                dimmed
                            } else if *is_danger {
                                danger
                            } else {
                                text
                            };
                            let mut item = div()
                                .id(("guise-menubar-item", mi * 1000 + ei))
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap(px(24.0))
                                .px(px(10.0))
                                .py(px(6.0))
                                .rounded(px(4.0))
                                .text_size(px(font))
                                .text_color(color)
                                .child(label.clone())
                                .child(match shortcut {
                                    Some(s) => {
                                        div().text_size(px(font_xs)).text_color(dimmed).child(s.clone())
                                    }
                                    None => div(),
                                });
                            if !*disabled {
                                item = item.hover(move |s| s.bg(surface_hover));
                                if ei == self.highlight {
                                    item = item.bg(surface_hover);
                                }
                                item = item.on_click(cx.listener(
                                    move |this, _ev, window, cx| {
                                        this.open = None;
                                        if let Some(Entry::Item {
                                            handler: Some(handler),
                                            ..
                                        }) = this.menus.get(mi).and_then(|m| m.entries.get(ei))
                                        {
                                            handler(window, cx);
                                        }
                                        cx.notify();
                                    },
                                ));
                            }
                            dropdown = dropdown.child(item);
                        }
                        Entry::Section(label) => {
                            dropdown = dropdown.child(
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
                            dropdown = dropdown.child(div().my(px(4.0)).h(px(1.0)).bg(border));
                        }
                    }
                }

                wrap = wrap.child(deferred(dropdown).with_priority(1));
            }

            bar = bar.child(wrap);
        }

        bar
    }
}
