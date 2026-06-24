//! Feedback: components that communicate state to the user.
//!
//! - [`Alert`] — an inline colored callout.
//! - [`Loader`] — an animated busy indicator.
//! - [`Progress`] — a horizontal completion bar.
//! - [`Notification`] — an elevated toast card.

mod alert;
mod loader;
mod notification;
mod progress;

pub use alert::Alert;
pub use loader::{Loader, LoaderVariant};
pub use notification::Notification;
pub use progress::Progress;
