//! Navigation: components for moving around an app, plus a [`StatusBar`] shell.
//!
//! - [`Breadcrumbs`], [`NavLink`], [`Stepper`], [`StatusBar`] are stateless
//!   `RenderOnce` builders.
//! - [`Pagination`] is a stateful entity that owns the current page.

mod breadcrumbs;
mod navigationmenu;
mod navlink;
mod pagination;
mod statusbar;
mod stepper;

pub use breadcrumbs::Breadcrumbs;
pub use navigationmenu::{NavigationMenu, NavigationMenuEvent};
pub use navlink::NavLink;
pub use pagination::{Pagination, PaginationEvent};
pub use statusbar::StatusBar;
pub use stepper::Stepper;
