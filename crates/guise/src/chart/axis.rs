//! Axis tick math — Heckbert "nice numbers". Pure logic.
//!
//! Ticks land on round values (1/2/5 × 10ⁿ) covering the data range, so
//! labels read naturally and gridlines align with the label column.

/// The nearest "nice" number to `x`: 1, 2, or 5 times a power of ten.
/// `round` picks the closest; otherwise the smallest nice number ≥ `x`.
fn nice_num(x: f32, round: bool) -> f32 {
    if x <= 0.0 || !x.is_finite() {
        return 1.0;
    }
    let exp = x.log10().floor();
    let base = 10.0_f32.powf(exp);
    let frac = x / base;
    let nice = if round {
        match frac {
            f if f < 1.5 => 1.0,
            f if f < 3.0 => 2.0,
            f if f < 7.0 => 5.0,
            _ => 10.0,
        }
    } else {
        match frac {
            f if f <= 1.0 => 1.0,
            f if f <= 2.0 => 2.0,
            f if f <= 5.0 => 5.0,
            _ => 10.0,
        }
    };
    nice * base
}

/// Round tick values covering `lo..=hi` (expanded outward to nice bounds),
/// aiming for about `target` intervals. Flat or junk ranges produce a
/// two-tick `[floor, floor+1]` fallback so charts always have a frame.
pub fn nice_ticks(lo: f32, hi: f32, target: usize) -> Vec<f32> {
    let (lo, hi) = if lo.is_finite() && hi.is_finite() && hi > lo {
        (lo, hi)
    } else if lo.is_finite() && lo == hi {
        (lo.floor(), lo.floor() + 1.0)
    } else {
        (0.0, 1.0)
    };
    let range = nice_num(hi - lo, false);
    let step = nice_num(range / target.max(1) as f32, true);
    let start = (lo / step).floor() * step;
    let end = (hi / step).ceil() * step;
    let count = ((end - start) / step).round() as usize;
    (0..=count).map(|i| start + i as f32 * step).collect()
}

/// A compact tick label: trailing zeros trimmed, thousands as `k`,
/// millions as `M`.
pub fn tick_label(value: f32) -> String {
    let (scaled, suffix) = if value.abs() >= 1_000_000.0 {
        (value / 1_000_000.0, "M")
    } else if value.abs() >= 1_000.0 {
        (value / 1_000.0, "k")
    } else {
        (value, "")
    };
    let text = format!("{scaled:.2}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    format!("{text}{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticks_cover_the_range_with_round_steps() {
        assert_eq!(nice_ticks(0.0, 100.0, 5), vec![0.0, 20.0, 40.0, 60.0, 80.0, 100.0]);
        assert_eq!(nice_ticks(0.0, 7.0, 5), vec![0.0, 2.0, 4.0, 6.0, 8.0]);
        let ticks = nice_ticks(-3.0, 14.0, 5);
        assert_eq!(ticks, vec![-5.0, 0.0, 5.0, 10.0, 15.0]);
    }

    #[test]
    fn ticks_expand_outward() {
        let ticks = nice_ticks(1.2, 9.7, 5);
        assert!(*ticks.first().unwrap() <= 1.2);
        assert!(*ticks.last().unwrap() >= 9.7);
        for pair in ticks.windows(2) {
            assert!(pair[1] > pair[0]);
        }
    }

    #[test]
    fn small_and_fractional_ranges_stay_round() {
        let ticks = nice_ticks(0.0, 0.9, 4);
        assert_eq!(ticks, vec![0.0, 0.2, 0.4, 0.6, 0.8, 1.0]);
    }

    #[test]
    fn degenerate_ranges_fall_back() {
        assert_eq!(nice_ticks(5.0, 5.0, 4).len() >= 2, true);
        assert_eq!(nice_ticks(f32::NAN, 3.0, 4), nice_ticks(0.0, 1.0, 4));
        assert!(nice_ticks(9.0, 2.0, 4).len() >= 2);
    }

    #[test]
    fn labels_are_compact() {
        assert_eq!(tick_label(0.0), "0");
        assert_eq!(tick_label(2.5), "2.5");
        assert_eq!(tick_label(20.0), "20");
        assert_eq!(tick_label(1500.0), "1.5k");
        assert_eq!(tick_label(2_000_000.0), "2M");
        assert_eq!(tick_label(-500.0), "-500");
        assert_eq!(tick_label(-1500.0), "-1.5k");
    }
}
