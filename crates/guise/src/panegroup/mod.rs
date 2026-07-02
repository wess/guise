//! `PaneGroup` — a recursive tree of **tabbed** panes (splits contain tabs),
//! the Zed / VS Code editor-group workspace model as a reusable gpui component.
//!
//! The window is one [`PaneTree`] of splits whose leaves are [`Pane`]s, each a
//! tabbed container of opaque [`ItemId`]s. The host owns the items (their real
//! content entities) and supplies, per item, a content element and a title; the
//! component owns the layout, the per-pane tab bars, dividers, and drag/drop
//! (reorder, move-between-panes, drop-to-split), emitting events for
//! activation, close, new, and tear-off. Window creation for tear-off stays a
//! host concern.
//!
//! This module currently provides the pure, testable model; the gpui component
//! (`PaneGroup` entity + render + drag) builds on top of it.

mod drag;
mod group;
mod id;
mod layout;
mod nav;
mod pane;
mod tree;

pub use drag::{DropEdge, TabDrag};
pub use id::{ItemId, ItemIds, PaneId, PaneIds, SplitId};
pub use layout::{compute_layout, Layout, Rect};
pub use nav::{neighbor, next, prev, Direction};
pub use group::{PaneGroup, PaneGroupEvent};
pub use pane::Pane;
pub use tree::{clamp_ratio, Node, PaneTree, MAX_RATIO, MIN_RATIO};
