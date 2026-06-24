# Flex layout (`guise::flex`)

A Flutter-flavored layout kit on top of gpui's flexbox. Use it when you think in
`Row`/`Column`/`Container`/`Expanded` rather than themed `Stack`/`Group`.

Several names (`Row`, `Column`, `Stack`, `Center`, `Align`) overlap with
`guise::layout` and the prelude, so this module is **not** glob-exported. Import
it explicitly:

```rust
use guise::flex::*;
```

All container types implement `ParentElement` (`.child` / `.children`).

## Row & Column

```rust
Column::new()
    .cross_axis_alignment(CrossAxisAlignment::Stretch)
    .gap(12.0)
    .child(Row::new().child(Expanded::new(header)).child(actions))
    .child(body)
```

| Method | Row default | Column default |
| --- | --- | --- |
| `main_axis_alignment(MainAxisAlignment)` | `Start` | `Start` |
| `cross_axis_alignment(CrossAxisAlignment)` | `Center` | `Center` |
| `main_axis_size(MainAxisSize)` | `Max` | `Min` |
| `gap(f32)` | `0` | `0` |

`MainAxisSize::Max` fills the main axis (so `SpaceBetween` has room to work);
`Min` shrinks to content. (Column defaults to `Min` — more predictable inside a
scrolling page than Flutter's `Max`.)

```rust
pub enum MainAxisAlignment { Start, End, Center, SpaceBetween, SpaceAround, SpaceEvenly }
pub enum CrossAxisAlignment { Start, End, Center, Stretch, Baseline }
pub enum MainAxisSize { Min, Max }
```

## Expanded, Flexible, Spacer

Inside a `Row`/`Column`:

```rust
Row::new()
    .child(Expanded::new(left).flex(2.0))   // takes 2 shares
    .child(Expanded::new(right).flex(1.0))  // takes 1 share
```

- **`Expanded::new(child).flex(f32)`** — fills its weighted share of the main axis
  (zero basis). Default weight `1.0`.
- **`Flexible::new(child).flex(f32)`** — grows up to its content.
- **`Spacer::new().flex(f32)`** — an empty element that pushes siblings apart.

## SizedBox

A fixed box, often used as a gap.

```rust
SizedBox::height(16.0)              // vertical gap
SizedBox::width(8.0)               // horizontal gap
SizedBox::square(40.0).child(icon)
SizedBox::expand()                 // fills both axes
SizedBox::new().with_width(100.0).with_height(40.0).child(thing)
```

An unset dimension is left to size naturally; only `expand()` fills.

## Container

A configurable box: size, padding, margin, color, radius, border, and child
alignment.

```rust
Container::new()
    .padding(EdgeInsets::all(12.0))
    .radius(8.0)
    .color(theme(cx).color(ColorName::Blue, 1))
    .alignment(Alignment::Center)
    .child(Text::new("Boxed").bold())
```

| Method | Notes |
| --- | --- |
| `child(el)` | single child |
| `width(f32)` / `height(f32)` | fixed size |
| `padding(EdgeInsets)` / `margin(EdgeInsets)` | |
| `color(Color)` | a concrete `Color` (resolve from the theme if you want it themed) |
| `radius(f32)` | |
| `border(width, Color)` | |
| `alignment(Alignment)` | centers/aligns the child via flex |

## Padding, Align, Center

```rust
Padding::all(12.0).child(content)
Padding::symmetric(16.0, 8.0).child(content)
Padding::only(EdgeInsets::only(0.0, 8.0, 0.0, 8.0)).child(content)

Align::new(Alignment::TopRight).child(badge)
Center::new().child(spinner)
```

## EdgeInsets

```rust
EdgeInsets::all(8.0)
EdgeInsets::symmetric(16.0, 8.0)   // horizontal, vertical
EdgeInsets::only(top, right, bottom, left)
EdgeInsets::horizontal(12.0)
EdgeInsets::vertical(6.0)
```

## Alignment

```rust
pub enum Alignment {
    TopLeft, TopCenter, TopRight,
    CenterLeft, Center, CenterRight,
    BottomLeft, BottomCenter, BottomRight,
}
```

## Stack & Positioned

Overlap children on the z-axis. The first child sits in normal flow and defines
the size; wrap overlays in `Positioned`.

```rust
Stack::new()
    .child(banner)                                  // base, defines size
    .child(Positioned::new(close_button).top(8.0).right(8.0))
    .child(Positioned::fill(scrim))                 // pinned to all edges
```

`Positioned` methods: `new(child)`, `fill(child)`, `top/right/bottom/left(f32)`,
`width/height(f32)`.

## Wrap

A row that wraps onto new lines.

```rust
Wrap::new().spacing(8.0).children(tags)
```

`spacing(f32)` is the gap on both axes.
