//! `TabBar` — a document-style tab strip (gpui entity).
//!
//! Owns the tab list and active index. Renders a horizontally scrollable
//! strip of tabs, each with a close button (shown while hovered or active),
//! plus an optional trailing add button. Emits [`TabBarEvent`] for selection,
//! close, and add clicks — closing does **not** remove the tab by itself, the
//! parent decides (e.g. after an unsaved-changes prompt) and calls
//! [`TabBar::remove_tab`].
//!
//! ```ignore
//! let bar = cx.new(|cx| TabBar::new(cx).tabs(["main.rs", "lib.rs"]).active(0));
//! cx.subscribe(&bar, |_this, bar, event: &TabBarEvent, cx| match event {
//!     TabBarEvent::Close(i) => {
//!         let i = *i;
//!         bar.update(cx, |b, cx| b.remove_tab(i, cx));
//!     }
//!     TabBarEvent::Add => bar.update(cx, |b, cx| b.add_tab("untitled", cx)),
//!     TabBarEvent::Select(_) => {}
//! })
//! .detach();
//! ```

use gpui::prelude::*;
use gpui::{div, px, Context, EventEmitter, IntoElement, SharedString, Window};

use crate::theme::{theme, Size};
use crate::{ActionIcon, CloseButton};

/// Emitted by [`TabBar`] on user interaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TabBarEvent {
    /// A tab was clicked; carries its index. The bar has already switched to it.
    Select(usize),
    /// A tab's close button was clicked; carries its index. The tab is *not*
    /// removed automatically — call [`TabBar::remove_tab`] to drop it.
    Close(usize),
    /// The trailing `+` button was clicked.
    Add,
}

/// Where the active index lands after removing `removed` from a list that is
/// now `new_len` items long.
fn active_after_remove(active: usize, removed: usize, new_len: usize) -> usize {
    if new_len == 0 {
        return 0;
    }
    let shifted = if removed < active { active - 1 } else { active };
    shifted.min(new_len - 1)
}

/// A document-style tab strip. Create with
/// `cx.new(|cx| TabBar::new(cx).tabs(["one", "two"]))`.
pub struct TabBar {
    tabs: Vec<SharedString>,
    active: usize,
    hovered: Option<usize>,
    with_add_button: bool,
}

impl EventEmitter<TabBarEvent> for TabBar {}

impl TabBar {
    pub fn new(_cx: &mut Context<Self>) -> Self {
        TabBar {
            tabs: Vec::new(),
            active: 0,
            hovered: None,
            with_add_button: true,
        }
    }

    /// Replace the tab labels (builder form; see [`TabBar::set_tabs`] for the
    /// post-construction method).
    pub fn tabs<I, S>(mut self, tabs: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.tabs = tabs.into_iter().map(Into::into).collect();
        self
    }

    /// The initially active tab.
    pub fn active(mut self, index: usize) -> Self {
        self.active = index;
        self
    }

    /// Show the trailing `+` button (default `true`).
    pub fn with_add_button(mut self, show: bool) -> Self {
        self.with_add_button = show;
        self
    }

    /// The index of the active tab.
    pub fn active_index(&self) -> usize {
        self.active
    }

    /// Number of tabs.
    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    /// Append a tab and make it active. Does not emit an event.
    pub fn add_tab(&mut self, label: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.tabs.push(label.into());
        self.active = self.tabs.len() - 1;
        cx.notify();
    }

    /// Remove the tab at `index` (no-op when out of range), keeping the
    /// active selection on the same document where possible. Does not emit.
    pub fn remove_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.tabs.len() {
            return;
        }
        self.tabs.remove(index);
        self.active = active_after_remove(self.active, index, self.tabs.len());
        self.hovered = None;
        cx.notify();
    }

    /// Replace every tab, clamping the active index. Does not emit.
    pub fn set_tabs(&mut self, tabs: Vec<SharedString>, cx: &mut Context<Self>) {
        self.tabs = tabs;
        self.active = self.active.min(self.tabs.len().saturating_sub(1));
        self.hovered = None;
        cx.notify();
    }

    /// Programmatically switch tabs (clamped). Does not emit.
    pub fn set_active(&mut self, index: usize, cx: &mut Context<Self>) {
        let clamped = index.min(self.tabs.len().saturating_sub(1));
        if self.active != clamped {
            self.active = clamped;
            cx.notify();
        }
    }
}

impl Render for TabBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        let surface = t.surface().hsla();
        let strip_bg = t.surface_hover().hsla();
        let border = t.border().hsla();
        let text = t.text().hsla();
        let dimmed = t.dimmed().hsla();
        let font = t.font_size(Size::Sm);

        let count = self.tabs.len();
        let active = if count == 0 {
            0
        } else {
            self.active.min(count - 1)
        };
        let hovered = self.hovered;

        let mut strip = div()
            .id("guise-tabbar-strip")
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .overflow_x_scroll();

        for (i, label) in self.tabs.iter().enumerate() {
            let is_active = i == active;
            let show_close = is_active || hovered == Some(i);

            // The close button keeps its slot when hidden so tab widths stay
            // stable; a hidden div paints nothing (no hitbox, no clicks).
            let mut close_slot = div().flex_none();
            if !show_close {
                close_slot = close_slot.invisible();
            }
            close_slot = close_slot.child(
                CloseButton::new(("guise-tabbar-close", i))
                    .size(Size::Xs)
                    .on_click(cx.listener(move |_this, _ev, _window, cx| {
                        // Don't let the click bubble into the tab (which
                        // would also select it).
                        cx.stop_propagation();
                        cx.emit(TabBarEvent::Close(i));
                    })),
            );

            let mut tab = div()
                .id(("guise-tabbar-tab", i))
                .flex_none()
                .flex()
                .items_center()
                .gap(px(6.0))
                .pl(px(12.0))
                .pr(px(6.0))
                .py(px(6.0))
                .border_r_1()
                .border_color(border)
                .text_size(px(font))
                .text_color(if is_active { text } else { dimmed })
                .child(label.clone())
                .child(close_slot)
                .on_hover(cx.listener(move |this, entered: &bool, _window, cx| {
                    if *entered {
                        this.hovered = Some(i);
                    } else if this.hovered == Some(i) {
                        this.hovered = None;
                    }
                    cx.notify();
                }))
                .on_click(cx.listener(move |this, _ev, _window, cx| {
                    this.active = i;
                    cx.emit(TabBarEvent::Select(i));
                    cx.notify();
                }));
            if is_active {
                tab = tab.bg(surface);
            } else {
                tab = tab.hover(move |s| s.text_color(text));
            }
            strip = strip.child(tab);
        }

        let mut bar = div()
            .flex()
            .items_center()
            .w_full()
            .bg(strip_bg)
            .border_b_1()
            .border_color(border)
            .child(strip);

        if self.with_add_button {
            bar = bar.child(
                div().flex_none().px(px(4.0)).child(
                    ActionIcon::new("guise-tabbar-add", "+")
                        .size(Size::Sm)
                        .on_click(cx.listener(|_this, _ev, _window, cx| cx.emit(TabBarEvent::Add))),
                ),
            );
        }

        bar
    }
}

#[cfg(test)]
mod tests {
    use super::active_after_remove;

    #[test]
    fn removing_before_active_shifts_it_left() {
        assert_eq!(active_after_remove(2, 0, 3), 1);
        assert_eq!(active_after_remove(3, 2, 3), 2);
    }

    #[test]
    fn removing_the_active_tab_keeps_its_slot_clamped() {
        // [a b c] active=1, remove 1 -> [a c] active=1 (c takes the slot).
        assert_eq!(active_after_remove(1, 1, 2), 1);
        // Removing the last while it is active clamps to the new tail.
        assert_eq!(active_after_remove(2, 2, 2), 1);
    }

    #[test]
    fn removing_after_active_leaves_it_alone() {
        assert_eq!(active_after_remove(0, 2, 2), 0);
        assert_eq!(active_after_remove(1, 3, 3), 1);
    }

    #[test]
    fn emptying_the_bar_resets_to_zero() {
        assert_eq!(active_after_remove(0, 0, 0), 0);
    }
}
