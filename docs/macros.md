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

## Why `col!`, not `column!`

The standard library already exports a `column!` macro (it returns the current
source column number). Naming ours `col!` avoids the clash when both are in
scope via globs.

## How they stay import-free

The macros expand to e.g. `flex::Row::new().child(a).child(b)`. `.child()` comes
from gpui's `ParentElement` trait, which the macro brings into scope anonymously
through a hidden re-export (`guise::__ParentElement`). You never have to import
the trait yourself.
