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

/// Derived state — React's `useMemo`. Returns a signal that recomputes from
/// `source` whenever it changes; watch the returned signal like any other.
pub fn use_memo<T: 'static, U: 'static>(
    cx: &mut App,
    source: &Signal<T>,
    f: impl Fn(&T) -> U + 'static,
) -> Signal<U> {
    let derived = Signal::new(cx, f(source.read(cx)));
    let out = derived.clone();
    cx.observe(source.entity(), move |observed, cx| {
        let next = f(observed.read(cx));
        out.set(cx, next);
    })
    .detach();
    derived
}

/// Run a side effect with the current value whenever `source` changes —
/// React's `useEffect` with one dependency.
///
/// The value is cloned out of the signal before `f` runs, so the effect may
/// freely read or write any signal — including `source` itself. Guard writes
/// to `source` with a condition, or the effect re-triggers itself forever.
pub fn use_effect<T: Clone + 'static>(
    cx: &mut App,
    source: &Signal<T>,
    f: impl Fn(&T, &mut App) + 'static,
) {
    cx.observe(source.entity(), move |observed, cx| {
        let value = observed.read(cx).clone();
        f(&value, cx);
    })
    .detach();
}
