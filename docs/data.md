# Data display

`Avatar`, `AvatarGroup`, `Badge`, `Image`, `Indicator`, `List`, `Table`,
`Timeline` are stateless builders. `TableView`, `DataView`, `TreeView`,
`Tabs`, `TabBar`, and `Accordion` are stateful entities — `TableView`,
`DataView`, and `TreeView` can also bind their data to a
[`Signal`](reactive.md#signal) collection.

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

## Image

A themed wrapper around gpui's `img()` element: remote URIs, embedded asset
paths, and filesystem paths all work, with the theme size/radius vocabulary
and a fallback slot shown while the source is loading or unavailable.

```rust
Image::new("https://example.com/cat.png")   // or a PathBuf, or an asset path
    .width(240.0)
    .height(160.0)
    .radius(Size::Md)
    .fit(ObjectFit::Cover)
    .fallback(|| Text::new("no image").dimmed())
```

| Method | Default | Notes |
| --- | --- | --- |
| `new(source)` | — | anything `Into<gpui::ImageSource>`: `&str`/`String` (an `http(s)://` URI, else an embedded-asset path), `Path`/`PathBuf` (local file), or raw/decoded image data |
| `width(f32)` / `height(f32)` | none | px — give it a size, an unsized image lays out at zero |
| `radius(Size)` | none | square corners by default, like Mantine |
| `circle()` | off | clip to a circle (an avatar); pair with equal width/height |
| `fit(ObjectFit)` | `Cover` | `Fill` / `Contain` / `Cover` / `ScaleDown` / `None` — re-exported from gpui |
| `fallback(closure)` | none | `Fn() -> impl IntoElement`, shown while loading or failed |

> **Note** gpui also exports a *type* named `gpui::Image` — that one is raw
> encoded bytes (an `Arc<gpui::Image>` is itself a valid `source` here), not
> an element. `guise::Image` is the element.

## List

A bulleted or numbered list of text items.

```rust
List::new().items(["First", "Second", "Third"])
List::new().ordered(true).item("Step one").item("Step two")
```

Methods: `new()`, `item(s)`, `items(iter)`, `ordered(bool)`, `size`,
`spacing(Size)`, `icon(impl Into<Glyph>)` (custom bullet — a Lucide `IconName`
or text).

`List` builds all its children eagerly — it's for content-sized lists. For
big flat collections reach for [`VirtualList`](#virtuallist) below, or bind a
signal with [`DataView`](#dataview-entity) and give it a `height`.

## VirtualList

Windowed rendering for large flat collections: only the rows in view are
built each frame, so a 100k-item list renders as cheaply as a 20-item one.
Items come from a factory closure and must share one height — that
uniformity is what makes the scroll math O(1).

```rust
VirtualList::new("log", lines.len(), move |i, _window, _cx| {
    Text::new(lines[i].clone()).size(Size::Sm)
})
.height(400.0)
```

Methods: `new(id, count, fn(usize, &mut Window, &mut App) -> impl IntoElement)`,
`height(px)` (viewport height, default `240.0`). The factory runs per visible
index, every frame — keep it cheap.

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

For typed rows, sorting, selection, and a virtualized body, use
[`TableView`](#tableview-entity) below.

## TableView (entity)

A rich, generic data table over typed rows: sortable headers, click /
⌘-click / ⇧-click selection, drag-resizable columns, and an optionally
virtualized body with a sticky header. Rows are any `T: 'static`; cells render
through per-column closures, re-invoked every frame so they show live data.

```rust
struct User { name: String, age: u32 }

let table = cx.new(|cx| {
    TableView::new(cx)
        .columns(vec![
            Column::new("Name")
                .text(|u: &User| u.name.clone().into())
                .sortable_by(|a, b| a.name.cmp(&b.name)),
            Column::new("Age")
                .width(80.0)
                .align(Align::End)
                .text(|u: &User| u.age.to_string().into())
                .sortable_by(|a, b| a.age.cmp(&b.age)),
        ])
        .rows(users)                          // or .bind_rows(&signal, cx)
        .selection_mode(SelectionMode::Multi)
        .striped(true)
        .with_border(true)
        .height(320.0)                        // fixed height => virtualized body
});

cx.subscribe(&table, |_this, _table, event: &TableViewEvent, _cx| match event {
    TableViewEvent::SelectionChanged(rows) => { /* source-row indices */ }
    TableViewEvent::Activated(row) => { /* double-click or Enter */ }
    TableViewEvent::Sorted(sort) => { /* Some((column, dir)) or None */ }
})
.detach();
```

| Method | Default | Notes |
| --- | --- | --- |
| `new(cx)` | — | construct inside `cx.new(\|cx\| ...)` |
| `columns(Vec<Column<T>>)` | empty | the column definitions |
| `rows(Vec<T>)` | empty | owned snapshot; replace later with `set_rows(rows, cx)` |
| `bind_rows(&Signal<Vec<T>>, cx)` | — | live rows: observes the [`Signal`](reactive.md#signal) and reads it at render; selection is pruned when rows disappear |
| `selection_mode(SelectionMode)` | `None` | `None` / `Single` / `Multi` |
| `striped(bool)` | `false` | zebra rows, by display position |
| `highlight_on_hover(bool)` | `false` | row hover fill |
| `with_border(bool)` | `false` | rounded outer border |
| `height(f32)` | auto | fixes the body height; the body becomes a virtualized `uniform_list` scroll region and the header stays sticky above it |
| `empty(closure)` | "No data" | `(&mut Window, &mut App) -> impl IntoElement`, rendered when there are no rows |

`Column<T>` builders:

| Method | Default | Notes |
| --- | --- | --- |
| `Column::new(title)` | — | header title |
| `width(f32)` | flexes | fixed pixel width |
| `flex(f32)` | `1.0` | grow factor when no fixed width |
| `min_width(f32)` | `60.0` | floor for both flex sizing and drag-resizing |
| `align(Align)` | `Start` | horizontal alignment of the header and cells |
| `sortable_by(cmp)` | not sortable | `(&T, &T) -> Ordering`; a header click cycles asc → desc → unsorted |
| `text(closure)` | — | `(&T) -> SharedString`; truncates with an ellipsis when narrow |
| `cell(closure)` | — | `(&T, &mut Window, &mut App) -> impl IntoElement`, rebuilt every frame |

Selection follows desktop conventions: click selects, ⌘-click toggles,
⇧-click selects a display-order range; ↑/↓ move the selection (⇧ extends,
and the row scrolls into view when virtualized), Enter activates, Escape
clears (and bubbles only when there is nothing to clear). Sorting stably
reorders display indices — the source `Vec<T>` is never mutated — and every
index in `TableViewEvent` refers to the **source** rows, so selections survive
resorting. Drag the strip at a header's right edge to resize a column; it
becomes fixed-width, honoring `min_width`. Read state back with `selected()`
and `sort_state()`.

> **Note** With `height(..)` the body is virtualized by gpui's `uniform_list`,
> which requires every row to share one height — keep cells single-line.

## DataView (entity)

*The* collection-binding component — a list or grid bound to a
`Signal<Vec<T>>`. The view observes the signal and repaints on every write;
filtering and sorting are *projections* applied at render over the borrowed
data (NSArrayController-style), so the source vector is never copied or
reordered, and selection reports **source** indices.

```rust
let todos = use_state(cx, vec!["Write docs".to_string(), "Ship".to_string()]);
let view = cx.new(|cx| {
    DataView::new(cx, &todos)
        .item(|todo, _ix, _window, _cx| Text::new(todo.clone()))
        .sort_by(|a, b| a.cmp(b))
        .selectable()
});
cx.subscribe(&view, |_this, _view, event: &DataViewEvent, _cx| {
    let DataViewEvent::Selected(ix) = event; // index into the SOURCE vec
})
.detach();

// Later, from anywhere — the view repaints by itself:
todos.update(cx, |list| list.push("Celebrate".into()));
```

| Method | Default | Notes |
| --- | --- | --- |
| `new(cx, &Signal<Vec<T>>)` | — | observes the signal; every `set`/`update` repaints |
| `item(closure)` | — | the item template: `Fn(&T, usize, &mut Window, &mut App) -> impl IntoElement`, re-invoked every frame with the borrowed item and its source index |
| `filter(pred)` | none | projection: hides non-matching items, source untouched |
| `sort_by(cmp)` | none | stable sort of the display order, source untouched |
| `layout(DataViewLayout)` | `List` | `List` or `Grid(cols)` — rows of equal-width cells |
| `gap(Size)` | `Sm` | spacing between items (and grid cells) |
| `empty(closure)` | dimmed "Nothing to show" | shown when the projection yields nothing |
| `selectable()` | off | single selection: hover highlight, primary-tint selected item |
| `height(px)` | content-sized | **virtualizes**: only the items in view are built; items (or grid rows) must share one height |

Emits `DataViewEvent::Selected(usize)` with the clicked item's **source**
index (stays valid under any filter/sort). Read back with `selected_index()`.
With `selectable()` each item gets its own padded, rounded row; without it the
template's element renders bare.

> **Tip** The filter closure gets no `cx`, so to drive it from live state (a
> search box) route the query through a shared `Rc<RefCell<String>>` that an
> observer keeps updated — the gallery's DataView section wires exactly this.

## TreeView (entity)

A hierarchical list with expandable branches, single selection, and keyboard
navigation. Nodes are plain `TreeNode` values (a node with children is a
branch); the view owns expansion and selection and emits `TreeViewEvent`,
every variant carrying the node id.

```rust
let tree = cx.new(|cx| {
    TreeView::new(cx)
        .nodes(vec![
            TreeNode::new("src", "src")
                .child(TreeNode::new("main", "main.rs"))
                .child(TreeNode::new("lib", "lib.rs")),
            TreeNode::new("readme", "README.md"),
        ])
        .expand("src")
});
cx.subscribe(&tree, |_this, _tree, event: &TreeViewEvent, _cx| match event {
    TreeViewEvent::Selected(id) => { /* selection moved */ }
    TreeViewEvent::Toggled(id, open) => { /* branch expanded/collapsed */ }
    TreeViewEvent::Activated(id) => { /* Enter or double-click */ }
})
.detach();
```

| Method | Default | Notes |
| --- | --- | --- |
| `new(cx)` | — | construct inside `cx.new(\|cx\| ...)` |
| `nodes(Vec<TreeNode>)` | empty | the tree data |
| `bind_nodes(&Signal<Vec<TreeNode>>, cx)` | — | live data from a [`Signal`](reactive.md#signal); expansion and selection survive updates (keyed by node id) |
| `expand(id)` / `collapse(id)` | all collapsed | per-branch initial expansion |
| `default_expanded(bool)` | `false` | start with every branch expanded (also applies to nodes assigned later) |
| `height(px)` | content-sized | **virtualizes**: only the rows in view are built; keyboard selection scrolls into view |

`TreeNode` builders: `new(id, label)`, `icon(IconName)` (branches fall back to
`IconName::Menu`, leaves to `IconName::Dot`), `child(node)`, `children(iter)`;
`is_leaf()` reads back. Ids ride on every event — keep them unique.

Clicking a row selects it (a branch click also toggles it); double-click or
Enter activates. With focus, ↑/↓ walk the visible rows, → expands a collapsed
branch or steps into an expanded one, ← collapses or steps to the parent. Rows
indent per depth, the chevron rotates when expanded, and the selected row gets
a primary-tint background. Read back with `selected_id()` and `expanded_ids()`
(sorted for determinism).

## Timeline

A vertical sequence of events with bullets and connectors. Items up to and
including `active` are highlighted.

```rust
Timeline::new()
    .active(1)
    .item_desc("Created", "Project initialized")
    .item_desc("Building", "Compiling sources")
    .item("Deploy")
```

Methods: `new()`, `item(title)`, `item_desc(title, description)`,
`active(usize)`, `color` (default `Blue`).

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

For document-style tabs (close buttons, an add button, no panels), use
[`TabBar`](#tabbar-entity) below.

## TabBar (entity)

A document-style tab strip: a horizontally scrollable row of labeled tabs,
each with a close button (visible while hovered or active), plus an optional
trailing `+` button. The active tab gets the surface background; inactive tabs
are dimmed. It renders no panels — pair it with your own content keyed off
`active_index()`.

```rust
let bar = cx.new(|cx| TabBar::new(cx).tabs(["main.rs", "lib.rs"]).active(0));
cx.subscribe(&bar, |_this, bar, event: &TabBarEvent, cx| match event {
    TabBarEvent::Close(i) => {
        let i = *i;
        bar.update(cx, |b, cx| b.remove_tab(i, cx));
    }
    TabBarEvent::Add => bar.update(cx, |b, cx| b.add_tab("untitled", cx)),
    TabBarEvent::Select(_) => {}
})
.detach();
```

| Method | Default | Notes |
| --- | --- | --- |
| `new(cx)` | — | construct inside `cx.new(\|cx\| ...)` |
| `tabs(iter)` | empty | tab labels, any `Into<SharedString>` items |
| `active(usize)` | `0` | initially active index |
| `with_add_button(bool)` | `true` | trailing `+` button |

Runtime: `active_index()`, `len()`, `is_empty()` read; `add_tab(label, cx)`
appends and activates, `remove_tab(index, cx)` drops a tab keeping the
selection on the same document where possible, `set_tabs(vec, cx)` replaces
everything (clamping the active index), `set_active(index, cx)` switches
programmatically. None of these emit events — events only report user
interaction:

```rust
pub enum TabBarEvent {
    Select(usize), // a tab was clicked; the bar has already switched to it
    Close(usize),  // a close button was clicked — the tab is NOT removed
    Add,           // the trailing + button
}
```

> **Note** Closing is a *request*: the bar never removes the tab itself, so
> the parent can intercept (an unsaved-changes prompt) and then call
> `remove_tab`. Close clicks never double as selects.

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
