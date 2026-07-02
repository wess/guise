# Tutorial: build a data workbench

A single, end-to-end walk through `guise`: eleven chapters that grow one app — a
SQL data workbench — from an empty window to a themed, keyboard-driven desktop
tool. Along the way you will compose stateless builders (`Button`, `Panel`,
`StatusBar`, `Checkbox`, `Rating`, `Modal`, `ConfirmModal`, `LoadingOverlay`,
`Sparkline`, `BarChart`) and stateful entities (`TextInput`, `TableView`,
`DataView`, `TreeView`, `SplitPanel`, `TabBar`, `Editor`, `MenuBar`,
`ContextMenu`, `ToastStack`, `Spotlight`), wired together with the reactive
layer (`Signal`, `Binding`, `use_memo`, `FormState`).

## What you're building

The workbench is a small but real desktop app: an
[`AppShell`](layout.md#appshell) frame (header with a `MenuBar` and theme
toggle, navbar, `StatusBar` footer); a sidebar `TreeView` driving which view
shows; a `SplitPanel` main area with a tabbed SQL `Editor` above a sortable
`TableView` of results; a `DataView` of saved queries filtered live through a
bound `TextInput`; dialogs, context menus, toasts, and a loading overlay for
feedback; a settings `Panel` of bound inputs with `FormState` validation; and a
`Spotlight` command palette plus a charts summary to finish.

### The two component patterns, again

Everything in this tutorial leans on the split described in
[Component model](components.md):

1. **Stateless builders** ([`RenderOnce`](components.md#1-stateless-builders-renderonce)) —
   values you construct fresh every render and hand to `.child(...)`. The
   parent owns any state; *controlled* inputs report changes through a handler
   or a [`Binding`](reactive.md#bindings).
2. **Stateful entities** ([`Render` + events](components.md#2-stateful-entities-render--events)) —
   components that own a buffer, focus handle, or open state. Create them once
   with `cx.new(...)`, store the `Entity`, render it as a child, and subscribe
   to its typed events with `cx.subscribe(...).detach()`.

Component headings below carry an "(entity)" suffix when they follow pattern 2,
matching the rest of the docs.

### How to read this tutorial

Every snippet assumes `use guise::prelude::*;` plus the gpui imports from
chapter 2, and fragments accumulate — a field added in chapter 5 is still there
in chapter 9. Constructors run in `Workbench::new(cx: &mut Context<Self>)`; UI
is built in `render`; anything else names its home in the prose. The `gallery`
crate (`cargo run -p gallery` in the guise repo) has compiling, running wiring
for every component used here — when in doubt, mine it.

> **Note** Chapters end with a **Checkpoint** summarizing what the app holds at
> that point, so you can skim, jump, or diff your own version against it.

## Project setup

Start a fresh binary crate:

```sh
cargo new workbench
cd workbench
```

`guise` is published as `guise-ui` (the `guise` name was taken on crates.io),
but its library target is named `guise` — so the dependency key and the import
differ, and no rename configuration is needed:

```toml
[package]
name = "workbench"
version = "0.1.0"
edition = "2021"

[dependencies]
gpui = "0.2"
guise-ui = "0.2"
```

> **Note** gpui ships on crates.io as of 0.2 — no git pin required. You need
> **Rust stable ≥ 1.96**; gpui uses recently stabilized library features.

### The smallest workbench

`src/main.rs`:

```rust
use gpui::prelude::*;
use gpui::{
    div, px, size, App, Application, Bounds, Context, IntoElement, SharedString,
    TitlebarOptions, Window, WindowBounds, WindowOptions,
};
use guise::prelude::*;

struct Workbench;

impl Render for Workbench {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.global::<Theme>();
        div()
            .size_full()
            .bg(t.body().hsla())
            .text_color(t.text().hsla())
            .font_family(t.font_family.clone())
            .p(px(24.0))
            .child(
                Stack::new()
                    .gap(Size::Md)
                    .child(Title::new("Workbench").order(2))
                    .child(Text::new("A data workbench, one chapter at a time.").dimmed()),
            )
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        // 1. Install the theme global exactly once, before any window opens.
        Theme::dark().init(cx);

        // 2. Open a window hosting the root view.
        let bounds = Bounds::centered(None, size(px(1100.0), px(760.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some(SharedString::new_static("workbench")),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_window, cx| cx.new(|_cx| Workbench),
        )
        .expect("open window");
        cx.activate(true);
    });
}
```

`cargo run` gives you a dark window with a title and a dimmed line of text.
Three things to notice:

- **`Theme::dark().init(cx)`** installs the theme as a gpui *global*. Every
  guise component reads it at render time; rendering a component before a theme
  is installed panics. See [Theming](theming.md).
- **`Stack`, `Title`, `Text`** are stateless builders — cheap values created
  fresh each render and dropped into `.child(...)`.
- The root `div()` paints the window itself — background, text color, and font
  come from the theme's
  [semantic colors](theming.md#semantic-colors-scheme-aware).

> **Tip** `use guise::prelude::*;` brings in every component, the theme types,
> the layout macros, and the reactive helpers. It deliberately excludes
> [`guise::flex`](flex.md) — those names (`Row`, `Column`, `Container`) overlap
> the themed layout module and must be imported explicitly.

> **Checkpoint** — a compiling binary: one `Workbench` view, one window, one
> installed theme.

## Theming

Before adding components, learn the vocabulary they all share.

### Tokens: Size, ColorName, Variant

Almost every builder takes the same three knobs:

- **`Size`** — `Xs..Xl`, indexing the theme's spacing / radius / font scales
  together ([token table](theming.md#sizing-tokens)).
- **`ColorName`** — one of 14 palette colors (`Blue`, `Teal`, `Grape`, …), each
  a 10-shade ramp ([the palette](theming.md#the-palette)). Most colored
  components also accept an explicit CSS color via
  [`color!`](theming.md#css-style-colors).
- **`Variant`** — `Filled`, `Light`, `Outline`, `Subtle`, `Default`,
  `Transparent`, `White`. One `(color, variant)` pair looks identical on
  `Button`, `Badge`, `Alert`, and friends because they all resolve through the
  same [`surface`](components.md#the-variant-system) function.

```rust
Group::new()
    .gap(Size::Sm)
    .child(Button::new("run", "Run").color(ColorName::Teal))
    .child(Button::new("fmt", "Format").variant(Variant::Light))
    .child(Button::new("cancel", "Cancel").variant(Variant::Default).size(Size::Xs))
```

### Branding the theme

`Theme::dark()` / `Theme::light()` are starting points. Override the semantic
colors app-wide with the `with_*` builders, tweak fields, then `init`:

```rust
Theme::dark()
    .with_primary(color!("#5f3dc4"))
    .with_body(color!(rgb(14, 14, 19)))
    .with_surface(color!(rgb(22, 22, 30)))
    .init(cx);
```

Every component reads the getters (`body`, `surface`, `text`, `dimmed`,
`border`, `primary`) at render time, so one override restyles the whole app —
details in [Theming](theming.md#semantic-colors-scheme-aware).

### Runtime light/dark toggle — and the borrow rule

The theme is a mutable global. Flipping the scheme and refreshing the window is
the entire dark-mode feature:

```rust
impl Render for Workbench {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 1. Resolve every theme value FIRST. theme reads borrow `cx` immutably…
        let t = cx.global::<Theme>();
        let body = t.body().hsla();
        let text = t.text().hsla();
        let is_dark = t.scheme.is_dark();

        // 2. …and cx.listener(...) needs `cx` mutably, so it must come after.
        let toggle = Button::new("theme", if is_dark { "Light mode" } else { "Dark mode" })
            .variant(Variant::Default)
            .size(Size::Xs)
            .on_click(cx.listener(|_this, _ev, window, cx| {
                let next = cx.global::<Theme>().scheme.toggled();
                cx.global_mut::<Theme>().scheme = next;
                window.refresh();
            }));

        div().size_full().bg(body).text_color(text).p(px(24.0)).child(toggle)
    }
}
```

> **Warning** Resolve theme values into locals *before* any `cx.listener(...)`
> or content closure. `cx.global::<Theme>()` borrows `cx` immutably; a listener
> borrows it mutably. Interleave them and the borrow checker rejects the render
> function. The same rule applies to closures stored on elements (`.hover`,
> `.on_click`): they must be `'static`, so capture resolved `Hsla`/`f32`
> values, never the `&Theme` borrow itself.

`cx.listener` is the bridge between an element handler
(`Fn(&ClickEvent, &mut Window, &mut App)`) and your view: it hands the closure
`&mut Self` plus a `&mut Context<Self>` — see
[event handlers](components.md#event-handlers-compose-with-cxlistener).

> **Checkpoint** — a branded theme, a working light/dark toggle, and the
> resolve-before-listeners rule internalized. Everything after this chapter
> obeys it silently.

## Laying out the shell

Time to give the workbench its frame.

### AppShell

`AppShell` is the application frame: fixed-size header / navbar / aside /
footer regions around a scrollable main area (the shell's children). Regions
take a pixel size plus a **builder closure re-invoked every render**, so they
always show live data.

```rust
let shell = AppShell::new()
    .header(44.0, |_window, _cx| {
        div()
            .flex()
            .items_center()
            .justify_between()
            .h_full()
            .px(px(14.0))
            .child(Title::new("Workbench").order(5))
            .child(
                Button::new("theme", "Toggle theme")
                    .variant(Variant::Default)
                    .size(Size::Xs)
                    .on_click(|_ev, window, cx| {
                        let next = cx.global::<Theme>().scheme.toggled();
                        cx.global_mut::<Theme>().scheme = next;
                        window.refresh();
                    }),
            )
    })
    .navbar(240.0, |_, _| {
        div()
            .p(px(10.0))
            .child(Text::new("Queries").size(Size::Sm).dimmed())
    })
    .child(
        Container::new()
            .size(Size::Md)
            .padding(Size::Md)
            .child(Space::y(Size::Md))
            .child(Title::new("Results").order(4))
            .child(Space::y(Size::Sm))
            .child(Text::new("Nothing to show yet.").dimmed().size(Size::Sm)),
    );
```

Methods: `header(f32, closure)`, `navbar(f32, closure)`, `aside(f32, closure)`,
`footer(f32, closure)`; the closures take `(&mut Window, &mut App)` and return
any `IntoElement`. Main-area content comes via `ParentElement`
(`.child`/`.children`).

> **Warning** Region closures are `Fn` and re-invoked each frame, so they
> cannot *move* a captured `cx.listener` into a button — that would move out of
> a `Fn` capture. The theme toggle above therefore uses a plain, capture-free
> closure (the theme is a global; no view access needed). When a region *does*
> need app state, capture cheap clonable handles — `Entity` clones and
> `Signal`s — and clone them locally inside the closure before moving them into
> handlers. Chapters 5 and 11 use exactly this.

`Container` here is the themed, Mantine-style one — a centered column capped at
a `Size`-indexed max width. It shares a name with the Flutter-style
`flex::Container`; since we never glob-import `guise::flex`, the prelude's is
unambiguous. `Space::y(Size::Md)` inserts a fixed theme-scale gap where a
parent `gap` doesn't reach ([layout reference](layout.md)).

### StatusBar and the root column

`StatusBar` is a thin, three-slot bar (`left` / `center` / `right`, each
callable multiple times). Compose the window as a flex column — shell above,
status bar below:

```rust
let status = StatusBar::new()
    .left(Text::new("workbench").size(Size::Xs))
    .left(Badge::new(if is_dark { "Dark" } else { "Light" }).size(Size::Sm))
    .center(Text::new("Ready").size(Size::Xs).dimmed())
    .right(Text::new("0 rows").size(Size::Xs).dimmed());

div()
    .relative()
    .size_full()
    .flex()
    .flex_col()
    .bg(body)
    .text_color(text)
    .font_family(font)
    .child(div().flex_1().min_h(px(0.0)).child(shell))
    .child(status)
```

Methods: `new()`, `height(f32)` (default 28), `left`, `center`, `right` —
[reference](navigation.md#statusbar).

> **Tip** `flex_1().min_h(px(0.0))` is the classic flexbox move that lets the
> shell shrink below its content size so *its* scroll areas scroll instead of
> the window clipping. The `.relative()` on the root matters later: modals and
> toasts render as children of this full-size root.

> **Checkpoint** — the app has its skeleton: header (title + theme toggle),
> empty navbar, capped main column, status bar. `Workbench` is still a unit
> struct; that changes next.

## State & bindings

gpui is reactive at its core — entities notify, observers re-render. The
[`reactive`](reactive.md) module wraps that in a small, familiar API, and every
guise input can *bind* to it directly. If you have used Cocoa/SwiftUI bindings,
this is that model: the value flows both ways, and nobody writes glue handlers.

### Signal, use_state, watch

A [`Signal<T>`](reactive.md#signal) is a clonable handle to one observable
value (a thin wrapper over `Entity<T>`). `use_state` creates one; `watch`
re-renders the calling view whenever it changes. Give `Workbench` a
constructor and its first state:

```rust
struct Workbench {
    auto_refresh: Signal<bool>,
    filter: Signal<String>,
    queries: Signal<Vec<String>>,
    query_count: Signal<usize>,
    filter_input: Entity<TextInput>,
}

impl Workbench {
    fn new(cx: &mut Context<Self>) -> Self {
        let auto_refresh = use_state(cx, true);
        watch(cx, &auto_refresh);

        let queries = use_state(
            cx,
            vec![
                "orders.sql".to_string(),
                "revenue by month.sql".to_string(),
                "top customers.sql".to_string(),
            ],
        );
        watch(cx, &queries);

        // Derived state — React's useMemo. Recomputes whenever `queries` changes.
        let query_count = use_memo(cx, &queries, |list| list.len());
        watch(cx, &query_count);

        // A text signal, two-way bound to a TextInput entity.
        let filter = use_state(cx, String::new());
        watch(cx, &filter);
        let filter_input = cx.new(|cx| TextInput::new(cx).placeholder("Filter queries…"));
        TextInput::bind(&filter_input, &filter, cx);

        Workbench { auto_refresh, filter, queries, query_count, filter_input }
    }
}
```

And point the window at it: `|_window, cx| cx.new(Workbench::new)`.

### The two binding shapes

Bindings mirror the two component patterns
([Inputs → Binding inputs](inputs.md#binding-inputs)):

- **Controlled builders** take a [`Binding`](reactive.md#bindings) — get one
  from `signal.binding()` (the whole value) or `signal.lens(get, set)` (one
  field of a struct signal). The binding overrides the plain value setter;
  clicks write back through it, then run any `on_change`.
- **Stateful entities** bind once after creation with the associated function
  `X::bind(&entity, &signal, cx)` — the entity adopts the signal's value,
  edits write back, signal writes update the entity.

```rust
// Controlled: replaces `.checked(...)` + `.on_change(...)` entirely.
Switch::new("auto-refresh")
    .label("Auto refresh")
    .bind(self.auto_refresh.binding())
```

For one field of a struct signal, bind a lens instead:
`Checkbox::new("vim").bind(settings.lens(|s| s.vim, |s, v| s.vim = v))`. Both
directions are equality-guarded (`set_if_changed`), so an echoed value is a
no-op and updates cannot loop; `Binding::map` converts between types,
`Binding::constant` makes a read-only demo value.

Render the pieces (navbar closure capturing entity clones — the pattern from
the AppShell warning):

```rust
let filter_input = self.filter_input.clone();
let auto = self.auto_refresh.clone();

let shell = AppShell::new()
    // …header as before…
    .navbar(240.0, move |_window, _cx| {
        div()
            .p(px(10.0))
            .child(
                Stack::new()
                    .gap(Size::Sm)
                    .child(filter_input.clone())
                    .child(Switch::new("auto-refresh").label("Auto refresh").bind(auto.binding())),
            )
    });
```

And the status bar goes live:

```rust
.right(
    Text::new(format!("{} queries", self.query_count.get(cx)))
        .size(Size::Xs)
        .dimmed(),
)
```

> **Note** `watch(cx, &signal)` is what makes a *view* repaint when a signal
> changes — it is `cx.observe(signal.entity(), |_, _, cx| cx.notify())`. A
> bound entity repaints itself; the surrounding view only needs `watch` when it
> reads the signal in its own render (like the counts above). Signals shared
> across unrelated views travel through
> [`provide` / `use_context`](reactive.md#context--provider).

> **Checkpoint** — `Workbench` owns four signals and one entity. The navbar
> filter field round-trips through `TextInput::bind`, the switch through
> `.bind(binding())`, and the status bar shows a `use_memo`-derived count.

## Lists of data: DataView & TableView

Two collection components, two altitudes: `DataView` renders a
`Signal<Vec<T>>` through an item template; `TableView` is the full sortable,
selectable data grid over typed rows.

### DataView (entity)

The collection-bindings story: the view observes a `Signal<Vec<T>>` and
repaints on every write. Filtering and sorting are *projections* applied at
render — the source vector is never copied or reordered, and selection reports
**source** indices.

The filter closure receives no `cx` (it runs deep inside render), so a live
query flows in through a shared cell that an observer keeps fresh — this is the
exact wiring the gallery uses:

```rust
use std::cell::RefCell;
use std::rc::Rc;

// In Workbench::new, after `queries` and `filter` from chapter 5:
let query_cache: Rc<RefCell<String>> = Rc::new(RefCell::new(String::new()));
let filter_cache = query_cache.clone();
let query_list = cx.new(|cx| {
    DataView::new(cx, &queries)
        .item(|name: &String, _ix, _window, _cx| Text::new(name.clone()).size(Size::Sm))
        .filter(move |name: &String| {
            let query = filter_cache.borrow();
            query.is_empty() || name.to_lowercase().contains(&query.to_lowercase())
        })
        .sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()))
        .selectable()
});
cx.subscribe(&query_list, |this, _view, event: &DataViewEvent, cx| {
    let DataViewEvent::Selected(ix) = event;
    this.active_query = Some(*ix); // a new Option<usize> field
    cx.notify();
})
.detach();

// Copy each filter write into the cell, then nudge the DataView to repaint.
let filtered = query_list.clone();
cx.observe(filter.entity(), move |_this, changed, cx| {
    *query_cache.borrow_mut() = changed.read(cx).clone();
    filtered.update(cx, |_, cx| cx.notify());
})
.detach();
```

| Method | Default | Notes |
| --- | --- | --- |
| `new(cx, &Signal<Vec<T>>)` | — | observes the signal; every `set`/`update` repaints |
| `item(closure)` | — | `Fn(&T, usize, &mut Window, &mut App) -> impl IntoElement`, re-invoked every frame |
| `filter(pred)` | none | projection — hides non-matching items, source untouched |
| `sort_by(cmp)` | none | stable sort of the display order, source untouched |
| `layout(DataViewLayout)` | `List` | `List` or `Grid(cols)` |
| `gap(Size)` | `Sm` | spacing between items |
| `empty(closure)` | dimmed "Nothing to show" | shown when the projection yields nothing |
| `selectable()` | off | single selection: hover highlight, primary-tint selected row |

Emits `DataViewEvent::Selected(usize)` — the **source** index, valid under any
filter/sort. Read back with `selected_index()`.

> **Note** Why the extra `filtered.update(…, cx.notify())`? A gpui entity
> repaints when *it* is notified, not when its parent re-renders. The
> `DataView` observes `queries`, but the filter text lives in a signal it knows
> nothing about — so we forward the change. Whenever an entity renders data it
> does not observe, forward the notify by hand.

Because a write to `queries` repaints the view automatically, mutation is a
one-liner anywhere you have an `App` — a "New query" button is just
`this.queries.update(cx, |list| list.push("untitled.sql".into()))` inside a
`cx.listener`.

### TableView (entity)

The results grid: typed rows, per-column closures, sortable headers,
click / ⌘-click / ⇧-click selection, drag-resizable columns, and a virtualized
body when you fix its height. Define the row type and the table:

```rust
#[derive(Clone)]
struct Order {
    id: u32,
    customer: &'static str,
    total: f64,
    status: &'static str,
}

// In Workbench::new:
let orders = use_state(cx, seed_orders()); // your Vec<Order> fixture
let row_count = use_memo(cx, &orders, |rows| rows.len());
watch(cx, &row_count);

let results = cx.new(|cx| {
    TableView::new(cx)
        .columns(vec![
            Column::new("Id")
                .width(70.0)
                .text(|o: &Order| format!("#{}", o.id).into())
                .sortable_by(|a: &Order, b: &Order| a.id.cmp(&b.id)),
            Column::new("Customer").text(|o: &Order| o.customer.into()),
            Column::new("Total")
                .width(110.0)
                .align(Align::End)
                .text(|o: &Order| format!("${:.2}", o.total).into())
                .sortable_by(|a: &Order, b: &Order| a.total.total_cmp(&b.total)),
            Column::new("Status")
                .width(120.0)
                .cell(|o: &Order, _window, _cx| {
                    if o.status == "paid" {
                        Badge::new("Paid").color(ColorName::Teal)
                    } else {
                        Badge::new("Pending").color(ColorName::Orange)
                    }
                }),
        ])
        .bind_rows(&orders, cx)
        .selection_mode(SelectionMode::Multi)
        .striped(true)
        .highlight_on_hover(true)
        .with_border(true)
        .height(260.0)
});
cx.subscribe(&results, |this, _table, event: &TableViewEvent, cx| {
    this.table_status = match event {
        TableViewEvent::SelectionChanged(rows) => format!("Selected source rows: {rows:?}").into(),
        TableViewEvent::Activated(row) => format!("Activated row {row}").into(),
        TableViewEvent::Sorted(sort) => format!("Sort: {sort:?}").into(),
    };
    cx.notify();
})
.detach();
```

(`table_status` is a new `SharedString` field feeding the status bar's center
slot.)

| Method | Default | Notes |
| --- | --- | --- |
| `new(cx)` | — | build with `cx.new(...)` |
| `columns(Vec<Column<T>>)` | empty | column definitions |
| `rows(Vec<T>)` | empty | owned snapshot; replace later with `set_rows(rows, cx)` |
| `bind_rows(&Signal<Vec<T>>, cx)` | — | live rows: observes the signal, reads it at render |
| `selection_mode(SelectionMode)` | `None` | `None` / `Single` / `Multi` |
| `striped(bool)` | `false` | zebra rows by display position |
| `highlight_on_hover(bool)` | `false` | row hover fill |
| `with_border(bool)` | `false` | rounded outer border |
| `height(f32)` | auto | fixes the body height; body becomes a virtualized `uniform_list`, header stays sticky |
| `empty(closure)` | "No data" | `(&mut Window, &mut App) -> impl IntoElement` |

`Column<T>` builders — `Column::new(title)`, then:

| Method | Default | Notes |
| --- | --- | --- |
| `width(f32)` | flexes | fixed pixel width |
| `flex(f32)` | `1.0` | grow factor when no fixed width |
| `min_width(f32)` | `60.0` | floor for flex sizing and drag-resizing |
| `align(Align)` | `Start` | header and cell alignment |
| `sortable_by(cmp)` | not sortable | `(&T, &T) -> Ordering`; header click cycles asc → desc → none |
| `cell(closure)` | — | `(&T, &mut Window, &mut App) -> impl IntoElement`, rebuilt every frame |
| `text(closure)` | — | `(&T) -> SharedString`; truncates with an ellipsis |

Sorting stably reorders *display* indices — the source `Vec<T>` is never
mutated, and every index in `TableViewEvent` refers to the source rows, so
selections survive resorting. Read back with `selected()` / `sort_state()`.

> **Warning** With `height(..)` the body is virtualized by `uniform_list`,
> which measures the first row — keep cells single-line so every row shares one
> height.

For static string grids, the lighter [`Table`](data.md#table) builder skips all
of this. Render `self.results.clone()` in the main area,
`self.query_list.clone()` under the navbar filter, and point the status bar's
center at `self.table_status.clone()` and its right slot at
`format!("{} rows", self.row_count.get(cx))`.

> **Checkpoint** — the navbar filters a live `DataView` of saved queries; the
> main area shows a sortable, multi-select `TableView` bound to
> `Signal<Vec<Order>>`; the status bar narrates selection and sorting.

## The sidebar: TreeView + SplitPanel

The flat query list becomes a proper navigation tree, and the main area splits
into resizable panes.

### TreeView (entity)

A hierarchical list with expandable branches, single selection, and full
keyboard navigation. Nodes are plain `TreeNode` values; the view owns expansion
and selection and emits `TreeViewEvent`.

```rust
// A new field to route the main area:
enum MainView {
    Query,
    Settings,
}

// In Workbench::new:
let tree = cx.new(|cx| {
    TreeView::new(cx)
        .nodes(vec![
            TreeNode::new("queries", "queries")
                .child(TreeNode::new("orders", "orders.sql"))
                .child(TreeNode::new("revenue", "revenue.sql")),
            TreeNode::new("tables", "tables")
                .child(TreeNode::new("t-orders", "orders"))
                .child(TreeNode::new("t-customers", "customers")),
            TreeNode::new("settings", "settings").icon(IconName::Star),
        ])
        .expand("queries")
});
cx.subscribe(&tree, |this, _tree, event: &TreeViewEvent, cx| {
    if let TreeViewEvent::Activated(id) = event {
        this.view = if id.as_ref() == "settings" {
            MainView::Settings
        } else {
            MainView::Query
        };
        cx.notify();
    }
})
.detach();
```

Methods: `new(cx)`, `nodes(Vec<TreeNode>)`,
`bind_nodes(&Signal<Vec<TreeNode>>, cx)` (live data; expansion/selection
survive updates, keyed by id), `expand(id)` / `collapse(id)`,
`default_expanded(bool)`; read back with `expanded_ids()` / `selected_id()`.
`TreeNode`: `new(id, label)`, `icon(IconName)`, `child(node)`,
`children(iter)`. Events: `Selected(id)`, `Toggled(id, expanded)`,
`Activated(id)` (Enter or double-click). With focus, ↑/↓ walk the visible rows,
→ expands or steps into a branch, ← collapses or steps to the parent. Keep node
ids unique — every event carries one.

### SplitPanel (entity)

Two live panes separated by a draggable divider. Like `Tabs` and `AppShell`
regions, pane content is a **builder closure re-invoked every render**, so
panes show live data — including another `SplitPanel` for nested layouts.

```rust
// In Workbench::new — the results table moves into the bottom pane,
// framed by a stateless Panel:
let table = results.clone();
let split = cx.new(|cx| {
    SplitPanel::new(cx)
        .direction(SplitDirection::Vertical)
        .ratio(0.4)
        .min_first(140.0)
        .min_second(180.0)
        .first(|_, _| {
            div()
                .p(px(12.0))
                .child(Text::new("Editor — chapter 8.").size(Size::Sm).dimmed())
        })
        .second(move |_, _| {
            div().p(px(12.0)).child(
                Panel::new()
                    .title("Results")
                    .description("orders")
                    .child(table.clone()),
            )
        })
});
cx.subscribe(&split, |_this, _split, _event: &SplitPanelEvent, cx| cx.notify()).detach();
```

| Method | Default | Notes |
| --- | --- | --- |
| `new(cx)` | — | ratio `0.5`, horizontal |
| `direction(SplitDirection)` | `Horizontal` | `Horizontal` = side by side, `Vertical` = stacked |
| `first(closure)` / `second(closure)` | — | `Fn(&mut Window, &mut App) -> impl IntoElement`, rebuilt each frame |
| `ratio(f32)` | `0.5` | initial first-pane share, clamped `0..=1` |
| `min_first(f32)` / `min_second(f32)` | `40.0` | minimum pane size while dragging |
| `handle_size(f32)` | `6.0` | divider grab-area thickness |
| `current_ratio()` | — | read the live ratio |

Emits `SplitPanelEvent::Resized(f32)` continuously while dragging. Nesting
works out of the box — each drag payload carries its entity id, so an inner
divider never resizes the outer panel (build the inner split first, then
`.second(move |_, _| inner.clone())`, exactly like the gallery).

`Panel` is `Card` chrome plus header (icon, title, description, trailing
actions), body, and optional footer; collapse is *controlled* — the parent owns
the flag and flips it in `on_toggle`. Methods: `new()`, `id`, `title`,
`description`, `icon`, `action`/`actions`, `footer`, `padding` (default `Lg`),
`radius`, `with_border` (default `true`), `shadow`, `collapsible()`,
`collapsed(bool)`, `on_toggle(handler)` — [reference](panels.md#panel).

The split fills its parent, so give it a sized frame in the main area:

```rust
let main = match self.view {
    MainView::Query => div()
        .h(px(520.0))
        .w_full()
        .border_1()
        .border_color(border)
        .rounded(px(8.0))
        .overflow_hidden()
        .child(self.split.clone())
        .into_any_element(),
    MainView::Settings => settings_panel.into_any_element(), // chapter 10
};
```

The navbar closure now renders `tree.clone()` above the `DataView` from
chapter 6.

> **Checkpoint** — activating tree rows routes the main view; the query view is
> a vertical split with a placeholder editor pane and the results table inside
> a `Panel`. Dragging the divider resizes live.

## Editing: the Editor

The top pane becomes a real multi-tab SQL editor.

### Editor (entity)

A multiline code editor: line-number gutter, syntax highlighting (`Language::
{None, Rust, Sql, Json}`), selection, undo/redo, and the full macOS-convention
keyboard map. Cmd+Enter emits a `Run` event with the buffer.

```rust
const FIRST_QUERY: &str = "SELECT id, customer, total\nFROM orders\nORDER BY total DESC;";

// In Workbench::new:
let editor = cx.new(|cx| {
    Editor::new(cx)
        .language(Language::Sql)
        .rows(10)
        .placeholder("SELECT * FROM orders;")
        .value(FIRST_QUERY)
});
cx.subscribe(&editor, |this, _editor, event: &EditorEvent, cx| {
    if let EditorEvent::Run(source) = event {
        this.run_query(source.clone(), cx); // defined in chapter 9
    }
})
.detach();
```

| Method | Default | Notes |
| --- | --- | --- |
| `new(cx)` | — | construct inside `cx.new(\|cx\| ...)` |
| `value(&str)` | `""` | initial text (`text()` is the getter) |
| `language(Language)` | `None` | built-in `Rust` / `Sql` / `Json` tokenizers |
| `placeholder(text)` | — | dimmed hint while empty and unfocused |
| `read_only(bool)` | `false` | selection and copy still work |
| `line_numbers(bool)` | `true` | gutter, active line brightened |
| `tab_size(usize)` | `4` | spaces per tab stop |
| `font_size(f32)` | `13.0` | monospace, line height 1.5× |
| `rows(usize)` | — | minimum height in visible lines |

Runtime: `editor.read(cx).text()` reads the buffer;
`editor.update(cx, |e, cx| e.set_text("…", cx))` replaces it — note `set_text`
resets the cursor and undo history, which is exactly what tab switching wants.
`focus_handle()` lets a host focus it on open. Events:

```rust
pub enum EditorEvent {
    Change(String), // every edit, full new text
    Run(String),    // Cmd+Enter, current text
}
```

For a single-buffer tool, skip the events entirely and two-way bind:

```rust
let source = use_state(cx, String::new());
Editor::bind(&editor, &source, cx); // equality-guarded in both directions
```

### TabBar (entity)

A document tab strip: scrollable labeled tabs, per-tab close buttons, an
optional trailing `+`. Crucially, closing **emits an event but does not remove
the tab** — the parent decides (unsaved changes, confirmation) and calls
`remove_tab`.

The workbench keeps one `Editor` and swaps buffers per tab:

```rust
// New fields: tabbar: Entity<TabBar>, sources: Vec<String>, active_tab: usize.
let tabbar = cx.new(|cx| {
    TabBar::new(cx)
        .tabs(["orders.sql", "revenue.sql"])
        .active(0)
});
cx.subscribe(&tabbar, |this, bar, event: &TabBarEvent, cx| match event {
    TabBarEvent::Select(next) => {
        let next = *next;
        // Save the outgoing buffer, load the incoming one.
        this.sources[this.active_tab] = this.editor.read(cx).text();
        this.active_tab = next;
        let source = this.sources[next].clone();
        this.editor.update(cx, |e, cx| e.set_text(&source, cx));
    }
    TabBarEvent::Close(i) => {
        // Closing is a *request* — decide (unsaved changes? last tab?),
        // then remove.
        let i = *i;
        if this.sources.len() > 1 {
            this.sources.remove(i);
            bar.update(cx, |b, cx| b.remove_tab(i, cx));
            this.active_tab = bar.read(cx).active_index();
            let source = this.sources[this.active_tab].clone();
            this.editor.update(cx, |e, cx| e.set_text(&source, cx));
        }
    }
    TabBarEvent::Add => {
        this.sources[this.active_tab] = this.editor.read(cx).text();
        this.sources.push(String::new());
        bar.update(cx, |b, cx| {
            let n = b.len() + 1;
            b.add_tab(format!("untitled {n}"), cx);
        });
        this.active_tab = bar.read(cx).active_index();
        this.editor.update(cx, |e, cx| e.set_text("", cx));
    }
})
.detach();
```

Methods: `tabs(iter)`, `active(usize)`, `with_add_button(bool)` (default
`true`); runtime `add_tab(label, cx)` (appends and activates, no event),
`remove_tab(index, cx)`, `set_tabs(Vec, cx)`, `set_active(usize, cx)`; read
with `active_index()`, `len()`, `is_empty()`. Events:
`TabBarEvent::{Select(usize), Close(usize), Add}` — `Select` fires *after* the
bar has switched, and close clicks never double as selects
([reference](data.md#tabbar-entity)).

Swap the placeholder pane for the real thing — entity clones captured by the
pane closure:

```rust
let bar = tabbar.clone();
let buffer = editor.clone();
// …inside the SplitPanel construction:
.first(move |_, _| {
    div()
        .p(px(12.0))
        .child(Stack::new().gap(Size::Sm).child(bar.clone()).child(buffer.clone()))
})
```

Keyboard highlights: arrows move (±Shift extends, ±Option word-wise, ±Cmd
line/document); Enter copies the current indent; Cmd+A/C/X/V talk to the OS
clipboard; Cmd+Z / Cmd+Shift+Z undo/redo; **Cmd+Enter emits `Run`**. Escape and
Cmd+Tab bubble on purpose, so dialogs and focus management above keep working.

> **Checkpoint** — a tabbed, syntax-highlighted SQL editor over the results
> pane. Cmd+Enter fires `EditorEvent::Run` into `run_query`, which chapter 9
> now implements.

## Dialogs, menus & feedback

Everything overlay-shaped in one pass. All of these paint above their siblings
via gpui's `deferred()` + `occlude()` — deferral re-parents the element to
paint after (above) everything else in the window, and occlusion stops mouse
events leaking through. That is why modals and toast stacks want to live in a
full-size, `.relative()` root.

### ConfirmModal

A confirm/cancel dialog composed from [`Modal`](overlays.md#modal), and
*controlled* the same way: the parent owns a flag, renders the dialog only
while it is set, and flips it in the handlers — neither button closes anything
by itself. Deleting a saved query:

```rust
// Field: confirm_delete: Option<usize>. At the end of render, where `root`
// (the full-size column from chapter 4) is bound as `mut`:
if let Some(ix) = self.confirm_delete {
    root = root.child(
        ConfirmModal::new()
            .title("Delete query?")
            .message("The file will be removed from the workspace.")
            .confirm_label("Delete")
            .danger()
            .on_confirm(cx.listener(move |this, _ev, _window, cx| {
                this.queries.update(cx, |list| {
                    list.remove(ix);
                });
                this.confirm_delete = None;
                cx.notify();
            }))
            .on_cancel(cx.listener(|this, _ev, _window, cx| {
                this.confirm_delete = None;
                cx.notify();
            })),
    );
}
root
```

Methods: `new()`, `title`, `message`, `confirm_label`, `cancel_label`,
`danger()` (red confirm button), `width` (default 440), `on_confirm`,
`on_cancel` (also fired by the backdrop and the `×`) —
[reference](overlays.md#confirmmodal). For arbitrary dialog bodies, drop down
to `Modal::new().title(..).on_close(..)` with children.

### ContextMenu (entity)

A right-click menu at the pointer. No trigger element — call
`show(position, window, cx)` from a right-mouse-down and render the entity
anywhere (it paints nothing while closed):

```rust
use gpui::{MouseButton, MouseDownEvent};

// In Workbench::new:
let row_menu = cx.new(|cx| {
    ContextMenu::new(cx)
        .section("Rows")
        .item_icon(IconName::Copy, "Copy as CSV", |_window, _cx| { /* … */ })
        .item("Refresh", |_window, _cx| { /* … */ })
        .divider()
        .danger_item("Delete query", |_window, _cx| { /* … */ })
});
```

```rust
// In render — the frame around the split from chapter 7:
let frame = div()
    .id("main-frame")
    .relative()
    .h(px(520.0))
    .w_full()
    .border_1()
    .border_color(border)
    .rounded(px(8.0))
    .overflow_hidden()
    .on_mouse_down(
        MouseButton::Right,
        cx.listener(|this, ev: &MouseDownEvent, window, cx| {
            let position = ev.position;
            this.row_menu.update(cx, |menu, cx| menu.show(position, window, cx));
        }),
    )
    .child(self.split.clone());
```

Methods: `new(cx)`, `item(label, handler)`, `item_icon(IconName, label,
handler)`, `danger_item`, `section(label)`, `divider()`, `size` (default `Sm`),
`width` (default 220, also drives edge clamping), `show(position, window, cx)`,
`close(window, cx)`, `is_open()` — [reference](overlays.md#contextmenu-entity).

> **Note** Item handlers get `(&mut Window, &mut App)`, not your view. To
> mutate view state from one (say, set `confirm_delete`), capture a
> `WeakEntity` and update through it:
>
> ```rust
> let this = cx.weak_entity();
> // …
> .danger_item("Delete query", move |_window, cx| {
>     this.update(cx, |this, cx| {
>         this.confirm_delete = this.active_query;
>         cx.notify();
>     })
>     .ok();
> })
> ```

### Menu & MenuBar (entities)

[`Menu`](overlays.md#menu-entity) is a labelled trigger plus a dropdown of
items — same item builders as `ContextMenu`. `MenuBar` is the horizontal
application menu (File / Edit / …): once one menu is open, hovering a sibling
label switches to it, the classic desktop feel.

```rust
let menubar = cx.new(|cx| {
    MenuBar::new(cx)
        .menu("File", |m| {
            m.item_shortcut("New query", "⌘N", |_window, _cx| { /* … */ })
                .divider()
                .danger_item("Quit", |_window, cx| cx.quit())
        })
        .menu("View", |m| {
            m.item("Toggle theme", |window, cx| {
                let next = cx.global::<Theme>().scheme.toggled();
                cx.global_mut::<Theme>().scheme = next;
                window.refresh();
            })
        })
});
```

`MenuColumn` builders: `item`, `item_shortcut(label, "⌘T", handler)`,
`danger_item`, `disabled_item`, `section`, `divider` —
[reference](overlays.md#menubar-entity). Render `menubar.clone()` in the
AppShell header. The shortcut string is a *hint*, not a key binding — real
global shortcuts go through the native [window menu](windowmenu.md).

### ToastStack (entity)

A toast manager: holds live toasts, paints them as a deferred top-right stack.
Create it once, render it in the root, push from anywhere you can reach the
handle:

```rust
let toasts = cx.new(|_| ToastStack::new());     // in Workbench::new
root = root.child(self.toasts.clone());         // in render, on the root
```

Methods: `new()`, `duration(Option<Duration>)` (default 4 s auto-dismiss;
`None` keeps toasts until closed), `push(message, cx) -> id`,
`push_titled(title, message, color, cx) -> id`, `remove(id, cx)`, `clear(cx)`,
`len()`, `is_empty()` — [reference](feedback.md#toaststack-entity).

### LoadingOverlay + run_query

`LoadingOverlay` is stateless: render it as the **last child** of a
`.relative()` container and flip `visible`. While visible it dims the parent,
centers a `Loader`, and occludes the mouse so the content underneath is inert.
Methods: `new()`, `visible(bool)`, `loader(Loader)`.

Now the promised `run_query` — flip a flag, log, and simulate latency with a
spawned timer (the same pattern `ToastStack` uses internally):

```rust
use std::time::Duration;

// Fields: running: bool, log: Signal<Vec<String>> (created with use_state + watch).
impl Workbench {
    fn run_query(&mut self, source: String, cx: &mut Context<Self>) {
        self.running = true;
        self.log.update(cx, |lines| lines.push(format!("ran {} chars", source.len())));
        cx.notify();
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_millis(600)).await;
            this.update(cx, |this, cx| {
                this.running = false;
                this.toasts.update(cx, |t, cx| {
                    t.push_titled("Query finished", "6 rows in 0.6 s", ColorName::Teal, cx);
                });
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}
```

`cx.spawn` hands the async closure a `WeakEntity<Self>` plus an async app
handle; `this.update(...)` re-enters the view if it still exists (`.ok()`
swallows the case where the window closed mid-flight). Attach the overlay to
the main frame from the ContextMenu section — it is already `.relative()`:

```rust
.child(self.split.clone())
.child(LoadingOverlay::new().visible(self.running))
```

And the status bar center gets a live narration line:

```rust
let last_log = self
    .log
    .read(cx)
    .last()
    .cloned()
    .unwrap_or_else(|| "Ready".to_string());
```

> **Checkpoint** — Cmd+Enter runs a query: overlay on, log line appended,
> toast on completion. Right-click opens a context menu over the results,
> deletes route through a danger `ConfirmModal`, and the header has a real
> `MenuBar`.

## Forms

The settings view — every remaining input, each bound to a signal, plus
validation.

### Bound inputs, wholesale

All of these follow the entity-binding shape from chapter 5:
`cx.new` the entity, then `X::bind(&entity, &signal, cx)`. Full per-component
references live in [Inputs](inputs.md).

```rust
// In Workbench::new:
let api_key = use_state(cx, String::new());
let api_key_input = cx.new(|cx| {
    PasswordInput::new(cx)
        .label("API key")
        .placeholder("At least 20 characters")
        .description("The eye toggles visibility.")
});
PasswordInput::bind(&api_key_input, &api_key, cx);

let accent = use_state(cx, rgb(34, 139, 230)); // Signal<Hsla>
let accent_input = cx.new(|cx| ColorInput::new(cx).label("Accent color").value(rgb(34, 139, 230)));
ColorInput::bind(&accent_input, &accent, cx);

let environments = use_state(cx, vec!["prod".to_string(), "eu-west".to_string()]);
let tags_input = cx.new(|cx| {
    TagsInput::new(cx)
        .label("Environments")
        .placeholder("Type and press Enter…")
        .tags(["prod", "eu-west"])
        .max_tags(6)
});
TagsInput::bind(&tags_input, &environments, cx);

let sample_range = use_state(cx, (20.0, 80.0));
let range_input = cx.new(|cx| {
    RangeSlider::new(cx).min(0.0).max(100.0).min_gap(5.0).value((20.0, 80.0))
});
RangeSlider::bind(&range_input, &sample_range, cx);

let pin_code = use_state(cx, String::new());
let pin_input = cx.new(|cx| PinInput::new(cx).length(6).mask(true));
PinInput::bind(&pin_input, &pin_code, cx);

let rating = use_state(cx, 4.0f32);
watch(cx, &rating); // Rating is a controlled builder — the view renders it
```

Quick map: **`PasswordInput`** (entity) is `TextInput` in masked mode with an
eye toggle (`PasswordInputEvent::{Change, Submit}`). **`ColorInput`** (entity)
is a swatch plus a hex/CSS field — the swatch opens the theme palette, and
anything `css()`-parsable updates it live (`ColorInputEvent(Hsla)`).
**`TagsInput`** (entity) is a pill list with an inline editor — Enter or comma
commits, Backspace on an empty query pops the last pill
(`TagsInputEvent(Vec<String>)`). **`RangeSlider`** (entity) holds a
`(low, high)` pair with real drag-source thumbs snapped to `step` and `min_gap`
(`RangeSliderEvent((f64, f64))`). **`PinInput`** (entity) is segmented OTP
boxes — typing advances, Cmd+V pastes the whole code
(`PinInputEvent::{Change, Complete}`). **`Rating`** is a controlled star row;
`.bind(rating.binding())` replaces `value` + `on_change`.

> **Tip** Set `min`/`max`/`step`/`min_gap` *before* `value(..)` on
> `RangeSlider` — the pair is normalized against them.

### FormState validation

[`FormState`](reactive.md#forms) is a pure store of field values, validators,
and the errors from the last validation — reactive by living inside a `Signal`
(`use_form` is the shorthand):

```rust
use guise::reactive::validators;

let form = use_form(
    cx,
    FormState::new()
        .field("host", "localhost:5432")
        .validator("host", validators::required())
        .field("email", "")
        .validator("email", validators::email()),
);
watch(cx, &form);

// Entities feed the form through their Change events:
let host_input = cx.new(|cx| TextInput::new(cx).value("localhost:5432"));
cx.subscribe(&host_input, |this, _input, event: &TextInputEvent, cx| {
    if let TextInputEvent::Change(value) = event {
        this.form.update(cx, |f| f.set("host", value.clone()));
    }
})
.detach();

let email_input = cx.new(|cx| TextInput::new(cx).placeholder("you@example.com"));
// …same subscribe shape, writing to "email".
```

Built-in `validators`: `required()`, `min_len(n)`, `email()`. A `Validator` is
just `Box<dyn Fn(&str) -> Option<String>>`, so custom rules are closures.

At render, read errors and wrap the inputs in `Field` — the shared
label/description/error chrome every input draws
([reference](inputs.md#field)):

```rust
let host_error = self.form.read(cx).error("host").map(str::to_string);
let mut host_field = Field::new().label("Host").child(self.host_input.clone());
if let Some(error) = host_error {
    host_field = host_field.error(error);
}
// …and an `email_field` built the same way.
```

### The settings panel

Assemble it (this is the `MainView::Settings` arm from chapter 7):

```rust
let save = Button::new("save-settings", "Save")
    .on_click(cx.listener(|this, _ev, _window, cx| {
        this.form.update(cx, |f| {
            f.validate();
        });
        if this.form.read(cx).is_valid() {
            this.toasts.update(cx, |t, cx| {
                t.push_titled("Saved", "Settings updated.", ColorName::Teal, cx);
            });
        }
        cx.notify();
    }));

let settings_panel = Panel::new()
    .title("Settings")
    .description("Connection & workspace")
    .child(
        Stack::new()
            .gap(Size::Md)
            .child(host_field)
            .child(email_field)
            .child(
                Group::new()
                    .align(Align::Start)
                    .gap(Size::Lg)
                    .child(div().flex_1().child(self.api_key_input.clone()))
                    .child(div().flex_1().child(self.accent_input.clone())),
            )
            .child(self.tags_input.clone())
            .child(
                Stack::new()
                    .gap(Size::Xs)
                    .child(Text::new("Sampling range").size(Size::Sm))
                    .child(self.range_input.clone()),
            )
            .child(
                Group::new()
                    .align(Align::Center)
                    .gap(Size::Md)
                    .child(self.pin_input.clone())
                    .child(Text::new("2FA code").size(Size::Sm).dimmed()),
            )
            .child(Rating::new("satisfaction").bind(self.rating.binding()))
            .child(Group::new().justify(Justify::End).child(save)),
    );
```

Note the borrow choreography one more time: the error strings and every other
`cx` read happen *before* the `cx.listener` on the save button — same rule as
chapter 3, now at scale.

> **Checkpoint** — activating "settings" in the tree swaps the main area to a
> `Panel` of six bound inputs plus a validated connection form. Invalid saves
> paint red errors through `Field`; valid saves toast.

## Polish & ship

The last mile: a command palette, keyboard affordances, a charts summary, and
pointers onward.

### Spotlight (entity)

A command palette: centered overlay, search field, keyboard-navigable list —
type to filter, ↑/↓ to move, Enter to run, Esc to dismiss. Command handlers get
`(&mut Window, &mut App)`, so capture signals or entity clones for anything
stateful:

```rust
// In Workbench::new:
let palette_log = log.clone();
let spotlight = cx.new(|cx| {
    Spotlight::new(cx)
        .item_hint("Toggle theme", "⌘T", |window, cx| {
            let next = cx.global::<Theme>().scheme.toggled();
            cx.global_mut::<Theme>().scheme = next;
            window.refresh();
        })
        .item("Clear log", move |_window, cx| {
            palette_log.update(cx, |lines| lines.clear());
        })
});
```

Methods: `new(cx)`, `item(label, handler)`, `item_hint(label, hint, handler)`;
state: `is_open()`, `open(window, cx)`, `close(cx)` —
[reference](overlays.md#spotlight-entity).

Render `self.spotlight.clone()` in the root (it paints nothing while closed)
and open it from the header — an entity clone, cloned again locally inside the
region closure before moving into the handler:

```rust
let spotlight = self.spotlight.clone();
// …inside .header(44.0, move |_, _| { … }):
{
    let palette = spotlight.clone();
    Button::new("palette", "⌘K")
        .variant(Variant::Default)
        .size(Size::Xs)
        .on_click(move |_ev, window, cx| {
            palette.update(cx, |s, cx| s.open(window, cx));
        })
}
```

### Keyboard patterns

The workbench is already keyboard-friendly because the components are: the
`Editor` carries the full macOS map (Cmd+Enter to run), `TableView` and
`TreeView` navigate with arrows (Enter activates, Escape clears then bubbles),
and menus and `Spotlight` are arrows / Enter / Escape throughout. Two
conventions to preserve in your own code: let **Escape and Tab bubble** when
you have nothing to do with them (hosts use them to dismiss and move focus —
guise inputs already behave this way), and advertise shortcuts with `Kbd`:

```rust
Group::new()
    .align(Align::Center)
    .gap(Size::Xs)
    .child(Text::new("Run").size(Size::Xs).dimmed())
    .child(Kbd::new("⌘"))
    .child(Kbd::new("↵"))
```

App-global shortcuts (a real ⌘K) belong to gpui actions and the native
[window menu](windowmenu.md), the way the gallery binds its theme toggle.

### A charts summary row

`Sparkline`, `LineChart`, `BarChart`, `PieChart` are stateless builders painted
through gpui's `canvas` — minimal, axis-free visuals over plain `f32` series.
Derive them from live data at render:

```rust
let totals: Vec<f32> = self.orders.read(cx).iter().map(|o| o.total as f32).collect();

let summary = Group::new()
    .align(Align::Center)
    .gap(Size::Lg)
    .child(Sparkline::new(totals).fill().color(ColorName::Teal))
    .child(
        BarChart::entries([("Mon", 12.0), ("Tue", 9.0), ("Wed", 15.0), ("Thu", 7.0)])
            .height(64.0),
    );
```

`Sparkline` methods: `new(values)`, `color`, `stroke`, `fill()`, `width` /
`full_width()`, `height` (default 32). `BarChart`: `new(values)` /
`entries(pairs)` (labels render under the bars), `color` / `colors` (default: a
palette rotation), `gap(fraction)`, `width`, `height` (default 140). Colors
adapt to light/dark automatically.

Drop `summary` above the main frame, and the workbench is done: themed shell,
navigable sidebar, tabbed editor, live-bound tables, dialogs, toasts, settings,
palette, charts.

### Where to go next

- The per-area references — [buttons](buttons.md), [inputs](inputs.md),
  [layout](layout.md), [data display](data.md), [overlays](overlays.md),
  [feedback](feedback.md), [navigation](navigation.md),
  [typography](typography.md) — every component used here has a fuller entry.
- [Theming](theming.md) for palettes, scales, and CSS colors;
  [Reactive state](reactive.md) for signals, bindings, context, and forms.
- [Flex layout](flex.md) and the [layout macros](macros.md) (`row!`, `col!`,
  `zstack!`) when pixel-based, Flutter-style layout fits better than tokens.
- The **gallery** (`cargo run -p gallery` in the guise repo) — live, compiling
  wiring for every component, with a "view source" toggle per section.

> **Checkpoint** — you shipped a data workbench. More importantly, you now hold
> the whole mental model: builders vs entities, resolve-theme-before-listeners,
> signals and the two binding shapes, builder closures re-invoked per frame,
> deferred-and-occluded overlays, and notify-forwarding when an entity renders
> data it does not observe. Everything else in guise is a permutation of those.
