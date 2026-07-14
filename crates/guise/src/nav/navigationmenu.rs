//! `NavigationMenu` — a horizontal top-nav with optional dropdowns
//! (gpui entity).
//!
//! Items are leaves (click → event) or menus (click toggles a deferred
//! dropdown of entries). Emits [`NavigationMenuEvent`] with the picked id.

use gpui::prelude::*;
use gpui::{
    deferred, div, px, Context, EventEmitter, FocusHandle, IntoElement, SharedString, Window,
};

use crate::icon::{Icon, IconName};
use crate::theme::{theme, Size};

/// Emitted when a leaf item or dropdown entry is picked. Carries its id.
#[derive(Debug, Clone)]
pub struct NavigationMenuEvent(pub SharedString);

struct NavEntry {
    id: SharedString,
    label: SharedString,
}

struct NavItem {
    id: SharedString,
    label: SharedString,
    entries: Vec<NavEntry>,
}

/// A top navigation bar. Create with
/// `cx.new(|cx| NavigationMenu::new(cx).item("home", "Home").menu("docs", "Docs", [..]))`.
pub struct NavigationMenu {
    items: Vec<NavItem>,
    active: Option<SharedString>,
    open: Option<usize>,
    focus: FocusHandle,
}

impl EventEmitter<NavigationMenuEvent> for NavigationMenu {}

impl NavigationMenu {
    pub fn new(cx: &mut Context<Self>) -> Self {
        NavigationMenu {
            items: Vec::new(),
            active: None,
            open: None,
            focus: cx.focus_handle(),
        }
    }

    /// A leaf item: clicking emits its id.
    pub fn item(mut self, id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        self.items.push(NavItem {
            id: id.into(),
            label: label.into(),
            entries: Vec::new(),
        });
        self
    }

    /// A dropdown item: clicking opens `(id, label)` entries.
    pub fn menu<I, S1, S2>(
        mut self,
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        entries: I,
    ) -> Self
    where
        I: IntoIterator<Item = (S1, S2)>,
        S1: Into<SharedString>,
        S2: Into<SharedString>,
    {
        self.items.push(NavItem {
            id: id.into(),
            label: label.into(),
            entries: entries
                .into_iter()
                .map(|(id, label)| NavEntry {
                    id: id.into(),
                    label: label.into(),
                })
                .collect(),
        });
        self
    }

    /// Highlight the item (or dropdown owner) with this id as current.
    pub fn active(mut self, id: impl Into<SharedString>) -> Self {
        self.active = Some(id.into());
        self
    }

    /// Move the highlight at runtime (e.g. after routing).
    pub fn set_active(&mut self, id: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.active = Some(id.into());
        cx.notify();
    }

    fn pick(&mut self, id: SharedString, cx: &mut Context<Self>) {
        self.open = None;
        self.active = Some(id.clone());
        cx.emit(NavigationMenuEvent(id));
        cx.notify();
    }
}

impl Render for NavigationMenu {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        let radius = t.radius(Size::Sm);
        let surface = t.surface().hsla();
        let surface_hover = t.surface_hover().hsla();
        let border = t.border().hsla();
        let text_color = t.text().hsla();
        let dimmed = t.dimmed().hsla();
        let active_bg = t.primary().alpha(0.12);
        let active_fg = t.primary().hsla();
        let font = t.font_size(Size::Sm);

        let mut bar = div()
            .track_focus(&self.focus)
            .flex()
            .items_center()
            .gap(px(4.0));

        for (i, item) in self.items.iter().enumerate() {
            let has_menu = !item.entries.is_empty();
            let is_open = self.open == Some(i);
            let is_active = self.active.as_ref() == Some(&item.id)
                || item.entries.iter().any(|e| self.active.as_ref() == Some(&e.id));
            let id = item.id.clone();

            let mut label_row = div()
                .flex()
                .items_center()
                .gap(px(4.0))
                .child(item.label.clone());
            if has_menu {
                label_row = label_row.child(
                    div().text_color(dimmed).child(
                        Icon::new(if is_open {
                            IconName::ChevronUp
                        } else {
                            IconName::ChevronDown
                        })
                        .size(Size::Xs),
                    ),
                );
            }

            let mut trigger = div()
                .id(("guise-navmenu-item", i))
                .px(px(10.0))
                .py(px(6.0))
                .rounded(px(radius))
                .text_size(px(font))
                .text_color(if is_active { active_fg } else { text_color })
                .child(label_row)
                .on_click(cx.listener(move |this, _ev, _window, cx| {
                    if this.items[i].entries.is_empty() {
                        this.pick(this.items[i].id.clone(), cx);
                    } else {
                        this.open = if this.open == Some(i) { None } else { Some(i) };
                        cx.notify();
                    }
                }));
            let _ = id;
            if is_active {
                trigger = trigger.bg(active_bg);
            } else {
                trigger = trigger.hover(move |s| s.bg(surface_hover));
            }

            let mut cell = div().relative().child(trigger);
            if is_open && has_menu {
                let mut menu = div()
                    .absolute()
                    .top(px(34.0))
                    .left(px(0.0))
                    .min_w(px(180.0))
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .p(px(4.0))
                    .rounded(px(t.radius(t.default_radius)))
                    .border_1()
                    .border_color(border)
                    .bg(surface)
                    .shadow_md()
                    .occlude();
                for (j, entry) in item.entries.iter().enumerate() {
                    let entry_id = entry.id.clone();
                    menu = menu.child(
                        div()
                            .id(("guise-navmenu-entry", i * 100 + j))
                            .px(px(10.0))
                            .py(px(6.0))
                            .rounded(px(4.0))
                            .text_size(px(font))
                            .text_color(text_color)
                            .hover(move |s| s.bg(surface_hover))
                            .child(entry.label.clone())
                            .on_click(cx.listener(move |this, _ev, _window, cx| {
                                this.pick(entry_id.clone(), cx);
                            })),
                    );
                }
                cell = cell.child(deferred(menu));
            }
            bar = bar.child(cell);
        }
        bar
    }
}
