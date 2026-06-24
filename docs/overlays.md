# Overlays

`Modal`, `Menu`, `Tooltip` — UI that paints above the page.

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

> Item handlers get `(&mut Window, &mut App)`. To mutate a parent view from one,
> capture a `WeakEntity` of it and `update` inside.

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
