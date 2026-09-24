//! Tab drag-and-drop for [`PaneGroup`](super::PaneGroup): the drag payload +
//! ghost, the edge-zone geometry that turns a cursor over a pane into a drop
//! edge (mirroring Zed), and the drop affordances — the pane overlay, the
//! tab-strip insertion line, and the tear overlay for a drag that has left the
//! group and would pull the item into a window of its own.

use std::cell::Cell;
use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
  div, px, relative, AnyElement, App, Bounds, Context, EntityId, Hsla, Pixels, Point, SharedString,
  Size, Window,
};

use crate::theme::theme;
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

/// Where a tab drag let go, for a host that has to place whatever the release
/// creates. Both points are in the group's window coordinates.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct TearDrop {
  /// The pointer position the drag released at.
  pub position: Point<Pixels>,
  /// The pointer's offset inside the tab when the drag began. gpui anchors the
  /// ghost at `position - grab`, so a host that reuses that corner puts what
  /// it opens exactly where the ghost was.
  pub grab: Point<Pixels>,
}

/// What the owning group tells the live drag ghost, refreshed as the pointer
/// moves. gpui takes the ghost as an entity of its own the moment a drag
/// starts, so a shared cell is the group's only channel to it.
#[derive(Clone, Copy, Default, PartialEq)]
pub struct TearHint {
  /// The pointer has left the group: a release here tears the item off.
  pub outside: bool,
  /// The group's own size, so the ghost stands in for the window a release
  /// would open at the proportions it would really have.
  pub group: Size<Pixels>,
}

/// The payload carried while dragging a tab.
#[derive(Clone)]
pub struct TabDrag {
  pub group: EntityId,
  pub item: ItemId,
  pub from_pane: PaneId,
  pub label: SharedString,
}

/// The tab that follows the cursor while dragging. gpui paints it at the grab
/// point's offset into the real tab, so it carries the strip's metrics (height,
/// dot) — at any other size it slides out from under the cursor.
pub struct TabGhost {
  pub label: SharedString,
  pub dot: Option<Hsla>,
  pub height: f32,
  /// Shared with the owning group, which flips it as the pointer crosses the
  /// group's edge. The ghost has to stop promising to move a tab the moment a
  /// release would stop moving one.
  pub hint: Rc<Cell<TearHint>>,
}

impl Render for TabGhost {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let hint = self.hint.get();
    if hint.outside {
      return window_ghost(self.label.clone(), hint.group, cx);
    }
    let t = theme(cx);
    let bg = t.surface_hover().hsla();
    let text = t.text().hsla();
    let edge = t.primary().alpha(0.7);
    div()
      .flex()
      .items_center()
      .gap_1()
      .px_2()
      .h(px(self.height))
      .rounded(px(6.0))
      .bg(bg)
      .border_1()
      .border_color(edge)
      .shadow_lg()
      // Lifted, not opaque: the tab is in transit, and what it will land
      // on has to stay readable underneath it.
      .opacity(0.85)
      .text_color(text)
      .text_size(px(12.0))
      .when_some(self.dot, |d, color| {
        d.child(
          div()
            .flex_none()
            .w(px(6.0))
            .h(px(6.0))
            .rounded_full()
            .bg(color),
        )
      })
      .child(self.label.clone())
      .into_any_element()
  }
}

/// The ghost once the pointer is outside the group: a shrunken window carrying
/// the tab in its own strip, at the proportions the real one would have.
fn window_ghost(label: SharedString, group: Size<Pixels>, cx: &App) -> AnyElement {
  let t = theme(cx);
  let (w, h) = preview_size(group);
  div()
    .w(w)
    .h(h)
    .flex()
    .flex_col()
    .rounded(px(8.0))
    .overflow_hidden()
    .bg(t.body().hsla())
    .border_1()
    .border_color(t.primary().alpha(0.7))
    .shadow_lg()
    .child(
      div()
        .flex()
        .items_center()
        .w_full()
        .h(px(26.0))
        .px(px(6.0))
        .bg(t.surface().hsla())
        .child(
          div()
            .px(px(8.0))
            .py(px(3.0))
            .rounded(px(4.0))
            .bg(t.surface_hover().hsla())
            .text_color(t.text().hsla())
            .text_size(px(11.0))
            .child(label),
        ),
    )
    .into_any_element()
}

/// The window preview's share of the group it is leaving.
const PREVIEW_SCALE: f32 = 0.32;
/// The range that share is held to, so the ghost reads at a glance from a small
/// group without swallowing the screen from a large one.
const PREVIEW_MIN: (f32, f32) = (220.0, 150.0);
const PREVIEW_MAX: (f32, f32) = (400.0, 280.0);

fn preview_size(group: Size<Pixels>) -> (Pixels, Pixels) {
  let w = f32::from(group.width) * PREVIEW_SCALE;
  let h = f32::from(group.height) * PREVIEW_SCALE;
  (
    px(w.clamp(PREVIEW_MIN.0, PREVIEW_MAX.0)),
    px(h.clamp(PREVIEW_MIN.1, PREVIEW_MAX.1)),
  )
}

/// The half of the tear signal the group draws itself, at `at` in its own
/// coordinates. gpui paints the ghost into the window that owns the drag, so
/// past that window's edge it is clipped away and the ghost alone would leave
/// the gesture looking like nothing is happening — this stays on screen. Same
/// language as [`drop_overlay`]: a highlight over where a release would land.
pub fn tear_overlay(at: Point<Pixels>, group: Size<Pixels>) -> impl IntoElement {
  let (w, h) = preview_size(group);
  // Held whole inside the group: the edge the pointer left by is the edge the
  // card presses against, so it reads as being carried out that way.
  let x = f32::from(at.x) - f32::from(w) / 2.0;
  let y = f32::from(at.y) - f32::from(h) / 2.0;
  div()
    .absolute()
    .left(px(
      x.clamp(0.0, (f32::from(group.width) - f32::from(w)).max(0.0)),
    ))
    .top(px(
      y.clamp(0.0, (f32::from(group.height) - f32::from(h)).max(0.0)),
    ))
    .w(w)
    .h(h)
    .flex()
    .items_center()
    .justify_center()
    .rounded(px(8.0))
    .bg(gpui::rgba(ACCENT << 8 | 0x26))
    .border_2()
    .border_color(gpui::rgb(ACCENT))
    // It has to say what it is. The ghost that would have shown the tab
    // leaving is clipped away at the window's edge by the OS, so without a
    // label this is an unexplained rectangle pinned inside the window you
    // are dragging out of — which reads as the tab having got stuck.
    .child(
      div()
        .px(px(10.0))
        .text_size(px(12.0))
        .text_color(gpui::rgb(0xf2f2f7))
        .child("Release to open in a new window"),
    )
}

/// The accent every drop affordance shares, so the pane overlay and the strip's
/// insertion line read as one gesture.
const ACCENT: u32 = 0x4a9eff;

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
  let fill = gpui::rgba(ACCENT << 8 | 0x33);
  let border = gpui::rgb(ACCENT);
  let base = div().absolute().bg(fill).border_2().border_color(border);
  match edge {
    None => base.top_0().left_0().size_full(),
    Some(DropEdge::Left) => base.top_0().left_0().bottom_0().w(relative(0.5)),
    Some(DropEdge::Right) => base.top_0().right_0().bottom_0().w(relative(0.5)),
    Some(DropEdge::Top) => base.top_0().left_0().right_0().h(relative(0.5)),
    Some(DropEdge::Bottom) => base.bottom_0().left_0().right_0().h(relative(0.5)),
  }
}

/// The bar marking the gap a dragged tab would drop into. It is drawn inside
/// the tab it sits against — tabs are flush neighbours, so a tab's leading edge
/// *is* the gap before it, and `at_end` puts it on the last tab's trailing edge
/// for the one gap no tab leads.
pub fn insert_line(at_end: bool) -> impl IntoElement {
  let bar = div()
    .absolute()
    .top_0()
    .bottom_0()
    .w(px(2.0))
    .bg(gpui::rgb(ACCENT));
  if at_end {
    bar.right_0()
  } else {
    bar.left_0()
  }
}
