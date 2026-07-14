//! Animation toolkit: easing curves, springs, and mount/unmount presence.
//!
//! gpui animates by replaying a render-time interpolation over a duration
//! (`with_animation`); this module supplies the curves to drive it and the
//! [`Presence`] entity that latches an element through its exit animation
//! before unmounting. [`Transition`](crate::Transition) and
//! [`Collapse`](crate::Collapse) build on it.

pub mod ease;

mod presence;
mod spring;

pub use presence::{Presence, PresenceEvent};
pub use spring::Spring;

use std::time::Duration;

use gpui::Animation;

/// A named easing curve, storable on builders (`Copy`). `apply` maps
/// normalized time; `animation` builds a ready gpui [`Animation`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Easing {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    EaseInCubic,
    EaseOutCubic,
    EaseInOutCubic,
    EaseOutQuint,
    EaseOutExpo,
    EaseOutBack,
    EaseOutElastic,
    EaseOutBounce,
    /// CSS `cubic-bezier(x1, y1, x2, y2)`.
    CubicBezier(f32, f32, f32, f32),
    /// Physical spring; its duration comes from the spring itself.
    Spring(Spring),
}

impl Default for Easing {
    fn default() -> Self {
        Easing::EaseOut
    }
}

impl Easing {
    pub fn apply(self, t: f32) -> f32 {
        match self {
            Easing::Linear => ease::linear(t),
            Easing::EaseIn => ease::ease_in(t),
            Easing::EaseOut => ease::ease_out(t),
            Easing::EaseInOut => ease::ease_in_out(t),
            Easing::EaseInCubic => ease::ease_in_cubic(t),
            Easing::EaseOutCubic => ease::ease_out_cubic(t),
            Easing::EaseInOutCubic => ease::ease_in_out_cubic(t),
            Easing::EaseOutQuint => ease::ease_out_quint(t),
            Easing::EaseOutExpo => ease::ease_out_expo(t),
            Easing::EaseOutBack => ease::ease_out_back(t),
            Easing::EaseOutElastic => ease::ease_out_elastic(t),
            Easing::EaseOutBounce => ease::ease_out_bounce(t),
            Easing::CubicBezier(x1, y1, x2, y2) => ease::cubic_bezier(x1, y1, x2, y2, t),
            Easing::Spring(spring) => spring.easing()(t),
        }
    }

    /// A gpui [`Animation`] running this curve, **clamped** into `0..=1`.
    /// `duration_ms` is ignored for springs — they settle on their own clock.
    ///
    /// gpui debug-asserts that an animation's easing output stays within
    /// `0..=1`, which overshooting curves (`Spring`, `EaseOutBack`,
    /// `EaseOutElastic`, wide cubic-beziers) violate by design — unclamped
    /// they abort any debug build. The clamp flattens the overshoot peaks;
    /// to keep them, run [`clock`](Self::clock) and apply the curve inside
    /// the animator closure, where any value is legal:
    ///
    /// ```ignore
    /// el.with_animation(id, easing.clock(200), move |el, t| {
    ///     let delta = easing.apply(t); // may overshoot past 1.0
    ///     el.ml(px((1.0 - delta) * 8.0))
    /// })
    /// ```
    pub fn animation(self, duration_ms: u64) -> Animation {
        self.clock(duration_ms)
            .with_easing(move |t| self.apply(t).clamp(0.0, 1.0))
    }

    /// The un-eased gpui [`Animation`] for this curve: a linear clock sized
    /// for it (springs use their settle time). Pair with
    /// [`apply`](Self::apply) in the animator closure — see
    /// [`animation`](Self::animation) for why overshooting curves must run
    /// animator-side.
    pub fn clock(self, duration_ms: u64) -> Animation {
        let duration = match self {
            Easing::Spring(spring) => Duration::from_secs_f32(spring.settle_seconds()),
            _ => Duration::from_millis(duration_ms),
        };
        Animation::new(duration)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_variant_hits_the_endpoints() {
        let variants = [
            Easing::Linear,
            Easing::EaseIn,
            Easing::EaseOut,
            Easing::EaseInOut,
            Easing::EaseInCubic,
            Easing::EaseOutCubic,
            Easing::EaseInOutCubic,
            Easing::EaseOutQuint,
            Easing::EaseOutExpo,
            Easing::EaseOutBack,
            Easing::EaseOutElastic,
            Easing::EaseOutBounce,
            Easing::CubicBezier(0.25, 0.1, 0.25, 1.0),
            Easing::Spring(Spring::default()),
        ];
        for easing in variants {
            assert!(easing.apply(0.0).abs() < 1e-3, "{easing:?} at 0");
            assert!((easing.apply(1.0) - 1.0).abs() < 1e-3, "{easing:?} at 1");
        }
    }

    /// Overshoot is a feature of these curves — and exactly why the entity
    /// animators run them via `clock()` + `apply()` instead of gpui's easing
    /// slot, which debug-asserts its output into `0..=1`.
    #[test]
    fn overshooting_curves_really_overshoot() {
        let overshooters = [
            Easing::EaseOutBack,
            Easing::EaseOutElastic,
            Easing::Spring(Spring::default()),
        ];
        for easing in overshooters {
            let peak = (1..100)
                .map(|i| easing.apply(i as f32 / 100.0))
                .fold(f32::MIN, f32::max);
            assert!(peak > 1.0, "{easing:?} never exceeded 1.0 (peak {peak})");
        }
    }
}
