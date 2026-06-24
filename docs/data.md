# Data display

`Avatar`, `AvatarGroup`, `Badge`, `Indicator`, `List`, `Table` are stateless
builders. `Tabs` and `Accordion` are stateful entities.

## Avatar

An initials badge (circle by default).

```rust
Avatar::new("AL").color(ColorName::Blue)
Avatar::new("GH").color(ColorName::Teal).variant(Variant::Filled).size(Size::Lg)
```

Methods: `new(initials)`, `color` (default `Gray`), `variant` (default `Light`),
`size` (default `Md`; dims xs 16 … xl 84), `radius` (omit for a full circle).

## AvatarGroup

Overlapping avatars with an optional `+N` overflow chip. Colors cycle through a
small palette automatically.

```rust
AvatarGroup::new()
    .avatars(["AL", "GH", "LT", "MK", "PR"])
    .limit(3)        // show 3, collapse the rest into "+2"
    .size(Size::Md)
```

Methods: `new()`, `avatar(initials)`, `avatars(iter)`, `size`, `limit(usize)`.

## Badge

A compact status pill.

```rust
Badge::new("New").color(ColorName::Teal)
Badge::new("Beta").variant(Variant::Outline).size(Size::Sm)
```

Methods: `new(label)`, `variant` (default `Light`), `color` (default `Blue`),
`size` (default `Md`).

## Indicator

A dot or count overlaid on a child's top-right corner.

```rust
Indicator::new(ThemeIcon::new("✉").color(ColorName::Grape)).label("3")
Indicator::new(Avatar::new("AL")).color(ColorName::Green)   // plain dot
```

Methods: `new(child)`, `label(impl Into<SharedString>)` (count; omit for a dot),
`color` (default `Red`), `disabled(bool)` (hide the indicator).

## List

A bulleted or numbered list of text items.

```rust
List::new().items(["First", "Second", "Third"])
List::new().ordered(true).item("Step one").item("Step two")
```

Methods: `new()`, `item(s)`, `items(iter)`, `ordered(bool)`, `size`,
`spacing(Size)`, `icon(glyph)` (custom bullet).

## Table

A simple table of string cells; columns size equally.

```rust
Table::new()
    .with_border(true)
    .striped(true)
    .highlight_on_hover(true)
    .head(["Name", "Role", "Status"])
    .row(["Ada", "Admin", "Active"])
    .row(["Grace", "Editor", "Active"])
```

Methods: `new()`, `head(iter)`, `row(iter)`, `striped(bool)`,
`highlight_on_hover(bool)`, `with_border(bool)`.

## Tabs (entity)

A tab bar with switchable panels. Panel content is a builder closure, re-invoked
each render so panels show live data.

```rust
let tabs = cx.new(|cx| {
    Tabs::new(cx)
        .tab("Overview", |_w, _cx| Text::new("Overview panel").dimmed())
        .tab("Members", |_w, _cx| Text::new("Members panel").dimmed())
        .active(0)
});
```

Methods: `new(cx)`, `tab(label, |window, app| content)`, `active(usize)`. Read
with `active_index()`.

## Accordion (entity)

Collapsible sections. Single-open by default; `multiple(true)` allows many.

```rust
let acc = cx.new(|cx| {
    Accordion::new(cx)
        .item("What is guise?", |_w, _cx| Text::new("A component library for gpui."))
        .item("Is it themeable?", |_w, _cx| Text::new("Yes — light/dark + the full palette."))
        .default_open(0)
});
```

Methods: `new(cx)`, `item(label, |window, app| content)`, `multiple(bool)`,
`default_open(usize)`.

> Panel content closures take `(&mut Window, &mut App)` and return any
> `IntoElement`. They run every frame, so keep them cheap and side-effect-free.
