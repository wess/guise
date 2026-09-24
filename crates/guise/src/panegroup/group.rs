//! The `PaneGroup` gpui component: renders the [`PaneTree`] as nested flex
//! splits with a draggable divider per split and a tab bar per pane, pulling
//! each item's content and title from host callbacks (the `SplitPanel`
//! pattern). Mutating the layout is done through the model methods; user
//! interactions (activate/close/new tab, and later tear-off) surface as
//! [`PaneGroupEvent`]s the host reacts to.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
  div, px, relative, AnyElement, App, ClickEvent, Context, Div, DragMoveEvent, Empty, EntityId,
  EventEmitter, FocusHandle, Hsla, IntoElement, MouseButton, MouseUpEvent, Pixels, Point,
  ScrollHandle, SharedString, Size, Stateful, Window, WindowControlArea,
};

use crate::devtools::Probed;
use crate::style::FlexExt;
use crate::theme::theme;
use crate::SplitDirection;

use super::drag::{
  drop_edge, drop_overlay, insert_line, tear_overlay, DropEdge, TabDrag, TabGhost, TearDrop,
  TearHint,
};
use super::motion::{slots, Ghosting, Slot, TabMotion};
use super::tree::clamp_ratio;
use super::{
  compute_layout, neighbor, Direction, ItemId, Node, Pane, PaneId, PaneIds, PaneTree, Rect, SplitId,
};

/// Per-item content, re-invoked every render so content stays live.
type RenderItem = Rc<dyn Fn(ItemId, &mut Window, &mut App) -> AnyElement>;
/// Per-item title for its tab.
type ItemTitle = Rc<dyn Fn(ItemId, &App) -> SharedString>;
/// Optional per-item status dot color for its tab (`None` = no dot). Lets the
/// host surface a state indicator (e.g. an agent's working/blocked state) in the
/// tab strip without the group knowing what the color means.
type ItemDot = Rc<dyn Fn(ItemId, &App) -> Option<gpui::Hsla>>;

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
  /// A pane's split control was clicked; the host should create an item and
  /// call [`PaneGroup::split`] on *this* pane — not the focused one, which is
  /// a different pane whenever the click is what focused this one.
  SplitRequested {
    pane: PaneId,
    axis: SplitDirection,
    first: bool,
  },
  /// The focused pane changed.
  FocusChanged(PaneId),
  /// An item was torn off (via [`PaneGroup::tear_off`]); the host should move
  /// its content into a new window. The item is already detached from here.
  /// `drop` locates the gesture that asked for it, so the host can open the
  /// window where the drag let go; a tear with no gesture behind it (a menu
  /// item) leaves it unset and the host places the window itself.
  TearOff {
    item: ItemId,
    drop: Option<TearDrop>,
  },
  /// A tab was right-clicked, at this window position. The host owns what a
  /// tab's context menu offers, so the group only reports the gesture.
  ContextMenu {
    item: ItemId,
    position: gpui::Point<gpui::Pixels>,
  },
}

/// How wide a tab may get. Without a ceiling two tabs in a wide pane each take
/// half the window, which reads as a segmented control rather than tabs.
const MAX_TAB_W: f32 = 240.0;

/// How narrow a tab may get before the strip scrolls instead. Under this a tab
/// shows two or three characters of a title, which is not enough to pick one
/// tab out of a row, so past here the width has to come from somewhere else.
const MIN_TAB_W: f32 = 96.0;

/// Width of the invisible stage a tab is laid out on while it grows in. Only
/// has to be wider than any tab; it is clipped away either way.
const STAGE: f32 = 4096.0;

/// The hover-group name for one tab, so its close button can react to the tab
/// being pointed at rather than only to itself.
fn tab_group(pane: PaneId, index: usize) -> SharedString {
  SharedString::from(format!("pg-tab-{}-{index}", pane.0))
}

/// One pane's tab strip. gpui measures a scroller during prepaint, so a render
/// only ever reads the *previous* frame's extents: the first render of a strip
/// sees zero overflow and would draw no overflow chrome at all. `seen` is the
/// overflow the last render drew for, and a mismatch buys one more frame.
struct TabStrip {
  scroll: ScrollHandle,
  seen: Option<f32>,
  /// The active item the last render scrolled into view, so cycling tabs from
  /// the keyboard follows the active one without pinning the strip to it and
  /// undoing every scroll the user makes by hand.
  followed: Option<ItemId>,
  /// Which of the strip's children the last render put a tab in, and which of
  /// those held a settled one. Everything read back off the scroll handle is
  /// that render's, so it has to be counted in that render's children — a
  /// ghost holds a slot exactly as a tab does, and one closing shifts every
  /// index after it. Tabs still growing in are left out of `settled`: their
  /// width is the animation's, not one to collapse from later.
  tabs: Vec<usize>,
  settled: Vec<Option<ItemId>>,
}

impl TabStrip {
  fn new() -> Self {
    Self {
      scroll: ScrollHandle::new(),
      seen: None,
      followed: None,
      tabs: Vec::new(),
      settled: Vec::new(),
    }
  }
}

/// How many tabs sit off each end of `strip`, given where in its children each
/// tab sits. A tab counts by its middle rather than by whether any of it is
/// clipped, so the two numbers plus the tabs you would count on screen come to
/// the number of tabs there are. Child bounds are recorded before the scroll
/// offset is applied, so it is added back here.
fn hidden_tabs(strip: &ScrollHandle, tabs: &[usize]) -> (usize, usize) {
  let view = strip.bounds();
  let offset = strip.offset().x;
  let mut before = 0;
  let mut after = 0;
  for &child in tabs {
    let Some(tab) = strip.bounds_for_item(child) else {
      continue;
    };
    let middle = tab.center().x + offset;
    if middle < view.left() {
      before += 1;
    } else if middle > view.right() {
      after += 1;
    }
  }
  (before, after)
}

/// Scroll `strip` most of a viewport towards `dir` (`1` = left, `-1` = right).
/// Short of a viewport so a tab straddling the edge stays on screen and the eye
/// keeps its place.
fn page_strip(strip: &ScrollHandle, dir: f32) {
  let step = f32::from(strip.bounds().size.width) * 0.75;
  let max = f32::from(strip.max_offset().x);
  let mut offset = strip.offset();
  offset.x = px((f32::from(offset.x) + dir * step).clamp(-max, 0.0));
  strip.set_offset(offset);
}

/// An overflow arrow's label. The count is what makes the arrow an answer
/// rather than a hint, but it comes from a measurement that lands a frame
/// behind, so the bare chevron stands in until the number is real.
fn arrow_label(count: usize, leading: bool) -> SharedString {
  let chevron = if leading { "\u{2039}" } else { "\u{203a}" };
  match (count, leading) {
    (0, _) => SharedString::from(chevron),
    (n, true) => SharedString::from(format!("{chevron} {n}")),
    (n, false) => SharedString::from(format!("{n} {chevron}")),
  }
}

/// One end of an overflowing tab strip: how many tabs lie that way, and an
/// arrow that pages towards them.
fn overflow_arrow(
  id: (&'static str, usize),
  label: SharedString,
  leading: bool,
  tab_h: f32,
  colors: (Hsla, Hsla, Hsla),
  on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
  let (text, hover_bg, border) = colors;
  div()
    .id(id)
    .flex_none()
    .flex()
    .items_center()
    .h(px(tab_h))
    .px_1()
    .gap_1()
    .text_size(px(11.0))
    .text_color(text)
    .border_color(border)
    .when(leading, |d| d.border_r_1())
    .when(!leading, |d| d.border_l_1())
    .hover(|s| s.bg(hover_bg))
    .child(label)
    .on_click(on_click)
}

/// A pane's split control: a stroked rectangle divided the way the split would
/// divide the pane, over a hit area the size of the `+` beside it.
///
/// Drawn rather than lettered. The control has to say *which* way it splits,
/// and the glyphs that mean "split" (`\u{2016}`, `\u{2261}`) read as neither a
/// pane nor a direction at 11px — the shape of the result is the only thing
/// that carries it at this size.
fn split_control(
  id: (&'static str, usize),
  axis: SplitDirection,
  tab_h: f32,
  colors: (Hsla, Hsla),
  on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
  let (line, hover_bg) = colors;
  // Sized off the tab bar rather than fixed: the host sets `tab_h`, and an
  // icon that ignores it drifts off centre in a taller bar.
  let h = (tab_h * 0.4).clamp(9.0, 14.0);
  let w = h * 1.2;
  let frame = div()
    .relative()
    .w(px(w))
    .h(px(h))
    .border_1()
    .border_color(line)
    .rounded(px(2.0));
  div()
    .id(id)
    .flex_none()
    .flex()
    .items_center()
    .justify_center()
    .w(px(tab_h))
    .h(px(tab_h))
    .hover(|s| s.bg(hover_bg))
    .child(match axis {
      // Horizontal lays the panes side by side, so the divider the icon
      // draws is the vertical one.
      SplitDirection::Horizontal => frame.child(
        div()
          .absolute()
          .top_0()
          .bottom_0()
          .left(relative(0.5))
          .w(px(1.0))
          .bg(line),
      ),
      SplitDirection::Vertical => frame.child(
        div()
          .absolute()
          .left_0()
          .right_0()
          .top(relative(0.5))
          .h(px(1.0))
          .bg(line),
      ),
    })
    .on_click(on_click)
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
  item_dot: Option<ItemDot>,
  /// The pane a tab is dragged over + the edge the drop would take (`None` =
  /// center = add as a tab). Drives the drop overlay; cleared on drop.
  drag_over: Option<(PaneId, Option<DropEdge>)>,
  /// The tab strip a tab is dragged over + the gap the drop would open, as an
  /// index into that pane's tabs (`0..=len`). Drives the insertion line, and
  /// the drop reads it back so the tab lands where the line promised.
  tab_drop: Option<(PaneId, usize)>,
  /// The item currently being dragged (tracked from `on_drag_move`); if the
  /// drag is released *outside* the group it's torn off. Cleared on any drop.
  dragging: Option<ItemId>,
  /// Per-pane tab strips, kept across renders so a pane's scroll position
  /// survives a repaint and so its overflow can be compared frame to frame.
  /// Behind a `RefCell` because rendering is `&self` the whole way down.
  strips: RefCell<HashMap<PaneId, TabStrip>>,
  /// Tab-strip motion. Shared, because a tab growing in is measured by a
  /// `'static` prepaint listener that cannot reach the group.
  motion: Rc<RefCell<TabMotion>>,
  /// Shared with the live drag ghost so it can turn into a window-to-be the
  /// moment a release would stop being a move within this group.
  hint: Rc<Cell<TearHint>>,
  /// Where the pointer is, in the group's own coordinates, while it is outside
  /// the group. Drives the tear overlay; `None` whenever a release would still
  /// land inside.
  tear_at: Option<Point<Pixels>>,
  /// The pointer's offset inside the tab the drag started from, captured when
  /// gpui builds the ghost and handed on with the tear.
  grab: Rc<Cell<Point<Pixels>>>,
  /// When set, the focused pane fills the group (the rest is hidden).
  zoomed: bool,
  /// Height of each pane's tab bar in px (also the titlebar height when the
  /// group doubles as the titlebar). Defaults to 28.
  tab_height: f32,
  /// When set (`leading`, `trailing`) px, the group doubles as the window
  /// titlebar: the top-left pane's tab bar reserves `leading` px on the left
  /// (for window controls like the macOS traffic lights) and the top-right
  /// pane's tab bar reserves `trailing` px on the right, with a
  /// window-draggable filler after its tabs. The host overlays its own
  /// controls in those insets and renders the group flush to the window top.
  titlebar: Option<(f32, f32)>,
  /// Whether each tab bar draws its own split controls.
  split_controls: bool,
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
      item_dot: None,
      drag_over: None,
      tab_drop: None,
      dragging: None,
      strips: RefCell::new(HashMap::new()),
      motion: Rc::new(RefCell::new(TabMotion::default())),
      hint: Rc::new(Cell::new(TearHint::default())),
      tear_at: None,
      grab: Rc::new(Cell::new(Point::default())),
      tab_height: 28.0,
      zoomed: false,
      titlebar: None,
      split_controls: true,
    }
  }

  /// Set the per-pane tab-bar height in px (also the titlebar height when the
  /// group is the titlebar). Defaults to 28.
  pub fn tab_height(mut self, px: f32) -> Self {
    self.tab_height = px;
    self
  }

  /// Show the split controls at the right of every tab bar. On by default;
  /// turn them off when the host draws its own somewhere else, such as in a
  /// titlebar that is no longer the tab bar.
  pub fn split_controls(mut self, show: bool) -> Self {
    self.split_controls = show;
    self
  }

  /// Change the titlebar integration after construction, for a host whose
  /// setting for it can be flipped while the window is open.
  pub fn set_titlebar(&mut self, insets: Option<(f32, f32)>) {
    self.titlebar = insets;
  }

  /// Change whether the tab bars draw split controls, after construction.
  pub fn set_split_controls(&mut self, show: bool) {
    self.split_controls = show;
  }

  /// Make the group double as the window titlebar: the top-row tab bars
  /// reserve `leading`/`trailing` px for the host's window controls and the
  /// top-right filler becomes a window-drag region. Render the group flush to
  /// the window top and overlay your controls in the insets.
  pub fn titlebar(mut self, leading: f32, trailing: f32) -> Self {
    self.titlebar = Some((leading, trailing));
    self
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

  /// Supply each item's optional tab status-dot color (`None` = no dot),
  /// re-invoked every render. The host decides what a color means.
  pub fn on_item_dot(mut self, f: impl Fn(ItemId, &App) -> Option<gpui::Hsla> + 'static) -> Self {
    self.item_dot = Some(Rc::new(f));
    self
  }

  // queries

  pub fn focused_pane(&self) -> PaneId {
    self.focused
  }

  /// The active item of the focused pane.
  pub fn active_item(&self) -> ItemId {
    self.panes[&self.focused].active()
  }

  /// Every item across every pane, in pane-layout order.
  pub fn items(&self) -> Vec<ItemId> {
    self
      .tree
      .panes()
      .into_iter()
      .filter_map(|p| self.panes.get(&p))
      .flat_map(|p| p.items().iter().copied())
      .collect()
  }

  /// Panes in traversal order, each with its items and active item.
  pub fn panes_with_items(&self) -> Vec<(PaneId, Vec<ItemId>, ItemId)> {
    self
      .tree
      .panes()
      .into_iter()
      .filter_map(|p| {
        self
          .panes
          .get(&p)
          .map(|pane| (p, pane.items().to_vec(), pane.active()))
      })
      .collect()
  }

  pub fn pane_of(&self, item: ItemId) -> Option<PaneId> {
    self
      .panes
      .iter()
      .find(|(_, p)| p.contains(item))
      .map(|(&id, _)| id)
  }

  // mutation

  /// Add `item` as a new tab in `pane` and activate it.
  pub fn add_item(&mut self, pane: PaneId, item: ItemId, cx: &mut Context<Self>) {
    if let Some(p) = self.panes.get_mut(&pane) {
      // Re-adding an item just activates it; only a tab that is genuinely
      // new has anywhere to grow in from.
      let fresh = !p.contains(item);
      p.add(item, None);
      if fresh {
        self.motion.borrow_mut().opened(item);
      }
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
    let slot = self.panes.get(&pane).and_then(|p| p.index_of(item));
    let emptied = self
      .panes
      .get_mut(&pane)
      .map(|p| p.remove(item))
      .unwrap_or(false);
    if let Some(slot) = slot {
      self.motion.borrow_mut().closed(pane, slot, item);
    }
    if emptied {
      // Capture the collapsing pane's traversal position before the
      // tree forgets it, so focus can land on its neighbor instead of
      // jumping to the first pane in the window.
      let idx = self.tree.panes().iter().position(|&p| p == pane);
      self.panes.remove(&pane);
      // Last pane can't be removed from the tree; keep an empty group
      // valid by leaving it — but that shouldn't happen (host keeps ≥1).
      if self.tree.remove(pane) && self.focused == pane {
        let order = self.tree.panes();
        let next = idx
          .and_then(|i| order.get(i.saturating_sub(1)).or_else(|| order.last()))
          .copied()
          .unwrap_or(pane);
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
    self.end_drag(); // a drop landed inside; not a tear-off
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
          self.motion.borrow_mut().opened(item);
          self.set_focus(to_pane, cx);
        } else {
          self.reattach_somewhere(item, cx);
        }
      }
    }
    cx.notify();
  }

  /// Move `item` into `pane`'s tab strip at `index` — the gap the insertion
  /// line marked, counted in the strip exactly as the user saw it. Used on a
  /// tab-strip drop, for both reordering and moving between panes.
  pub fn move_item_at(
    &mut self,
    item: ItemId,
    to_pane: PaneId,
    index: usize,
    cx: &mut Context<Self>,
  ) {
    self.drag_over = None;
    self.tab_drop = None;
    self.dragging = None;
    let Some(from) = self.pane_of(item) else {
      cx.notify();
      return;
    };
    if from == to_pane {
      if let Some(p) = self.panes.get_mut(&to_pane) {
        let before = p.items().to_vec();
        p.move_to_gap(item, index);
        let after = p.items().to_vec();
        self.motion.borrow_mut().reordered(&before, &after);
      }
      cx.notify();
      return;
    }
    self.detach(from, item);
    if let Some(p) = self.panes.get_mut(&to_pane) {
      p.add(item, Some(index));
      self.motion.borrow_mut().opened(item);
      self.set_focus(to_pane, cx);
    } else {
      self.reattach_somewhere(item, cx);
    }
    cx.notify();
  }

  /// Reorder `item` within its pane to `index` (a tab-bar drop).
  pub fn reorder_in_pane(&mut self, item: ItemId, index: usize, cx: &mut Context<Self>) {
    self.end_drag();
    if let Some(pane) = self.pane_of(item) {
      if let Some(p) = self.panes.get_mut(&pane) {
        if let Some(from) = p.index_of(item) {
          let before = p.items().to_vec();
          p.reorder(from, index);
          let after = p.items().to_vec();
          self.motion.borrow_mut().reordered(&before, &after);
          cx.notify();
        }
      }
    }
  }

  /// Forget the in-flight drag: no drop target, nothing dragged, and a ghost
  /// that is a plain tab again next time one starts.
  fn end_drag(&mut self) {
    self.drag_over = None;
    self.tab_drop = None;
    self.dragging = None;
    self.tear_at = None;
    self.hint.set(TearHint::default());
  }

  /// Track the pointer against the group's own bounds: `at` is where it is in
  /// group coordinates once it has left, `None` while a release would still
  /// land inside.
  fn set_tear(&mut self, at: Option<Point<Pixels>>, group: Size<Pixels>, cx: &mut Context<Self>) {
    self.hint.set(TearHint {
      outside: at.is_some(),
      group,
    });
    // The overlay follows the pointer and the tab strip shows the gap the
    // item would leave, so this has to reach the group's own render too.
    if self.tear_at != at {
      self.tear_at = at;
      cx.notify();
    }
  }

  /// Remove `item` from `pane`, collapsing the pane out of the tree if empty.
  fn detach(&mut self, pane: PaneId, item: ItemId) {
    if let Some(p) = self.panes.get_mut(&pane) {
      let slot = p.index_of(item);
      if p.remove(item) {
        self.panes.remove(&pane);
        self.tree.remove(pane);
      }
      if let Some(slot) = slot {
        self.motion.borrow_mut().closed(pane, slot, item);
      }
    }
  }

  /// Last-resort: put a detached item back into any surviving pane.
  fn reattach_somewhere(&mut self, item: ItemId, cx: &mut Context<Self>) {
    if let Some(&pane) = self.tree.panes().first() {
      if let Some(p) = self.panes.get_mut(&pane) {
        p.add(item, None);
        self.motion.borrow_mut().opened(item);
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

  // navigation

  /// Focus the pane in `dir` from the focused one (via layout geometry).
  pub fn focus_direction(&mut self, dir: Direction, cx: &mut Context<Self>) {
    let layout = compute_layout(&self.tree, Rect::new(0.0, 0.0, 1000.0, 1000.0), 0.0);
    if let Some(pane) = neighbor(&layout, self.focused, dir) {
      self.set_focus(pane, cx);
    }
  }

  /// Activate the next / previous tab in the focused pane (wrapping).
  pub fn activate_next(&mut self, cx: &mut Context<Self>) {
    self.cycle_focused(true, cx);
  }

  pub fn activate_prev(&mut self, cx: &mut Context<Self>) {
    self.cycle_focused(false, cx);
  }

  fn cycle_focused(&mut self, next: bool, cx: &mut Context<Self>) {
    if let Some(p) = self.panes.get_mut(&self.focused) {
      if next {
        p.activate_next();
      } else {
        p.activate_prev();
      }
      let item = p.active();
      cx.emit(PaneGroupEvent::Activated(item));
      cx.notify();
    }
  }

  // split management

  /// Reset every divider to an even split.
  pub fn equalize(&mut self, cx: &mut Context<Self>) {
    for (split, _) in self.tree.list_dividers() {
      self.tree.set_ratio(split, 0.5);
    }
    cx.notify();
  }

  /// Nudge the divider adjacent to the focused pane in a direction by `step`.
  pub fn resize_focused(&mut self, dir: Direction, step: f32, cx: &mut Context<Self>) {
    let (axis, delta) = match dir {
      Direction::Left => (SplitDirection::Horizontal, -step),
      Direction::Right => (SplitDirection::Horizontal, step),
      Direction::Up => (SplitDirection::Vertical, -step),
      Direction::Down => (SplitDirection::Vertical, step),
    };
    if let Some(split) = self.tree.nearest_split(self.focused, axis) {
      if let Some(r) = self.tree.ratio(split) {
        self.tree.set_ratio(split, r + delta);
        cx.notify();
      }
    }
  }

  /// Toggle zoom: the focused pane fills the group.
  pub fn toggle_zoom(&mut self, cx: &mut Context<Self>) {
    self.zoomed = !self.zoomed;
    cx.notify();
  }

  pub fn is_zoomed(&self) -> bool {
    self.zoomed
  }

  // close / tear-off

  /// Ask the host to close the focused pane's active item (it drops the
  /// content, then calls [`close_item`](Self::close_item)).
  pub fn close_focused(&mut self, cx: &mut Context<Self>) {
    if let Some(p) = self.panes.get(&self.focused) {
      cx.emit(PaneGroupEvent::CloseRequested(p.active()));
    }
  }

  /// Detach `item` and emit [`PaneGroupEvent::TearOff`] so the host can move
  /// its content to a new window. The host wires the gesture (e.g. a tab
  /// dragged outside the window, or a menu item).
  pub fn tear_off(&mut self, item: ItemId, cx: &mut Context<Self>) {
    self.tear_off_at(item, None, cx);
  }

  /// [`tear_off`](Self::tear_off), reporting where the drag that asked for it
  /// released so the host can open the window there.
  pub fn tear_off_at(&mut self, item: ItemId, drop: Option<TearDrop>, cx: &mut Context<Self>) {
    self.end_drag();
    if let Some(pane) = self.pane_of(item) {
      // Don't tear off the group's last remaining item.
      if self.tree.panes().len() == 1 && self.panes.get(&pane).is_some_and(|p| p.len() == 1) {
        return;
      }
      self.detach(pane, item);
      if !self.tree.contains(self.focused) {
        if let Some(&p) = self.tree.panes().first() {
          self.focused = p;
        }
      }
    }
    cx.emit(PaneGroupEvent::TearOff { item, drop });
    cx.notify();
  }

  // persistence accessors

  /// Capture the current split/tab arrangement (see
  /// [`LayoutSnapshot`](super::LayoutSnapshot) for the encode/decode pair).
  pub fn snapshot(&self) -> super::LayoutSnapshot {
    fn walk(node: &Node, panes: &HashMap<PaneId, Pane>) -> super::LayoutSnapshot {
      match node {
        Node::Leaf(id) => {
          let pane = &panes[id];
          super::LayoutSnapshot::Pane {
            items: pane.items().iter().map(|item| item.0).collect(),
            active: pane.active_index(),
          }
        }
        Node::Split {
          axis,
          ratio,
          first,
          second,
          ..
        } => super::LayoutSnapshot::Split {
          axis: *axis,
          ratio: *ratio,
          first: Box::new(walk(first, panes)),
          second: Box::new(walk(second, panes)),
        },
      }
    }
    walk(self.tree.root(), &self.panes)
  }

  /// Replace the whole layout with a snapshot. The host must be able to
  /// render every item id the snapshot names (re-register content builders
  /// first). Fails (returning `false`, layout untouched) when the snapshot
  /// has an empty pane or duplicate item ids.
  pub fn restore(&mut self, snapshot: &super::LayoutSnapshot, cx: &mut Context<Self>) -> bool {
    let all_items = snapshot.item_ids();
    if all_items.is_empty() {
      return false;
    }
    let mut seen = std::collections::HashSet::new();
    if !all_items.iter().all(|id| seen.insert(*id)) {
      return false;
    }

    fn build(
      snap: &super::LayoutSnapshot,
      ids: &mut PaneIds,
      panes: &mut HashMap<PaneId, Pane>,
      splits: &mut u64,
    ) -> Option<Node> {
      match snap {
        super::LayoutSnapshot::Pane { items, active } => {
          let (first, rest) = items.split_first()?;
          let pane_id = ids.next();
          let mut pane = Pane::new(ItemId(*first));
          for item in rest {
            pane.add(ItemId(*item), None);
          }
          pane.activate((*active).min(items.len() - 1));
          panes.insert(pane_id, pane);
          Some(Node::Leaf(pane_id))
        }
        super::LayoutSnapshot::Split {
          axis,
          ratio,
          first,
          second,
        } => {
          *splits += 1;
          let id = SplitId(*splits);
          let first = build(first, ids, panes, splits)?;
          let second = build(second, ids, panes, splits)?;
          Some(Node::Split {
            id,
            axis: *axis,
            ratio: clamp_ratio(*ratio),
            first: Box::new(first),
            second: Box::new(second),
          })
        }
      }
    }

    let mut ids = PaneIds::new();
    let mut panes = HashMap::new();
    let mut splits = 0;
    let Some(root) = build(snapshot, &mut ids, &mut panes, &mut splits) else {
      return false;
    };
    let first_pane = {
      let mut leaves = Vec::new();
      fn collect(node: &Node, out: &mut Vec<PaneId>) {
        match node {
          Node::Leaf(id) => out.push(*id),
          Node::Split { first, second, .. } => {
            collect(first, out);
            collect(second, out);
          }
        }
      }
      collect(&root, &mut leaves);
      leaves[0]
    };

    self.tree = PaneTree::from_parts(root, splits);
    self.panes = panes;
    self.ids = ids;
    self.focused = first_pane;
    self.zoomed = false;
    self.drag_over = None;
    self.tab_drop = None;
    self.dragging = None;
    self.strips.borrow_mut().clear();
    *self.motion.borrow_mut() = TabMotion::default();
    self.hint.set(TearHint::default());
    self.tear_at = None;
    self.grab.set(Point::default());
    cx.notify();
    true
  }

  /// The split tree (for serializing the layout).
  pub fn tree(&self) -> &PaneTree {
    &self.tree
  }

  /// The items of a pane, in tab order.
  pub fn pane_items(&self, pane: PaneId) -> Option<&[ItemId]> {
    self.panes.get(&pane).map(|p| p.items())
  }
}

/// The leaf at the layout's top-left corner (descend `first` always).
fn top_left(node: &Node) -> PaneId {
  match node {
    Node::Leaf(p) => *p,
    Node::Split { first, .. } => top_left(first),
  }
}

/// Every leaf whose tab bar sits on the layout's top edge. A horizontal split
/// puts both of its children up there; a vertical split only its first. These
/// are the tab bars that are visually part of the window titlebar, so they all
/// have to drag the window — not just the corner one.
fn top_edge(node: &Node, out: &mut Vec<PaneId>) {
  match node {
    Node::Leaf(p) => out.push(*p),
    Node::Split {
      axis: SplitDirection::Horizontal,
      first,
      second,
      ..
    } => {
      top_edge(first, out);
      top_edge(second, out);
    }
    Node::Split { first, .. } => top_edge(first, out),
  }
}

/// The leaf at the layout's top-right corner (right child of horizontal splits,
/// top child of vertical splits).
fn top_right(node: &Node) -> PaneId {
  match node {
    Node::Leaf(p) => *p,
    Node::Split {
      axis: SplitDirection::Horizontal,
      second,
      ..
    } => top_right(second),
    Node::Split { first, .. } => top_right(first),
  }
}

/// A tab on its way out of the strip: the shape it had, clipped to a width
/// closing on zero. It carries its own label because the item behind it is
/// already gone, and no handlers because there is nothing left to act on.
fn collapsing(g: &Ghosting, text: Hsla, tab_h: f32) -> AnyElement {
  div()
    .flex_none()
    .h(px(tab_h))
    .w(px(g.now))
    .overflow_hidden()
    .text_color(text)
    .child(
      // Held at the width it had, so the label stays put and the tab
      // reads as sliding under its neighbour instead of reflowing.
      div()
        .flex()
        .flex_none()
        .items_center()
        .px_2()
        .h(px(tab_h))
        .w(px(g.was))
        .child(
          div()
            .flex_1()
            .min_w_0()
            .text_size(px(12.0))
            .whitespace_nowrap()
            .text_ellipsis()
            .child(g.title.clone()),
        ),
    )
    .into_any_element()
}

/// A tab growing into the strip. It takes the room an ordinary tab of its width
/// would, scaled by how far in it is — floor and ceiling included, so the frame
/// the animation stops on is laid out under exactly the constraints the settled
/// tab meets and there is nothing left to snap.
///
/// Inside, the tab sits on a stage wider than any tab. Laid out against the
/// width it is animating towards it would only ever measure that width back,
/// and the target has to keep up with a title that changes mid-animation — as a
/// freshly spawned shell's does.
fn growing(
  tab: impl Styled + IntoElement,
  natural: f32,
  grown: f32,
  crowded: bool,
  tab_h: f32,
  measured: impl Fn(f32) + 'static,
) -> AnyElement {
  div()
    .flex_shrink_1()
    .h(px(tab_h))
    .w(px(natural * grown))
    .max_w(px(MAX_TAB_W * grown))
    .when(crowded, |d| d.min_w(px(MIN_TAB_W * grown)))
    .overflow_hidden()
    .child(
      div()
        .flex()
        .flex_row()
        .flex_none()
        .w(px(STAGE))
        .h(px(tab_h))
        .on_children_prepainted(move |bounds, _w, _cx| {
          if let Some(b) = bounds.first() {
            measured(f32::from(b.size.width));
          }
        })
        .child(tab.flex_none()),
    )
    .into_any_element()
}

impl gpui::Focusable for PaneGroup {
  fn focus_handle(&self, _cx: &App) -> FocusHandle {
    self.focus.clone()
  }
}

impl Render for PaneGroup {
  fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    // A collapsed pane leaves its strip behind, and pane ids are never
    // reused, so nothing would ever come back to claim it.
    let live = &self.panes;
    self
      .strips
      .borrow_mut()
      .retain(|pane, _| live.contains_key(pane));

    // Retire finished tab motion before the strips are built, and ask for
    // the next frame only while something is still moving. This is what
    // keeps a busy terminal's repaints from driving the animation: they
    // land here, find nothing in flight, and render the settled strip.
    let items = self.items();
    if self.motion.borrow_mut().settle(&items) {
      window.request_animation_frame();
    }

    let root = self.tree.root().clone();
    let render_item = self.render_item.clone();
    let item_title = self.item_title.clone();
    let item_dot = self.item_dot.clone();
    // Zoomed: only the focused pane, filling the group.
    let inner = if self.zoomed {
      self.pane_el(
        self.focused,
        &render_item,
        &item_title,
        &item_dot,
        window,
        cx,
      )
    } else {
      self.node_el(&root, &render_item, &item_title, &item_dot, window, cx)
    };
    let tear = self
      .tear_at
      .map(|at| tear_overlay(at, self.hint.get().group));
    div()
      .size_full()
      .relative()
      .track_focus(&self.focus)
      // Only the whole group's bounds can say whether a release would
      // still land inside it, and both the ghost and the overlay have to
      // know that before the release rather than after.
      .on_drag_move::<TabDrag>(
        cx.listener(|this, ev: &DragMoveEvent<TabDrag>, _window, cx| {
          this.dragging = Some(ev.drag(cx).item);
          let at =
            (!ev.bounds.contains(&ev.event.position)).then(|| ev.event.position - ev.bounds.origin);
          this.set_tear(at, ev.bounds.size, cx);
        }),
      )
      // A tab released outside the group (past the panes / the window) is
      // torn off into a new window (the host handles `TearOff`).
      .on_mouse_up_out(
        MouseButton::Left,
        cx.listener(|this, ev: &MouseUpEvent, _window, cx| {
          // A drag released over the group's own dead space (the `+`,
          // the titlebar filler) leaves `dragging` set with nothing to
          // clear it; requiring the crossing keeps that stale item
          // from being torn off by the next unrelated drag.
          let dragged = this.dragging.filter(|_| this.tear_at.is_some());
          let grab = this.grab.get();
          this.end_drag();
          cx.notify();
          if let Some(item) = dragged {
            let drop = TearDrop {
              position: ev.position,
              grab,
            };
            this.tear_off_at(item, Some(drop), cx);
          }
        }),
      )
      .child(inner)
      .children(tear)
      .probe("PaneGroup")
  }
}

impl PaneGroup {
  /// Render a tree node: a leaf pane, or a split (two children + divider).
  fn node_el(
    &self,
    node: &Node,
    render_item: &Option<RenderItem>,
    item_title: &Option<ItemTitle>,
    item_dot: &Option<ItemDot>,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) -> AnyElement {
    match node {
      Node::Leaf(pane) => self.pane_el(*pane, render_item, item_title, item_dot, window, cx),
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
        let f = self.node_el(first, render_item, item_title, item_dot, window, cx);
        let s = self.node_el(second, render_item, item_title, item_dot, window, cx);

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

        // A divider is 6px of otherwise unmarked background, so the
        // handle is what says it can be dragged at all. It rides the
        // hover group rather than the divider's own `hover` because it
        // is a child: styling the divider cannot reveal it.
        let handle = SharedString::from(format!("pg-divider-{}", split.0));
        let pill = div()
          .absolute()
          .rounded_full()
          .bg(theme(cx).primary().hsla())
          .invisible()
          .group_hover(handle.clone(), |s| s.visible());
        let mut divider = div()
          .id(("pg-divider", split.0 as usize))
          .group(handle)
          .relative()
          .flex_none()
          .flex()
          .items_center()
          .justify_center()
          .hover(move |st| st.bg(grip))
          .on_drag(DividerDrag { group, split }, |_, _off, _w, cx| {
            cx.new(|_| Empty)
          });
        divider = if horizontal {
          divider
            .w(px(6.0))
            .h_full()
            .cursor_col_resize()
            .child(div().w(px(1.0)).h_full().bg(line))
            .child(pill.w(px(3.0)).h(px(28.0)))
        } else {
          divider
            .h(px(6.0))
            .w_full()
            .cursor_row_resize()
            .child(div().h(px(1.0)).w_full().bg(line))
            .child(pill.h(px(3.0)).w(px(28.0)))
        };

        let mut container = div().size_full().flex().on_drag_move(cx.listener(
          move |this, ev: &DragMoveEvent<DividerDrag>, _w, cx| {
            let d = ev.drag(cx);
            if d.group != group || d.split != split {
              return;
            }
            let b = ev.bounds;
            let (pos, extent) = if horizontal {
              (
                f32::from(ev.event.position.x - b.left()),
                f32::from(b.size.width),
              )
            } else {
              (
                f32::from(ev.event.position.y - b.top()),
                f32::from(b.size.height),
              )
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
    item_dot: &Option<ItemDot>,
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
    let dimmed = t.dimmed().hsla();
    let tab_h = self.tab_height;

    let active = p.active();
    let group = cx.entity().entity_id();
    let count = p.len();
    let items = p.items();

    // Tabs on their way out keep the slot they held, shrinking, so the
    // strip closes the gap where the tab was instead of the neighbours
    // jumping into it. That makes them children of the scroller exactly as
    // tabs are, which is the order every per-child measurement below counts
    // in — ghosts included.
    let ghosts = self.motion.borrow().ghosts(pane);
    let ghost_slots: Vec<usize> = ghosts.iter().map(|g| g.slot).collect();
    let plan = slots(count, &ghost_slots);
    let tab_child: Vec<usize> = plan
      .iter()
      .enumerate()
      .filter_map(|(child, slot)| matches!(slot, Slot::Tab(_)).then_some(child))
      .collect();
    let settled: Vec<Option<ItemId>> = plan
      .iter()
      .map(|slot| match *slot {
        Slot::Tab(i) => {
          let item = items[i];
          self.motion.borrow().opening(item).is_none().then_some(item)
        }
        Slot::Ghost(_) => None,
      })
      .collect();

    // Reading the strip's extents here reads what prepaint left behind last
    // frame, which on the strip's very first render is nothing. Asking for
    // another frame whenever the measurement moves is what makes the arrows
    // show up at all. The arrows only ever appear on a strip that is already
    // overflowing, so the width they take cannot tip it back to fitting and
    // set them blinking.
    let (scroll, remeasure, counted) = {
      let mut strips = self.strips.borrow_mut();
      let strip = strips.entry(pane).or_insert_with(TabStrip::new);
      // A width only becomes real in layout, and a tab that is closing
      // needs one after its item has gone — so each frame reads the last
      // one's tabs back off the scroll handle and holds those widths for
      // whichever frame has to collapse one.
      {
        let mut motion = self.motion.borrow_mut();
        for (child, was) in strip.settled.iter().enumerate() {
          let (Some(&item), Some(b)) = (was.as_ref(), strip.scroll.bounds_for_item(child)) else {
            continue;
          };
          motion.laid_out(item, f32::from(b.size.width));
        }
      }
      strip.settled = settled;
      let now = f32::from(strip.scroll.max_offset().x);
      let moved = strip.seen.is_none_or(|seen| (seen - now).abs() > 0.5);
      strip.seen = Some(now);
      // Scrolling into view is the one thing that counts in *this* frame:
      // gpui applies it in prepaint, after the children it acts on have
      // been laid out again.
      if strip.followed != Some(active) {
        strip.followed = Some(active);
        if let Some(&child) = p.index_of(active).and_then(|i| tab_child.get(i)) {
          strip.scroll.scroll_to_item(child);
        }
      }
      let counted = std::mem::replace(&mut strip.tabs, tab_child);
      (strip.scroll.clone(), moved, counted)
    };
    if remeasure {
      window.request_animation_frame();
    }
    let offset = f32::from(scroll.offset().x);
    let max_offset = f32::from(scroll.max_offset().x);
    let (before, after) = hidden_tabs(&scroll, &counted);

    // The gap the insertion line marks, if the drag is over this strip.
    let gap = match self.tab_drop {
      Some((over, at)) if over == pane => Some(at),
      _ => None,
    };
    let tearing = self.tear_at.is_some();
    let mut children: Vec<AnyElement> = Vec::with_capacity(plan.len());
    for slot in &plan {
      let i = match *slot {
        Slot::Ghost(g) => {
          children.push(collapsing(&ghosts[g], text, tab_h));
          continue;
        }
        Slot::Tab(i) => i,
      };
      let item = items[i];
      let title = item_title
        .as_ref()
        .map(|f| f(item, cx))
        .unwrap_or_else(|| SharedString::from("untitled"));
      self.motion.borrow_mut().label(item, &title);
      let dot = item_dot.as_ref().and_then(|f| f(item, cx));
      let is_active = item == active;
      // The item is on its way out of the window, so its slot fades to
      // the gap it would leave rather than claiming it is still here.
      let leaving = tearing && self.dragging == Some(item);
      let shift = self.motion.borrow().slide(item);
      // Built once and shared with the close button below. The strip
      // re-renders with every frame of terminal output, so a second
      // `format!` here is a heap allocation per tab per frame for a
      // string that never changes.
      let hover_group = tab_group(pane, i);
      let tab = div()
        .id(("pg-tab", (pane.0 as usize) << 20 | i))
        .group(hover_group.clone())
        .relative()
        .flex()
        .items_center()
        .gap_1()
        .px_2()
        .h(px(tab_h))
        // Tabs size to their title up to MAX_TAB_W and shrink together as
        // more open, with the title ellipsizing rather than overflowing.
        // They deliberately do not *grow* to fill: stretching two tabs
        // across a wide pane reads as a segmented control, not as tabs.
        //
        // The floor only binds once a pane has a second tab. A lone tab
        // has nothing to be told apart from, so holding it open to the
        // floor would pad a three-letter title out over dead space.
        .flex_shrink_1()
        .max_w(px(MAX_TAB_W))
        .when(count > 1, |d| d.min_w(px(MIN_TAB_W)))
        .when(is_active && !leaving, |d| d.bg(active_bg))
        .when(leaving, |d| d.opacity(0.3))
        // A reordered tab is drawn back at the slot it left and closes
        // the distance, rather than appearing in its new place.
        .when_some(shift, |d, shift| d.left(px(shift)))
        .text_color(text)
        .hover(|s| s.bg(active_bg))
        .on_click(cx.listener(move |this, _ev, _w, cx| this.activate(pane, item, cx)))
        .on_mouse_down(
          MouseButton::Right,
          cx.listener(move |this, ev: &gpui::MouseDownEvent, _w, cx| {
            // Activate first: a menu acting on a tab the user did
            // not just select would be a trap.
            this.activate(pane, item, cx);
            cx.emit(PaneGroupEvent::ContextMenu {
              item,
              position: ev.position,
            });
          }),
        )
        // Drag this tab; the drop itself is handled by the tab bar and
        // the pane body below, which own the whole target area.
        .on_drag(
          TabDrag {
            group,
            item,
            from_pane: pane,
            label: title.clone(),
          },
          {
            let grab = self.grab.clone();
            let hint = self.hint.clone();
            move |d: &TabDrag, off, _w: &mut Window, cx: &mut App| {
              // Where inside the tab the drag began, so a host
              // placing what the release opens can put it exactly
              // where the ghost was.
              grab.set(off);
              let ghost = TabGhost {
                label: d.label.clone(),
                dot,
                height: tab_h,
                hint: hint.clone(),
              };
              cx.new(|_| ghost)
            }
          },
        )
        // Runs after the bar's own handler (capture order follows paint
        // order), refining its "append" into the nearer side of this
        // tab: past the middle the drop belongs after it, which is what
        // dragging a tab one place along asks for.
        .on_drag_move::<TabDrag>(
          cx.listener(move |this, ev: &DragMoveEvent<TabDrag>, _w, cx| {
            if !ev.bounds.contains(&ev.event.position) {
              return;
            }
            let past_middle = ev.event.position.x - ev.bounds.left() > ev.bounds.size.width / 2.0;
            let at = i + usize::from(past_middle);
            if this.tab_drop != Some((pane, at)) {
              this.tab_drop = Some((pane, at));
              cx.notify();
            }
          }),
        )
        .when_some(dot, |d, color| {
          d.child(
            div()
              .flex_none()
              .w(px(6.0))
              .h(px(6.0))
              .rounded_full()
              .bg(color),
          )
        })
        // The title takes the slack and gives it back first: it is the
        // only part of a tab that can afford to be cut.
        .child(
          div()
            .flex_1()
            .min_w_0()
            .text_size(px(12.0))
            .whitespace_nowrap()
            .text_ellipsis()
            .child(title),
        )
        .child(
          div()
            .id(("pg-tabclose", (pane.0 as usize) << 20 | i))
            .flex_none()
            .w(px(14.0))
            .h(px(14.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(3.0))
            .text_size(px(12.0))
            .text_color(text)
            // Only the active tab and whichever one you are pointing
            // at carry a close button. A row of × glyphs is noise,
            // and hiding them buys back the width the titles want.
            // A pane's last tab is no exception: closing it is a
            // normal thing to want, and the host decides what that
            // means (Sinclair closes the window).
            .when(!is_active, |d| d.invisible())
            .group_hover(hover_group, |s| s.visible())
            .hover(|s| s.bg(active_bg))
            .child("\u{00d7}")
            .on_click(cx.listener(move |_this, _ev, _w, cx| {
              cx.emit(PaneGroupEvent::CloseRequested(item));
            })),
        )
        // Last, so the line sits over the tab it marks the edge of.
        .when(gap == Some(i), |d| d.child(insert_line(false)))
        .when(gap == Some(count) && i + 1 == count, |d| {
          d.child(insert_line(true))
        });

      let (opening, natural) = {
        let motion = self.motion.borrow();
        (motion.opening(item), motion.width(item))
      };
      children.push(match opening {
        Some(grown) => {
          let motion = self.motion.clone();
          growing(tab, natural, grown, count > 1, tab_h, move |w| {
            motion.borrow_mut().laid_out(item, w);
          })
        }
        None => tab.into_any_element(),
      });
    }

    // Tabs scroll inside their own strip, which is what lets the floor above
    // hold: past it the width has to come out of the strip's ends instead of
    // out of every tab. Both arrows sit in the flow rather than over the
    // tabs, so neither one covers a tab you were about to click.
    let strip = div()
      .id(("pg-tabstrip", pane.0 as usize))
      .track_scroll(&scroll)
      .flex()
      .flex_row()
      .items_center()
      .flex_shrink_1()
      .min_w_0()
      .h(px(tab_h))
      .overflow_x_scroll()
      .children(children);
    let arrow = (text, active_bg, border);
    let lead_arrow = (offset < -0.5).then(|| {
      let scroll = scroll.clone();
      overflow_arrow(
        ("pg-taboverflow-lead", pane.0 as usize),
        arrow_label(before, true),
        true,
        tab_h,
        arrow,
        cx.listener(move |_this, _ev, _w, cx| {
          page_strip(&scroll, 1.0);
          cx.notify();
        }),
      )
    });
    let trail_arrow = (offset > 0.5 - max_offset).then(|| {
      let scroll = scroll.clone();
      overflow_arrow(
        ("pg-taboverflow-trail", pane.0 as usize),
        arrow_label(after, false),
        false,
        tab_h,
        arrow,
        cx.listener(move |_this, _ev, _w, cx| {
          page_strip(&scroll, -1.0);
          cx.notify();
        }),
      )
    });

    // Titlebar integration: the top-row tab bars reserve space for the
    // host's window controls, and the top-right filler drags the window.
    let is_top_left = self.titlebar.is_some() && top_left(self.tree.root()) == pane;
    let is_top_right = self.titlebar.is_some() && top_right(self.tree.root()) == pane;
    let is_top_edge = self.titlebar.is_some() && {
      let mut edge = Vec::new();
      top_edge(self.tree.root(), &mut edge);
      edge.contains(&pane)
    };
    let (leading, trailing) = self.titlebar.unwrap_or((0.0, 0.0));

    let tab_bar = div()
      .flex()
      .flex_row()
      .items_center()
      .w_full()
      .h(px(tab_h))
      .bg(surface)
      .border_b_1()
      .border_color(border)
      // gpui hands every drag move to every listener, so a strip claims
      // the insertion line only while the cursor is inside it and only
      // ever clears its own. Anywhere past the tabs — the `+`, the
      // window-drag filler — means append; a tab under the cursor
      // narrows that down afterwards.
      .on_drag_move::<TabDrag>(
        cx.listener(move |this, ev: &DragMoveEvent<TabDrag>, _w, cx| {
          let next = if ev.bounds.contains(&ev.event.position) {
            Some((pane, count))
          } else if matches!(this.tab_drop, Some((over, _)) if over == pane) {
            None
          } else {
            return;
          };
          if this.tab_drop != next {
            this.tab_drop = next;
            cx.notify();
          }
        }),
      )
      .on_drop(cx.listener(move |this, d: &TabDrag, _w, cx| {
        let at = match this.tab_drop {
          Some((over, at)) if over == pane => at,
          _ => count,
        };
        this.move_item_at(d.item, pane, at, cx);
      }))
      .when(is_top_right, |d| d.pr(px(trailing)))
      // The space reserved for the platform's window controls is part of
      // the titlebar, so it drags too rather than being dead padding.
      .when(is_top_left, |d| {
        d.child(
          div()
            .id(("pg-titleinset", pane.0 as usize))
            .flex_none()
            .w(px(leading))
            .h_full()
            .window_control_area(WindowControlArea::Drag)
            .on_mouse_down(MouseButton::Left, |_, window, _| window.start_window_move()),
        )
      })
      .when_some(lead_arrow, |d, a| d.child(a))
      .child(strip)
      .when_some(trail_arrow, |d, a| d.child(a))
      .child(
        div()
          .id(("pg-newtab", pane.0 as usize))
          // The one control on the bar that must never be squeezed
          // out: it is how you get back to a pane with room in it.
          .flex_none()
          .px_2()
          .h(px(tab_h))
          .flex()
          .items_center()
          .text_color(text)
          .hover(|s| s.bg(active_bg))
          .child("+")
          .on_click(cx.listener(move |_this, _ev, _w, cx| {
            cx.emit(PaneGroupEvent::NewRequested(pane));
          })),
      )
      // A flexible gap separates the tabs from the controls that follow, so
      // those sit against the right edge however few tabs there are. On a
      // top-row bar it doubles as the window-drag region (double-click
      // zooms, per the platform titlebar convention).
      .child(if is_top_edge {
        div()
          .id(("pg-titledrag", pane.0 as usize))
          .flex_1()
          .h_full()
          .window_control_area(WindowControlArea::Drag)
          .on_mouse_down(MouseButton::Left, |_, window, _| window.start_window_move())
          .into_any_element()
      } else {
        div().flex_1().h_full().into_any_element()
      })
      // Splitting was reachable only from a keybinding or a menu, which is
      // why most windows only ever have one pane. The controls are drawn
      // dimmed so a row of panes does not turn into a row of buttons, and a
      // host that puts its own splits somewhere else turns them off.
      .when(self.split_controls, |bar| {
        bar
          .child(split_control(
            ("pg-splitright", pane.0 as usize),
            SplitDirection::Horizontal,
            tab_h,
            (dimmed, active_bg),
            cx.listener(move |_this, _ev, _w, cx| {
              cx.emit(PaneGroupEvent::SplitRequested {
                pane,
                axis: SplitDirection::Horizontal,
                first: false,
              });
            }),
          ))
          .child(split_control(
            ("pg-splitdown", pane.0 as usize),
            SplitDirection::Vertical,
            tab_h,
            (dimmed, active_bg),
            cx.listener(move |_this, _ev, _w, cx| {
              cx.emit(PaneGroupEvent::SplitRequested {
                pane,
                axis: SplitDirection::Vertical,
                first: false,
              });
            }),
          ))
      });

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
      .on_drag_move::<TabDrag>(
        cx.listener(move |this, ev: &DragMoveEvent<TabDrag>, _w, cx| {
          // Every pane hears every move, not just the one under the
          // pointer, so a pane it has left has to give the target up
          // instead of claiming a center drop from wherever it is.
          // The root handler owns `dragging`, since only the group's
          // own bounds can tell a move from a tear.
          let next = if ev.bounds.contains(&ev.event.position) {
            Some((pane, drop_edge(ev.bounds, ev.event.position)))
          } else if matches!(this.drag_over, Some((p, _)) if p == pane) {
            None
          } else {
            this.drag_over
          };
          if this.drag_over != next {
            this.drag_over = next;
            cx.notify();
          }
        }),
      )
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
