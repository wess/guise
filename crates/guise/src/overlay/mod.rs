//! Overlays: floating UI that paints above the page.
//!
//! - [`Modal`] is a controlled `RenderOnce` overlay — render it (as a child of
//!   a full-size root) only while `opened`, and pass an `on_close` handler.
//! - [`Tooltip`] is a small view plus the [`tooltip`] helper for gpui's
//!   built-in `.tooltip(...)` attachment.
//! - [`Menu`] is a stateful entity: a trigger plus a deferred action list.
//! - [`MenuBar`] is a stateful entity: a row of dropdown menus for an app menu.
//! - [`ContextMenu`] is a stateful entity: a right-click menu at the pointer.
//! - [`HoverCard`] is a stateful entity: a `Popover` that opens on hover.
//! - [`LoadingOverlay`] is a stateless dimming layer for a `.relative()` parent.
//! - [`ConfirmModal`] is a controlled confirm/cancel dialog built on `Modal`.
//! - [`OverlayHost`] is a stateful entity owning the window's modal stack and
//!   toast queue: open dialogs from any handler, focus restores on close.

mod confirm;
mod contextmenu;
mod host;
mod drawer;
mod hovercard;
mod loading;
mod menu;
mod menubar;
mod modal;
mod popover;
mod spotlight;
mod tooltip;
mod tour;

pub use confirm::ConfirmModal;
pub use contextmenu::ContextMenu;
pub use drawer::{Drawer, Side};
pub use host::{ModalCloser, OverlayHost};
pub use hovercard::HoverCard;
pub use loading::LoadingOverlay;
pub use menu::Menu;
pub use menubar::{MenuBar, MenuColumn};
pub use modal::Modal;
pub use popover::{Placement, Popover};
pub use spotlight::Spotlight;
pub use tooltip::{tooltip, Tooltip};
pub use tour::{Tour, TourEvent};
