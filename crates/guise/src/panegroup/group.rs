//! The `PaneGroup` gpui component: renders the [`PaneTree`] as nested flex
//! splits with a draggable divider per split and a tab bar per pane, pulling
//! each item's content and title from host callbacks (the `SplitPanel`
//! pattern). Mutating the layout is done through the model methods; user
//! interactions (activate/close/new tab, and later tear-off) surface as
//! [`PaneGroupEvent`]s the host reacts to.

use std::collections::HashMap;
use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    div, px, AnyElement, App, Context, DragMoveEvent, Empty, EntityId, EventEmitter, FocusHandle,
    IntoElement, SharedString, Window,
};

use crate::style::FlexExt;
use crate::theme::theme;
use crate::SplitDirection;

use super::drag::{drop_edge, drop_overlay, DropEdge, TabDrag};
use super::tree::clamp_ratio;
use super::{ItemId, Node, Pane, PaneId, PaneIds, PaneTree, SplitId};

/// Per-item content, re-invoked every render so content stays live.
type RenderItem = Rc<dyn Fn(ItemId, &mut Window, &mut App) -> AnyElement>;
/// Per-item title for its tab.
type ItemTitle = Rc<dyn Fn(ItemId, &App) -> SharedString>;

/// Interactions the host reacts to. The component owns layout; the host owns
/// items (creating/destroying their real content) and window management.
#[derive(Clone, Debug)]
pub enum PaneGroupEvent {
    /// An item's tab was clicked / it became active.
    Activated(ItemId),
    /// An item's close button was clicked; the host should drop the item and
    /// call [`PaneGroup::close_item`].
    CloseRequested(ItemId),
    /// The `+` on a pane's tab bar was clicked; the host should create an item
    /// and call [`PaneGroup::add_item`] on this pane.
    NewRequested(PaneId),
    /// The focused pane changed.
    FocusChanged(PaneId),
}

/// The divider being dragged, identifying its split and owning group.
#[derive(Clone, Copy)]
struct DividerDrag {
    group: EntityId,
    split: SplitId,
}

/// A recursive tree of tabbed panes. Construct with a first item, wire content
/// with [`PaneGroup::on_render_item`] / [`PaneGroup::on_item_title`], subscribe
/// for [`PaneGroupEvent`]s, and drive layout through the model methods.
pub struct PaneGroup {
    tree: PaneTree,
    panes: HashMap<PaneId, Pane>,
    ids: PaneIds,
    focused: PaneId,
    focus: FocusHandle,
    render_item: Option<RenderItem>,
    item_title: Option<ItemTitle>,
    /// The pane a tab is dragged over + the edge the drop would take (`None` =
    /// center = add as a tab). Drives the drop overlay; cleared on drop.
    drag_over: Option<(PaneId, Option<DropEdge>)>,
}

impl EventEmitter<PaneGroupEvent> for PaneGroup {}

impl PaneGroup {
    /// A new group with a single pane holding `first`.
    pub fn new(first: ItemId, cx: &mut Context<Self>) -> Self {
        let mut ids = PaneIds::new();
        let root = ids.next();
        let mut panes = HashMap::new();
        panes.insert(root, Pane::new(first));
        Self {
            tree: PaneTree::new(root),
            panes,
            ids,
            focused: root,
            focus: cx.focus_handle(),
            render_item: None,
            item_title: None,
            drag_over: None,
        }
    }

    /// Supply each item's content element (re-invoked every render).
    pub fn on_render_item(
        mut self,
        f: impl Fn(ItemId, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.render_item = Some(Rc::new(f));
        self
    }

    /// Supply each item's tab title.
    pub fn on_item_title(mut self, f: impl Fn(ItemId, &App) -> SharedString + 'static) -> Self {
        self.item_title = Some(Rc::new(f));
        self
    }

    // --- queries --------------------------------------------------------

    pub fn focused_pane(&self) -> PaneId {
        self.focused
    }

    /// The active item of the focused pane.
    pub fn active_item(&self) -> ItemId {
        self.panes[&self.focused].active()
    }

    /// Every item across every pane, in pane-layout order.
    pub fn items(&self) -> Vec<ItemId> {
        self.tree
            .panes()
            .into_iter()
            .filter_map(|p| self.panes.get(&p))
            .flat_map(|p| p.items().iter().copied())
            .collect()
    }

    pub fn pane_of(&self, item: ItemId) -> Option<PaneId> {
        self.panes
            .iter()
            .find(|(_, p)| p.contains(item))
            .map(|(&id, _)| id)
    }

    // --- mutation -------------------------------------------------------

    /// Add `item` as a new tab in `pane` and activate it.
    pub fn add_item(&mut self, pane: PaneId, item: ItemId, cx: &mut Context<Self>) {
        if let Some(p) = self.panes.get_mut(&pane) {
            p.add(item, None);
            self.set_focus(pane, cx);
        }
    }

    /// Add `item` to the focused pane.
    pub fn add_to_focused(&mut self, item: ItemId, cx: &mut Context<Self>) {
        let pane = self.focused;
        self.add_item(pane, item, cx);
    }

    /// Split `pane` in `dir`, putting `item` in the new pane (on the `first`
    /// side when true). Returns the new pane id.
    pub fn split(
        &mut self,
        pane: PaneId,
        dir: SplitDirection,
        first: bool,
        item: ItemId,
        cx: &mut Context<Self>,
    ) -> PaneId {
        let new_pane = self.ids.next();
        if self.tree.split(pane, dir, new_pane, first).is_some() {
            self.panes.insert(new_pane, Pane::new(item));
            self.set_focus(new_pane, cx);
        }
        new_pane
    }

    /// Activate `item` in `pane` and focus that pane.
    pub fn activate(&mut self, pane: PaneId, item: ItemId, cx: &mut Context<Self>) {
        if let Some(p) = self.panes.get_mut(&pane) {
            if p.activate_item(item) {
                self.set_focus(pane, cx);
                cx.emit(PaneGroupEvent::Activated(item));
            }
        }
    }

    /// Remove `item`; if its pane empties, collapse it out of the tree.
    pub fn close_item(&mut self, item: ItemId, cx: &mut Context<Self>) {
        let Some(pane) = self.pane_of(item) else {
            return;
        };
        let emptied = self.panes.get_mut(&pane).map(|p| p.remove(item)).unwrap_or(false);
        if emptied {
            self.panes.remove(&pane);
            // Last pane can't be removed from the tree; keep an empty group
            // valid by leaving it — but that shouldn't happen (host keeps ≥1).
            if self.tree.remove(pane) && self.focused == pane {
                let next = self.tree.panes().first().copied().unwrap_or(pane);
                self.set_focus(next, cx);
            }
        }
        cx.notify();
    }

    /// Move `item` onto `to_pane`: `edge = Some(..)` splits that pane and puts
    /// the item in the new split; `None` adds it as a tab. The item is detached
    /// from its source pane, collapsing that pane if it empties. Used on tab
    /// drop.
    pub fn move_item(
        &mut self,
        item: ItemId,
        to_pane: PaneId,
        edge: Option<DropEdge>,
        cx: &mut Context<Self>,
    ) {
        self.drag_over = None;
        let Some(from) = self.pane_of(item) else {
            cx.notify();
            return;
        };
        let from_single = self.panes.get(&from).is_some_and(|p| p.len() == 1);
        // Dropping a pane's only item back onto itself is a no-op.
        if from == to_pane && (edge.is_none() || from_single) {
            cx.notify();
            return;
        }
        self.detach(from, item);
        match edge {
            Some(edge) => {
                let (axis, first) = edge.split();
                let new_pane = self.ids.next();
                if self.tree.split(to_pane, axis, new_pane, first).is_some() {
                    self.panes.insert(new_pane, Pane::new(item));
                    self.set_focus(new_pane, cx);
                    return;
                }
                // Target vanished; fall back to a tab in some remaining pane.
                self.reattach_somewhere(item, cx);
            }
            None => {
                if let Some(p) = self.panes.get_mut(&to_pane) {
                    p.add(item, None);
                    self.set_focus(to_pane, cx);
                } else {
                    self.reattach_somewhere(item, cx);
                }
            }
        }
        cx.notify();
    }

    /// Reorder `item` within its pane to `index` (a tab-bar drop).
    pub fn reorder_in_pane(&mut self, item: ItemId, index: usize, cx: &mut Context<Self>) {
        if let Some(pane) = self.pane_of(item) {
            if let Some(p) = self.panes.get_mut(&pane) {
                if let Some(from) = p.index_of(item) {
                    p.reorder(from, index);
                    cx.notify();
                }
            }
        }
    }

    /// Remove `item` from `pane`, collapsing the pane out of the tree if empty.
    fn detach(&mut self, pane: PaneId, item: ItemId) {
        if let Some(p) = self.panes.get_mut(&pane) {
            if p.remove(item) {
                self.panes.remove(&pane);
                self.tree.remove(pane);
            }
        }
    }

    /// Last-resort: put a detached item back into any surviving pane.
    fn reattach_somewhere(&mut self, item: ItemId, cx: &mut Context<Self>) {
        if let Some(&pane) = self.tree.panes().first() {
            if let Some(p) = self.panes.get_mut(&pane) {
                p.add(item, None);
                self.set_focus(pane, cx);
            }
        }
    }

    /// Set the divider ratio of a split.
    pub fn set_ratio(&mut self, split: SplitId, ratio: f32, cx: &mut Context<Self>) {
        if self.tree.set_ratio(split, clamp_ratio(ratio)) {
            cx.notify();
        }
    }

    fn set_focus(&mut self, pane: PaneId, cx: &mut Context<Self>) {
        let changed = self.focused != pane;
        self.focused = pane;
        if changed {
            cx.emit(PaneGroupEvent::FocusChanged(pane));
        }
        cx.notify();
    }
}

impl gpui::Focusable for PaneGroup {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for PaneGroup {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let root = self.tree.root().clone();
        let render_item = self.render_item.clone();
        let item_title = self.item_title.clone();
        div()
            .size_full()
            .track_focus(&self.focus)
            .child(self.node_el(&root, &render_item, &item_title, window, cx))
    }
}

impl PaneGroup {
    /// Render a tree node: a leaf pane, or a split (two children + divider).
    fn node_el(
        &self,
        node: &Node,
        render_item: &Option<RenderItem>,
        item_title: &Option<ItemTitle>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match node {
            Node::Leaf(pane) => self.pane_el(*pane, render_item, item_title, window, cx),
            Node::Split {
                id,
                axis,
                ratio,
                first,
                second,
            } => {
                let horizontal = matches!(axis, SplitDirection::Horizontal);
                let ratio = *ratio;
                let split = *id;
                let group = cx.entity().entity_id();
                let f = self.node_el(first, render_item, item_title, window, cx);
                let s = self.node_el(second, render_item, item_title, window, cx);

                let line = theme(cx).border().hsla();
                let grip = theme(cx).primary().alpha(0.35);

                let first_pane = div()
                    .flex_basis(px(0.0))
                    .grow(ratio)
                    .overflow_hidden()
                    .child(f);
                let second_pane = div()
                    .flex_basis(px(0.0))
                    .grow(1.0 - ratio)
                    .overflow_hidden()
                    .child(s);

                let mut divider = div()
                    .id(("pg-divider", split.0 as usize))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .hover(move |st| st.bg(grip))
                    .on_drag(DividerDrag { group, split }, |_, _off, _w, cx| cx.new(|_| Empty));
                divider = if horizontal {
                    divider
                        .w(px(6.0))
                        .h_full()
                        .cursor_col_resize()
                        .child(div().w(px(1.0)).h_full().bg(line))
                } else {
                    divider
                        .h(px(6.0))
                        .w_full()
                        .cursor_row_resize()
                        .child(div().h(px(1.0)).w_full().bg(line))
                };

                let mut container = div().size_full().flex().on_drag_move(cx.listener(
                    move |this, ev: &DragMoveEvent<DividerDrag>, _w, cx| {
                        let d = ev.drag(cx);
                        if d.group != group || d.split != split {
                            return;
                        }
                        let b = ev.bounds;
                        let (pos, extent) = if horizontal {
                            (f32::from(ev.event.position.x - b.left()), f32::from(b.size.width))
                        } else {
                            (f32::from(ev.event.position.y - b.top()), f32::from(b.size.height))
                        };
                        if extent > 0.0 {
                            this.set_ratio(split, pos / extent, cx);
                        }
                    },
                ));
                container = if horizontal {
                    container.flex_row()
                } else {
                    container.flex_col()
                };
                container
                    .child(first_pane)
                    .child(divider)
                    .child(second_pane)
                    .into_any_element()
            }
        }
    }

    /// Render one pane: its tab bar over the active item's content.
    fn pane_el(
        &self,
        pane: PaneId,
        render_item: &Option<RenderItem>,
        item_title: &Option<ItemTitle>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(p) = self.panes.get(&pane) else {
            return div().into_any_element();
        };
        let t = theme(cx);
        let surface = t.surface().hsla();
        let text = t.text().hsla();
        let border = t.border().hsla();
        let active_bg = t.surface_hover().hsla();

        let active = p.active();
        let group = cx.entity().entity_id();
        let tabs = p.items().iter().copied().enumerate().map(|(i, item)| {
            let title = item_title
                .as_ref()
                .map(|f| f(item, cx))
                .unwrap_or_else(|| SharedString::from("untitled"));
            let is_active = item == active;
            div()
                .id(("pg-tab", (pane.0 as usize) << 20 | i))
                .flex()
                .items_center()
                .gap_1()
                .px_2()
                .h(px(28.0))
                .when(is_active, |d| d.bg(active_bg))
                .text_color(text)
                .hover(|s| s.bg(active_bg))
                .on_click(cx.listener(move |this, _ev, _w, cx| this.activate(pane, item, cx)))
                // Drag this tab; drop on a tab to reorder / move-into, or on a
                // pane body to split (handled on the content wrapper below).
                .on_drag(
                    TabDrag {
                        group,
                        item,
                        from_pane: pane,
                        label: title.clone(),
                    },
                    |d, _off, _w, cx| cx.new(|_| d.clone()),
                )
                .on_drop(cx.listener(move |this, d: &TabDrag, _w, cx| {
                    if d.from_pane != pane {
                        this.move_item(d.item, pane, None, cx);
                    }
                    this.reorder_in_pane(d.item, i, cx);
                }))
                .child(div().text_size(px(12.0)).child(title))
                .child(
                    div()
                        .id(("pg-tabclose", (pane.0 as usize) << 20 | i))
                        .text_size(px(12.0))
                        .text_color(text)
                        .hover(|s| s.text_color(text))
                        .child("\u{00d7}")
                        .on_click(cx.listener(move |_this, _ev, _w, cx| {
                            cx.emit(PaneGroupEvent::CloseRequested(item));
                        })),
                )
        });

        let tab_bar = div()
            .flex()
            .flex_row()
            .items_center()
            .w_full()
            .h(px(28.0))
            .bg(surface)
            .border_b_1()
            .border_color(border)
            .children(tabs)
            .child(
                div()
                    .id(("pg-newtab", pane.0 as usize))
                    .px_2()
                    .h(px(28.0))
                    .flex()
                    .items_center()
                    .text_color(text)
                    .hover(|s| s.bg(active_bg))
                    .child("+")
                    .on_click(cx.listener(move |_this, _ev, _w, cx| {
                        cx.emit(PaneGroupEvent::NewRequested(pane));
                    })),
            );

        let content = render_item
            .as_ref()
            .map(|f| f(active, window, cx))
            .unwrap_or_else(|| div().into_any_element());

        // The drop edge over this pane, if a tab is being dragged onto it.
        let over = match self.drag_over {
            Some((p, edge)) if p == pane => Some(edge),
            _ => None,
        };
        let body = div()
            .relative()
            .flex_1()
            .overflow_hidden()
            .on_drag_move::<TabDrag>(cx.listener(
                move |this, ev: &DragMoveEvent<TabDrag>, _w, cx| {
                    let edge = drop_edge(ev.bounds, ev.event.position);
                    if this.drag_over != Some((pane, edge)) {
                        this.drag_over = Some((pane, edge));
                        cx.notify();
                    }
                },
            ))
            .on_drop(cx.listener(move |this, d: &TabDrag, _w, cx| {
                let edge = match this.drag_over {
                    Some((p, e)) if p == pane => e,
                    _ => None,
                };
                this.move_item(d.item, pane, edge, cx);
            }))
            .child(content)
            .when_some(over, |el, edge| el.child(drop_overlay(edge)));

        div()
            .flex()
            .flex_col()
            .size_full()
            .child(tab_bar)
            .child(body)
            .into_any_element()
    }
}
