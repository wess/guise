# Layout (themed)

The `guise::layout` components use the theme's spacing tokens. They cover the
common cases; for a full Flutter-style box model see [Flex layout](flex.md), and
for terse construction see [Macros](macros.md).

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

## Space

A fixed gap on one axis, sized by the theme spacing scale — for the places a
parent `gap` doesn't cover.

```rust
Stack::new()
    .child(Title::new("Heading").order(3))
    .child(Space::y(Size::Md))
    .child(Text::new("Body copy."))
```

Methods: `Space::x(Size)` (horizontal, for rows), `Space::y(Size)` (vertical,
for columns). The block is `flex_none`, so flex parents never squash it.

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

## Container

A centered column with a capped width, for readable line lengths on any
window width. Implements `ParentElement`.

```rust
Container::new()
    .size(Size::Sm)
    .padding(Size::Md)
    .child(Title::new("Article").order(2))
    .child(Text::new("Readable line lengths on any window width."))
```

| Method | Default | Notes |
| --- | --- | --- |
| `size(Size)` | `Md` | max width: `Xs..Xl` → 540 / 720 / 960 / 1140 / 1320 px |
| `padding(Size)` | `Md` | horizontal padding inside the capped column |

> **Note** `guise::layout::Container` intentionally shares its name with
> [`guise::flex::Container`](flex.md), the Flutter-style pixel box — one more
> reason `flex` is not glob-exported. If both are in scope, name this one as
> `guise::layout::Container`.

## ScrollArea

A bounded, scrollable container — desktop UIs scroll, but most builders assume
their content fits. Each instance needs a unique id so gpui can track its scroll
offset. Implements `ParentElement`.

There are two ways to bound one, and which is right is a layout question rather
than a preference. `max_height` fixes the height in pixels — for a list that
takes a known slice of a larger layout:

```rust
ScrollArea::new("log")
    .max_height(240.0)
    .children(rows)
```

`fill` takes whatever the parent has left over instead — for a pane that should
be as tall as the window, where any fixed number is wrong at every size but one:

```rust
div().flex().flex_col().size_full()
    .child(header)
    .child(ScrollArea::new("settings").fill().children(sections))
```

### Scrollbar

A `ScrollArea` draws a scrollbar over its edge whenever the content overflows —
a draggable thumb, a click-to-jump track, and nothing at all while the content
fits. It floats rather than taking layout space, so `.scrollbar(false)` changes
nothing about where the content sits.

```rust
ScrollArea::new("log").max_height(240.0).scrollbar(false).children(rows)
```

For a container you scroll yourself, use `Scrollbar` directly: track the
container with a `ScrollHandle`, put it in a `.relative()` parent, and add the
bar beside it. A virtualized `list(state, ..)` has no handle, so
`Scrollbar::for_list` reads its `ListState` instead — the gallery does this for
its own page.

```rust
let handle = ScrollHandle::new();
div().relative().h(px(300.0))
    .child(div().id("feed").overflow_y_scroll().track_scroll(&handle).children(rows))
    .child(Scrollbar::new("feed-bar", &handle))

div().relative().flex_1()
    .child(list(state.clone(), render_row).size_full())
    .child(Scrollbar::for_list("page-bar", &state))
```

`.horizontal(true)` runs the bar along the bottom edge.

It works both as a flex child (it grows into the leftover main axis) and inside
a plain block parent, which is what a route body usually is. The parent still
has to be bounded itself — filling an unbounded parent sizes to the content, and
there is nothing to scroll. The two compose: `.fill().max_height(600.0)` grows
with the window but never past 600px.

Methods: `new(id)`, `max_height(f32)`, `fill()`, `horizontal(bool)` (scroll the
x axis instead).

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

## AppShell

The application frame: header, navbar, aside, and footer regions around a
scrollable main area. Regions take a fixed px size plus a content closure
re-invoked every render (live data, like Tabs panels); the main area is the
shell's children, laid out as a scrollable column. The shell fills its parent —
place it at the window root (or inside a sized box for a framed demo). Regions
get the theme surface background and a hairline border on their inner edge.
Implements `ParentElement` for the main area.

```rust
AppShell::new()
    .header(48.0, |_window, _cx| Text::new("guise"))
    .navbar(220.0, |_window, _cx| Text::new("nav links"))
    .footer(28.0, |_window, _cx| Text::new("status").size(Size::Xs))
    .child(Title::new("Main content").order(2))
```

| Method | Default | Notes |
| --- | --- | --- |
| `header(f32, closure)` | none | height in px; full width, top |
| `navbar(f32, closure)` | none | width in px; left column |
| `aside(f32, closure)` | none | width in px; right column |
| `footer(f32, closure)` | none | height in px; full width, bottom |

Region closures take `(&mut Window, &mut App)` and return any `IntoElement`.
Every region is a flex column with `overflow_hidden`, so oversized content
clips instead of breaking the frame.

## Breakpoints

Desktop windows resize like browser windows; `Breakpoint` and `Responsive`
make layout react declaratively. Thresholds are Tailwind-flavored: `Sm` 640,
`Md` 768, `Lg` 1024, `Xl` 1280 (narrower is `Xs`).

```rust
let bp = Breakpoint::from_window(window);      // read during render
let columns = Responsive::new(1).md(2).xl(4).resolve(bp);
SimpleGrid::new(columns).children(cards)

if bp.at_least(Breakpoint::Lg) { /* show the sidebar */ }
```

`Responsive<T>` is mobile-first: the base value applies from `Xs` up, each
override kicks in at its breakpoint **and above** (`.md(2)` also covers `Lg`
unless `.lg(..)`/`.xl(..)` says otherwise). `for_window(window)` combines the
read and the resolve. Because the value re-resolves every render, layouts
adapt live as the window resizes — no listeners to wire.
