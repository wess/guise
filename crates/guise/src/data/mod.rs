//! Data display: components that present structured content.
//!
//! - [`Avatar`], [`List`], [`Table`] are stateless `RenderOnce` builders.
//! - [`Tabs`] and [`Accordion`] are gpui entities that own selection /
//!   expansion state. Their panel content is supplied as a builder closure so
//!   it can be rebuilt each frame.

mod accordion;
mod avatar;
mod avatargroup;
mod list;
mod table;
mod tabs;
mod timeline;

pub use accordion::Accordion;
pub use avatar::Avatar;
pub use avatargroup::AvatarGroup;
pub use list::List;
pub use table::Table;
pub use tabs::Tabs;
pub use timeline::Timeline;

use gpui::{AnyElement, App, Window};

/// A panel-content builder: re-invoked each render so stateful panels (Tabs,
/// Accordion) can show arbitrary, always-fresh content.
pub(crate) type Content = Box<dyn Fn(&mut Window, &mut App) -> AnyElement + 'static>;
