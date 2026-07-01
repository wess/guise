# Feedback

`Alert`, `Loader`, `Progress`, `RingProgress`, `Notification`, `Skeleton` are
stateless builders. `ToastStack` is a stateful entity that manages a positioned
stack of toasts.

## Alert

An inline colored callout for info / success / warning / error.

```rust
Alert::new("Your changes have been saved.")
    .title("Success")
    .color(ColorName::Teal)
    .icon("✓")
    .on_close(cx.listener(|this, _ev, _w, cx| { this.show_alert = false; cx.notify(); }))
```

| Method | Default |
| --- | --- |
| `new(message)` | — |
| `title(impl Into<SharedString>)` | none |
| `variant(Variant)` | `Light` |
| `color(ColorName)` | `Blue` |
| `icon(impl Into<SharedString>)` | none (leading glyph) |
| `on_close(handler)` | none (adds an `×`) |

Filled variants render all text on the fill; light/outline keep readable body text.

## Loader

An animated busy indicator (driven by gpui's animation API).

```rust
Loader::new().color(ColorName::Blue)                       // dots
Loader::new().variant(LoaderVariant::Bars).size(Size::Lg)  // bars
```

| Method | Default |
| --- | --- |
| `variant(LoaderVariant)` | `Dots` (`Dots` \| `Bars`) |
| `size(Size)` | `Md` |
| `color(ColorName)` | `Blue` |

> The animated units carry stable element ids. If you place several loaders as
> direct siblings, wrap each in an id'd parent (`div().id("loader-a").child(...)`)
> so the ids stay unique.

## Progress

A determinate bar; `value` is a percentage in `0.0..=100.0`.

```rust
Progress::new(60.0).color(ColorName::Teal).size(Size::Md)
```

| Method | Default |
| --- | --- |
| `new(value)` | clamped 0–100 |
| `color(ColorName)` | `Blue` |
| `size(Size)` | `Md` (heights: xs 4 … xl 16) |
| `radius(Size)` | half the height (pill) |

The bar is `w_full`; place it in a stretched container.

## RingProgress

A circular determinate gauge with a centered label.

```rust
RingProgress::new(72.0).size(96.0).color(ColorName::Teal)
RingProgress::new(40.0).label("4/10")
```

| Method | Default |
| --- | --- |
| `new(value)` | clamped 0–100 |
| `size(f32)` | `80.0` (diameter) |
| `color(ColorName)` | `Blue` |
| `label(impl Into<SharedString>)` | rounded percentage |

> gpui has no arc/conic primitive, so the fill is rendered as a clipped column
> rising from the bottom of the circle (a gauge), not a stroked ring. A true ring
> would need a custom `canvas` paint pass.

## Notification

An elevated toast card with an accent bar. Positioning/stacking is the host's
job — this is the visual card.

```rust
Notification::new("Deployment finished in 42s.")
    .title("Build complete")
    .color(ColorName::Teal)
    .icon("✓")
    .on_close(cx.listener(|this, _, _, cx| { /* dismiss */ }))
```

Methods: `new(message)`, `title`, `color` (default `Blue`), `icon`, `on_close`.

## ToastStack (entity)

A toast manager: holds a list of live toasts and paints them as a deferred,
top-right stack above the page. Hold the entity, render it in a full-size root,
and push from anywhere.

```rust
let toasts = cx.new(|_| ToastStack::new());               // auto-dismiss after 4s
let sticky = cx.new(|_| ToastStack::new().duration(None)); // keep until closed
// later, from a handler:
toasts.update(cx, |t, cx| {
    t.push_titled("Saved", "Your changes were saved.", ColorName::Teal, cx);
});
```

Methods: `new()`, `duration(Option<Duration>)` (chainable) /
`set_duration(...)` (on a built stack, e.g. inside `entity.update(cx, ...)`),
`push(message, cx) -> id`, `push_titled(title, message, color, cx) -> id`,
`remove(id, cx)`, `clear(cx)`, `len()`, `is_empty()`.

> **Note** Toasts auto-dismiss 4 seconds after being pushed (a timer spawned
> per push — the delay in force *at push time* is the one that applies; pass
> `duration(None)` to keep toasts until closed). Each card also has a close
> button, and ids are never reused, so a hand-closed toast can't be
> double-removed by its timer.

## Skeleton

An animated loading placeholder.

```rust
Skeleton::new().height(14.0).width(220.0)   // a line
Skeleton::new().circle(40.0)                // a circle (e.g. avatar)
```

| Method | Default |
| --- | --- |
| `width(f32)` | full width when unset |
| `height(f32)` | `16.0` |
| `radius(Size)` | `Sm` |
| `circle(size)` | makes a circle of `size` |
