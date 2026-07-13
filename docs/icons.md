# Icons

[Lucide](https://lucide.dev) is guise's built-in icon set. The Lucide icon font
ships inside the crate and registers itself with gpui's text system the first
time a glyph renders — no asset pipeline, no app setup. Because icons are font
glyphs, they inherit the surrounding text color by default and tint/scale like
text.

```rust
Icon::new(IconName::Check)
Icon::new(IconName::Search).size(Size::Lg).color(ColorName::Blue)
```

Because `Icon` is just an `IntoElement`, it slots into any slot that takes one —
e.g. a button section:

```rust
Button::new("save", "Save").left_section(Icon::new(IconName::Check))
```

Methods: `new(name)`, `size(Size)` (default `Md`; 14…32px), `color(ColorName)`
(defaults to inheriting the parent text color).

## IconName

Every icon on [lucide.dev](https://lucide.dev/icons) is a variant, named by
PascalCasing the kebab-case icon name (`arrow-right` → `IconName::ArrowRight`,
`trash-2` → `IconName::Trash2`). Browse lucide.dev to pick one.

```rust
Icon::new(IconName::Rocket)
Icon::new(IconName::TriangleAlert).color(ColorName::Yellow)
```

Helpers:

| method | returns |
| --- | --- |
| `IconName::glyph()` | the `&'static str` font glyph, to render yourself |
| `IconName::name()` | the upstream kebab-case name (`"arrow-right"`) |
| `IconName::all()` | `&'static [IconName]` — the full set, in name order |
| `LUCIDE_VERSION` | the bundled Lucide release |

Names from the pre-Lucide glyph set that no longer exist upstream are kept as
aliases: `IconName::Close` is `IconName::X`, `IconName::Warning` is
`IconName::TriangleAlert`.

## Glyph — icon slots on components

Components with an icon slot (`ActionIcon`, `ThemeIcon`, `Alert`,
`Notification`, `NavLink`, `List`) accept a `Glyph`: either a Lucide
`IconName` or a short piece of text (an emoji, `"+"`, …). Both convert
implicitly:

```rust
ActionIcon::new("edit", IconName::Pencil)   // Lucide icon
ActionIcon::new("party", "🎉")              // plain text still works
Alert::new("Saved.").icon(IconName::Check)
```

## Regenerating

`bun scripts/icons.ts` re-fetches the font, codepoint table, and license from
the `lucide-static` npm package and regenerates
`crates/guise/src/icon/lucide.rs`. Lucide is ISC licensed; the license text is
vendored at `crates/guise/assets/lucide/license.txt`.
