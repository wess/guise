# Transitions

Mount animations built on gpui's animation API — the same `with_animation`
mechanism [`Loader`](feedback.md) uses, but one-shot instead of repeating.

> gpui has no transform/scale on elements, so motion is expressed through opacity
> and margin offsets. A true height-collapsing `Collapse` would need a measured
> content height; the one here fades.

## Transition

Plays a one-shot entrance animation around its child. Give it a stable id so the
animation has identity.

```rust
Transition::new("hero")
    .kind(TransitionKind::SlideUp)
    .duration_ms(220)
    .child(Card::new().child(content))
```

Methods: `new(id)`, `kind(TransitionKind)` (default `Fade`), `duration_ms(u64)`
(default `200`), `child(impl IntoElement)`.

`TransitionKind` is `Fade` | `SlideUp` | `SlideDown` | `SlideLeft` | `SlideRight`.

## Collapse

Reveals its child with a fade when `open`, renders nothing when closed. Useful
for accordions, inline detail panels, and expanding sections.

```rust
Collapse::new("details")
    .open(self.expanded)
    .child(Text::new("Hidden detail").dimmed())
```

Methods: `new(id)`, `open(bool)`, `duration_ms(u64)` (default `180`),
`child(impl IntoElement)`.
