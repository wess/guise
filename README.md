# guise

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
- Components: [Buttons](docs/buttons.md) · [Inputs](docs/inputs.md) · [Typography](docs/typography.md) · [Layout](docs/layout.md) · [Feedback](docs/feedback.md) · [Data](docs/data.md) · [Overlays](docs/overlays.md) · [Navigation](docs/navigation.md)
- Systems: [Flex layout](docs/flex.md) · [Macros](docs/macros.md) · [Reactive state](docs/reactive.md) · [Window menu](docs/windowmenu.md) · [Architecture](docs/architecture.md)

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

## Components

| Group   | Components                                              |
| ------- | ------------------------------------------------------- |
| Layout  | `Stack`, `Group`, `Center`                              |
| Surface | `Paper`, `Card`                                         |
| Typography | `Text`, `Title`                                     |
| Inputs  | `Button`, `TextInput`, `Checkbox`, `Switch`, `Radio`, `Select` |
| Overlays | `Modal`, `Menu`, `Tooltip`                             |
| Feedback | `Alert`, `Loader`, `Progress`, `Notification`         |
| Data    | `Badge`, `Divider`, `Avatar`, `AvatarGroup`, `List`, `Table`, `Tabs`, `Accordion` |
| Navigation | `Breadcrumbs`, `NavLink`, `Stepper`, `Pagination`, `StatusBar` |
| Polish  | `ActionIcon`, `ThemeIcon`, `CloseButton`, `CopyButton`, `Anchor`, `Code`, `Kbd`, `Chip`, `Indicator`, `Skeleton`, `SegmentedControl` |

Inputs come in two flavors that match how each control behaves in gpui:

- **Controlled** (`Checkbox`, `Switch`, `Radio`) are `RenderOnce` builders —
  the parent view owns the value and passes a change handler via
  `cx.listener(...)`.
- **Stateful** (`TextInput`, `Select`) are gpui entities that own their
  buffer / open-state. Create with `cx.new(|cx| TextInput::new(cx))` and
  subscribe to their events (`TextInputEvent`, `SelectEvent`).

Overlays paint above the page: `Modal` is a controlled `RenderOnce` backdrop
(render it while `opened`, pass `on_close`), `Menu` is a stateful trigger +
deferred action list, and `Tooltip` plugs into gpui's built-in `.tooltip(...)`
via the `tooltip(...)` helper.

Feedback components communicate state: `Alert` (inline callout), `Loader`
(animated pulsing dots/bars via gpui's animation API), `Progress` (a
determinate bar), and `Notification` (an elevated toast card).

Data display: `Avatar`, `List`, and `Table` are stateless builders; `Tabs` and
`Accordion` are stateful entities whose panel content is a builder closure
(`|window, app| ...`) re-invoked each frame so panels can show live data.

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

## Variants

Colored components share Mantine's variant system: `Filled`, `Light`,
`Outline`, `Subtle`, `Default`, `Transparent`, `White`.

## Building

Requires Rust stable ≥ 1.96. gpui is pulled from the zed repo at a pinned rev;
the root `Cargo.toml` mirrors zed's `[patch.crates-io]` entries (cargo patches
don't propagate through git dependencies).

```sh
cargo run -p gallery     # launch the component gallery
cargo test -p guise      # run the library's unit tests
```

## License

MIT — see [LICENSE](LICENSE).
