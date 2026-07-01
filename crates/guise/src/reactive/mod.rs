//! A lightweight, React-flavored state layer over gpui.
//!
//! gpui already has reactive entities and globals; this module wraps them in a
//! small, familiar API:
//!
//! - [`Signal`] — an observable state cell (React's `useState`). Clone it
//!   freely; all clones point at the same value and notify on change.
//! - [`Binding`] — a two-way connection to a value (SwiftUI's `Binding`).
//!   Get one from [`Signal::binding`] or [`Signal::lens`] and hand it to a
//!   component's `.bind(...)`.
//! - [`provide`] / [`use_context`] — the Context/Provider pattern, backed by
//!   gpui globals (one value per type, the gpui idiom).
//! - [`use_state`] / [`watch`] / [`use_memo`] / [`use_effect`] — hook-style
//!   helpers. `watch` re-renders the calling view whenever a signal changes.
//!
//! The canonical pattern — shared state via context:
//!
//! ```ignore
//! // At the root: create state and provide it.
//! let count = reactive::use_state(cx, 0i32);
//! reactive::provide(cx, count.clone());
//!
//! // In a descendant view's constructor: read it and subscribe.
//! let count = reactive::use_context::<Signal<i32>>(cx).unwrap();
//! reactive::watch(cx, &count);
//!
//! // Anywhere with access to the app/context:
//! count.update(cx, |n| *n += 1); // notifies every watcher
//! ```

mod binding;
mod context;
mod form;
mod hooks;
mod signal;

pub use binding::Binding;
pub use context::{has_context, provide, use_context, use_context_ref};
pub use form::{use_form, validators, FormState, Validator};
pub use hooks::{use_effect, use_memo, use_state, watch};
pub use signal::Signal;
