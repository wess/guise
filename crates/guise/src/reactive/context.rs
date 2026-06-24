//! Context/Provider: share a value by type through gpui globals.
//!
//! Like React Context, but keyed by the value's Rust type (the gpui global
//! idiom), so there is one provided value per type. To share several values of
//! the same shape, wrap each in a distinct newtype.

use gpui::{App, Global};

/// Wrapper that lets any `'static` type be stored as a gpui global.
struct ContextCell<T>(T);

impl<T: 'static> Global for ContextCell<T> {}

/// Provide `value` to the whole app (React's `<Context.Provider>`). A later
/// call with the same type replaces the previous value.
pub fn provide<T: 'static>(cx: &mut App, value: T) {
    cx.set_global(ContextCell(value));
}

/// Read a provided value by type, cloned (React's `useContext`). Returns
/// `None` if nothing of type `T` was provided.
pub fn use_context<T: 'static + Clone>(cx: &App) -> Option<T> {
    cx.try_global::<ContextCell<T>>().map(|cell| cell.0.clone())
}

/// Borrow a provided value by type, without cloning.
pub fn use_context_ref<T: 'static>(cx: &App) -> Option<&T> {
    cx.try_global::<ContextCell<T>>().map(|cell| &cell.0)
}

/// Whether a value of type `T` has been provided.
pub fn has_context<T: 'static>(cx: &App) -> bool {
    cx.has_global::<ContextCell<T>>()
}
