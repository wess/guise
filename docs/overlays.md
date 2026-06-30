# Overlays

`Modal`, `Drawer`, `Menu`, `Popover`, `Spotlight`, `Tooltip` — UI that paints
above the page. Most are built on the same mechanism: a `deferred()` layer
(optionally `occlude()`d) so it overlays sibling content.

`Popover` is the reusable anchored-floating primitive; `Menu`/`Select` predate
it and still hand-roll their own dropdown, but new flyouts should build on it.

## Modal

A centered dialog over a dimming backdrop. **Controlled**: the parent owns an
`opened` flag and renders the `Modal` only while it is `true`, passing an
`on_close` handler. Implements `ParentElement`.

```rust
// In render, after the page content:
let mut root = div().relative().size_full().child(page);
if self.modal_open {
    root = root.child(
        Modal::new()
            .title("Delete project?")
            .on_close(cx.listener(|this, _ev, _w, cx| { this.modal_open = false; cx.notify(); }))
            .child(Text::new("This cannot be undone.").dimmed())
            .child(
                Group::new().justify(Justify::End)
                    .child(Button::new("cancel", "Cancel").variant(Variant::Default)
                        .on_click(cx.listener(|this, _, _, cx| { this.modal_open = false; cx.notify(); })))
                    .child(Button::new("confirm", "Delete").color(ColorName::Red)
                        .on_click(cx.listener(|this, _, _, cx| { this.modal_open = false; cx.notify(); }))),
            ),
    );
}
root
```

| Method | Default |
| --- | --- |
| `new()` | — |
| `title(impl Into<SharedString>)` | none (adds a header with a close `×`) |
| `width(f32)` | `440.0` |
| `padding(Size)` | `Lg` |
| `radius(Size)` | `Md` |
| `on_close(handler)` | none — clicking the backdrop or `×` calls it |

Notes:

- Render the modal as a child of a **full-size root** (`div().relative().size_full()`)
  so the backdrop covers the window. The backdrop sizes itself to the viewport
  and paints via `deferred`, so it overlays everything.
- The dialog stops click propagation, so clicking inside it won't close it.
- `on_close` takes `Fn(&ClickEvent, &mut Window, &mut App)`, so `cx.listener`
  works directly.

## Menu (entity)

A dropdown of actions: a trigger plus a deferred list of items, section labels,
and dividers. Each item carries its own handler and the menu auto-closes.

```rust
let menu = cx.new(|cx| {
    Menu::new(cx, "Actions")
        .section("Edit")
        .item("Copy", |_w, _app| { /* ... */ })
        .item("Rename", |_w, _app| { /* ... */ })
        .divider()
        .danger_item("Delete", |_w, _app| { /* ... */ })
});
```

Methods: `new(cx, trigger)`, `item(label, |window, app| ...)`,
`danger_item(label, handler)` (red), `section(label)`, `divider()`, `size`.

Keyboard: the trigger takes focus when opened, so ↑/↓ move the highlight across
items, Enter runs the highlighted item, and Esc closes.

> Item handlers get `(&mut Window, &mut App)`. To mutate a parent view from one,
> capture a `WeakEntity` of it and `update` inside.

## MenuBar (entity)

A horizontal application menu — a row of top-level labels (File / Edit / View /
…), each opening a dropdown. Once any menu is open, moving the pointer onto a
sibling label switches to it, the way a desktop menu bar does. This is the
themed, in-window counterpart to the native OS menu (see
[Window menu](windowmenu.md)): use it when you draw your own titlebar, or on
platforms with no native menu bar.

```rust
let bar = cx.new(|cx| {
    MenuBar::new(cx)
        .menu("File", |m| {
            m.item_shortcut("New Tab", "⌘T", |_w, _app| { /* ... */ })
                .item_shortcut("New Window", "⌘N", |_w, _app| { /* ... */ })
                .divider()
                .danger_item("Quit", |_w, _app| { /* ... */ })
        })
        .menu("Edit", |m| {
            m.item_shortcut("Undo", "⌘Z", |_w, _app| {})
                .disabled_item("Redo")
                .divider()
                .section("Clipboard")
                .item_shortcut("Copy", "⌘C", |_w, _app| {})
                .item_shortcut("Paste", "⌘V", |_w, _app| {})
        })
});
```

Drop it into a titlebar strip or a [`StatusBar`](navigation.md#statusbar) slot.
Each menu is built with a `MenuColumn` (the closure argument), which offers
`item`, `item_shortcut(label, shortcut, handler)`, `danger_item` (red),
`disabled_item(label)` (greyed, inert), `section(label)`, and `divider()`.
`MenuColumn` is also exported, so menus can be assembled programmatically and
added with `MenuBar::push(column)`.

Keyboard (while open): ←/→ switch menus, ↑/↓ move the highlight, Enter runs the
highlighted item, Esc closes. Set the top-level label size with `size(Size)`.

## Popover (entity)

The reusable anchored-floating primitive: a trigger plus a deferred panel
positioned relative to it. Both the trigger and content are **builder closures**,
re-invoked each render so they show live data. Closes on Esc or a second trigger
click; call `close(cx)` from a content action to dismiss.

```rust
let pop = cx.new(|cx| {
    Popover::new(
        cx,
        |_w, _app| Button::new("trigger", "Options").into_any_element(),
        |_w, _app| Text::new("Panel content").into_any_element(),
    )
    .placement(Placement::Bottom)
    .width(220.0)
});
```

Methods: `new(cx, trigger_fn, content_fn)`, `placement(Placement)`,
`width(f32)`. State: `is_open()`, `open(cx)`, `close(cx)`, `toggle(cx)`.
`Placement` is `Bottom` | `BottomEnd` | `Top` | `TopEnd`.

## Drawer

A panel that slides in from a window edge over a scrim. **Controlled** like
`Modal`: render it only while opened, place it in a full-size root, pass an
`on_close`. Implements `ParentElement`.

```rust
if self.drawer_open {
    root = root.child(
        Drawer::new()
            .title("Filters")
            .side(Side::Right)
            .size(360.0)
            .on_close(cx.listener(|this, _ev, _w, cx| { this.drawer_open = false; cx.notify(); }))
            .child(filters),
    );
}
```

Methods: `new()`, `title`, `side(Side)` (default `Right`), `size(f32)` (width for
left/right, height for top/bottom), `padding(Size)`, `on_close`. `Side` is
`Left` | `Right` | `Top` | `Bottom`.

## Spotlight (entity)

A command palette: a centered overlay with a search field and a
keyboard-navigable command list. Type to filter, ↑/↓ to move, Enter to run, Esc
to dismiss. Render it in a full-size root; open it from an action.

```rust
let palette = cx.new(|cx| {
    Spotlight::new(cx)
        .item("New file", |_w, _app| { /* ... */ })
        .item_hint("Toggle theme", "⌘T", |_w, _app| { /* ... */ })
});
// open from a handler that has a window:
palette.update(cx, |s, cx| s.open(window, cx));
```

Methods: `new(cx)`, `item(label, handler)`, `item_hint(label, hint, handler)`.
State: `is_open()`, `open(window, cx)`, `close(cx)`.

## Tooltip

A small themed bubble, plugged into gpui's built-in `.tooltip(...)`. Use the
`tooltip(...)` helper, which returns the builder closure gpui expects, and
attach it to any interactive element:

```rust
div()
    .id("hoverme")
    .child("Hover me")
    .tooltip(guise::tooltip("Helpful hint"))
```

`tooltip(label)` returns `impl Fn(&mut Window, &mut App) -> AnyView`. The
`Tooltip` view itself is public if you need a custom builder.
