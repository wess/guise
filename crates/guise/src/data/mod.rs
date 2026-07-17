//! Data display: components that present structured content.
//!
//! - [`Avatar`], [`List`], [`Table`] are stateless `RenderOnce` builders.
//! - [`Tabs`] and [`Accordion`] are gpui entities that own selection /
//!   expansion state. Their panel content is supplied as a builder closure so
//!   it can be rebuilt each frame.

mod accordion;
mod avatar;
mod avatargroup;
mod dataview;
mod list;
mod tabbar;
mod table;
mod tableview;
mod tabs;
mod timeline;
mod tree;
mod virtuallist;

pub use accordion::Accordion;
pub use avatar::Avatar;
pub use avatargroup::AvatarGroup;
pub use dataview::{DataView, DataViewEvent, DataViewLayout};
pub use list::List;
pub use tabbar::{TabBar, TabBarEvent};
pub use table::Table;
pub use tableview::{Column, SelectionMode, SortDir, TableView, TableViewEvent};
pub use tabs::Tabs;
pub use timeline::Timeline;
pub use tree::{TreeNode, TreeView, TreeViewEvent};
pub use virtuallist::VirtualList;

use gpui::{AnyElement, App, Window};

/// A panel-content builder: re-invoked each render so stateful panels (Tabs,
/// Accordion) can show arbitrary, always-fresh content.
pub(crate) type Content = Box<dyn Fn(&mut Window, &mut App) -> AnyElement + 'static>;
