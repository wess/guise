# Typography

`Text`, `Title`, `Anchor`, `Code`, `Kbd`. All stateless builders.

## Text

Themed body text.

```rust
Text::new("A line of body text.")
    .size(Size::Md)
    .dimmed()          // secondary color
```

| Method | Default | Notes |
| --- | --- | --- |
| `new(content)` | — | |
| `size(Size)` | `Md` | maps to `font_size` scale |
| `weight(FontWeight)` | `NORMAL` | gpui's `FontWeight` |
| `medium()` | — | shorthand for weight 500 |
| `bold()` | — | shorthand for weight 700 |
| `color(Color)` | theme text | explicit color |
| `dimmed()` | — | use the muted/secondary color |

## Title

A heading; `order` 1–6 sets the level (1 largest).

```rust
Title::new("Section heading").order(2)
```

| Method | Default |
| --- | --- |
| `new(content)` | — |
| `order(u8)` | `1` (clamped to 1..=6) |
| `color(Color)` | theme text |

Font sizes by order: 1 → 34, 2 → 26, 3 → 22, 4 → 18, 5 → 16, 6 → 14 (px), bold.

## Anchor

A clickable, colored text link.

```rust
Anchor::new("docs-link", "Read the docs")
    .color(ColorName::Blue)
    .on_click(cx.listener(|this, _, _, cx| { /* navigate */ }))
```

Methods: `new(id, label)`, `color` (default `Blue`), `size` (default `Md`),
`on_click`.

## Code

Inline code, on a tinted chip.

```rust
Code::new("guise::Button")              // neutral
Code::new("Variant::Light").color(ColorName::Grape)  // tinted
```

Methods: `new(content)`, `color(ColorName)`.

> gpui has no generic monospace fallback. For true monospace, set a real mono
> `font_family` on the surrounding element.

## Kbd

A keyboard-key cap, for shortcut hints.

```rust
Group::new()
    .child(Kbd::new("⌘"))
    .child(Kbd::new("K"))
```

Method: `new(key)`.
