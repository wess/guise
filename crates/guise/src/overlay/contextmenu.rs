//! `ContextMenu` — a right-click action menu at the pointer (gpui entity).
//!
//! Unlike [`Menu`](super::Menu) there is no trigger element: the parent calls
//! [`ContextMenu::show`] with the window coordinates of a right-click
//! (`MouseDownEvent.position`) and renders the entity somewhere in its tree.
//! While open it paints a deferred, full-viewport backdrop (click-away closes)
//! with the menu positioned absolutely at the stored point, clamped to stay
//! on screen. Item clicks run their handler and close; Escape closes.
//!
//! ```ignore
//! let menu = cx.new(|cx| {
//!     ContextMenu::new(cx)
//!         .item_icon(IconName::Copy, "Copy", |_w, _app| { /* ... */ })
//!         .item("Rename", |_w, _app| { /* ... */ })
//!         .divider()
//!         .danger_item("Delete", |_w, _app| { /* ... */ })
//! });
//!
//! // In the parent's render:
//! div()
//!     .id("target")
//!     .on_mouse_down(MouseButton::Right, cx.listener(move |this, ev: &MouseDownEvent, window, cx| {
//!         let position = ev.position;
//!         this.menu.update(cx, |menu, cx| menu.show(position, window, cx));
//!     }))
//!     .child("Right-click me")
//!     .child(menu.clone()) // renders nothing while closed
//! ```

use gpui::prelude::*;
use gpui::{
    anchored, deferred, div, point, px, App, Context, FocusHandle, IntoElement, KeyDownEvent,
    Pixels, Point, SharedString, Window,
};

use crate::icon::{Icon, IconName};
use crate::input::control_metrics;
use crate::theme::{theme, ColorName, Size};

type ItemHandler = Box<dyn Fn(&mut Window, &mut App) + 'static>;

enum Entry {
    Item {
        label: SharedString,
        icon: Option<IconName>,
        danger: bool,
        handler: Option<ItemHandler>,
    },
    Section(SharedString),
    Divider,
}

/// Margin kept between the menu and the window edges when clamping.
const EDGE_MARGIN: f32 = 8.0;

/// Clamp a menu origin so `extent` stays inside `viewport` with a margin on
/// both edges. Falls back to the margin when the menu is larger than the
/// viewport (top/left wins).
fn clamp_origin(pos: f32, extent: f32, viewport: f32, margin: f32) -> f32 {
    pos.min(viewport - extent - margin).max(margin)
}

/// Estimated pixel height of the open menu. gpui hands elements no bounds of
/// their own before paint, so edge clamping works from this heuristic (item
/// rows track the font metrics used at render time).
fn estimated_height(entries: &[Entry], font: f32, font_xs: f32) -> f32 {
    let body: f32 = entries
        .iter()
        .map(|entry| match entry {
            Entry::Item { .. } => font * 1.5 + 12.0,
            Entry::Section(_) => font_xs * 1.5 + 8.0,
            Entry::Divider => 9.0,
        })
        .sum();
    body + 8.0
}

/// A pointer-positioned action menu. Create with
/// `cx.new(|cx| ContextMenu::new(cx).item(..))` and open it from a
/// right-click handler via [`ContextMenu::show`].
pub struct ContextMenu {
    entries: Vec<Entry>,
    open: bool,
    position: Point<Pixels>,
    focus: FocusHandle,
    /// Whatever was focused before `show` grabbed focus, restored on close so
    /// a text field the user was typing in gets its caret back.
    prev_focus: Option<FocusHandle>,
    size: Size,
    width: f32,
}

impl ContextMenu {
    pub fn new(cx: &mut Context<Self>) -> Self {
        ContextMenu {
            entries: Vec::new(),
            open: false,
            position: Point::default(),
            focus: cx.focus_handle(),
            prev_focus: None,
            size: Size::Sm,
            width: 220.0,
        }
    }

    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Fixed menu width in pixels (default `220.0`). Also drives the
    /// horizontal edge clamp, so keep it accurate for long labels.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
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
            icon: None,
            danger: false,
            handler: Some(Box::new(handler)),
        });
        self
    }

    /// Add an action item with a leading icon.
    pub fn item_icon(
        mut self,
        icon: IconName,
        label: impl Into<SharedString>,
        handler: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.entries.push(Entry::Item {
            label: label.into(),
            icon: Some(icon),
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
            icon: None,
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

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Open the menu at a window-coordinate point — pass
    /// `MouseDownEvent.position` from a `MouseButton::Right` handler. Grabs
    /// focus so Escape closes; the previous focus is restored on close.
    pub fn show(&mut self, position: Point<Pixels>, window: &mut Window, cx: &mut Context<Self>) {
        self.position = position;
        self.open = true;
        self.prev_focus = window.focused(cx);
        window.focus(&self.focus);
        cx.notify();
    }

    /// Close the menu, handing focus back to whatever held it before `show`.
    pub fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open = false;
        self.restore_focus(window, cx);
        cx.notify();
    }

    /// Hand focus back, unless something else (an item handler, a click into
    /// another field) already took it.
    fn restore_focus(&mut self, window: &mut Window, _cx: &mut Context<Self>) {
        if let Some(prev) = self.prev_focus.take() {
            if self.focus.is_focused(window) {
                window.focus(&prev);
            }
        }
    }

    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            return;
        }
        if event.keystroke.key.as_str() == "escape" {
            self.close(window, cx);
            cx.stop_propagation();
        }
    }
}

impl Render for ContextMenu {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut root = div();
        if !self.open {
            return root;
        }

        let t = theme(cx);
        let (_, _, font) = control_metrics(self.size);
        let font_xs = t.font_size(Size::Xs);
        let radius = t.radius(t.default_radius);
        let surface_color = t.surface().hsla();
        let surface_hover = t.surface_hover().hsla();
        let border = t.border().hsla();
        let text = t.text().hsla();
        let dimmed = t.dimmed().hsla();
        let danger = t
            .color(ColorName::Red, if t.scheme.is_dark() { 5 } else { 6 })
            .hsla();

        let viewport = window.viewport_size();
        let height = estimated_height(&self.entries, font, font_xs);
        let x = clamp_origin(
            f32::from(self.position.x),
            self.width,
            f32::from(viewport.width),
            EDGE_MARGIN,
        );
        let y = clamp_origin(
            f32::from(self.position.y),
            height,
            f32::from(viewport.height),
            EDGE_MARGIN,
        );

        let mut menu = div()
            .id("guise-contextmenu")
            .occlude()
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key))
            .on_click(|_ev, _window, cx| cx.stop_propagation())
            .absolute()
            .left(px(x))
            .top(px(y))
            .w(px(self.width))
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
                    icon,
                    danger: is_danger,
                    ..
                } => {
                    let mut item = div()
                        .id(("guise-contextmenu-item", i))
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .px(px(10.0))
                        .py(px(6.0))
                        .rounded(px(4.0))
                        .text_size(px(font))
                        .text_color(if *is_danger { danger } else { text })
                        .hover(move |s| s.bg(surface_hover));
                    if let Some(icon) = icon {
                        item = item.child(Icon::new(*icon).size(Size::Sm));
                    }
                    item = item.child(label.clone());
                    menu = menu.child(item.on_click(cx.listener(move |this, _ev, window, cx| {
                        this.open = false;
                        // Restore first: a handler that focuses something
                        // (rename → input) still wins.
                        this.restore_focus(window, cx);
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

        // Transparent full-viewport backdrop: occludes the page and closes the
        // menu on click-away.
        let backdrop = div()
            .id("guise-contextmenu-backdrop")
            .occlude()
            .absolute()
            .top(px(0.0))
            .left(px(0.0))
            .w(viewport.width)
            .h(viewport.height)
            .on_click(cx.listener(|this, _ev, window, cx| {
                this.close(window, cx);
            }))
            .child(menu);

        // The stored point is in window coordinates but `.absolute()` insets
        // resolve against the parent, so anchor the overlay at the window
        // origin: backdrop and menu then land in window space no matter where
        // in the tree this entity is rendered.
        root = root.child(deferred(
            anchored().position(point(px(0.0), px(0.0))).child(backdrop),
        ));
        root
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_keeps_fitting_menu_in_place() {
        assert_eq!(clamp_origin(100.0, 220.0, 800.0, 8.0), 100.0);
    }

    #[test]
    fn clamp_pulls_back_from_right_edge() {
        // 700 + 220 overflows an 800px viewport: clamp to 800 - 220 - 8.
        assert_eq!(clamp_origin(700.0, 220.0, 800.0, 8.0), 572.0);
    }

    #[test]
    fn clamp_never_goes_past_the_margin() {
        assert_eq!(clamp_origin(-40.0, 220.0, 800.0, 8.0), 8.0);
        // Menu taller than the viewport: pin to the top margin.
        assert_eq!(clamp_origin(300.0, 900.0, 600.0, 8.0), 8.0);
    }

    #[test]
    fn estimated_height_sums_entry_kinds() {
        let entries = vec![
            Entry::Item {
                label: SharedString::new_static("Copy"),
                icon: None,
                danger: false,
                handler: None,
            },
            Entry::Section(SharedString::new_static("Danger")),
            Entry::Divider,
        ];
        let expected = (14.0 * 1.5 + 12.0) + (12.0 * 1.5 + 8.0) + 9.0 + 8.0;
        assert_eq!(estimated_height(&entries, 14.0, 12.0), expected);
    }
}
