# Buttons

`Button`, `ActionIcon`, `CloseButton`, `ThemeIcon`. All are stateless
`RenderOnce` builders and share the [`Variant`](components.md#the-variant-system)
/ `ColorName` / `Size` vocabulary.

## Button

A labelled, clickable button.

```rust
Button::new("save", "Save changes")
    .variant(Variant::Filled)
    .color(ColorName::Blue)
    .size(Size::Md)
    .left_section(Icon::new(IconName::Check))
    .on_click(cx.listener(|this, _ev, _window, cx| {
        // ...
        cx.notify();
    }))
```

| Method | Default | Notes |
| --- | --- | --- |
| `new(id, label)` | — | `id` must be unique among siblings (needed for click handling) |
| `variant(Variant)` | `Filled` | |
| `color(ColorName)` | `Blue` | |
| `size(Size)` | `Sm` | heights: xs 30, sm 36, md 42, lg 50, xl 60 |
| `radius(Size)` | theme default | |
| `full_width(bool)` | `false` | stretches to the container width |
| `disabled(bool)` | `false` | dims, removes hover + handler |
| `left_section(impl IntoElement)` | — | content before the label (icon) |
| `right_section(impl IntoElement)` | — | content after the label |
| `on_click(handler)` | — | `Fn(&ClickEvent, &mut Window, &mut App)` |

## ActionIcon

A compact, square icon-only button. Same variant/color/size surface as `Button`.

```rust
ActionIcon::new("edit", IconName::Pencil)
    .variant(Variant::Light)
    .color(ColorName::Blue)
    .on_click(cx.listener(|this, _, _, cx| { /* ... */ }))
```

| Method | Default |
| --- | --- |
| `new(id, icon)` | — (`impl Into<Glyph>`: an `IconName` or text — see [icons](icons.md)) |
| `variant(Variant)` | `Subtle` |
| `color(ColorName)` | `Gray` |
| `size(Size)` | `Md` (square: xs 18 … xl 44) |
| `radius(Size)` | `Sm` |
| `disabled(bool)` | `false` |
| `on_click(handler)` | — |

## CloseButton

A subtle square `×` button — the dismiss control used by modals, alerts, etc.

```rust
CloseButton::new("close").on_click(cx.listener(|this, _, _, cx| {
    this.open = false;
    cx.notify();
}))
```

| Method | Default |
| --- | --- |
| `new(id)` | — |
| `size(Size)` | `Md` |
| `on_click(handler)` | — |

## ThemeIcon

A decorative, non-interactive colored chip wrapping a single glyph.

```rust
ThemeIcon::new(IconName::Star).color(ColorName::Yellow).variant(Variant::Filled)
```

| Method | Default |
| --- | --- |
| `new(icon)` | — (`impl Into<Glyph>`) |
| `variant(Variant)` | `Filled` |
| `color(ColorName)` | `Blue` |
| `size(Size)` | `Md` |
| `radius(Size)` | `Sm` |

## CopyButton (entity)

A button that writes text to the system clipboard, then shows a transient
"Copied" state. A gpui entity (it owns the copied flag + reset timer) — create
it with `cx.new` and store the handle.

```rust
let copy = cx.new(|_| CopyButton::new("guise::Button::new(\"id\", \"Label\")"));
// ...later, render it:
copy.clone()
```

| Method | Notes |
| --- | --- |
| `new(text)` | the string to copy |
| `label(text)` | idle label (default `"Copy"`) |
| `text()` / `set_text(text)` | read / replace the copied text |

On click it calls `cx.write_to_clipboard(...)`, flips to "✓ Copied" for ~1.2s
(via a spawned timer), then reverts. The gallery uses one per section to copy
the "view source" snippet.

> Want a count or dot on top of any of these? Wrap it in
> [`Indicator`](data.md#indicator).
