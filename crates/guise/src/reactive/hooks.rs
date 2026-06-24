//! Hook-style helpers built on [`Signal`] and gpui's observation.

use gpui::{App, Context};

use super::signal::Signal;

/// Create a piece of local state — React's `useState`. Call it in a view's
/// constructor and store the returned [`Signal`].
pub fn use_state<T: 'static>(cx: &mut App, initial: T) -> Signal<T> {
    Signal::new(cx, initial)
}

/// Re-render the calling view whenever `signal` changes — the wiring behind a
/// component "subscribing" to state. Call once per signal in the view's
/// constructor (where `cx` is the view's `Context`).
pub fn watch<V: 'static, T: 'static>(cx: &mut Context<V>, signal: &Signal<T>) {
    cx.observe(signal.entity(), |_view, _observed, cx| cx.notify())
        .detach();
}
