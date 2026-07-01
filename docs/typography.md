# Typography

`Text`, `Title`, `Anchor`, `Code`, `Kbd`, `Mark`, `Blockquote`, `Spoiler`. All
stateless builders — `Spoiler` is *controlled*: the parent owns the expanded
flag and flips it in `on_toggle`.

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

## Mark

An inline highlighted span — the highlighter pen. Inherits the surrounding font
size unless `size` is set, so it drops into a `Group` next to `Text` runs.

```rust
Group::new()
    .gap(Size::Xs)
    .child(Text::new("Highlight the"))
    .child(Mark::new("important part"))
    .child(Text::new("of a sentence."))
```

Methods: `new(content)`, `color(ColorName)` (default `Yellow` — a light wash in
light mode, a translucent tint in dark), `size(Size)` (inherits when unset).

## Blockquote

A quoted passage behind a left accent border, with an optional icon and
citation. Content is `text(..)`, `ParentElement` children, or both — text
renders first, the citation last.

```rust
Blockquote::new()
    .icon(IconName::Info)
    .text("Life is like an npm install — you never know what you are going to get.")
    .cite("– Forrest Gump")
```

| Method | Default | Notes |
| --- | --- | --- |
| `new()` | — | `.child(..)` / `.children(..)` also accepted |
| `text(str)` | none | shorthand for a single text child |
| `color(ColorName)` | `Blue` | border, icon, and background wash |
| `cite(str)` | none | dimmed attribution line; include your own dash |
| `icon(IconName)` | none | accent-colored glyph above the quote |
| `padding(Size)` | `Lg` | |
| `radius(Size)` | theme default | right-side corners (the accent border owns the left) |

## Spoiler

Clips tall content to a max height behind an [`Anchor`](#anchor)-styled
"Show more" toggle. **Controlled**: the parent owns `expanded` and flips it in
`on_toggle`, exactly like `Modal`'s `opened`/`on_close` pair. Implements
`ParentElement`.

```rust
Spoiler::new("bio-spoiler")
    .max_height(60.0)
    .expanded(self.bio_open)
    .on_toggle(cx.listener(|this, _, _, cx| {
        this.bio_open = !this.bio_open;
        cx.notify();
    }))
    .child(Text::new(LONG_BIO).size(Size::Sm))
```

| Method | Default | Notes |
| --- | --- | --- |
| `new(id)` | — | the id sits on the toggle link |
| `max_height(f32)` | `100.0` | visible px while collapsed (overflow hidden) |
| `expanded(bool)` | `false` | the parent owns this flag |
| `show_label(str)` | `"Show more"` | |
| `hide_label(str)` | `"Hide"` | |
| `color(ColorName)` | `Blue` | toggle link color |
| `size(Size)` | `Sm` | toggle label font size |
| `on_toggle(handler)` | — | wire with `cx.listener(...)` |

> **Note** The toggle is always visible — gpui elements don't expose their
> measured height at build time, so there is no "content fits, hide the
> toggle" check.
