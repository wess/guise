//! `RangeSlider` — a two-thumb value track (gpui entity).
//!
//! Holds a `(low, high)` pair in `min..=max`, snapped to `step` and kept at
//! least `min_gap` apart. Each thumb is a real gpui drag source (`on_drag` +
//! `on_drag_move`), so dragging tracks the pointer even outside the element;
//! clicking the track jumps the nearest thumb; arrow keys nudge the last
//! active thumb. Emits [`RangeSliderEvent`] on change.
//!
//! ```ignore
//! let range = cx.new(|cx| RangeSlider::new(cx).min(0.0).max(100.0).value((20.0, 80.0)));
//! cx.subscribe(&range, |_this, _slider, event: &RangeSliderEvent, _cx| {
//!     let (low, high) = event.0;
//! })
//! .detach();
//! ```

use gpui::prelude::*;
use gpui::{
    canvas, div, px, relative, App, Bounds, Context, DragMoveEvent, Empty, Entity, EntityId,
    EventEmitter, FocusHandle, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, Pixels,
    SharedString, Window,
};

use crate::reactive::Signal;
use crate::theme::{theme, ColorName, Size};

/// Emitted when either end of the range changes. Carries `(low, high)`.
#[derive(Debug, Clone, Copy)]
pub struct RangeSliderEvent(pub (f64, f64));

/// The drag payload for a thumb. `owner` scopes `on_drag_move` to the
/// instance that started the drag (the listener fires for every active drag
/// of this type in the window).
struct ThumbDrag {
    owner: EntityId,
    thumb: usize,
}

/// A two-thumb range slider. Create with `cx.new(|cx| RangeSlider::new(cx))`.
pub struct RangeSlider {
    value: (f64, f64),
    min: f64,
    max: f64,
    step: f64,
    min_gap: f64,
    color: ColorName,
    size: Size,
    focus: FocusHandle,
    disabled: bool,
    /// The thumb arrow keys move: the one last dragged or clicked toward.
    active: usize,
    /// Track bounds captured each frame (canvas trick) for click hit-testing.
    bounds: Bounds<Pixels>,
}

impl EventEmitter<RangeSliderEvent> for RangeSlider {}

impl RangeSlider {
    pub fn new(cx: &mut Context<Self>) -> Self {
        RangeSlider {
            value: (25.0, 75.0),
            min: 0.0,
            max: 100.0,
            step: 1.0,
            min_gap: 0.0,
            color: ColorName::Blue,
            size: Size::Md,
            focus: cx.focus_handle(),
            disabled: false,
            active: 0,
            bounds: Bounds::default(),
        }
    }

    /// The `(low, high)` pair. Set `min`/`max`/`step`/`min_gap` first — the
    /// value is normalized against them.
    pub fn value(mut self, value: (f64, f64)) -> Self {
        self.value = normalize_pair(value, self.min, self.max, self.step, self.min_gap);
        self
    }

    pub fn min(mut self, min: f64) -> Self {
        self.min = min;
        self
    }

    pub fn max(mut self, max: f64) -> Self {
        self.max = max;
        self
    }

    pub fn step(mut self, step: f64) -> Self {
        self.step = step.max(f64::EPSILON);
        self
    }

    /// Minimum distance the thumbs keep between each other (default 0).
    pub fn min_gap(mut self, min_gap: f64) -> Self {
        self.min_gap = min_gap.max(0.0);
        self
    }

    pub fn color(mut self, color: ColorName) -> Self {
        self.color = color;
        self
    }

    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// The current `(low, high)` pair.
    pub fn value_pair(&self) -> (f64, f64) {
        self.value
    }

    /// Two-way bind this slider's range to a `Signal<(f64, f64)>`. The signal
    /// is the source of truth: the slider adopts its value now (normalized),
    /// drags write back through [`Signal::set_if_changed`], and signal writes
    /// move the thumbs without emitting [`RangeSliderEvent`]. Equality guards
    /// on both directions prevent update loops.
    pub fn bind(entity: &Entity<RangeSlider>, signal: &Signal<(f64, f64)>, cx: &mut App) {
        let initial = signal.get(cx);
        entity.update(cx, |this, cx| this.sync_value(initial, cx));
        let sink = signal.clone();
        cx.subscribe(entity, move |_slider, event: &RangeSliderEvent, cx| {
            sink.set_if_changed(cx, event.0);
        })
        .detach();
        let slider = entity.downgrade();
        cx.observe(signal.entity(), move |observed, cx| {
            let value = *observed.read(cx);
            slider
                .update(cx, |this, cx| this.sync_value(value, cx))
                .ok();
        })
        .detach();
    }

    /// Programmatic set: normalize and repaint without emitting an event.
    fn sync_value(&mut self, raw: (f64, f64), cx: &mut Context<Self>) {
        let next = normalize_pair(raw, self.min, self.max, self.step, self.min_gap);
        if next != self.value {
            self.value = next;
            cx.notify();
        }
    }

    fn fraction(&self, v: f64) -> f32 {
        if self.max <= self.min {
            0.0
        } else {
            (((v - self.min) / (self.max - self.min)) as f32).clamp(0.0, 1.0)
        }
    }

    /// Move one thumb toward `raw`, respecting step, bounds and the gap.
    fn set_thumb(&mut self, thumb: usize, raw: f64, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        self.active = thumb;
        let next = clamp_thumb(
            self.value,
            thumb,
            raw,
            self.min,
            self.max,
            self.step,
            self.min_gap,
        );
        if next != self.value {
            self.value = next;
            cx.emit(RangeSliderEvent(next));
        }
        cx.notify();
    }

    /// The raw value under a window-space x, from the captured track bounds.
    fn value_at(&self, x: Pixels) -> Option<f64> {
        let width = self.bounds.size.width;
        if width <= px(0.0) {
            return None;
        }
        let frac = ((x - self.bounds.left()) / width).clamp(0.0, 1.0);
        Some(self.min + frac as f64 * (self.max - self.min))
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.disabled {
            return;
        }
        window.focus(&self.focus);
        // A press on a knob starts a drag (the knobs' `on_drag` doesn't stop
        // this event from bubbling here) — jumping a thumb toward the press
        // would move it by up to half a knob, or move the *other* thumb when
        // they sit close. Only track-presses jump.
        let width = f32::from(self.bounds.size.width);
        if width > 0.0 {
            let x = f32::from(event.position.x - self.bounds.left());
            let (thumb_w, _) = self.metrics();
            let (f0, f1) = (self.fraction(self.value.0), self.fraction(self.value.1));
            if let Some(thumb) = thumb_under(x, width, f0, f1, thumb_w) {
                self.active = thumb;
                cx.notify();
                return;
            }
        }
        if let Some(raw) = self.value_at(event.position.x) {
            let thumb = nearest_thumb(self.value.0, self.value.1, raw);
            self.set_thumb(thumb, raw, cx);
        }
        cx.notify();
    }

    fn on_drag_move(
        &mut self,
        event: &DragMoveEvent<ThumbDrag>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (owner, thumb) = {
            let drag = event.drag(cx);
            (drag.owner, drag.thumb)
        };
        if owner != cx.entity_id() {
            return;
        }
        let width = event.bounds.size.width;
        if width <= px(0.0) {
            return;
        }
        let frac = ((event.event.position.x - event.bounds.left()) / width).clamp(0.0, 1.0);
        let raw = self.min + frac as f64 * (self.max - self.min);
        self.set_thumb(thumb, raw, cx);
    }

    fn on_key(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let current = if self.active == 0 {
            self.value.0
        } else {
            self.value.1
        };
        match event.keystroke.key.as_str() {
            "left" | "down" => self.set_thumb(self.active, current - self.step, cx),
            "right" | "up" => self.set_thumb(self.active, current + self.step, cx),
            "home" => self.set_thumb(self.active, self.min, cx),
            "end" => self.set_thumb(self.active, self.max, cx),
            _ => return,
        }
        cx.stop_propagation();
    }

    fn metrics(&self) -> (f32, f32) {
        match self.size {
            Size::Xs => (12.0, 4.0),
            Size::Sm => (14.0, 5.0),
            Size::Md => (16.0, 6.0),
            Size::Lg => (20.0, 8.0),
            Size::Xl => (24.0, 10.0),
        }
    }
}

/// Snap `raw` to the step grid.
fn snap(raw: f64, step: f64) -> f64 {
    (raw / step).round() * step
}

/// Move one end of `current` toward `raw`, snapped and kept `min_gap` away
/// from the other end, inside `min..=max`.
fn clamp_thumb(
    current: (f64, f64),
    thumb: usize,
    raw: f64,
    min: f64,
    max: f64,
    step: f64,
    min_gap: f64,
) -> (f64, f64) {
    let snapped = snap(raw, step);
    if thumb == 0 {
        let upper = (current.1 - min_gap).max(min);
        (snapped.max(min).min(upper), current.1)
    } else {
        let lower = (current.0 + min_gap).min(max);
        (current.0, snapped.min(max).max(lower))
    }
}

/// Order, snap and clamp a raw pair, enforcing the gap where the range allows.
fn normalize_pair(raw: (f64, f64), min: f64, max: f64, step: f64, min_gap: f64) -> (f64, f64) {
    let (a, b) = if raw.0 <= raw.1 { raw } else { (raw.1, raw.0) };
    let lo = snap(a, step).max(min).min((max - min_gap).max(min));
    let hi = snap(b, step).min(max).max((lo + min_gap).min(max));
    (lo, hi)
}

/// The knob whose painted extent contains local `x`, if any. Knob 1 paints
/// last (topmost) and wins the subsequent drag when the knobs overlap, so it
/// is checked first to stay consistent.
fn thumb_under(x: f32, width: f32, f0: f32, f1: f32, thumb_w: f32) -> Option<usize> {
    let hit = |frac: f32| (x - frac * width).abs() <= thumb_w / 2.0;
    if hit(f1) {
        Some(1)
    } else if hit(f0) {
        Some(0)
    } else {
        None
    }
}

/// Which thumb a click at `raw` should move.
fn nearest_thumb(lo: f64, hi: f64, raw: f64) -> usize {
    if raw <= lo {
        0
    } else if raw >= hi {
        1
    } else if raw - lo < hi - raw {
        0
    } else {
        1
    }
}

impl Render for RangeSlider {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        let accent = t.color(self.color, t.primary_shade()).hsla();
        let track_color = if t.scheme.is_dark() {
            t.color(ColorName::Dark, 4)
        } else {
            t.color(ColorName::Gray, 2)
        }
        .hsla();
        let knob_bg = t.surface().hsla();
        let label_color = t.dimmed().hsla();
        let font_xs = t.font_size(Size::Xs);

        let (thumb, track_h) = self.metrics();
        let container_h = thumb + 4.0;
        let track_top = (container_h - track_h) / 2.0;
        let (f0, f1) = (self.fraction(self.value.0), self.fraction(self.value.1));
        let owner = cx.entity_id();

        let track = div()
            .absolute()
            .left(px(0.0))
            .right(px(0.0))
            .top(px(track_top))
            .h(px(track_h))
            .rounded(px(track_h / 2.0))
            .bg(track_color);

        let fill = div()
            .absolute()
            .left(relative(f0))
            .w(relative((f1 - f0).max(0.0)))
            .top(px(track_top))
            .h(px(track_h))
            .rounded(px(track_h / 2.0))
            .bg(accent);

        let knob = |i: usize, frac: f32| {
            div()
                .id(("guise-rangeslider-thumb", i))
                .absolute()
                .left(relative(frac))
                .ml(px(-thumb / 2.0))
                .top(px(2.0))
                .w(px(thumb))
                .h(px(thumb))
                .rounded(px(thumb / 2.0))
                .bg(knob_bg)
                .border_2()
                .border_color(accent)
                .cursor_grab()
                .on_drag(
                    ThumbDrag { owner, thumb: i },
                    |_drag, _offset, _window, cx| cx.new(|_| Empty),
                )
        };

        // Invisible canvas capturing the container's bounds for click math.
        let this = cx.entity();
        let bounds_probe = canvas(
            move |bounds, _window, cx| {
                this.update(cx, |this, _| this.bounds = bounds);
            },
            |_, _, _, _| {},
        )
        .absolute()
        .size_full();

        let slider = div()
            .id("guise-rangeslider")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_drag_move::<ThumbDrag>(cx.listener(Self::on_drag_move))
            .relative()
            .w_full()
            .h(px(container_h))
            .child(bounds_probe)
            .child(track)
            .child(fill)
            .child(knob(0, f0))
            .child(knob(1, f1));

        let value_label =
            div()
                .text_size(px(font_xs))
                .text_color(label_color)
                .child(SharedString::from(format!(
                    "{} \u{2013} {}",
                    self.value.0, self.value.1
                )));

        let column = div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .child(slider)
            .child(value_label);

        if self.disabled {
            column.opacity(0.5)
        } else {
            column
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_thumb_snaps_and_respects_bounds() {
        assert_eq!(
            clamp_thumb((20.0, 80.0), 0, 33.4, 0.0, 100.0, 1.0, 0.0),
            (33.0, 80.0)
        );
        assert_eq!(
            clamp_thumb((20.0, 80.0), 0, -10.0, 0.0, 100.0, 1.0, 0.0),
            (0.0, 80.0)
        );
        assert_eq!(
            clamp_thumb((20.0, 80.0), 1, 250.0, 0.0, 100.0, 1.0, 0.0),
            (20.0, 100.0)
        );
    }

    #[test]
    fn clamp_thumb_enforces_the_gap() {
        // Low thumb pushed past high stops `min_gap` short of it.
        assert_eq!(
            clamp_thumb((20.0, 50.0), 0, 60.0, 0.0, 100.0, 1.0, 10.0),
            (40.0, 50.0)
        );
        // High thumb pushed past low stops `min_gap` above it.
        assert_eq!(
            clamp_thumb((20.0, 50.0), 1, 5.0, 0.0, 100.0, 1.0, 10.0),
            (20.0, 30.0)
        );
        // The gap clamp never escapes min/max even when the gap can't fit.
        assert_eq!(
            clamp_thumb((0.0, 5.0), 0, -20.0, 0.0, 100.0, 1.0, 10.0),
            (0.0, 5.0)
        );
    }

    #[test]
    fn clamp_thumb_snaps_to_coarse_steps() {
        assert_eq!(
            clamp_thumb((0.0, 100.0), 0, 37.0, 0.0, 100.0, 25.0, 0.0),
            (25.0, 100.0)
        );
        assert_eq!(
            clamp_thumb((0.0, 100.0), 0, 38.0, 0.0, 100.0, 25.0, 0.0),
            (50.0, 100.0)
        );
    }

    #[test]
    fn normalize_orders_and_clamps_the_pair() {
        assert_eq!(
            normalize_pair((80.0, 20.0), 0.0, 100.0, 1.0, 0.0),
            (20.0, 80.0)
        );
        assert_eq!(
            normalize_pair((-5.0, 120.0), 0.0, 100.0, 1.0, 0.0),
            (0.0, 100.0)
        );
        assert_eq!(
            normalize_pair((40.0, 45.0), 0.0, 100.0, 1.0, 10.0),
            (40.0, 50.0)
        );
        // A gap wider than the range collapses to the range itself.
        assert_eq!(
            normalize_pair((0.0, 100.0), 0.0, 100.0, 1.0, 500.0),
            (0.0, 100.0)
        );
    }

    #[test]
    fn thumb_under_hits_knob_extents_only() {
        // 400px track, values 50/52 of 0..100 → knob centers at 200 and 208px,
        // a 16px knob spans ±8.
        let (f0, f1) = (0.5, 0.52);
        // Inside the high knob (and the low one) → the topmost wins.
        assert_eq!(thumb_under(202.4, 400.0, f0, f1, 16.0), Some(1));
        // Only inside the low knob.
        assert_eq!(thumb_under(196.0, 400.0, f0, f1, 16.0), Some(0));
        // On the bare track.
        assert_eq!(thumb_under(100.0, 400.0, f0, f1, 16.0), None);
        assert_eq!(thumb_under(300.0, 400.0, f0, f1, 16.0), None);
        // Coincident knobs: the topmost (high) one wins.
        assert_eq!(thumb_under(200.0, 400.0, 0.5, 0.5, 16.0), Some(1));
    }

    #[test]
    fn nearest_thumb_splits_the_track() {
        assert_eq!(nearest_thumb(20.0, 80.0, 5.0), 0);
        assert_eq!(nearest_thumb(20.0, 80.0, 30.0), 0);
        assert_eq!(nearest_thumb(20.0, 80.0, 70.0), 1);
        assert_eq!(nearest_thumb(20.0, 80.0, 95.0), 1);
        // Coincident thumbs: clicks left move the low, right the high.
        assert_eq!(nearest_thumb(50.0, 50.0, 40.0), 0);
        assert_eq!(nearest_thumb(50.0, 50.0, 60.0), 1);
    }
}
