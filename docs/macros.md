# Layout macros

Terse builders for the common containers. They're in the prelude, so
`use guise::prelude::*;` is all you need — the macros bring `.child()` into scope
themselves (no extra trait import).

Each macro takes comma-separated children; a trailing comma is fine.

## Containers

One macro **per container component** — every type that takes a variadic list of
children.

| Macro | Builds | Spacing |
| --- | --- | --- |
| `row![ … ]` | [`flex::Row`](flex.md#row--column) | none (use `SizedBox`/`Spacer`) |
| `col![ … ]` | [`flex::Column`](flex.md#row--column) | none |
| `zstack![ … ]` | [`flex::Stack`](flex.md#stack--positioned) (overlap) | — |
| `wrap![ … ]` | [`flex::Wrap`](flex.md#wrap) | default spacing |
| `vstack![ … ]` | [`layout::Stack`](layout.md#stack) (themed) | token gap |
| `hstack![ … ]` | [`layout::Group`](layout.md#group) (themed) | token gap |
| `center![ … ]` | [`layout::Center`](layout.md#center) | — |
| `paper![ … ]` | [`Paper`](layout.md#paper) | — |
| `card![ … ]` | [`Card`](layout.md#card) | — |
| `modal![ … ]` | [`Modal`](overlays.md#modal) | — |

```rust
use guise::prelude::*;

col![
    row![avatar, name, Spacer::new(), actions],
    SizedBox::height(8.0),
    body,
]
```

Because a macro returns the underlying builder, you can keep chaining:

```rust
row![left, right].main_axis_alignment(MainAxisAlignment::SpaceBetween)
```

## Component shorthands

A few of the most common leaf components have shorthand macros too. They expand
to `Type::new(...)`, so every builder method still chains.

| Macro | Builds | Notes |
| --- | --- | --- |
| `text!(...)` | [`Text`](typography.md#text) | accepts `format!` args |
| `title!(...)` | [`Title`](typography.md#title) | accepts `format!` args |
| `code!(...)` | [`Code`](typography.md#code) | accepts `format!` args |
| `kbd!(...)` | [`Kbd`](typography.md#kbd) | accepts `format!` args |
| `button!(id, label)` | [`Button`](buttons.md#button) | forwards args |
| `badge!(label)` | [`Badge`](data.md#badge) | forwards args |

The content macros take `format!`-style arguments, which is the real win over
the plain constructor:

```rust
text!("Signed in as {name}")          // = Text::new(format!("Signed in as {name}"))
title!("Page {}", n).order(2)
button!("save", "Save").variant(Variant::Filled).color(ColorName::Blue)
```

This is a deliberately small set. Most components **don't** get a macro: for a
builder with several setters, `Type::new(...)` chained with methods is already
the clearest form, and stateful entities (`TextInput`, `Select`, …) are created
with `cx.new(...)` where a macro doesn't fit. The shorthands exist only where
they genuinely read better.

## `color!` — CSS color literals

`color!` produces a gpui `Hsla` from CSS notation. See
[Theming → CSS-style colors](theming.md#css-style-colors).

```rust
color!(rgb(34, 139, 230))      color!(rgba(34, 139, 230, 0.5))
color!(hsl(210, 80, 52))       color!(teal)        color!("#228be6")
```

## `style!` — CSS-like style blocks

`style!` expands to an element transform you apply with `.apply(...)` (from the
`StyleExt` trait, in the prelude). It maps CSS-ish properties onto gpui's builder
methods, so a block of declarations reads like a stylesheet.

```rust
use guise::prelude::*;

gpui::div().apply(style! {
    display: flex;
    direction: column;
    align: center;
    justify: between;
    gap: 8;
    padding: 16;
    width: full;
    height: 200;
    radius: 12;
    background: "#11151c";              // string → css() shorthand
    color: color!(rgb(230, 230, 230));  // or any color! / Hsla expr
    border: color!("#2a2f3a");          // 1px border of this color
    weight: semibold;
    opacity: 0.95;
})
```

- **Numbers are pixels.** `padding: 16` → `.p(px(16.))`.
- **Colors** are a string literal (parsed by `css`) or any `Into<Hsla>` expression
  (e.g. `color!(..)`).
- **Every declaration ends with `;`.**
- **No theme tokens.** `style!` is pure and has no `cx`, so `Size::Md`-based
  spacing/radius/font aren't available — use raw px here, or the builder methods
  (which read the theme) for token values.

Supported properties: `background`, `color`, `border`; `display: flex`;
`direction: row|column|col`; `align: start|center|end|stretch`;
`justify: start|center|end|between|around|evenly`; `position: absolute|relative`;
`weight: bold|semibold|medium|normal`; `width`/`height` (`full` or px), `size`,
`min_width`, `min_height`, `padding`/`px`/`py`/`pt`/`pr`/`pb`/`pl`,
`margin`/`mx`/`my`/`mt`/`mr`/`mb`/`ml`, `radius`, `gap`, `font_size`, `opacity`.

Because it's just a transform, it composes with everything: keep chaining
interactive methods (`.id(..)`, `.on_click(..)`, `.hover(..)`) after `.apply(..)`.

## Why `col!`, not `column!`

The standard library already exports a `column!` macro (it returns the current
source column number). Naming ours `col!` avoids the clash when both are in
scope via globs.

## How they stay import-free

The macros expand to e.g. `flex::Row::new().child(a).child(b)`. `.child()` comes
from gpui's `ParentElement` trait, which the macro brings into scope anonymously
through a hidden re-export (`guise::__ParentElement`). You never have to import
the trait yourself.
