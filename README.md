# guise

[![crates.io](https://img.shields.io/crates/v/guise-ui.svg)](https://crates.io/crates/guise-ui)
[![docs.rs](https://img.shields.io/docsrs/guise-ui)](https://docs.rs/guise-ui)
[![CI](https://github.com/wess/guise/actions/workflows/ci.yml/badge.svg)](https://github.com/wess/guise/actions/workflows/ci.yml)
[![license](https://img.shields.io/crates/l/guise-ui.svg)](https://github.com/wess/guise/blob/main/LICENSE)

A component library for [gpui](https://github.com/zed-industries/zed) — the
GPU-accelerated Rust UI framework that powers Zed — **and Tailor, the visual
interface builder for it**.

`guise` gives gpui a batteries-included component layer: a themed palette,
sizing tokens, 130+ composable components, a reactive state layer with two-way
bindings, an animation system with keyframes and a scrubbable playhead, and the
full [Lucide](https://lucide.dev) icon set embedded as the default icons — no
asset pipeline needed.

**[Tailor](docs/tailor.md)** is the other half: a drag-and-drop builder that
lays those same components out on a canvas and exports idiomatic Rust. Write the
interface or draw it — either way you end up with the same components.

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

## Tailor — the visual interface builder

**Tailor** is a drag-and-drop interface builder for gpui and guise, shaped like
Interface Builder and Android Studio's layout editor. Lay out a screen from real
components, wire the state and the actions, and export idiomatic Rust that has
no dependency on Tailor left in it.

```sh
cargo run -p tailor-app                     # from a checkout (binary: tailordev)
```

Or take the app: every [release](https://github.com/wess/guise/releases)
attaches **`Tailor.dmg`**, signed and built from this repository, with the MCP
server beside the executable in the bundle.

The canvas is not a drawing of your interface — it *is* your interface. A
`Button` on it is a `guise::Button`, reading the same theme, laid out by the
same flexbox. There is no second rendering path to keep in step, so a component
cannot look right in the builder and wrong in the app.

| Part | What it does |
| --- | --- |
| **The workbench** | A searchable library of all 101 placeable components, the node outline, the artboard, a five-tab inspector (Attributes, Size, Style, Connections, Identity) and a Problems panel. Every panel resizes, folds away, and remembers where you left it. |
| **Direct manipulation** | Eight resize knobs around the selection, drag to move, snapping to the grid and to siblings' edges with guides drawn where it caught, a live size readout, and arrow-key nudging. |
| **What comes out** | A `Render` entity when the document holds state, a `RenderOnce` builder when it does not. State variables become `Signal<T>` fields, events become `cx.listener` / `cx.subscribe`, and every resolved colour is hoisted into a `let` at the top of `render` the way guise's conventions require. |
| **A live window** | A second window rendering the document for real, following every edit — with the guise DevTools inspector in it, and right-click-to-inspect the way a browser does it. |
| **An MCP server** | `tailor-mcp` drives the same document model, so an agent can place components, wire state and generate Rust with no window open. It saves after every change and the app watches the file, so a screen built by an agent appears on the canvas as it is built. |
| **An editor jump** | Both directions. **Open in Editor** (⌥⌘O) puts your cursor on the line a component generated; `tailordev --reveal <file>:<line>` goes the other way, and a Zed task binds it to a key. Zed, VS Code, Sublime, IntelliJ, Emacs and Neovim. |

Full documentation starts at [`docs/tailor.md`](docs/tailor.md), and
[the tutorial](docs/tailortutorial.md) builds a complete app end to end — every
code block in it is output Tailor actually produced.

## Documentation

Full docs live in [`docs/`](docs/readme.md) (also rendered at
[wess.github.io/guise](https://wess.github.io/guise/docs.html)):

- **[Tutorial](docs/tutorial.md)** — build a complete app step by step ([web version](https://wess.github.io/guise/tutorial.html))
- **[App walkthrough](docs/appguide.md)** — a project tracker wired the way a real guise app fits together
- **[Motion tutorial](docs/motiontutorial.md)** — one animated panel, nine chapters ([web version](https://wess.github.io/guise/motiontutorial.html))
- [Getting started](docs/gettingstarted.md) · [Theming](docs/theming.md) · [Component model](docs/components.md)
- Components: [Buttons](docs/buttons.md) · [Icons](docs/icons.md) · [Inputs](docs/inputs.md) · [Dates & times](docs/dates.md) · [File handling](docs/files.md) · [Typography](docs/typography.md) · [Layout](docs/layout.md) · [Panels](docs/panels.md) · [Feedback](docs/feedback.md) · [Data](docs/data.md) · [Charts](docs/charts.md) · [GPU View](docs/gpuview.md) · [Editor](docs/editor.md) · [Markdown editor](docs/markdowneditor.md) · [AI](docs/ai.md) · [Overlays](docs/overlays.md) · [Navigation](docs/navigation.md)
- Systems: [Flex layout](docs/flex.md) · [Macros](docs/macros.md) · [Motion & transitions](docs/transitions.md) · [Drag & drop](docs/dnd.md) · [Reactive state](docs/reactive.md) · [Software update](docs/update.md) · [Settings](docs/settings.md) · [DevTools](docs/devtools.md) · [Window menu & chrome](docs/windowmenu.md) · [Architecture](docs/architecture.md) · [Size & performance](docs/performance.md)
- **Tailor**: [Overview](docs/tailor.md) · [Tutorial](docs/tailortutorial.md) · [The canvas](docs/tailorcanvas.md) · [Components & slots](docs/tailorcomponents.md) · [State & actions](docs/tailorstate.md) · [Generated code](docs/tailorcodegen.md) · [MCP server](docs/tailormcp.md) · [Zed & other editors](docs/tailorzed.md)
- [Releasing](docs/release.md) · [Changelog](CHANGELOG.md)

## Workspace

- **`crates/guise`** — the component library, published as `guise-ui`. The only
  crate here that reaches crates.io.
- **`crates/gallery`** — a live showcase of every component
  (`cargo run -p gallery`).
- **`crates/tailor/`** — Tailor, in six `publish = false` crates: `model` (the
  document, catalog and file format), `codegen` (document → Rust), `store`
  (project files, settings, export), `render` (document → live components),
  `app` (the workbench) and `mcp` (the MCP server).
- **`extensions/zed/`** — a Zed extension registering `tailor-mcp` as a context
  server. Its own cargo workspace; it targets `wasm32-wasip2`.
- **`site/`** — the Bun generator that renders `docs/` into the website.

Nothing about Tailor is in `guise-ui`: `cargo package -p guise-ui --list` is the
proof, and it is why the library still depends on nothing but gpui and std.

## How guise compares

Two other component libraries target gpui:
[gpui-component](https://github.com/longbridge/gpui-component) (shadcn-flavored,
backs Longbridge Pro) and
[adabraka-ui](https://github.com/Augani/adabraka-ui). A quick orientation, as
of July 2026:

|  | **guise** | **gpui-component** | **adabraka-ui** |
| --- | --- | --- | --- |
| Design language | open-color palette + token scales | shadcn/ui | shadcn/ui |
| Components | 130+ | 60+ | ~140 |
| Visual builder | Tailor, in this repo | — | — |
| Reactive layer | `Signal` / `Binding` / lenses, reactive `Form` | — (entities + subscriptions) | — |
| Icons | all 1,991 Lucide glyphs as an embedded font, zero setup | 99 Lucide SVGs via an assets crate | ~1,600 SVGs, copied into your app manually |
| Theming | open-color palette, JSON theme files, 6 presets, per-slot overrides | ~140 tokens, JSON themes with hot reload, 22 presets | 19 presets, theme behind a global `Mutex` |
| Code editor | 10-language highlighter + diagnostics API | tree-sitter (~35 languages) + LSP client | tree-sitter (22 languages) |
| Docking / panels | `PaneGroup` splits-with-tabs + layout persistence | `DockArea` + floating `Tiles` + serialization | resizable/split panels |
| Charts | 6 types with axes, legends, hover | 6 types incl. candlestick & Sankey on a plot framework | 11 types |
| Motion | easing curves, spring physics, exit animations | basic easing | large effects library |
| Drag & drop | typed payloads, sortable lists | panel docking | draggable + sortable |
| Date/time pickers | yes | yes (incl. range presets) | yes |
| Tests | 520+ incl. gpui entity harness | ~580 incl. render tests | minimal |
| gpui dependency | crates.io releases | crates.io releases; dev tracks zed main | a custom gpui fork |
| License | MIT | Apache-2.0 | MIT |

Reach for **guise** if you want chainable builders, SwiftUI-style two-way
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

The theme carries the full [open-color](https://yeun.github.io/open-color/)
palette (14 colors × 10 shades), `xs..xl` scales for spacing, radius and font size, and scheme-aware semantic
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
| Editor  | `Editor` (highlighting for Rust, SQL, JSON, TOML, Python, JS/TS, Go, C, Markdown; LSP-shaped diagnostics), `MarkdownEditor` (Obsidian-style live preview), `Markdown` (its read-only sibling) |
| AI      | `AIChatView`, `AIMessage`, `AIComposer`, `AIStreamingText`, `AIThinking`, `AIReasoning`, `AIToolCall`, `AICitation`, `AISources`, `AIModelPicker`, `AITokenMeter`, `AICost`, `AISettings` — transport-agnostic; the host owns the request |
| Overlays | `Modal`, `Drawer`, `Menu`, `MenuBar`, `ContextMenu`, `HoverCard`, `LoadingOverlay`, `ConfirmModal`, `Popover`, `Spotlight`, `Tooltip`, `Tour`, `OverlayHost` (window-level modal stack + toasts) |
| Feedback | `Alert`, `Loader`, `Progress`, `RingProgress`, `Notification`, `ToastStack` |
| Data    | `Badge`, `Divider`, `Avatar`, `AvatarGroup`, `List`, `VirtualList`, `Table`, `TableView`, `DataView`, `TreeView`, `TabBar`, `Image`, `Timeline`, `Tabs`, `Accordion`, `Carousel` |
| Workspace | `PaneGroup` — Zed-style splits-with-tabs, drag-to-split, and layout snapshots that persist |
| Charts  | `Sparkline`, `LineChart`, `AreaChart`, `BarChart`, `ScatterChart`, `PieChart` — with optional axes, legends, and hover readouts |
| Scenes  | `GpuView`, `GpuScene`, `GpuTexture` — retained native GPU surfaces for maps, simulations, and sprite-heavy status worlds |
| Navigation | `Breadcrumbs`, `NavLink`, `NavigationMenu`, `Stepper`, `Pagination`, `StatusBar` |
| Drag & drop | `Draggable`, `DropTarget`, `SortableList` — typed payloads |
| Motion  | `Transition`, `Collapse` (true height animation), `Presence` (exit animations), `Easing` curves + `Spring` physics |
| Update  | `Updater` (release check + in-place install), `UpdatePrompt`, `UpdateNotice` — a whole self-update feature |
| Settings | `SettingsView`, `SettingsSection`, `SettingsRow` — the settings screen: page list, groups, and label/control rows |
| DevTools | `DevTools` — an in-app Safari-style inspector: Elements, Network, Sources, Timelines, Storage, Layers, Logs, Audit |
| App chrome | `About` (with honest `BuildKind` dating), `WindowControls`, `ResizeHandles` |
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

## Macros

Terse builders, available from the prelude:

```rust
col![
    row![avatar, name, Spacer::new(), actions],   // guise::flex::Row
    divider,
    body,
]
```

`row!`/`col!`/`zstack!`/`wrap!` build `flex` containers; `vstack!`/`hstack!`
build the themed `layout::Stack`/`Group`. Two more are declaration blocks
rather than containers: `style!` for inline styling and `motion!` for
animation, plus `sequence!` for motions on one clock. See
[Macros](docs/macros.md).

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

An animation system in the shape of [anime.js](https://animejs.com), split in
two. The **description** is pure — a `Motion` is keyframed tracks over a
duration, a `Sequence` puts motions on one clock, a `Stagger` turns an index
into a delay, and `sample(t)` maps a millisecond offset to a `Frame` with no
state and nothing to tick. The **clock** is a thin shell: `.animate(id, clip)`
plays a clip once when an element mounts, and `Animator` is an entity holding a
playhead you can play, pause, reverse, scrub and re-speed.

```rust
div().child(card).animate("card", motion! {
    duration: 420;
    ease: out back;
    opacity: 0 => 1;
    y: 12 => 0;
})
```

Full easing matrix (`in`/`out`/`in_out` × ten curves, plus `steps`, a CSS
`cubic-bezier` solver and closed-form `Spring` physics), keyframes, stagger
with grid distance, `Presence` for exit animations on conditionals, and the
narrower `Transition`/`Collapse` wrappers when a fade or a real-height reveal
is all you need.

```sh
cargo run -p guise-ui --example motion      # the API, in one window
cargo run -p guise-ui --example checklist   # the motion tutorial's app
```

See [Motion & transitions](docs/transitions.md) and the
[motion tutorial](docs/motiontutorial.md).

## AI chat

`guise::ai` is everything a model-facing app needs on screen — a transcript, a
prompt box, streaming feedback, reasoning blocks, tool calls, citations, and the
meters around a request:

```rust
let chat = cx.new(|cx| AIChatView::new(cx).max_width(760.0));
let composer = cx.new(|cx| AIComposer::new(cx).hint("Shift+Enter for a new line"));

chat.update(cx, |chat, cx| {
    chat.push(AITurn::user(prompt), cx);
    chat.begin_reply(cx);
});
chat.update(cx, |chat, cx| chat.push_delta(&token, cx));   // as tokens arrive
```

**None of it opens a socket.** A component library is the wrong place to keep
someone's API key, so the host owns the request and these own what the user sees
while it happens — which is also what makes them portable: the same
`AIChatView` drives a local model, a hosted API, or a replayed transcript,
because all it ever receives is text. Message bodies render through
`markdown::Markdown`, the read-only sibling of `MarkdownEditor`. See
[AI](docs/ai.md).

## Self-update

`guise::update` is a whole self-update feature, not just its UI: it checks a
release feed, installs the new version **in place**, and restarts into it.

```rust
let updater = Updater::github("Acme", env!("CARGO_PKG_VERSION"), "acme/acme")
    .codesign_requirement("anchor apple generic and certificate leaf[subject.OU] = TEAMID")
    .before_restart(|cx| save_session(cx));

guise::update::start(updater.clone(), cx);      // at launch, then hourly
guise::update::check_now(updater, cx);          // "Check for Updates…"
```

On macOS the release `.dmg` is mounted and rsynced onto the installed `.app`, so
the bundle keeps its path *and* inode and LaunchServices never sees a stale
registration; a Linux AppImage is renamed over itself. Everything else opens the
release page — and the prompt's button says so rather than promising an install it
can't perform. Updates are verified against your `codesign` requirement before
anything touches the installed app. `UpdatePrompt` and `UpdateNotice` are ordinary
entities, so they work embedded in a window you own as readily as in the ones
`update::open` creates. See [Software update](docs/update.md).

## DevTools

`guise::devtools` is Safari's Web Inspector, aimed at the gpui app it is running
inside — Elements, Network, Sources, Timelines, Storage, Layers, Logs and Audit.

```rust
DevToolsState::new().init(cx);          // once at startup
let devtools = cx.new(DevTools::new);   // then put it wherever you like
```

The Elements tree is real introspection: every component tags its root with
`.probe("Name")`, which snapshots the element's `StyleRefinement` and brackets
`prepaint`, so the tree is the live component hierarchy and the Styles sidebar
shows the element's actual declarations, its real box model, and the source
location it was constructed at. Do the same in your own components and they
appear alongside the library's.

It reads as an **indented tree with YAML-flow props**, not as markup — these are
components built from builder calls, not tags, so there is no
attributes-versus-children distinction to draw and `<Button … />` would promise
a model gpui does not have. The recorder is scoped to one window, so put the
inspector in the window whose components you want to see.

Logs, Network, Storage and Timelines are reported by the host — nothing in
`guise` opens a socket — and Audit runs rules over the recorded tree (WCAG text
contrast, hit-target size, collapsed containers, children escaping their
parent). A probe costs one boolean check per element per frame while the
inspector is closed, and an app that never constructs one links none of it.

```sh
cargo run -p guise-ui --example devtools
```

See [DevTools](docs/devtools.md).

## Variants

Colored components share one variant system: `Filled`, `Light`,
`Outline`, `Subtle`, `Default`, `Transparent`, `White`.

## Installation

`guise` builds against **crates.io gpui 0.2.2** — no git pins, no patch
sections:

```toml
[dependencies]
guise-ui = "1.5"
gpui = "0.2.2"
```

> The crate is named **`guise-ui`** (the `guise` name was taken on crates.io),
> but its library is named `guise` — so you still write
> `use guise::prelude::*;`.

Pinning via git works too:

```toml
guise-ui = { git = "https://github.com/wess/guise", tag = "v1.5.4" }
```

## Building

Requires Rust stable.

```sh
cargo run -p gallery        # launch the component gallery
cargo run -p tailor-app     # launch Tailor, the interface builder
cargo test -p guise-ui      # the library's tests (unit + gpui entity harness)
cargo test -p tailor-model -p tailor-codegen -p tailor-store   # Tailor's pure half
cd site && bun run build.ts # render docs/ into site/dist
```

Cutting a release — the version, the lockfile, the tag, the signed `Tailor.dmg`
and the crates.io publish — is written down in
[releasing](docs/release.md).

## License

MIT — see [LICENSE](LICENSE).

♥ [Sponsor this project](https://github.com/sponsors/wess)
