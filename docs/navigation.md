# Navigation

`Breadcrumbs`, `NavLink`, `Stepper`, `StatusBar` are stateless builders;
`Pagination` is a stateful entity.

## Breadcrumbs

A trail of locations; the last item is rendered as the current location. Build
clickable ancestors with `link(label, handler)` — dimmed until hovered — and
inert ones with `item`/`items`.

```rust
Breadcrumbs::new()
    .link("Home", cx.listener(|this, _ev, _w, cx| { this.page = Page::Home; cx.notify(); }))
    .link("Projects", cx.listener(|this, _ev, _w, cx| { this.page = Page::Projects; cx.notify(); }))
    .item("guise")

Breadcrumbs::new().items(["Home", "Projects", "guise"]).separator("›")  // all inert
```

Methods: `new()`, `item(s)` (inert), `link(label, handler)` (handler is
`Fn(&ClickEvent, &mut Window, &mut App)`, so `cx.listener` works directly),
`items(iter)` (bulk inert items), `separator(s)` (default `/`).

> **Note** The last item is never clickable, even when built with `link` — its
> handler is ignored, since the tail of the trail is the page you're already on.

## NavLink

A sidebar navigation row with active state. Build a sidebar by folding a list:

```rust
let links = ["Dashboard", "Components", "Settings"];
let sidebar = links.iter().enumerate().fold(Stack::new().gap(Size::Xs), |stack, (i, label)| {
    stack.child(
        NavLink::new(("nav", i), *label)
            .icon(IconName::House)
            .active(self.active == i)
            .on_click(cx.listener(move |this, _, _, cx| { this.active = i; cx.notify(); })),
    )
});
```

| Method | Default |
| --- | --- |
| `new(id, label)` | — |
| `description(s)` | none (second line) |
| `icon(impl Into<Glyph>)` | none (a Lucide `IconName` or text) |
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

## NavigationMenu (entity)

A horizontal top-nav. Items are leaves (click → event) or menus (click opens
a dropdown of entries); the active item — or the owner of the active entry —
gets a primary tint.

```rust
let nav = cx.new(|cx| {
    NavigationMenu::new(cx)
        .item("home", "Home")
        .menu("docs", "Docs", [("tutorial", "Tutorial"), ("api", "API")])
        .item("about", "About")
        .active("home")
});
cx.subscribe(&nav, |_this, _nav, NavigationMenuEvent(id), _cx| {
    // route on id: "home", "tutorial", "api", "about"
})
.detach();
```

Methods: `item(id, label)`, `menu(id, label, entries)`, `active(id)` at
construction, `set_active(id, cx)` at runtime (e.g. after routing). Picking
an entry closes the dropdown and moves the highlight.
