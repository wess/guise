# guise

[![crates.io](https://img.shields.io/crates/v/guise-ui.svg)](https://crates.io/crates/guise-ui)
[![docs.rs](https://img.shields.io/docsrs/guise-ui)](https://docs.rs/guise-ui)
[![CI](https://github.com/wess/guise/actions/workflows/ci.yml/badge.svg)](https://github.com/wess/guise/actions/workflows/ci.yml)
[![license](https://img.shields.io/crates/l/guise-ui.svg)](https://github.com/wess/guise/blob/main/LICENSE)

A [Mantine](https://mantine.dev)-inspired component library for
[gpui](https://github.com/zed-industries/zed) — the GPU-accelerated Rust UI
framework that powers Zed.

`guise` brings Mantine's ergonomics to gpui: a themed palette, sizing tokens,
120+ composable components, a reactive state layer with two-way bindings, and
the full [Lucide](https://lucide.dev) icon set embedded as the default icons —
no asset pipeline needed.

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

Full docs live in [`docs/`](docs/readme.md) (also rendered at
[wess.github.io/guise](https://wess.github.io/guise/docs.html)):

- **[Tutorial](docs/tutorial.md)** — build a complete app step by step ([web version](https://wess.github.io/guise/tutorial.html))
- **[App walkthrough](docs/appguide.md)** — a project tracker wired the way a real guise app fits together
- [Getting started](docs/gettingstarted.md) · [Theming](docs/theming.md) · [Component model](docs/components.md)
- Components: [Buttons](docs/buttons.md) · [Icons](docs/icons.md) · [Inputs](docs/inputs.md) · [Dates & times](docs/dates.md) · [File handling](docs/files.md) · [Typography](docs/typography.md) · [Layout](docs/layout.md) · [Panels](docs/panels.md) · [Feedback](docs/feedback.md) · [Data](docs/data.md) · [Charts](docs/charts.md) · [Editor](docs/editor.md) · [Markdown editor](docs/markdowneditor.md) · [Overlays](docs/overlays.md) · [Navigation](docs/navigation.md)
- Systems: [Flex layout](docs/flex.md) · [Macros](docs/macros.md) · [Transitions & animation](docs/transitions.md) · [Drag & drop](docs/dnd.md) · [Reactive state](docs/reactive.md) · [Window menu](docs/windowmenu.md) · [Architecture](docs/architecture.md)

## Workspace

- **`crates/guise`** — the component library.
- **`crates/gallery`** — a live showcase of every component (the Mantine-docs
  equivalent). Run it with `cargo run -p gallery`.

## How guise compares

Two other component libraries target gpui:
[gpui-component](https://github.com/longbridge/gpui-component) (shadcn-flavored,
backs Longbridge Pro) and
[adabraka-ui](https://github.com/Augani/adabraka-ui). A quick orientation, as
of July 2026:

|  | **guise** | **gpui-component** | **adabraka-ui** |
| --- | --- | --- | --- |
| Design language | Mantine | shadcn/ui | shadcn/ui |
| Components | 120+ | 60+ | ~140 |
| Reactive layer | `Signal` / `Binding` / lenses, reactive `Form` | — (entities + subscriptions) | — |
| Icons | all 1,991 Lucide glyphs as an embedded font, zero setup | 99 Lucide SVGs via an assets crate | ~1,600 SVGs, copied into your app manually |
| Theming | Mantine palette, JSON theme files, 6 presets, per-slot overrides | ~140 tokens, JSON themes with hot reload, 22 presets | 19 presets, theme behind a global `Mutex` |
| Code editor | 10-language highlighter + diagnostics API | tree-sitter (~35 languages) + LSP client | tree-sitter (22 languages) |
| Docking / panels | `PaneGroup` splits-with-tabs + layout persistence | `DockArea` + floating `Tiles` + serialization | resizable/split panels |
| Charts | 6 types with axes, legends, hover | 6 types incl. candlestick & Sankey on a plot framework | 11 types |
| Motion | easing curves, spring physics, exit animations | basic easing | large effects library |
| Drag & drop | typed payloads, sortable lists | panel docking | draggable + sortable |
| Date/time pickers | yes | yes (incl. range presets) | yes |
| Tests | 300+ incl. gpui entity harness | ~580 incl. render tests | minimal |
| gpui dependency | crates.io releases | crates.io releases; dev tracks zed main | a custom gpui fork |
| License | MIT | Apache-2.0 | MIT |

Reach for **guise** if you want Mantine's ergonomics, SwiftUI-style two-way
bindings, and icons that just work with zero asset setup. **gpui-component**
is the bigger ecosystem — a production code editor with LSP, a full dock
system, and a WASM story. **adabraka-ui** ships the largest effects/animation
collection.

## Theme

Install a theme once at startup; every component reads it from the gpui global:

```rust
guise::Theme::dark().init(cx);       // or Theme::light()
guise::Theme::catppuccin().init(cx); // or nord / tokyonight / gruvbox / dracula / solarized_light
```

The theme carries the full Mantine / open-color palette (14 colors × 10 shades),
`xs..xl` scales for spacing, radius and font size, and scheme-aware semantic
colors (`body`, `surface`, `text`, `dimmed`, `border`, plus `success` /
`warning` / `danger` / `info` feedback accents). Themes also load from flat
JSON files — `Theme::from_json(source)` — with no serde dependency.

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
| Layout  | `Stack`, `Group`, `Center`, `SimpleGrid`, `ScrollArea`, `AppShell`, `Container`, `Space`, `Panel`, `SplitPanel`, `Breakpoint`/`Responsive` |
| Surface | `Paper`, `Card`                                         |
| Typography | `Text`, `Title`, `Mark`, `Blockquote`, `Spoiler`    |
| Inputs  | `Button`, `TextInput`, `TextArea`, `NumberInput`, `PasswordInput`, `PinInput`, `Checkbox`, `Switch`, `Radio`, `RadioGroup`, `CheckboxGroup`, `Select`, `Combobox`, `Autocomplete`, `Slider`, `RangeSlider`, `Rating`, `ColorInput`, `TagsInput`, `Transfer`, `Field` |
| Dates & files | `Calendar`, `DatePicker`, `TimePicker`, `FileInput`, `Dropzone` (with pure `Date`/`Time` models) |
| Editor  | `Editor` (highlighting for Rust, SQL, JSON, TOML, Python, JS/TS, Go, C, Markdown; LSP-shaped diagnostics), `MarkdownEditor` (Obsidian-style live preview) |
| Overlays | `Modal`, `Drawer`, `Menu`, `MenuBar`, `ContextMenu`, `HoverCard`, `LoadingOverlay`, `ConfirmModal`, `Popover`, `Spotlight`, `Tooltip`, `Tour`, `OverlayHost` (window-level modal stack + toasts) |
| Feedback | `Alert`, `Loader`, `Progress`, `RingProgress`, `Notification`, `ToastStack` |
| Data    | `Badge`, `Divider`, `Avatar`, `AvatarGroup`, `List`, `VirtualList`, `Table`, `TableView`, `DataView`, `TreeView`, `TabBar`, `Image`, `Timeline`, `Tabs`, `Accordion`, `Carousel` |
| Charts  | `Sparkline`, `LineChart`, `AreaChart`, `BarChart`, `ScatterChart`, `PieChart` — with optional axes, legends, and hover readouts |
| Navigation | `Breadcrumbs`, `NavLink`, `NavigationMenu`, `Stepper`, `Pagination`, `StatusBar` |
| Drag & drop | `Draggable`, `DropTarget`, `SortableList` — typed payloads |
| Motion  | `Transition`, `Collapse` (true height animation), `Presence` (exit animations), `Easing` curves + `Spring` physics |
| Polish  | `Icon` (all of [Lucide](https://lucide.dev) embedded), `ActionIcon`, `ThemeIcon`, `CloseButton`, `CopyButton`, `Anchor`, `Code`, `Kbd`, `Chip`, `Indicator`, `Skeleton`, `SegmentedControl` |

Inputs come in two flavors that match how each control behaves in gpui:

- **Controlled** (`Checkbox`, `Switch`, `Radio`, `Rating`, and the `RadioGroup` /
  `CheckboxGroup` wrappers) are `RenderOnce` builders — the parent view owns the
  value and passes a change handler via `cx.listener(...)`.
- **Stateful** (`TextInput`, `TextArea`, `NumberInput`, `PasswordInput`,
  `PinInput`, `Select`, `Combobox`, `Autocomplete`, `Slider`, `RangeSlider`,
  `ColorInput`, `TagsInput`, `DatePicker`, `TimePicker`, `FileInput`,
  `Transfer`) are gpui entities that own their buffer / selection. Create with
  `cx.new(...)` and subscribe to their events. `Field` is the shared
  label/description/error chrome these compose.

Overlays paint above the page (a `deferred` layer): `Modal` and `Drawer` are
controlled backdrops, `ConfirmModal` a confirm/cancel dialog, `Menu` a
keyboard-navigable action list, `MenuBar` a themed in-window application menu,
`ContextMenu` opens at the pointer, `Popover` is the anchored-floating
primitive, `HoverCard` its hover-triggered sibling, `Spotlight` a command
palette, `Tour` a step-by-step onboarding walkthrough, and **`OverlayHost`**
owns a window-level modal stack and toast queue — open dialogs from any
handler, focus restores on close.

Data display scales from stateless builders (`Avatar`, `List`, `Table`,
`Timeline`) to virtualized, signal-bound entities: `TableView<T>` (typed rows,
sorting, selection, virtualized body), `DataView<T>` (list/grid over a
`Signal<Vec<T>>` with filter/sort projections and windowed rendering),
`TreeView` (expandable hierarchy, virtualizable), and `VirtualList` (100k rows
render as cheaply as 20). `PaneGroup` is the Zed-style splits-with-tabs
workspace, including drag-to-split and layout snapshots that persist across
sessions.

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
build the themed `layout::Stack`/`Group`. There's also a CSS-like `style!`
block for inline styling — see
[Macros](docs/macros.md#style--css-like-style-blocks).

## Reactive state (`guise::reactive`)

A small React-flavored layer over gpui's reactivity:

```rust
let count = use_state(cx, 0i32);      // Signal<i32>
provide(cx, count.clone());           // Context.Provider, keyed by type
watch(cx, &count);                    // re-render this view on change
count.update(cx, |n| *n += 1);        // notifies every watcher
```

**Bindings** are SwiftUI-style two-way connections: controlled builders take
`.bind(signal.binding())`, stateful entities bind with
`X::bind(&entity, &signal, cx)` — the value flows both directions with no
hand-written change handlers. `signal.lens(get, set)` projects one struct
field; `binding.map(from, into)` converts types both ways.

**Forms**: every `Form` field is its own `Signal<String>`, so inputs bind
straight to fields; rules (including cross-field like `equals_field`) run on
submit and errored fields re-validate live as they're edited:

```rust
let form = Form::new(cx)
    .field(cx, "email", "")
    .rule("email", validators::email());
TextInput::bind(&email_input, &form.signal("email"), cx);
if let Some(values) = form.submit(cx) { /* … */ }
```

See [Reactive state](docs/reactive.md).

## Motion

Easing curves (including a CSS `cubic-bezier` solver), closed-form `Spring`
physics, `Transition` entrances, a `Collapse` that animates real height both
directions, and `Presence` for exit animations on conditionals:

```rust
Collapse::new("details")
    .open(self.expanded)
    .height(120.0)
    .easing(Easing::Spring(Spring::default()))
    .child(detail_panel())
```

See [Transitions & animation](docs/transitions.md).

## Variants

Colored components share Mantine's variant system: `Filled`, `Light`,
`Outline`, `Subtle`, `Default`, `Transparent`, `White`.

## Installation

`guise` builds against **crates.io gpui 0.2.2** — no git pins, no patch
sections:

```toml
[dependencies]
guise-ui = "0.10"
gpui = "0.2.2"
```

> The crate is named **`guise-ui`** (the `guise` name was taken on crates.io),
> but its library is named `guise` — so you still write
> `use guise::prelude::*;`.

Pinning via git works too:

```toml
guise-ui = { git = "https://github.com/wess/guise", tag = "v0.10.0" }
```

## Building

Requires Rust stable.

```sh
cargo run -p gallery     # launch the component gallery
cargo test -p guise-ui   # run the library's tests (unit + gpui entity harness)
```

## License

MIT — see [LICENSE](LICENSE).
