//! Responsive breakpoints: window width → a token, and per-breakpoint values.
//!
//! Desktop windows resize like browser windows; these helpers make layout
//! decisions declarative. `Breakpoint::from_window(window)` during render,
//! then either branch on it or resolve a [`Responsive`] value.
//!
//! ```ignore
//! let bp = Breakpoint::from_window(window);
//! let columns = Responsive::new(1).md(2).xl(4).resolve(bp);
//! SimpleGrid::new(columns).children(cards)
//! ```

use gpui::Window;

/// Width class thresholds (px), Tailwind-flavored: `Sm` 640, `Md` 768,
/// `Lg` 1024, `Xl` 1280. Anything narrower is `Xs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Breakpoint {
    Xs,
    Sm,
    Md,
    Lg,
    Xl,
}

impl Breakpoint {
    /// The breakpoint for a width in px.
    pub fn of(width: f32) -> Breakpoint {
        match width {
            w if w >= 1280.0 => Breakpoint::Xl,
            w if w >= 1024.0 => Breakpoint::Lg,
            w if w >= 768.0 => Breakpoint::Md,
            w if w >= 640.0 => Breakpoint::Sm,
            _ => Breakpoint::Xs,
        }
    }

    /// The breakpoint for the window's current viewport width.
    pub fn from_window(window: &Window) -> Breakpoint {
        Breakpoint::of(f32::from(window.viewport_size().width))
    }

    /// Mobile-first comparison: `bp.at_least(Breakpoint::Md)` reads like
    /// CSS `min-width: 768px`.
    pub fn at_least(self, other: Breakpoint) -> bool {
        self >= other
    }
}

/// A value that changes at breakpoints, mobile-first: the base applies from
/// `Xs` up, and each override kicks in at its breakpoint and above.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Responsive<T> {
    base: T,
    sm: Option<T>,
    md: Option<T>,
    lg: Option<T>,
    xl: Option<T>,
}

impl<T: Clone> Responsive<T> {
    /// The value below the first override.
    pub fn new(base: T) -> Self {
        Responsive {
            base,
            sm: None,
            md: None,
            lg: None,
            xl: None,
        }
    }

    pub fn sm(mut self, value: T) -> Self {
        self.sm = Some(value);
        self
    }

    pub fn md(mut self, value: T) -> Self {
        self.md = Some(value);
        self
    }

    pub fn lg(mut self, value: T) -> Self {
        self.lg = Some(value);
        self
    }

    pub fn xl(mut self, value: T) -> Self {
        self.xl = Some(value);
        self
    }

    /// The value in effect at `bp`: the largest override at or below it,
    /// else the base.
    pub fn resolve(&self, bp: Breakpoint) -> T {
        let ladder = [
            (Breakpoint::Xl, &self.xl),
            (Breakpoint::Lg, &self.lg),
            (Breakpoint::Md, &self.md),
            (Breakpoint::Sm, &self.sm),
        ];
        for (level, value) in ladder {
            if bp.at_least(level) {
                if let Some(v) = value {
                    return v.clone();
                }
            }
        }
        self.base.clone()
    }

    /// Shorthand: resolve against the window's current width.
    pub fn for_window(&self, window: &Window) -> T {
        self.resolve(Breakpoint::from_window(window))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widths_map_to_breakpoints() {
        assert_eq!(Breakpoint::of(0.0), Breakpoint::Xs);
        assert_eq!(Breakpoint::of(639.9), Breakpoint::Xs);
        assert_eq!(Breakpoint::of(640.0), Breakpoint::Sm);
        assert_eq!(Breakpoint::of(767.0), Breakpoint::Sm);
        assert_eq!(Breakpoint::of(768.0), Breakpoint::Md);
        assert_eq!(Breakpoint::of(1024.0), Breakpoint::Lg);
        assert_eq!(Breakpoint::of(1280.0), Breakpoint::Xl);
        assert_eq!(Breakpoint::of(3840.0), Breakpoint::Xl);
    }

    #[test]
    fn at_least_is_mobile_first() {
        assert!(Breakpoint::Lg.at_least(Breakpoint::Md));
        assert!(Breakpoint::Md.at_least(Breakpoint::Md));
        assert!(!Breakpoint::Sm.at_least(Breakpoint::Md));
    }

    #[test]
    fn responsive_resolves_the_nearest_override_below() {
        let cols = Responsive::new(1).md(2).xl(4);
        assert_eq!(cols.resolve(Breakpoint::Xs), 1);
        assert_eq!(cols.resolve(Breakpoint::Sm), 1);
        assert_eq!(cols.resolve(Breakpoint::Md), 2);
        assert_eq!(cols.resolve(Breakpoint::Lg), 2); // inherits md
        assert_eq!(cols.resolve(Breakpoint::Xl), 4);
    }

    #[test]
    fn base_only_is_constant() {
        let pad = Responsive::new("tight");
        assert_eq!(pad.resolve(Breakpoint::Xs), "tight");
        assert_eq!(pad.resolve(Breakpoint::Xl), "tight");
    }
}
