//! Spring easing from a closed-form damped harmonic oscillator. No
//! simulation loop: position is evaluated directly at normalized time, so a
//! spring plugs into gpui's `with_easing` like any other curve.

/// A mass-1 spring. Stiffness sets speed, damping sets wobble: `damping <
/// 2·√stiffness` is underdamped (overshoots and rings), anything above
/// settles without crossing 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spring {
    pub stiffness: f32,
    pub damping: f32,
}

impl Default for Spring {
    fn default() -> Self {
        // A gentle UI spring: slight overshoot, settles fast.
        Spring {
            stiffness: 170.0,
            damping: 22.0,
        }
    }
}

impl Spring {
    pub fn new(stiffness: f32, damping: f32) -> Self {
        Spring {
            stiffness: stiffness.max(1.0),
            damping: damping.max(0.0),
        }
    }

    /// A bouncier preset (visible ring before settling).
    pub fn wobbly() -> Self {
        Spring {
            stiffness: 180.0,
            damping: 12.0,
        }
    }

    /// No overshoot at all (critically damped).
    pub fn stiff() -> Self {
        Spring {
            stiffness: 210.0,
            damping: 2.0 * 210.0_f32.sqrt(),
        }
    }

    /// Spring position at time `seconds`, from 0 toward 1.
    pub fn position(self, seconds: f32) -> f32 {
        if seconds <= 0.0 {
            return 0.0;
        }
        let w0 = self.stiffness.sqrt();
        let zeta = self.damping / (2.0 * w0);
        if zeta < 1.0 {
            // Underdamped: decaying cosine around the target.
            let wd = w0 * (1.0 - zeta * zeta).sqrt();
            let decay = (-zeta * w0 * seconds).exp();
            1.0 - decay * ((wd * seconds).cos() + (zeta * w0 / wd) * (wd * seconds).sin())
        } else {
            // Critically damped / overdamped: pure approach.
            let decay = (-w0 * seconds).exp();
            1.0 - decay * (1.0 + w0 * seconds)
        }
    }

    /// Seconds until the spring stays within 1% of the target — pass this as
    /// the `Animation` duration so the curve completes on screen.
    pub fn settle_seconds(self) -> f32 {
        let w0 = self.stiffness.sqrt();
        let zeta = (self.damping / (2.0 * w0)).min(1.0);
        // The underdamped envelope e^(-ζωt) reaches 1% at 4.6 time constants
        // (ln 0.01 ≈ -4.6); critical damping's extra (1 + ωt) factor pushes
        // that to ~6.6, so scale with ζ. Clamped for the undamped edge case.
        let constants = 4.6 + 2.0 * zeta;
        (constants / (zeta * w0).max(0.5)).min(10.0)
    }

    /// The spring as a normalized easing over its settle duration.
    pub fn easing(self) -> impl Fn(f32) -> f32 + 'static {
        let total = self.settle_seconds();
        move |t: f32| {
            if t >= 1.0 {
                1.0
            } else {
                self.position(t * total)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_at_zero_ends_at_one() {
        for spring in [Spring::default(), Spring::wobbly(), Spring::stiff()] {
            let ease = spring.easing();
            assert_eq!(ease(0.0), 0.0);
            assert_eq!(ease(1.0), 1.0);
            assert!((ease(0.999) - 1.0).abs() < 0.02);
        }
    }

    #[test]
    fn underdamped_overshoots() {
        let ease = Spring::wobbly().easing();
        let max = (0..=1000)
            .map(|i| ease(i as f32 / 1000.0))
            .fold(f32::MIN, f32::max);
        assert!(max > 1.01, "wobbly spring should ring past 1, got {max}");
    }

    #[test]
    fn critically_damped_never_crosses_one() {
        let ease = Spring::stiff().easing();
        for i in 0..=1000 {
            assert!(ease(i as f32 / 1000.0) <= 1.0 + 1e-4);
        }
    }

    #[test]
    fn position_is_monotone_toward_target_early_on() {
        let spring = Spring::default();
        let quarter = spring.position(spring.settle_seconds() * 0.25);
        let half = spring.position(spring.settle_seconds() * 0.5);
        assert!(quarter > 0.1);
        assert!(half >= quarter * 0.9);
    }

    #[test]
    fn settle_time_is_sane() {
        for spring in [Spring::default(), Spring::wobbly(), Spring::stiff()] {
            let s = spring.settle_seconds();
            assert!(s > 0.05 && s < 10.0, "settle {s}");
        }
    }
}
