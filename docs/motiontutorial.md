# Tutorial: animate a release checklist

Nine short chapters that build one animated panel — a release checklist that
runs itself. Each chapter adds one idea from [Motion &
transitions](transitions.md), and the last one is the whole file.

The finished app is in the repository:

```sh
cargo run -p guise-ui --example checklist
```

Every snippet below is lifted from it, so what you read is what compiles.

## What you're building

A card that slides in. Five rows that arrive one after another. A **Run**
button that starts a single playhead, which every row reads to decide whether
it is waiting, working, or done — the working one breathing while it works. A
progress bar that fills off the same clock. A **shipped** badge that slides in
at the end, and slides back *out* when you rewind.

Nine ideas, in the order you need them:

| Chapter | Idea |
| --- | --- |
| 1 | A first entrance, on the element's own box |
| 2 | Easing, and picking one on purpose |
| 3 | Staggering a list |
| 4 | Keyframes: more than two states |
| 5 | A playhead you own |
| 6 | Reading values that aren't styles |
| 7 | Animating only sometimes |
| 8 | Exits, which need a latch |
| 9 | The whole file, and what it costs |

Everything assumes `use guise::prelude::*;`.

## 1. A first entrance

The panel is a `div` with some children. To animate it, you don't wrap it —
you animate the box it already is:

```rust
div()
    .w(px(360.0))
    .p(px(18.0))
    .rounded(px(12.0))
    .bg(surface)
    .child(Title::new("Release checklist").order(4))
    .animate("panel", motion! {
        enter: slide_up 20;
        duration: 420;
    })
```

`.animate(id, clip)` comes from the `Motioned` trait and is in the prelude. It
plays the clip once, from the moment the element first lays out.

**Why not a wrapper?** There is one — `Animated::new(id).motion(..).child(..)`
— and it exists for children that aren't styleable. But a wrapper is a new
flex item: a `w_full` child suddenly measures against the wrapper instead of
the row it was in. Animating the box you already have changes nothing about
the layout, which means turning an animation on can never move anything.

`motion! { … }` is the declaration block. It is to an animation what
[`style!`](macros.md#style--css-like-style-blocks) is to a box:

```rust
motion! {
    enter: slide_up 20;     // a preset, and how far it travels
    duration: 420;          // ms
}
```

`enter:` picks the constructor, so it comes first if you use one. The presets
are `fade`, `slide_up`, `slide_down`, `slide_left`, `slide_right`, and each
has an `exit:` twin.

> **Replaying** A mounted one-shot has already run. The only way to play it
> again is to hand the element a **new id** — which is why the finished app
> keeps an `epoch: usize` and animates `("panel", self.epoch)`. Bumping the
> epoch is what the "Replay entrance" button does.

## 2. Easing

`duration` says how long; easing says what it feels like. The default is a
soft ease-out, and most of the time that is the right answer. When it isn't:

```rust
motion! {
    enter: slide_up 20;
    duration: 420;
    ease: out back;      // overshoots the target and settles
}
```

An easing is a **direction** and a **shape**:

```rust
ease: in quad;       ease: out cubic;     ease: in_out sine;
```

Shapes: `quad`, `cubic`, `quart`, `quint`, `sine`, `expo`, `circ`, `back`,
`elastic`, `bounce`. Three more stand alone — `linear`, `spring`, and
`steps(4)` — and any [`Easing`](transitions.md#easing) expression works if you
want a `CubicBezier`.

Two rules of thumb. Things **entering** want `out` (fast, then settle); things
**leaving** want `in` (drift, then go). `back` and `elastic` overshoot, which
reads as physical on something small and as broken on something large.

The panel uses `out back` because a card that overshoots by a couple of pixels
feels like it has weight.

## 3. Staggering the rows

Five rows arriving together is a slideshow. Five rows arriving 70ms apart is a
list being dealt.

Stagger is not a timeline feature here. One element is one clip, so staggering
is a function from an index to a delay, which you fold into each row's own
motion:

```rust
let rise = Stagger::new(70.0).start(120.0);   // 70ms apart, after 120ms

for index in 0..STEPS.len() {
    row.animate(("row", index), motion! {
        enter: slide_left 18;
        duration: 380;
        ease: out back;
        delay: rise.at(index, STEPS.len());
    })
}
```

`start(120.0)` holds everyone back long enough for the panel itself to land
first.

That independence is the point: the list can grow, shrink or reorder
mid-flight and nothing restarts, because no row is waiting on any other. It
also means a delayed row is genuinely **invisible until its turn** — during a
leading `delay`, every track reports its *starting* value, so a row cannot
flash at full opacity and then animate.

`Stagger` does more than a straight line when you need it — `from(Center)` to
ripple outward, `grid(cols, rows)` to measure distance in two dimensions,
`ease(..)` to bunch the early ones up. See
[Stagger](transitions.md#stagger).

## 4. Keyframes

Two states is `from => to`. More than two is a list:

```rust
motion! {
    duration: 900;
    ease: in_out quad;
    y: 0 => [-30, 0];        // up, then back
    radius: 6 => [26, 6];
}
```

Legs with no duration of their own split whatever the motion's `duration` has
left, so `[-30, 0]` is 450ms each. When one leg needs its own timing, put a
`Keyframe` in the list instead of a bare number:

```rust
bg: soft => [
    Keyframe::to(accent).duration(500.0),
    Keyframe::to(soft).ease(Easing::Out(Curve::Expo)),
];
```

A leg longer than the whole motion stretches it rather than being cut off.

## 5. A playhead you own

Everything so far fires once, on mount, and forgets. A **Run** button needs
something you can play, pause and rewind — so the clock moves into an entity:

```rust
let run = cx.new(|cx| {
    Animator::new(
        motion! {
            duration: PER_STEP * STEPS.len() as f32;
            ease: linear;
            custom("step"): 0 => STEPS.len() as i32;
            w: 0 => TRACK;
        },
        cx,
    )
});
```

Then in `render`:

```rust
let frame = self.run.read(cx).frame(window);
let playing = self.run.read(cx).is_playing();
```

and from a handler:

```rust
this.run.update(cx, |run, cx| run.toggle(cx));
```

`play`, `pause`, `toggle`, `restart`, `stop`, `seek(ms)`,
`seek_progress(0..=1)`, `reverse`, `set_speed`. It emits
`AnimatorEvent::Begin` and `AnimatorEvent::Complete`.

**There is no per-frame callback**, and you don't want one: your `render`
already runs every frame, so reading `frame(window)` there *is* the update
hook. That call is also what asks the window for the next frame while the clip
is still moving — the whole repaint loop, in one line. Nothing ticks and
nothing mutates per frame, so a paused animation costs nothing at all.

## 6. Values that aren't styles

The clip above tweens two things. `w` is a real style — it fills the progress
bar. `custom("step")` is not a style at all: it is a number counting 0 → 5 that
the frame carries for you to read.

```rust
let reached = frame.number_or(Prop::Custom("step"), 0.0);
let filled = frame.number_or(Prop::Width, 0.0);

let done = reached >= index as f32 + 1.0;
let working = !done && reached > index as f32;
```

One playhead, five rows, and no per-row state anywhere. Each row asks the same
frame where the run has got to and styles itself accordingly. Rewinding works
for free, because "where the run has got to" is the only fact in play.

That is what `Prop::Custom` is for — a counter, a scroll offset, a chart's
value, anything you want interpolated but intend to apply yourself.
`Prop::Rotate` and `Prop::Scale` are the same deal: gpui can only transform an
`Image` or an `Svg`, so the frame carries the number and you hand it to that
component's own `Transformation`.

## 7. Animating only sometimes

The working row breathes. The other four must not:

```rust
div()
    .w(px(8.0))
    .h(px(8.0))
    .rounded(px(999.0))
    .bg(if done || working { accent } else { dimmed })
    .animate_when(working, ("pulse", index), motion! {
        duration: 700;
        ease: in_out sine;
        repeat: forever;
        alternate;
        opacity: 1 => 0.3;
    })
```

`repeat: forever` with `alternate` is a breath: out, back, out, back.

`animate_when` exists because the obvious spelling cannot work.
`.when(cond, |el| el.animate(..))` fails to compile — `animate` changes the
element's type and `when` has to hand back the type it was given.
`animate_when` keeps the type stable by running an empty clip when the
condition is false, and gives the two states different element ids, so the
clip starts from its beginning each time the condition turns on.

## 8. Exits need a latch

`if done { badge }` cannot animate out. The frame the flag flips, the element
is gone — there is nothing left to animate.

[`Presence`](transitions.md#presence--exit-animations) is the latch. It holds
the element through its exit, then stops rendering:

```rust
let shipped = cx.new(|cx| {
    Presence::new(cx)
        .kind(TransitionKind::SlideLeft)
        .duration_ms(220)
        .content(|_window, cx| {
            let color = theme(cx).color(ColorName::Teal, 5).hsla();
            div().px(px(9.0)).py(px(3.0)).rounded(px(999.0)).bg(color)
                .child("shipped")
                .into_any_element()
        })
});
```

Drive it from the playhead rather than a timer, so rewinding takes it away
again:

```rust
cx.subscribe(&run, |this, _run, event: &AnimatorEvent, cx| {
    let done = matches!(event, AnimatorEvent::Complete);
    this.shipped.update(cx, |badge, cx| badge.set_open(done, cx));
})
.detach();
```

The content closure runs every frame while visible, so the badge reads the
live theme — the same rule `Tabs` and `Accordion` panels follow.

## 9. The whole thing, and what it costs

```sh
cargo run -p guise-ui --example checklist
```

Roughly 230 lines, and the animation part of it is the six `motion!` blocks
you have already read.

Three things worth carrying out of it.

**An endless clip pins a core.** `repeat: forever` asks the window for a frame
forever, and it will keep doing that off-screen, in a background tab, and while
the user is reading something else. That is why the pulse is scoped to the one
row that is working. Use looping motion for a hint, never for a screen.

**Offsets are insets, except when they can't be.** `x` and `y` are relative
insets: the element moves and its neighbours do not care. But an element
pinned with `absolute()` *is* its inset, so animating one would drag it off its
pin. Add the `margins;` flag and the offsets become margins instead — same
visible motion, no fight. (Tailor emits exactly this for a node in a free-form
container.)

**Reach for the narrow tools first.** A fade or a slide on mount is
[`Transition`](transitions.md#transition). Revealing gated content at its real
height, both directions, is [`Collapse`](transitions.md#collapse). Neither
needs any of this. `motion!` earns its place when there is more than one
property, more than two states, or a clock somebody has to hold.

## Where to go next

- [Motion & transitions](transitions.md) — the full reference: every easing,
  every prop, sequences, and the `Animator` API.
- [Macros](macros.md#motion--animation-as-a-declaration-block) — `motion!` and
  `sequence!` in detail.
- [Tailor: the canvas](https://github.com/wess/tailor/blob/main/docs/canvas.md#motion) — the same motions, set on a
  node in the interface builder, generating the same code.
- `cargo run -p guise-ui --example motion` — a second demo, aimed at the API
  rather than at a screen.
