# Layout (themed)

The `guise::layout` components use the theme's spacing tokens. They cover the
common cases; for a full Flutter-style box model see [Flex layout](flex.md), and
for terse construction see [Layout macros](macros.md).

All container components implement `ParentElement`, so `.child(...)` /
`.children(...)` work.

## Stack

A vertical flex column with token-based spacing.

```rust
Stack::new()
    .gap(Size::Md)
    .align(Align::Stretch)
    .child(Title::new("Heading").order(3))
    .child(Text::new("Body").dimmed())
```

| Method | Default |
| --- | --- |
| `gap(Size)` | `Md` |
| `align(Align)` | `Stretch` |
| `justify(Justify)` | `Start` |

## Group

A horizontal flex row.

```rust
Group::new()
    .justify(Justify::Between)
    .child(Title::new("Title").order(4))
    .child(Button::new("more", "More"))
```

| Method | Default |
| --- | --- |
| `gap(Size)` | `Md` |
| `align(Align)` | `Center` |
| `justify(Justify)` | `Start` |
| `wrap(bool)` | `true` |
| `grow(bool)` | `false` — stretch children to share width |

## Center

Centers its children on both axes.

```rust
Center::new().child(Loader::new())
```

`inline(bool)` (default `false`) shrinks to content instead of filling the parent.

## Align & Justify enums

```rust
pub enum Align { Start, Center, End, Stretch }
pub enum Justify { Start, Center, End, Between, Around }
```

## SimpleGrid

Equal-width columns that wrap into rows. gpui's flexbox has no CSS-grid track
system, so this lays children out as a column of flex rows, each holding up to
`cols` equal-weight cells (the final short row is padded so columns stay aligned).
Implements `ParentElement`.

```rust
SimpleGrid::new(3)
    .spacing(Size::Md)
    .child(card_a)
    .child(card_b)
    .child(card_c)
    .child(card_d)
```

Methods: `new(cols)`, `spacing(Size)` (default `Md`).

## ScrollArea

A bounded, scrollable container — desktop UIs scroll, but most builders assume
their content fits. Each instance needs a unique id so gpui can track its scroll
offset. Implements `ParentElement`.

```rust
ScrollArea::new("log")
    .max_height(240.0)
    .children(rows)
```

Methods: `new(id)`, `max_height(f32)`, `horizontal(bool)` (scroll the x axis
instead).

## Paper

A raised surface: themed background, radius, padding, optional border and shadow.

```rust
Paper::new()
    .padding(Size::Lg)
    .radius(Size::Md)
    .with_border(true)
    .shadow(Size::Sm)
    .child(content)
```

| Method | Default |
| --- | --- |
| `padding(Size)` | `Md` |
| `radius(Size)` | theme default |
| `with_border(bool)` | `false` |
| `shadow(Size)` | none |

## Card

A `Paper` preset: bordered, padded, lightly raised, column layout.

```rust
Card::new().child(
    Stack::new().gap(Size::Sm).child(Title::new("Title").order(4)).child(body),
)
```

Defaults: `padding` `Lg`, `radius` `Md`, `with_border` `true`, `shadow` `Sm`.
Same setter methods as `Paper`.

## Divider

A separating line; horizontal by default.

```rust
Divider::new()                       // full-width hairline
Divider::new().label("Section")      // centered label
Divider::vertical()                  // 1px tall divider for rows
```

`Orientation` is `Horizontal` | `Vertical`.
