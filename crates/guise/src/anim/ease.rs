//! Easing curves. Pure `f32 -> f32` maps over normalized time (0..=1),
//! guaranteed to hit 0 at 0 and 1 at 1 (mid-curve values may overshoot —
//! that's what back/elastic are for).

use std::f32::consts::PI;

pub fn linear(t: f32) -> f32 {
    t
}

pub fn ease_in(t: f32) -> f32 {
    t * t
}

pub fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t) * (1.0 - t)
}

pub fn ease_in_out(t: f32) -> f32 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
    }
}

pub fn ease_in_cubic(t: f32) -> f32 {
    t * t * t
}

pub fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

pub fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

pub fn ease_out_quint(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(5)
}

pub fn ease_out_expo(t: f32) -> f32 {
    if t >= 1.0 {
        1.0
    } else {
        1.0 - 2.0_f32.powf(-10.0 * t)
    }
}

/// Overshoots past 1 near the end, then settles.
pub fn ease_out_back(t: f32) -> f32 {
    const C1: f32 = 1.70158;
    const C3: f32 = C1 + 1.0;
    1.0 + C3 * (t - 1.0).powi(3) + C1 * (t - 1.0).powi(2)
}

/// Springy decaying oscillation into place.
pub fn ease_out_elastic(t: f32) -> f32 {
    const C4: f32 = (2.0 * PI) / 3.0;
    if t <= 0.0 {
        0.0
    } else if t >= 1.0 {
        1.0
    } else {
        2.0_f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * C4).sin() + 1.0
    }
}

/// Ball-drop bounce into place.
pub fn ease_out_bounce(t: f32) -> f32 {
    const N1: f32 = 7.5625;
    const D1: f32 = 2.75;
    if t < 1.0 / D1 {
        N1 * t * t
    } else if t < 2.0 / D1 {
        let t = t - 1.5 / D1;
        N1 * t * t + 0.75
    } else if t < 2.5 / D1 {
        let t = t - 2.25 / D1;
        N1 * t * t + 0.9375
    } else {
        let t = t - 2.625 / D1;
        N1 * t * t + 0.984375
    }
}

/// A CSS `cubic-bezier(x1, y1, x2, y2)` timing curve. Solves x(s) = t for s
/// by bisection (the bezier x is monotonic for valid control points), then
/// evaluates y(s).
pub fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, t: f32) -> f32 {
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }
    let sample = |c1: f32, c2: f32, s: f32| {
        // Cubic bezier with P0=0, P3=1: 3(1-s)²s·c1 + 3(1-s)s²·c2 + s³.
        let inv = 1.0 - s;
        3.0 * inv * inv * s * c1 + 3.0 * inv * s * s * c2 + s * s * s
    };
    let (mut lo, mut hi) = (0.0_f32, 1.0_f32);
    let mut s = t;
    for _ in 0..24 {
        let x = sample(x1, x2, s);
        if (x - t).abs() < 1e-5 {
            break;
        }
        if x < t {
            lo = s;
        } else {
            hi = s;
        }
        s = (lo + hi) * 0.5;
    }
    sample(y1, y2, s)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CURVES: [fn(f32) -> f32; 10] = [
        linear,
        ease_in,
        ease_out,
        ease_in_out,
        ease_in_cubic,
        ease_out_cubic,
        ease_in_out_cubic,
        ease_out_quint,
        ease_out_expo,
        ease_out_bounce,
    ];

    #[test]
    fn endpoints_are_exact() {
        for f in CURVES {
            assert!((f(0.0)).abs() < 1e-4);
            assert!((f(1.0) - 1.0).abs() < 1e-3);
        }
        assert!((ease_out_back(0.0)).abs() < 1e-4);
        assert!((ease_out_back(1.0) - 1.0).abs() < 1e-4);
        assert_eq!(ease_out_elastic(0.0), 0.0);
        assert_eq!(ease_out_elastic(1.0), 1.0);
    }

    #[test]
    fn monotone_curves_are_monotone() {
        for f in [linear, ease_in, ease_out, ease_in_out, ease_out_cubic] {
            let mut last = f(0.0);
            for i in 1..=100 {
                let v = f(i as f32 / 100.0);
                assert!(v >= last - 1e-6);
                last = v;
            }
        }
    }

    #[test]
    fn back_and_elastic_overshoot() {
        let back_max = (0..=100)
            .map(|i| ease_out_back(i as f32 / 100.0))
            .fold(f32::MIN, f32::max);
        assert!(back_max > 1.05);
        let elastic_max = (0..=1000)
            .map(|i| ease_out_elastic(i as f32 / 1000.0))
            .fold(f32::MIN, f32::max);
        assert!(elastic_max > 1.05);
    }

    #[test]
    fn bezier_matches_css_ease() {
        // cubic-bezier(0.25, 0.1, 0.25, 1.0) is the CSS "ease" curve.
        let ease = |t| cubic_bezier(0.25, 0.1, 0.25, 1.0, t);
        assert!((ease(0.0)).abs() < 1e-4);
        assert!((ease(1.0) - 1.0).abs() < 1e-4);
        assert!((ease(0.5) - 0.8024).abs() < 0.01);
        assert!((ease(0.25) - 0.4085).abs() < 0.01);
    }

    #[test]
    fn bezier_linear_control_points_are_linear() {
        for i in 0..=10 {
            let t = i as f32 / 10.0;
            assert!((cubic_bezier(1.0 / 3.0, 1.0 / 3.0, 2.0 / 3.0, 2.0 / 3.0, t) - t).abs() < 1e-3);
        }
    }
}
