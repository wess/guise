# Transitions & animation

Motion in guise is three layers: **easing curves** (`guise::anim`), the
**one-shot wrappers** (`Transition`, `Collapse`), and **`Presence`** for exit
animations. Everything rides gpui's `with_animation` — the library supplies
the curves and the state so you don't hand-roll either.

> gpui has no transform/scale on elements, so motion is expressed through
> opacity, margin offsets, and (for `Collapse`) real height.

## Easing

`Easing` is a `Copy` enum you can store on any builder. Every variant maps
normalized time 0..=1 and hits both endpoints exactly.

```rust
use guise::anim::{Easing, Spring};

Easing::EaseOutBack                      // overshoot + settle
Easing::CubicBezier(0.25, 0.1, 0.25, 1.0) // CSS "ease"
Easing::Spring(Spring::wobbly())          // physical spring
```

Variants: `Linear`, `EaseIn`, `EaseOut` (default), `EaseInOut`, `EaseInCubic`,
`EaseOutCubic`, `EaseInOutCubic`, `EaseOutQuint`, `EaseOutExpo`, `EaseOutBack`,
`EaseOutElastic`, `EaseOutBounce`, `CubicBezier(x1, y1, x2, y2)`,
`Spring(Spring)`.

The raw curves are plain functions in `guise::anim::ease` if you're driving
`with_animation` yourself. Two ways to get a gpui `Animation` from an
`Easing`:

- `animation(duration_ms)` — the curve installed in gpui's easing slot,
  **clamped** into `0..=1`. gpui debug-asserts easing output into that range,
  which overshooting curves (`Spring`, `EaseOutBack`, `EaseOutElastic`)
  violate by design — unclamped they abort any debug build. The clamp
  flattens overshoot peaks.
- `clock(duration_ms)` — the un-eased linear clock (springs still size it by
  `settle_seconds()`). Apply the curve yourself inside the animator, where
  overshoot is legal — this is what `Transition`/`Collapse`/`Presence` do,
  so their springs keep the full overshoot:

```rust
el.with_animation(id, easing.clock(200), move |el, t| {
    let delta = easing.apply(t);            // may pass 1.0 and settle back
    el.ml(px((1.0 - delta) * 8.0))          // offsets may overshoot
        .opacity(delta.clamp(0.0, 1.0))     // opacity must not
})
```

### Springs

`Spring { stiffness, damping }` is a closed-form damped oscillator — no
simulation loop. `damping < 2·√stiffness` overshoots and rings; more damping
approaches without crossing. Springs carry their own clock:
`settle_seconds()` says how long until it stays within 1% of the target, and
`Easing::Spring` ignores the surrounding `duration_ms` in favor of it.

Presets: `Spring::default()` (slight overshoot, fast settle),
`Spring::wobbly()` (visible ring), `Spring::stiff()` (no overshoot).

## Transition

Plays a one-shot entrance animation around its child. Give it a stable id so
the animation has identity.

```rust
Transition::new("hero")
    .kind(TransitionKind::SlideUp)
    .easing(Easing::Spring(Spring::default()))
    .duration_ms(220)
    .child(Card::new().child(content))
```

Methods: `new(id)`, `kind(TransitionKind)` (default `Fade`), `easing(Easing)`,
`duration_ms(u64)` (default `200`), `child(impl IntoElement)`.

`TransitionKind` is `Fade` | `SlideUp` | `SlideDown` | `SlideLeft` | `SlideRight`.

## Collapse

Reveals gated content. Give it the content height and it animates that height
**open and closed** — a real collapse, content clipped while moving:

```rust
Collapse::new("details")
    .open(self.expanded)
    .height(120.0)             // content height in px
    .easing(Easing::EaseInOutCubic)
    .child(detail_panel())
```

With a height, the child stays mounted at height 0 while closed so it can
animate back open. Without one, `Collapse` falls back to the old behavior:
fade in on open, unmount instantly on close.

Methods: `new(id)`, `open(bool)`, `height(f32)`, `easing(Easing)`,
`duration_ms(u64)` (default `180`), `child(impl IntoElement)`.

## Presence — exit animations

A stateless conditional (`if self.show { modal }`) can't animate out: the
element is gone the frame the flag flips. `Presence` is a small entity that
latches the element through its exit.

```rust
let presence = cx.new(|cx| {
    Presence::new(cx)
        .kind(TransitionKind::SlideUp)
        .duration_ms(160)
        .content(|_window, _cx| {
            Modal::new("settings").child(settings_form()).into_any_element()
        })
});

// open / close from handlers:
presence.update(cx, |p, cx| p.set_open(true, cx));
presence.update(cx, |p, cx| p.set_open(false, cx));  // plays exit, then unmounts
```

The content closure is re-invoked every frame while visible (live data, same
rule as Tabs/Accordion panels). `set_open(false)` plays the exit animation,
then stops rendering and emits `PresenceEvent::Hidden` — subscribe if you
need to clean up after the element is truly gone. Rapid toggles are safe:
each open/close bumps an internal epoch, so a reopen cancels a pending hide.

Methods: `content(fn(&mut Window, &mut App) -> AnyElement)`,
`kind(TransitionKind)`, `easing(Easing)`, `duration_ms(u64)` (default `180`),
`set_open(bool, cx)`, `toggle(cx)`, `is_open()`. Emits `PresenceEvent::Shown`
/ `PresenceEvent::Hidden`.

Wrap a `Modal`, `Drawer`, or any overlay in a `Presence` to give it an exit
animation — the overlay itself doesn't need to know.
