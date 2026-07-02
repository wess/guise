//! `Binding` — a two-way connection to a value (SwiftUI's `Binding`).
//!
//! A [`Signal`] is the store; a `Binding` is the connection: a getter and a
//! setter over `App`. Components accept one through `.bind(...)` (controlled
//! builders) or `X::bind(entity, &signal, cx)` (stateful entities), so a value
//! flows both ways without hand-written change handlers.
//!
//! ```ignore
//! // Whole signal as a binding:
//! let dark = use_state(cx, false);
//! Switch::new("dark-mode").bind(dark.binding());
//!
//! // One field of a struct signal (a lens):
//! let settings = use_state(cx, Settings::default());
//! Slider::bind(&volume_slider, &settings, cx); // entities bind to signals
//! Checkbox::new("mute").bind(settings.lens(|s| s.muted, |s, v| s.muted = v));
//! ```

use std::rc::Rc;

use gpui::App;

use super::signal::Signal;

type Getter<T> = Rc<dyn Fn(&App) -> T>;
type Setter<T> = Rc<dyn Fn(&mut App, T)>;

/// A two-way connection to a value: a getter and a setter over `App`.
///
/// Cheap to clone (both accessors are `Rc`-shared) and `'static`, so it can be
/// captured by element closures. Build one from a [`Signal`] with
/// [`Signal::binding`] or [`Signal::lens`], or from raw accessors with
/// [`Binding::new`].
pub struct Binding<T> {
    get: Getter<T>,
    set: Setter<T>,
}

impl<T> Clone for Binding<T> {
    fn clone(&self) -> Self {
        Binding {
            get: self.get.clone(),
            set: self.set.clone(),
        }
    }
}

impl<T: 'static> Binding<T> {
    /// Create a binding from a getter and a setter.
    pub fn new(get: impl Fn(&App) -> T + 'static, set: impl Fn(&mut App, T) + 'static) -> Self {
        Binding {
            get: Rc::new(get),
            set: Rc::new(set),
        }
    }

    /// Read the current value.
    pub fn get(&self, cx: &App) -> T {
        (self.get)(cx)
    }

    /// Write a new value.
    pub fn set(&self, cx: &mut App, value: T) {
        (self.set)(cx, value)
    }

    /// Bidirectional transform (e.g. `Binding<f64>` <-> `Binding<String>`):
    /// `from` converts on read, `into` converts back on write.
    pub fn map<U: 'static>(
        &self,
        from: impl Fn(T) -> U + 'static,
        into: impl Fn(U) -> T + 'static,
    ) -> Binding<U> {
        let get = self.get.clone();
        let set = self.set.clone();
        Binding {
            get: Rc::new(move |cx| from(get(cx))),
            set: Rc::new(move |cx, value| set(cx, into(value))),
        }
    }

    /// Read-only binding over a fixed value; writes are a no-op. Useful for
    /// disabled or demo states.
    pub fn constant(value: T) -> Self
    where
        T: Clone,
    {
        Binding {
            get: Rc::new(move |_cx| value.clone()),
            set: Rc::new(|_cx, _value| {}),
        }
    }
}

impl<T: Clone + PartialEq + 'static> Signal<T> {
    /// The whole signal as a binding. Writes go through
    /// [`Signal::set_if_changed`], so equal values skip the notify.
    pub fn binding(&self) -> Binding<T> {
        let read = self.clone();
        let write = self.clone();
        Binding::new(
            move |cx| read.get(cx),
            move |cx, value| write.set_if_changed(cx, value),
        )
    }
}

impl<T: 'static> Signal<T> {
    /// Project a field: signal of a struct -> binding of one field (a lens).
    /// Writes that leave the field unchanged skip the notify.
    pub fn lens<U: Clone + PartialEq + 'static>(
        &self,
        get: impl Fn(&T) -> U + 'static,
        set: impl Fn(&mut T, U) + 'static,
    ) -> Binding<U> {
        let get = Rc::new(get);
        let project = get.clone();
        let read = self.clone();
        let write = self.clone();
        Binding::new(
            move |cx| project(read.read(cx)),
            move |cx, value| {
                if get(write.read(cx)) == value {
                    return;
                }
                write.update(cx, |slot| set(slot, value));
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clone_shares_the_accessors() {
        let binding = Binding::new(|_cx| 1i32, |_cx, _value| {});
        let clone = binding.clone();
        assert_eq!(Rc::strong_count(&binding.get), 2);
        assert_eq!(Rc::strong_count(&clone.set), 2);
    }

    #[test]
    fn map_shares_the_source_accessors() {
        let binding = Binding::new(|_cx| 1.5f64, |_cx, _value| {});
        let _mapped: Binding<String> = binding.map(|v| v.to_string(), |s| s.parse().unwrap_or(0.0));
        // `map` captures the source's accessors rather than copying values.
        assert_eq!(Rc::strong_count(&binding.get), 2);
        assert_eq!(Rc::strong_count(&binding.set), 2);
    }
}
