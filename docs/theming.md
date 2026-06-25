# Theming

The theme is the single source of truth for color, spacing, radius, and
typography. It is installed once as a gpui global and read by every component.

```rust
use guise::prelude::*;

Theme::light().init(cx);   // or Theme::dark()
```

Read it anywhere you have an `&App` (or a `&Context<_>`, which derefs to it):

```rust
let t = guise::theme::theme(cx);   // &Theme
let accent = t.primary().hsla();
```

## The `Theme` struct

| Field | Type | Meaning |
| --- | --- | --- |
| `scheme` | `ColorScheme` | `Light` or `Dark` |
| `palette` | `Palette` | the 14×10 named-color ramps |
| `primary_color` | `ColorName` | default fill color (Blue) |
| `primary_shade_light` / `primary_shade_dark` | `usize` | filled shade per scheme (6 / 8) |
| `white` / `black` | `Color` | absolute white/black |
| `spacing` / `radius` / `font_size` | `Scale` | `xs..xl` token scales (px) |
| `default_radius` | `Size` | corner radius when a component doesn't specify one |
| `line_height` | `f32` | base line height |
| `font_family` | `SharedString` | default UI font |

Construct with `Theme::light()` / `Theme::dark()`, tweak fields, then `.init(cx)`:

```rust
let mut theme = Theme::dark();
theme.primary_color = ColorName::Grape;
theme.default_radius = Size::Md;
theme.init(cx);
```

## Colors

`Color` is a 24-bit RGB triple that converts into gpui's `Hsla`:

```rust
let c = Color::hex("#228be6");     // parse (panics on bad literal — use for constants)
let c = Color::new(34, 139, 230);  // from components
c.hsla();        // opaque gpui::Hsla
c.alpha(0.2);    // gpui::Hsla with alpha
c.contrasting(); // black or white, whichever reads on top of `c`
```

### CSS-style colors

Write colors the way you would in CSS — hex, `rgb`/`rgba`, `hsl`/`hsla`, or a
named color — with the `color!` macro (compile-time literals) or `css(..)`
(runtime strings). Both produce a gpui `Hsla`, which carries alpha and drops
straight into `.bg(..)` / `.text_color(..)` and any guise `.color(..)`.

```rust
use guise::prelude::*;

color!(rgb(34, 139, 230))
color!(rgba(34, 139, 230, 0.5))
color!(hsl(210, 80, 52))        // s / l are percentages (no `%` token)
color!(hsla(210, 80, 52, 0.5))
color!(teal)                    // a CSS named color
color!("#228be6")               // any CSS string: hex (#rgb, #rgba, #rrggbb, #rrggbbaa),
color!("hsl(210, 80%, 52%)")    // functional with `%`, or a named color

css("rgba(34,139,230,.5)")?     // runtime parse (config, theme files) -> Result<Hsla>
```

> Bare hex like `#228be6` can't be a Rust macro token, so pass hex as a string:
> `color!("#228be6")`. Everything else works as bare tokens.

**On components.** Any colored component's `.color(..)` accepts a palette
`ColorName` *or* an explicit CSS color — guise derives the variant shades
(filled/light/outline/hover) from a single custom color:

```rust
Button::new("go", "Go").color(color!(rgba(112, 72, 232, 1)))
Badge::new("New").variant(Variant::Light).color(color!("#e64980"))
```

This is the `ColorValue` type (`ColorName` and `Hsla` both `Into<ColorValue>`).
It's wired into `Button`, `Badge`, `ActionIcon`, `Alert`, and `Chip`; other
components still take a `ColorName`.

### The palette

14 named colors, each a 10-step ramp from lightest (`0`) to darkest (`9`) —
the Mantine / open-color values.

```rust
t.color(ColorName::Teal, 6);    // a specific shade
t.palette.shades(ColorName::Red).get(3);
```

`ColorName` variants: `Dark`, `Gray`, `Red`, `Pink`, `Grape`, `Violet`,
`Indigo`, `Blue`, `Cyan`, `Teal`, `Green`, `Lime`, `Yellow`, `Orange`.
`ColorName::ALL` iterates them; `.label()` gives the lowercase name.

### Semantic colors (scheme-aware)

These resolve differently in light vs. dark mode — use them instead of hard-coding:

| Method | Use for |
| --- | --- |
| `t.body()` | window / page background |
| `t.surface()` | raised surfaces (Paper, Card, Menu) |
| `t.surface_hover()` | hover/recessed fill |
| `t.text()` | primary text |
| `t.dimmed()` | secondary text |
| `t.border()` | borders and dividers |
| `t.primary()` | the primary color at its scheme shade |

```rust
div().bg(t.surface().hsla()).text_color(t.text().hsla())
```

Override any of these (and the primary accent) app-wide with a CSS color via the
`with_*` builders. Overrides are opaque (alpha is dropped — semantic colors are
solid) and apply everywhere the getter is read, so the whole UI restyles:

```rust
Theme::dark()
    .with_primary(color!("#7048e8"))          // hex as a string
    .with_body(color!(rgb(11, 11, 15)))
    .with_surface(color!(rgb(20, 20, 28)))
    .with_text(color!("hsl(220, 15%, 92%)"))
    .init(cx);
```

The setters are named `with_*` (not `body`/`primary`) to avoid clashing with the
same-named getters; each accepts anything `Into<Hsla>` — a `color!`, `css(..)`,
or a palette `Color`.

## Sizing tokens

`Size` (`Xs`, `Sm`, `Md`, `Lg`, `Xl`; default `Md`) indexes three scales:

```rust
t.spacing(Size::Md);   // 16.0
t.radius(Size::Sm);    // 4.0
t.font_size(Size::Lg); // 18.0
```

Defaults (px):

| Scale | xs | sm | md | lg | xl |
| --- | --- | --- | --- | --- | --- |
| spacing | 10 | 12 | 16 | 20 | 32 |
| radius | 2 | 4 | 8 | 16 | 32 |
| font_size | 12 | 14 | 16 | 18 | 20 |

Override a whole scale on the theme before `init`:

```rust
let mut theme = Theme::light();
theme.spacing = Scale::new(8.0, 12.0, 16.0, 24.0, 40.0);
theme.init(cx);
```

## Switching light / dark at runtime

The theme is a mutable global; flip it and request a redraw:

```rust
let dark = cx.global::<Theme>().scheme.is_dark();
cx.global_mut::<Theme>().scheme = if dark { ColorScheme::Light } else { ColorScheme::Dark };
cx.refresh_windows();
```

Because every component reads `theme(cx)` at render time, the whole UI restyles
on the next frame — there is nothing to thread through your views.
