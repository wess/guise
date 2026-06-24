# Navigation

`Breadcrumbs`, `NavLink`, `Stepper`, `StatusBar` are stateless builders;
`Pagination` is a stateful entity.

## Breadcrumbs

A trail of locations; the last item is rendered as the current location.

```rust
Breadcrumbs::new().items(["Home", "Projects", "guise"])
Breadcrumbs::new().items(["a", "b"]).separator("›")
```

Methods: `new()`, `item(s)`, `items(iter)`, `separator(s)` (default `/`).

## NavLink

A sidebar navigation row with active state. Build a sidebar by folding a list:

```rust
let links = ["Dashboard", "Components", "Settings"];
let sidebar = links.iter().enumerate().fold(Stack::new().gap(Size::Xs), |stack, (i, label)| {
    stack.child(
        NavLink::new(("nav", i), *label)
            .icon("•")
            .active(self.active == i)
            .on_click(cx.listener(move |this, _, _, cx| { this.active = i; cx.notify(); })),
    )
});
```

| Method | Default |
| --- | --- |
| `new(id, label)` | — |
| `description(s)` | none (second line) |
| `icon(s)` | none |
| `color(ColorName)` | `Blue` (active tint) |
| `active(bool)` | `false` |
| `on_click(handler)` | — |

## Stepper

A horizontal progress indicator. `active` is the current step; earlier steps
render as completed (with a check), later ones as pending.

```rust
Stepper::new()
    .step_desc("Account", "Create account")
    .step_desc("Profile", "Add details")
    .step("Review")
    .active(1)
    .color(ColorName::Teal)
```

Methods: `new()`, `step(label)`, `step_desc(label, description)`, `active(usize)`,
`color(ColorName)`.

## Pagination (entity)

A page selector that owns its current page and renders a windowed list with
ellipses (`1 … 4 5 6 … 20`) plus prev/next arrows.

```rust
let pages = cx.new(|cx| Pagination::new(cx, 10).active(1).color(ColorName::Blue));
```

Methods: `new(cx, total)`, `active(usize)`, `color`. Read with `active_page()`.
Emits `PaginationEvent(usize)` (the 1-based page).

## StatusBar

A themed app status bar with left / center / right slots — the bottom shell most
desktop apps want.

```rust
StatusBar::new()
    .left(Text::new("guise gallery").size(Size::Xs))
    .left(Badge::new("Dark").size(Size::Sm).color(ColorName::Grape))
    .center(Text::new("Ready").size(Size::Xs).dimmed())
    .right(Text::new("v0.1.0").size(Size::Xs).dimmed())
```

Methods: `new()`, `height(f32)` (default 28), `left(el)`, `center(el)`,
`right(el)` (each appends to that slot).

### Pinning it to the bottom

Make the window root a flex column with a scrolling body above the bar:

```rust
div().relative().size_full().flex().flex_col()
    .child(div().id("scroll").flex_1().min_h(px(0.0)).overflow_y_scroll().child(content))
    .child(status_bar)
```

`min_h(px(0.0))` lets the scroll area shrink inside the column so it scrolls
instead of pushing the bar off-screen.
