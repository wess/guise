# Architecture

## Workspace

```
guise/
├── Cargo.toml            # workspace; patches crates.io gpui onto a pinned zed rev
├── docs/                 # human docs (this directory)
├── site/                 # docs-website generator (Bun; one page per docs/*.md, via render/nav.ts)
└── crates/
    ├── guise/            # the library — published as `guise-ui`, lib name `guise`
    └── gallery/          # a live showcase (cargo run -p gallery)
```

## The gpui dependency

The manifests request `gpui = "0.2.2"` from crates.io, but the root
`[patch.crates-io]` block redirects that onto a pinned zed rev — the
components track gpui's git line, which is newer than the crates.io API.
Cargo patches don't propagate through git dependencies, so a consumer pinning
guise via git must mirror the workspace's `[patch.crates-io]` section
(including zed's own `async-process` / `async-task` patches).
(`thirdparty/block/` is a leftover vendored crate; no manifest references
it.)

The library package is **`guise-ui`** — the `guise` name was taken on
crates.io — with `[lib] name = "guise"`. Cargo commands address the package as
`-p guise-ui`, while code imports stay `use guise::...`.

## Library module map (`crates/guise/src`)

| Module | Contents |
| --- | --- |
| `theme/` | `Theme`, `Color`, `Palette`, `Scale`, `Size`, `ColorScheme`, JSON theme files (`Theme::from_json`), prebuilt presets (`Theme::preset`) |
| `style.rs` | the `Variant` system and `surface()` resolver |
| `layout/` | themed `Stack`, `Group`, `Center`, `SimpleGrid`, `AppShell`, `Container`, `Space`, plus `Breakpoint`/`Responsive` |
| `flex/` | Flutter-style `Row`, `Column`, `Container`, `Expanded`, … |
| `input/` | `TextInput`, `TextArea`, `NumberInput`, `PasswordInput`, `PinInput`, `Select`, `Combobox`, `Checkbox`, `Switch`, `Radio`, `RadioGroup`, `CheckboxGroup`, `SegmentedControl`, `Slider`, `RangeSlider`, `Rating`, `ColorInput`, `TagsInput`, `Field`, `Autocomplete`, `Calendar`, `DatePicker`, `TimePicker`, `FileInput`, `Dropzone`, `Transfer`, the `Date`/`Time` models, the `TextEdit` model, the shared single-line key map (`keys.rs`) |
| `editor/` | `Editor` entity, the `EditorModel` buffer, `Language` highlighters (Rust, SQL, JSON, TOML, Python, JS/TS, Go, C, Markdown), `Diagnostic`/`Severity` |
| `markdown/` | `MarkdownEditor` entity (live-preview markdown) over pure `block` / `inline` / `layout` passes |
| `data/` | `Avatar`, `AvatarGroup`, `List`, `VirtualList`, `Table`, `TableView`, `DataView`, `TreeView`, `TabBar`, `Timeline`, `Tabs`, `Accordion` |
| `chart/` | `Sparkline`, `LineChart`, `AreaChart`, `BarChart`, `ScatterChart`, `PieChart` — canvas-painted builders with optional axes/legends/hover |
| `feedback/` | `Alert`, `Loader`, `Progress`, `RingProgress`, `Notification`, `ToastStack` |
| `overlay/` | `Modal`, `ConfirmModal`, `Drawer`, `Menu`, `MenuBar`, `ContextMenu`, `Popover`, `HoverCard`, `LoadingOverlay`, `Spotlight`, `Tooltip`, `Tour`, `OverlayHost` (window-level modal stack + toasts) |
| `nav/` | `Breadcrumbs`, `NavLink`, `NavigationMenu`, `Stepper`, `Pagination`, `StatusBar` |
| `reactive/` | `Signal`, `Binding`, Context/Provider, hooks (`use_state`/`watch`/`use_memo`/`use_effect`), `Form` (per-field signals) + `FormState` |
| `macros.rs` | the `row!`/`col!`/… layout macros |
| `anim/` | `Easing` curves, `Spring` physics, `Presence` (exit animations) |
| `dnd/` | `Draggable`, `DropTarget`, `SortableList` — typed drag payloads |
| `transition.rs` | `Transition` / `Collapse` (true height) animations |
| `webview.rs` | `WebView` — native embedded web view via `wry` (default-on `webview` feature) |
| root files | `Button`, `Badge`, `Card`, `Paper`, `Panel`, `SplitPanel`, `Image`, `Mark`, `Blockquote`, `Spoiler`, `Text`, `Title`, `Anchor`, `Code`, `Kbd`, `Icon`, `ActionIcon`, `ThemeIcon`, `CloseButton`, `CopyButton`, `Chip`, `Indicator`, `Skeleton`, `Divider`, `ScrollArea`, `Carousel` |

## Conventions

- **One component per file**, lowercase names, no `-`/`_`/spaces; group with
  directories (`input/select.rs`), not concatenated names.
- **Read everything from the theme** via `guise::theme::theme(cx)` — never
  hardcode a color or size. This is what makes light/dark switching free.
- Builder methods take `mut self` and return `Self` (chainable).
- Container components implement `ParentElement` (just `extend`); `.child` /
  `.children` come for free.
- Resolve all theme values into locals **before** any `cx.listener(...)` or
  content-builder call — `theme(cx)` borrows `cx` immutably and those need it
  mutably, so a late `theme(cx)` read overlaps the borrow and won't compile.
- Closures stored on elements (`.hover`, `.on_click`) must be `'static` — capture
  resolved `Hsla`/`f32` values, not the `&Theme` borrow.

## Adding a component

1. Create a file under the right module (or the crate root for a loose one).
2. Define a `#[derive(IntoElement)]` builder + `impl RenderOnce`, or a
   `Render` + `EventEmitter` entity if it owns state. Resolve visuals from
   `theme(cx)`.
3. Re-export it from the module's `mod.rs`, then from `lib.rs`, then add it to
   the `prelude`.
4. Add a showcase to `crates/gallery/`.
5. Write the component's docs section on the right `docs/` page — and if that
   page is new, register it in `site/render/nav.ts` so the website picks it up.
6. For pure logic (parsing, range math, an editing model), add `#[cfg(test)]`
   tests next to the code.

See the [component model](components.md) for the two patterns in detail.

## Commands

```sh
cargo run -p gallery        # launch the showcase
cargo check -p guise-ui     # fast type-check (package is guise-ui; lib name is guise)
cargo test -p guise-ui      # unit tests
cargo build -p gallery      # full build of the binary
```
