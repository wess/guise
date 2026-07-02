# Panels

`Panel` is a stateless builder — `Card` chrome plus header/footer framing and a
controlled collapse. `SplitPanel` is a stateful entity: two live panes with a
draggable divider, emitting `Resized` events.

## Panel

A titled surface: [`Card`](layout.md#card) chrome plus a header row (collapse
chevron, icon, title/description on the left, actions on the right), the body,
and an optional footer. The header gets a bottom divider whenever the body is
visible. Collapsing is controlled, like `Modal` — the parent owns the flag and
flips it in `on_toggle`. Implements `ParentElement` for the body.

```rust
Panel::new()
    .id("status")
    .title("Project status")
    .description("Weekly summary")
    .icon(ThemeIcon::new("▦").color(ColorName::Blue))
    .action(ActionIcon::new("status-more", "…").size(Size::Sm))
    .collapsible()
    .collapsed(self.collapsed)
    .on_toggle(cx.listener(|this, _ev, _window, cx| {
        this.collapsed = !this.collapsed;
        cx.notify();
    }))
    .footer(Text::new("Updated 5 minutes ago").size(Size::Xs).dimmed())
    .child(Text::new("Everything on track."))
```

| Method | Default | Notes |
| --- | --- | --- |
| `new()` | — | body content via `.child`/`.children` |
| `id(impl Into<ElementId>)` | none | scopes the chevron's element id; set one when several collapsible panels are siblings |
| `title(text)` | none | semibold header title |
| `description(text)` | none | dimmed line under the title |
| `icon(impl IntoElement)` | none | leading header content (e.g. a `ThemeIcon`) |
| `action(impl IntoElement)` | — | appends one trailing header action |
| `actions(Vec<AnyElement>)` | — | replaces the trailing actions |
| `footer(impl IntoElement)` | none | rendered under the body behind a top divider |
| `padding(Size)` | `Lg` | body padding; header/footer use 0.75× vertically |
| `radius(Size)` | `Md` | |
| `with_border(bool)` | `true` | |
| `shadow(Size)` | `Sm` | |
| `collapsible()` | off | shows the chevron `ActionIcon` in the header |
| `collapsed(bool)` | `false` | hides body + footer; the parent owns the flag |
| `on_toggle(handler)` | — | `Fn(&ClickEvent, &mut Window, &mut App)`; flip the parent's flag |

> **Note** `collapsed(true)` only takes effect together with `collapsible()` —
> a non-collapsible panel always shows its body.

## SplitPanel (entity)

Two live panes separated by a draggable divider. Pane content is a **builder
closure re-invoked every render** (like Tabs panels), so panes show live data —
including another `SplitPanel`'s element for nested layouts. Give the element a
sized parent; the panel fills it. Dragging clamps to `min_first`/`min_second`
and emits `SplitPanelEvent::Resized(ratio)`.

```rust
let split = cx.new(|cx| {
    SplitPanel::new(cx)
        .direction(SplitDirection::Horizontal)
        .ratio(0.35)
        .min_first(140.0)
        .min_second(200.0)
        .first(|_w, _cx| Text::new("Sidebar"))
        .second(|_w, _cx| Text::new("Main content"))
});
cx.subscribe(&split, |_this, _split, event: &SplitPanelEvent, cx| {
    let SplitPanelEvent::Resized(_ratio) = event; // persist it, relayout, …
    cx.notify();
})
.detach();

// In render:
div().h(px(300.0)).w_full().child(self.split.clone())
```

| Method | Default | Notes |
| --- | --- | --- |
| `new(cx)` | — | |
| `direction(SplitDirection)` | `Horizontal` | `Horizontal` = side by side (col-resize cursor), `Vertical` = stacked (row-resize) |
| `first(closure)` / `second(closure)` | — | `Fn(&mut Window, &mut App) -> impl IntoElement`, rebuilt each frame |
| `ratio(f32)` | `0.5` | initial first-pane share, clamped to `0..=1` |
| `min_first(f32)` / `min_second(f32)` | `40.0` | minimum pane size in px while dragging |
| `handle_size(f32)` | `6.0` | divider grab-area thickness (min 1) |
| `current_ratio()` | — | read the live ratio |

Events: `SplitPanelEvent::Resized(f32)` — emitted continuously while dragging.

Nesting works out of the box: gpui delivers `on_drag_move` for every active
drag of a payload type anywhere in the window, so each divider's drag payload
carries its owning entity id and an inner divider never resizes the outer
panel. Nest by returning the inner entity's clone from a pane closure:

```rust
let inner = cx.new(|cx| {
    SplitPanel::new(cx)
        .direction(SplitDirection::Vertical)
        .first(|_w, _cx| Text::new("Editor"))
        .second(|_w, _cx| Text::new("Terminal"))
});
let outer = cx.new(|cx| {
    SplitPanel::new(cx)
        .first(|_w, _cx| Text::new("Sidebar"))
        .second(move |_w, _cx| inner.clone())
});
```
