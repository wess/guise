//! Tab drag-and-drop for [`PaneGroup`](super::PaneGroup): the drag payload +
//! ghost, the edge-zone geometry that turns a cursor over a pane into a drop
//! edge (mirroring Zed), and the drop overlay.

use gpui::{
    div, px, relative, Bounds, Context, EntityId, IntoElement, ParentElement, Pixels, Point,
    Render, SharedString, Styled, Window,
};

use crate::SplitDirection;

use super::{ItemId, PaneId};

/// Which edge of a pane a drop targets — or the center (add as a tab, no split).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DropEdge {
    Left,
    Right,
    Top,
    Bottom,
}

impl DropEdge {
    /// The split axis and whether the dropped item's pane goes first (left/top).
    pub fn split(self) -> (SplitDirection, bool) {
        match self {
            DropEdge::Left => (SplitDirection::Horizontal, true),
            DropEdge::Right => (SplitDirection::Horizontal, false),
            DropEdge::Top => (SplitDirection::Vertical, true),
            DropEdge::Bottom => (SplitDirection::Vertical, false),
        }
    }
}

/// The payload carried while dragging a tab, and its own ghost renderer.
#[derive(Clone)]
pub struct TabDrag {
    pub group: EntityId,
    pub item: ItemId,
    pub from_pane: PaneId,
    pub label: SharedString,
}

impl Render for TabDrag {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px(px(10.0))
            .py(px(4.0))
            .rounded(px(6.0))
            .bg(gpui::rgb(0x2a2a2e))
            .text_color(gpui::rgb(0xf2f2f7))
            .text_size(px(12.0))
            .child(self.label.clone())
    }
}

/// Fraction of a pane's short side that counts as an edge zone.
const EDGE: f32 = 0.28;

/// Cursor within `bounds` → the nearest edge when in that edge's zone, or `None`
/// for the center (add as a tab) or outside the pane.
pub fn drop_edge(bounds: Bounds<Pixels>, cursor: Point<Pixels>) -> Option<DropEdge> {
    let w = f32::from(bounds.size.width);
    let h = f32::from(bounds.size.height);
    if w <= 0.0 || h <= 0.0 {
        return None;
    }
    let x = f32::from(cursor.x - bounds.origin.x);
    let y = f32::from(cursor.y - bounds.origin.y);
    if x < 0.0 || y < 0.0 || x > w || y > h {
        return None;
    }
    let edge = w.min(h) * EDGE;
    if x >= edge && x <= w - edge && y >= edge && y <= h - edge {
        return None;
    }
    let (left, right, top, bottom) = (x, w - x, y, h - y);
    let min = left.min(right).min(top).min(bottom);
    Some(if min == left {
        DropEdge::Left
    } else if min == right {
        DropEdge::Right
    } else if min == top {
        DropEdge::Top
    } else {
        DropEdge::Bottom
    })
}

/// A translucent highlight over the half of the pane the split would occupy (or
/// the whole pane for a center/tab drop), shown while a tab is dragged over it.
pub fn drop_overlay(edge: Option<DropEdge>) -> impl IntoElement {
    let fill = gpui::rgba(0x4a9eff33);
    let border = gpui::rgb(0x4a9eff);
    let base = div().absolute().bg(fill).border_2().border_color(border);
    match edge {
        None => base.top_0().left_0().size_full(),
        Some(DropEdge::Left) => base.top_0().left_0().bottom_0().w(relative(0.5)),
        Some(DropEdge::Right) => base.top_0().right_0().bottom_0().w(relative(0.5)),
        Some(DropEdge::Top) => base.top_0().left_0().right_0().h(relative(0.5)),
        Some(DropEdge::Bottom) => base.bottom_0().left_0().right_0().h(relative(0.5)),
    }
}
