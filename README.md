# guise

[![crates.io](https://img.shields.io/crates/v/guise-ui.svg)](https://crates.io/crates/guise-ui)
[![docs.rs](https://img.shields.io/docsrs/guise-ui)](https://docs.rs/guise-ui)
[![CI](https://github.com/wess/guise/actions/workflows/ci.yml/badge.svg)](https://github.com/wess/guise/actions/workflows/ci.yml)
[![license](https://img.shields.io/crates/l/guise-ui.svg)](https://github.com/wess/guise/blob/main/LICENSE)

A [Mantine](https://mantine.dev)-inspired component library for
[gpui](https://github.com/zed-industries/zed) — the GPU-accelerated Rust UI
framework that powers Zed.

`guise` brings Mantine's ergonomics to gpui: a themed palette, sizing tokens,
and composable components built on gpui's `RenderOnce` builder pattern.

```rust
use guise::prelude::*;

Stack::new()
    .gap(Size::Md)
    .child(Title::new("Welcome").order(1))
    .child(
        Button::new("save", "Save changes")
            .variant(Variant::Filled)
            .color(ColorName::Blue)
            .on_click(|_, window, cx| { /* ... */ }),
    )
```

## Documentation

Full docs live in [`docs/`](docs/readme.md):

- [Getting started](docs/gettingstarted.md) · [Theming](docs/theming.md) · [Component model](docs/components.md)
- Components: [Buttons](docs/buttons.md) · [Icons](docs/icons.md) · [Inputs](docs/inputs.md) · [Typography](docs/typography.md) · [Layout](docs/layout.md) · [Feedback](docs/feedback.md) · [Data](docs/data.md) · [Overlays](docs/overlays.md) · [Navigation](docs/navigation.md)
- Systems: [Flex layout](docs/flex.md) · [Macros](docs/macros.md) · [Transitions](docs/transitions.md) · [Reactive state](docs/reactive.md) · [Window menu](docs/windowmenu.md) · [Architecture](docs/architecture.md)

## Workspace

- **`crates/guise`** — the component library.
- **`crates/gallery`** — a live showcase of every component (the Mantine-docs
  equivalent). Run it with `cargo run -p gallery`.

## Theme

Install a theme once at startup; every component reads it from the gpui global:

```rust
guise::Theme::dark().init(cx);   // or Theme::light()
```

The theme carries the full Mantine / open-color palette (14 colors × 10 shades),
`xs..xl` scales for spacing, radius and font size, and scheme-aware semantic
colors (`body`, `surface`, `text`, `dimmed`, `border`).

### CSS-style colors

Write colors the CSS way — hex, `rgb`/`rgba`, `hsl`/`hsla`, or named — with the
`color!` macro (compile-time) or `css(..)` (runtime strings). Both produce a
gpui `Hsla`, usable in `.bg(..)`/`.text_color(..)`, in any component `.color(..)`,
and in the theme `with_*` overrides:

```rust
Button::new("go", "Go").color(color!(rgba(112, 72, 232, 1)))
Badge::new("New").color(color!("#e64980"))

Theme::dark()
    .with_primary(color!("#7048e8"))
    .with_body(color!(rgb(11, 11, 15)))
    .with_text(color!("hsl(220, 15%, 92%)"))
    .init(cx);                                   // restyles the whole UI
```

`color!` takes `rgb(..)`/`rgba(..)`/`hsl(..)`/`hsla(..)`/named tokens or a CSS
string (hex must be a string — `#228be6` isn't a Rust token). Component
`.color(..)` accepts a palette `ColorName` *or* an explicit color (the
`ColorValue` type) and derives variant shades from a single custom color. See
[Theming](docs/theming.md).

## Components

| Group   | Components                                              |
| ------- | ------------------------------------------------------- |
| Layout  | `Stack`, `Group`, `Center`, `SimpleGrid`, `ScrollArea`  |
| Surface | `Paper`, `Card`                                         |
| Typography | `Text`, `Title`                                     |
| Inputs  | `Button`, `TextInput`, `TextArea`, `NumberInput`, `Checkbox`, `Switch`, `Radio`, `RadioGroup`, `CheckboxGroup`, `Select`, `Combobox`, `Slider`, `Field` |
| Overlays | `Modal`, `Drawer`, `Menu`, `Popover`, `Spotlight`, `Tooltip` |
| Feedback | `Alert`, `Loader`, `Progress`, `RingProgress`, `Notification`, `ToastStack` |
| Data    | `Badge`, `Divider`, `Avatar`, `AvatarGroup`, `List`, `Table`, `Timeline`, `Tabs`, `Accordion` |
| Navigation | `Breadcrumbs`, `NavLink`, `Stepper`, `Pagination`, `StatusBar` |
| Polish  | `Icon`, `ActionIcon`, `ThemeIcon`, `CloseButton`, `CopyButton`, `Anchor`, `Code`, `Kbd`, `Chip`, `Indicator`, `Skeleton`, `SegmentedControl` |
| Motion  | `Transition`, `Collapse`                                |

Inputs come in two flavors that match how each control behaves in gpui:

- **Controlled** (`Checkbox`, `Switch`, `Radio`, and the `RadioGroup` /
  `CheckboxGroup` wrappers) are `RenderOnce` builders — the parent view owns the
  value and passes a change handler via `cx.listener(...)`.
- **Stateful** (`TextInput`, `TextArea`, `NumberInput`, `Select`, `Combobox`,
  `Slider`) are gpui entities that own their buffer / selection. Create with
  `cx.new(...)` and subscribe to their events. `Field` is the shared
  label/description/error chrome these compose.

Overlays paint above the page (a `deferred` layer): `Modal` and `Drawer` are
controlled backdrops (render while `opened`, pass `on_close`), `Menu` is a
keyboard-navigable trigger + deferred action list, `Popover` is the reusable
anchored-floating primitive, `Spotlight` is a command palette, and `Tooltip`
plugs into gpui's built-in `.tooltip(...)` via the `tooltip(...)` helper.

Feedback components communicate state: `Alert` (inline callout), `Loader`
(animated pulsing dots/bars), `Progress` / `RingProgress` (determinate bar and
circular gauge), `Notification` (an elevated toast card), and `ToastStack` (a
positioned, stacking toast manager).

Data display: `Avatar`, `List`, `Table`, and `Timeline` are stateless builders;
`Tabs` and `Accordion` are stateful entities whose panel content is a builder
closure (`|window, app| ...`) re-invoked each frame so panels can show live data.

Navigation: `Breadcrumbs`, `NavLink`, and `Stepper` are stateless builders;
`Pagination` is a stateful entity (windowed page list with ellipses);
`StatusBar` is a themed app shell with left/center/right slots. The gallery also
wires a **native window menu** (`cx.set_menus`) whose actions toggle the theme
and quit.

## Flex layout (`guise::flex`)

A Flutter-flavored layout kit on top of gpui's flexbox: `Row`, `Column`,
`Container`, `Padding`, `Align`, `Center`, `Expanded`/`Flexible` (real flex
weights), `Spacer`, `SizedBox`, `Stack`/`Positioned`, and `Wrap`, with
`MainAxisAlignment` / `CrossAxisAlignment` / `EdgeInsets`. It is **not**
glob-exported (names overlap with `guise::layout`); import it as
`use guise::flex::*`.

## Layout macros

Terse builders, available from the prelude:

```rust
col![
    row![avatar, name, Spacer::new(), actions],   // guise::flex::Row
    divider,
    body,
]
```

`row!`/`col!`/`zstack!`/`wrap!` build `flex` containers; `vstack!`/`hstack!`
build the themed `layout::Stack`/`Group`. (It's `col!`, not `column!`, to avoid
the std `column!` macro.)

There's also a CSS-like `style!` block for inline styling, applied with `.apply`:

```rust
gpui::div().apply(style! {
    display: flex;
    direction: column;
    gap: 8;
    padding: 16;
    radius: 12;
    background: "#11151c";
    color: color!(rgb(230, 230, 230));
    border: color!("#2a2f3a");
})
```

Numbers are pixels; colors take a CSS string or any `color!`/`Hsla`. See
[Macros](docs/macros.md#style--css-like-style-blocks).

## Reactive state (`guise::reactive`)

A small React-flavored layer over gpui's reactivity:

```rust
// Create state and provide it (React's useState + Context.Provider).
let count = use_state(cx, 0i32);
provide(cx, count.clone());

// In a descendant view's constructor: read it back and subscribe.
let count = use_context::<Signal<i32>>(cx).unwrap();
watch(cx, &count);          // re-render this view when it changes

// Anywhere with app/context access:
count.update(cx, |n| *n += 1);   // notifies every watcher
```

- **`Signal<T>`** — a clonable observable state cell (all clones share one value).
- **`provide` / `use_context`** — Context/Provider, keyed by type (the gpui idiom).
- **`use_state` / `watch`** — hook-style helpers.
- **`use_form` / `FormState`** — field values, validators, and errors keyed by
  name, with built-in `required` / `min_len` / `email` validators.

## Transitions

Mount animations on gpui's animation API: `Transition` plays a one-shot
fade/slide as its child appears (`TransitionKind::Fade | SlideUp | SlideDown |
SlideLeft | SlideRight`), and `Collapse` reveals gated content with a fade.

```rust
Transition::new("hero").kind(TransitionKind::SlideUp).child(card)
Collapse::new("details").open(self.expanded).child(detail)
```

## Variants

Colored components share Mantine's variant system: `Filled`, `Light`,
`Outline`, `Subtle`, `Default`, `Transparent`, `White`.

## Installation

```sh
cargo add guise-ui
```

or in `Cargo.toml`:

```toml
[dependencies]
guise-ui = "0.2"
gpui = "0.2"
```

> The crate is published on crates.io as **`guise-ui`** (the `guise` name was
> taken), but its library is named `guise` — so you still write
> `use guise::prelude::*;`.

`guise` tracks the published [`gpui`](https://crates.io/crates/gpui) `0.2`
release on crates.io.

## Building

Requires Rust stable.

```sh
cargo run -p gallery     # launch the component gallery
cargo test -p guise      # run the library's unit tests
```

## License

MIT — see [LICENSE](LICENSE).
