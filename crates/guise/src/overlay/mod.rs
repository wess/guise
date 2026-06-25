//! Overlays: floating UI that paints above the page.
//!
//! - [`Modal`] is a controlled `RenderOnce` overlay — render it (as a child of
//!   a full-size root) only while `opened`, and pass an `on_close` handler.
//! - [`Tooltip`] is a small view plus the [`tooltip`] helper for gpui's
//!   built-in `.tooltip(...)` attachment.
//! - [`Menu`] is a stateful entity: a trigger plus a deferred action list.

mod drawer;
mod menu;
mod modal;
mod popover;
mod spotlight;
mod tooltip;

pub use drawer::{Drawer, Side};
pub use menu::Menu;
pub use modal::Modal;
pub use popover::{Placement, Popover};
pub use spotlight::Spotlight;
pub use tooltip::{tooltip, Tooltip};
