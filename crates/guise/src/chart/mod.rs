//! Charts — minimal, axis-free data visuals painted through gpui's `canvas`.
//!
//! - [`Sparkline`] — a tiny inline trend line, optional area fill.
//! - [`LineChart`] — a sparkline grown up: light gridlines + area fill.
//! - [`BarChart`] — vertical bars, optional category labels below.
//! - [`PieChart`] — proportional slices, optional donut hole and legend.
//!
//! All four are stateless `RenderOnce` builders over plain `f32` series (or
//! `(label, value)` pairs for bars and pies). Colors default to a rotation
//! over the theme palette hues; override with `.color(..)` / `.colors(..)`.
//!
//! ```ignore
//! use guise::chart::{BarChart, PieChart, Sparkline};
//!
//! Sparkline::new([3.0, 5.0, 2.0, 8.0, 6.0]).fill()
//! BarChart::entries([("Mon", 12.0), ("Tue", 9.0), ("Wed", 15.0)])
//! PieChart::entries([("Rust", 62.0), ("TOML", 25.0), ("Other", 13.0)]).donut(0.6)
//! ```

mod bar;
mod line;
mod pie;
mod sparkline;

pub use bar::BarChart;
pub use line::LineChart;
pub use pie::PieChart;
pub use sparkline::Sparkline;

use gpui::{point, px, Bounds, Hsla, PathBuilder, Pixels, Point, Window};

use crate::style::ColorValue;
use crate::theme::{ColorName, Theme};

/// The default color rotation for multi-item charts (bars, pie slices): the
/// twelve chromatic palette hues, ordered so neighbors contrast. `Dark` and
/// `Gray` are left out — they read as chrome, not data.
pub(crate) const SERIES_HUES: [ColorName; 12] = [
    ColorName::Blue,
    ColorName::Teal,
    ColorName::Grape,
    ColorName::Orange,
    ColorName::Green,
    ColorName::Red,
    ColorName::Indigo,
    ColorName::Yellow,
    ColorName::Cyan,
    ColorName::Pink,
    ColorName::Lime,
    ColorName::Violet,
];

/// The palette shade charts draw with — the same accent convention as
/// `style::surface` (brighter in dark mode, deeper in light mode).
pub(crate) fn chart_shade(t: &Theme) -> usize {
    if t.scheme.is_dark() {
        4
    } else {
        6
    }
}

/// Resolve one chart color: named palette colors take the chart accent shade,
/// explicit colors pass through untouched.
pub(crate) fn resolve_color(t: &Theme, color: ColorValue) -> Hsla {
    match color {
        ColorValue::Named(name) => t.color(name, chart_shade(t)).hsla(),
        ColorValue::Custom(c) => c,
    }
}

/// The color of series item `index`: cycle the caller's overrides when given,
/// otherwise rotate [`SERIES_HUES`].
pub(crate) fn series_color(t: &Theme, overrides: &[ColorValue], index: usize) -> Hsla {
    if overrides.is_empty() {
        t.color(SERIES_HUES[index % SERIES_HUES.len()], chart_shade(t))
            .hsla()
    } else {
        resolve_color(t, overrides[index % overrides.len()])
    }
}

// --- pure geometry ----------------------------------------------------------

/// `(min, max)` of a series, skipping non-finite values. `None` when the
/// series is empty or has no finite values.
pub(crate) fn min_max(values: &[f32]) -> Option<(f32, f32)> {
    let mut lo = f32::INFINITY;
    let mut hi = f32::NEG_INFINITY;
    for &v in values {
        // f32::min/max ignore NaN but would absorb infinities into the
        // range, so all non-finite entries are skipped explicitly.
        if !v.is_finite() {
            continue;
        }
        lo = lo.min(v);
        hi = hi.max(v);
    }
    (lo.is_finite() && hi.is_finite()).then_some((lo, hi))
}

/// Map a series into `0.0..=1.0` (0 = min, 1 = max) for line-style charts.
/// Flat series and non-finite entries map to `0.5`. Empty when the series has
/// no finite values.
pub(crate) fn normalize(values: &[f32]) -> Vec<f32> {
    let Some((lo, hi)) = min_max(values) else {
        return Vec::new();
    };
    // Any positive span divides cleanly — an epsilon here would flatten
    // genuinely varying small-magnitude series.
    let span = hi - lo;
    values
        .iter()
        .map(|&v| {
            if !v.is_finite() || span <= 0.0 {
                0.5
            } else {
                ((v - lo) / span).clamp(0.0, 1.0)
            }
        })
        .collect()
}

/// Bar heights as fractions of the tallest bar, baseline at zero. Negative and
/// non-finite values clamp to `0.0` (no negative bars in v1). All zeros when
/// nothing is positive.
pub(crate) fn bar_heights(values: &[f32]) -> Vec<f32> {
    let max = values.iter().copied().fold(0.0_f32, f32::max);
    values
        .iter()
        .map(|&v| {
            if max > 0.0 && v.is_finite() {
                (v.max(0.0) / max).min(1.0)
            } else {
                0.0
            }
        })
        .collect()
}

/// Horizontal layout of bar `i` of `n` in a strip `w` wide: `(x, bar_width)`,
/// with `gap` the fraction of each slot left empty (split evenly on both sides).
pub(crate) fn bar_slot(i: usize, n: usize, w: f32, gap: f32) -> (f32, f32) {
    let slot = w / n.max(1) as f32;
    let bar = slot * (1.0 - gap.clamp(0.0, 0.9));
    (i as f32 * slot + (slot - bar) / 2.0, bar)
}

/// Cumulative `(start, end)` sweep fractions of a full turn, one span per
/// value. Non-positive and non-finite values become zero-width spans; if the
/// total is zero every span is `(0.0, 0.0)`.
pub(crate) fn slice_spans(values: &[f32]) -> Vec<(f32, f32)> {
    let clean: Vec<f32> = values
        .iter()
        .map(|&v| if v.is_finite() && v > 0.0 { v } else { 0.0 })
        .collect();
    let total: f32 = clean.iter().sum();
    if total <= 0.0 {
        return clean.iter().map(|_| (0.0, 0.0)).collect();
    }
    let mut acc = 0.0;
    clean
        .iter()
        .map(|&v| {
            let start = acc / total;
            acc += v;
            (start, acc / total)
        })
        .collect()
}

/// A point on a circle around `center`. `fraction` runs clockwise from
/// 12 o'clock: `0.25` is 3 o'clock, `0.5` is 6 o'clock (screen coordinates,
/// y grows downward).
pub(crate) fn arc_point(center: (f32, f32), radius: f32, fraction: f32) -> (f32, f32) {
    let angle = fraction * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
    (
        center.0 + radius * angle.cos(),
        center.1 + radius * angle.sin(),
    )
}

// --- shared painting --------------------------------------------------------

/// Stroke a min/max-normalized polyline across `bounds`, optionally filling
/// the area between the line and the bottom edge first. Paint-phase only:
/// coordinates handed to `paint_path` are window-absolute, so every point is
/// offset by `bounds.origin`.
pub(crate) fn paint_polyline(
    window: &mut Window,
    bounds: Bounds<Pixels>,
    values: &[f32],
    stroke: f32,
    line: Hsla,
    area: Option<Hsla>,
) {
    let ys = normalize(values);
    if ys.len() < 2 {
        return;
    }
    let w = f32::from(bounds.size.width);
    let h = f32::from(bounds.size.height);
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    // Inset vertically by half the stroke so the extremes aren't shaved off.
    let inset = stroke / 2.0;
    let step = w / (ys.len() - 1) as f32;
    let pts: Vec<Point<Pixels>> = ys
        .iter()
        .enumerate()
        .map(|(i, t)| {
            bounds.origin
                + point(
                    px(i as f32 * step),
                    px(inset + (h - 2.0 * inset) * (1.0 - t)),
                )
        })
        .collect();

    if let Some(color) = area {
        let mut pb = PathBuilder::fill();
        pb.move_to(bounds.origin + point(px(0.0), px(h)));
        for p in &pts {
            pb.line_to(*p);
        }
        pb.line_to(bounds.origin + point(px(w), px(h)));
        pb.close();
        if let Ok(path) = pb.build() {
            window.paint_path(path, color);
        }
    }

    let mut pb = PathBuilder::stroke(px(stroke));
    pb.move_to(pts[0]);
    for p in &pts[1..] {
        pb.line_to(*p);
    }
    if let Ok(path) = pb.build() {
        window.paint_path(path, line);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn min_max_finds_extremes_and_skips_non_finite() {
        assert_eq!(min_max(&[3.0, -1.0, 7.0]), Some((-1.0, 7.0)));
        assert_eq!(min_max(&[3.0, f32::NAN, 7.0]), Some((3.0, 7.0)));
        assert_eq!(min_max(&[1.0, f32::INFINITY, 2.0]), Some((1.0, 2.0)));
        assert_eq!(min_max(&[1.0, f32::NEG_INFINITY, 2.0]), Some((1.0, 2.0)));
        assert_eq!(min_max(&[]), None);
        assert_eq!(min_max(&[f32::NAN]), None);
        assert_eq!(min_max(&[f32::INFINITY]), None);
    }

    #[test]
    fn normalize_maps_min_to_zero_and_max_to_one() {
        assert_eq!(normalize(&[2.0, 4.0, 6.0]), vec![0.0, 0.5, 1.0]);
    }

    #[test]
    fn normalize_centers_flat_series() {
        assert_eq!(normalize(&[5.0, 5.0, 5.0]), vec![0.5, 0.5, 0.5]);
        assert!(normalize(&[f32::NAN, f32::NAN]).is_empty());
    }

    #[test]
    fn normalize_centers_infinities_and_keeps_the_finite_trend() {
        assert_eq!(normalize(&[1.0, 2.0, f32::INFINITY]), vec![0.0, 1.0, 0.5]);
    }

    #[test]
    fn normalize_scales_tiny_spans() {
        // A range below f32::EPSILON still varies — no epsilon flattening.
        assert_eq!(normalize(&[0.0, 5.0e-8, 1.0e-7]), vec![0.0, 0.5, 1.0]);
    }

    #[test]
    fn bar_heights_scale_to_tallest_and_clamp_negatives() {
        assert_eq!(bar_heights(&[5.0, 10.0, -3.0]), vec![0.5, 1.0, 0.0]);
        assert_eq!(bar_heights(&[-1.0, 0.0]), vec![0.0, 0.0]);
        assert_eq!(bar_heights(&[]), Vec::<f32>::new());
    }

    #[test]
    fn bar_slot_centers_bars_within_slots() {
        // 2 bars in 100px with a 0.2 gap: slots of 50, bars of 40, inset 5.
        assert_eq!(bar_slot(0, 2, 100.0, 0.2), (5.0, 40.0));
        assert_eq!(bar_slot(1, 2, 100.0, 0.2), (55.0, 40.0));
        // No gap fills the slot exactly.
        assert_eq!(bar_slot(0, 4, 100.0, 0.0), (0.0, 25.0));
    }

    #[test]
    fn slice_spans_accumulate_to_one() {
        let spans = slice_spans(&[1.0, 1.0, 2.0]);
        assert_eq!(spans, vec![(0.0, 0.25), (0.25, 0.5), (0.5, 1.0)]);
    }

    #[test]
    fn slice_spans_zero_out_junk_values() {
        let spans = slice_spans(&[-2.0, 4.0, f32::NAN, 4.0]);
        assert_eq!(spans, vec![(0.0, 0.0), (0.0, 0.5), (0.5, 0.5), (0.5, 1.0)]);
        assert_eq!(slice_spans(&[0.0, -1.0]), vec![(0.0, 0.0), (0.0, 0.0)]);
    }

    #[test]
    fn arc_point_hits_the_cardinal_directions() {
        let close = |(x, y): (f32, f32), (ex, ey): (f32, f32)| {
            assert!((x - ex).abs() < 1e-4 && (y - ey).abs() < 1e-4, "({x},{y})");
        };
        let c = (10.0, 10.0);
        close(arc_point(c, 5.0, 0.0), (10.0, 5.0)); // 12 o'clock
        close(arc_point(c, 5.0, 0.25), (15.0, 10.0)); // 3 o'clock
        close(arc_point(c, 5.0, 0.5), (10.0, 15.0)); // 6 o'clock
        close(arc_point(c, 5.0, 0.75), (5.0, 10.0)); // 9 o'clock
    }
}
