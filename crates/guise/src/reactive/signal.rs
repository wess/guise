//! `Signal` — an observable state cell backed by a gpui entity.

use gpui::{App, AppContext, Entity};

/// A clonable handle to a piece of reactive state. React's `useState` value:
/// reading is cheap, writing notifies every observer (see [`super::watch`]).
///
/// All clones share one backing entity, so passing a `Signal` around — or
/// providing it as context — gives every holder the same live value.
pub struct Signal<T> {
    entity: Entity<T>,
}

impl<T: 'static> Signal<T> {
    /// Create a new signal holding `value`.
    pub fn new(cx: &mut App, value: T) -> Self {
        Signal {
            entity: cx.new(|_cx| value),
        }
    }

    /// The backing entity — pass to `cx.observe(...)` or [`super::watch`].
    pub fn entity(&self) -> &Entity<T> {
        &self.entity
    }

    /// Borrow the current value.
    pub fn read<'a>(&self, cx: &'a App) -> &'a T {
        self.entity.read(cx)
    }

    /// Clone out the current value.
    pub fn get(&self, cx: &App) -> T
    where
        T: Clone,
    {
        self.entity.read(cx).clone()
    }

    /// Replace the value and notify observers.
    pub fn set(&self, cx: &mut App, value: T) {
        self.entity.update(cx, |slot, cx| {
            *slot = value;
            cx.notify();
        });
    }

    /// Mutate the value in place and notify observers.
    pub fn update(&self, cx: &mut App, f: impl FnOnce(&mut T)) {
        self.entity.update(cx, |slot, cx| {
            f(slot);
            cx.notify();
        });
    }
}

impl<T> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Signal {
            entity: self.entity.clone(),
        }
    }
}
