# Architecture

## Workspace

```
guise/
├── Cargo.toml            # workspace manifest (plain crates.io gpui)
├── docs/                 # human docs (this directory)
├── site/                 # docs-website generator (Bun; one page per docs/*.md, via render/nav.ts)
├── scripts/              # the app bundle, the DMG, the app icon, the icon-font generator
├── extensions/zed/       # a Zed context server for tailor-mcp — its own workspace (wasm32-wasip2)
└── crates/
    ├── guise/            # the library — published as `guise-ui`, lib name `guise`
    ├── gallery/          # a live showcase (cargo run -p gallery)
    └── tailor/           # Tailor, the visual interface builder (see tailor.md)
        ├── model/        #   the document: catalog, node tree, file format
        ├── codegen/      #   document -> idiomatic guise Rust
        ├── store/        #   project files, recents, settings, export, the editor bridge
        ├── render/       #   document -> live guise components
        ├── app/          #   the gpui workbench (cargo run -p tailor-app)
        └── mcp/          #   an MCP server over the same document model
```

Only `crates/guise` is published. The gallery and the six Tailor crates are
`publish = false`; they are in the workspace so CI builds them and so the
library and the builder can never drift apart. `extensions/zed/` is deliberately
*outside* the workspace — it targets `wasm32-wasip2`, which is not a target the
rest of the repository can be built for.

The version lives once, in `[workspace.package]`, and covers both things the
repository ships: the library on crates.io and the Tailor app in the release
assets. `Cargo.lock` is committed and CI builds `--locked`, so bumping the
version means regenerating the lockfile in the same commit.

## The gpui dependency

Everything builds against **crates.io `gpui = "0.2.2"`** — no git pins and
no `[patch.crates-io]` section, so `guise-ui` installs as a plain registry
dependency and publishes cleanly. The few style/scroll APIs the crates.io
snapshot lacks are shimmed in `style.rs` (`FlexExt`) via gpui's raw
`StyleRefinement`. (`thirdparty/block/` is a leftover vendored crate; no
manifest references it.)

The library package is **`guise-ui`** — the `guise` name was taken on
crates.io — with `[lib] name = "guise"`. Cargo commands address the package as
`-p guise-ui`, while code imports stay `use guise::...`.

## Library module map (`crates/guise/src`)

| Module | Contents |
| --- | --- |
| `theme/` | `Theme`, `Color`, `Palette`, `Scale`, `Size`, `ColorScheme`, JSON theme files (`Theme::from_json`), prebuilt presets (`Theme::preset`), and the `ThemeManager` registry + `ThemeChoice` |
| `style.rs` | the `Variant` system and `surface()` resolver |
| `layout/` | themed `Stack`, `Group`, `Center`, `SimpleGrid`, `AppShell`, `Container`, `Space`, plus `Breakpoint`/`Responsive` |
| `flex/` | Flutter-style `Row`, `Column`, `Container`, `Expanded`, … |
| `input/` | `TextInput`, `TextArea`, `NumberInput`, `PasswordInput`, `PinInput`, `Select`, `Combobox`, `Checkbox`, `Switch`, `Radio`, `RadioGroup`, `CheckboxGroup`, `SegmentedControl`, `Slider`, `RangeSlider`, `Rating`, `ColorInput`, `TagsInput`, `Field`, `Autocomplete`, `Calendar`, `DatePicker`, `TimePicker`, `FileInput`, `Dropzone`, `Transfer`, the `Date`/`Time` models, the `TextEdit` model, the shared single-line key map (`keys.rs`), and `line.rs` — the shared field element (glyph-accurate caret, hit-testing, scrolling, IME) every single-line input is built on |
| `editor/` | `Editor` entity, the `EditorModel` buffer, `Language` highlighters (Rust, SQL, JSON, TOML, Python, JS/TS, Go, C, Markdown), `Diagnostic`/`Severity` |
| `markdown/` | `MarkdownEditor` entity (live-preview markdown) and the read-only `Markdown` renderer, over pure `block` / `inline` / `layout` passes |
| `ai/` | `AIChatView`, `AIMessage`, `AIComposer`, `AIStreamingText`, `AIThinking`, `AIReasoning`, `AIToolCall`, `AICitation`, `AISources`, `AIModelPicker`, `AITokenMeter`, `AICost`, `AISettings` — transport-agnostic; the host owns the request |
| `data/` | `Avatar`, `AvatarGroup`, `List`, `VirtualList`, `Table`, `TableView`, `DataView`, `TreeView`, `TabBar`, `Timeline`, `Tabs`, `Accordion` |
| `chart/` | `Sparkline`, `LineChart`, `AreaChart`, `BarChart`, `ScatterChart`, `PieChart` — canvas-painted builders with optional axes/legends/hover |
| `feedback/` | `Alert`, `Loader`, `Progress`, `RingProgress`, `Notification`, `ToastStack` |
| `overlay/` | `Modal`, `ConfirmModal`, `Drawer`, `Menu`, `MenuBar`, `ContextMenu`, `Popover`, `HoverCard`, `LoadingOverlay`, `Spotlight`, `Tooltip`, `Tour`, `OverlayHost` (window-level modal stack + toasts) |
| `nav/` | `Breadcrumbs`, `NavLink`, `NavigationMenu`, `Stepper`, `Pagination`, `StatusBar` |
| `panegroup/` | The Zed-style workspace: a pure `PaneTree` of splits whose leaves are tabbed `Pane`s, plus `compute_layout`, `nav` and snapshot encode/decode — with the gpui entity layered on top. The host owns the items; the component owns layout, tab bars, dividers and drag/drop |
| `icon/` | `Icon`, `IconName`, `Glyph` — all 1,991 Lucide glyphs, drawn from an icon font embedded in the crate. `lucide.rs` is generated by `bun scripts/icons.ts`; never hand-edit it |
| `reactive/` | `Signal`, `Binding`, Context/Provider, hooks (`use_state`/`watch`/`use_memo`/`use_effect`), `Form` (per-field signals) + `FormState` |
| `settings/` | `SettingsView`, `SettingsSection`, `SettingsRow` — settings-screen chrome only; the schema and the write path stay in the app |
| `devtools/` | `DevTools` — a Safari-shaped inspector for the app itself: an Elements tree recorded by `Probed::probe` (with real `StyleRefinement` snapshots), plus Logs / Network / Storage / Timelines fed by the host, Sources read off disk, and an Audit computed from the tree |
| `update/` | self-update: `Updater`/`UpdateConfig` (release check + in-place install, gpui-free), SHA-256 verification of the download (`checksum.rs`), and the `UpdatePrompt`/`UpdateNotice` entities that drive it |
| `macros.rs` | the `row!`/`col!`/… container macros, plus `style!` and `color!` |
| `anim/` | The animation system: `Easing`/`Curve`, `Spring`, keyframed `Motion`, `Sequence`, `Stagger`, the `Animated`/`.animate(..)` one-shots, the `Animator` playhead, `Presence` (exit animations), and the `motion!` / `sequence!` macros |
| `dnd/` | `Draggable`, `DropTarget`, `SortableList` — typed drag payloads |
| `transition.rs` | `Transition` / `Collapse` (true height) animations |
| `webview.rs` | `WebView` — native embedded web view via `wry` (default-on `webview` feature) |
| root files | `Button`, `Badge`, `Card`, `Paper`, `Panel`, `SplitPanel`, `Image`, `Mark`, `Blockquote`, `Spoiler`, `Text`, `Title`, `Anchor`, `Code`, `Kbd`, `Icon`, `ActionIcon`, `ThemeIcon`, `CloseButton`, `CopyButton`, `Chip`, `Indicator`, `Skeleton`, `Divider`, `ScrollArea`, `Carousel`, `ThemePicker` |

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
3. End `render` with `.probe("Name")` — or `.probe_any` when the root is
   already a composed component rather than a styled element — plus an
   `.attr(..)` per meaningful `Copy` prop. Without it the component is invisible
   in the DevTools Elements tree.
4. Re-export it from the module's `mod.rs`, then from `lib.rs`, then add it to
   the `prelude`.
5. Add a showcase to `crates/gallery/`.
6. Write the component's docs section on the right `docs/` page — and if that
   page is new, register it in `site/render/nav.ts` so the website picks it up.
7. For pure logic (parsing, range math, an editing model), add `#[cfg(test)]`
   tests next to the code. For wiring that needs a live app — signals, bindings,
   entity events, the theme global — use the gpui test harness in
   `src/apptests.rs`.
8. If Tailor should be able to place it, add a `comp!` entry to
   `crates/tailor/model/src/catalog/` and an arm to
   `crates/tailor/render/src/nodes/build.rs`. Editing one without the other is
   how a canvas and an export drift apart.

See the [component model](components.md) for the two patterns in detail.

## Commands

```sh
cargo run -p gallery        # launch the showcase
cargo run -p tailor-app     # launch Tailor, the interface builder (binary: tailordev)
cargo check -p guise-ui     # fast type-check (package is guise-ui; lib name is guise)
cargo test -p guise-ui      # the library's tests: inline #[cfg(test)] + src/apptests.rs
cargo test -p tailor-model -p tailor-codegen -p tailor-store   # Tailor's gpui-free half
cargo build --workspace --locked                               # what CI builds
cd site && bun run build.ts                                    # docs/ -> site/dist
```
