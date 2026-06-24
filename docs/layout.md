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
