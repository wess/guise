# guise

A component library for [gpui](https://github.com/zed-industries/zed)
(Zed's GPU-accelerated Rust UI framework). Workspace: `crates/guise` (library) +
`crates/gallery` (live showcase), with `docs/` (markdown) rendered by `site/` (Bun).
Full human docs live in [`docs/`](docs/readme.md);
[`docs/architecture.md`](docs/architecture.md) is the map,
[`docs/tutorial.md`](docs/tutorial.md) the walkthrough.

## Commands

```sh
cargo run -p gallery      # launch the showcase
cargo check -p guise-ui   # fast type-check (package is guise-ui; lib name is guise)
cargo test -p guise-ui    # 520+ tests: inline #[cfg(test)] + src/apptests.rs
cargo build --workspace --locked   # what CI builds
cd site && bun run build.ts        # docs/ -> site/dist
```

## Build constraints

- Everything builds against **plain crates.io `gpui = "0.2.2"`** — no git pins,
  no `[patch.crates-io]`. Don't use gpui APIs newer than that snapshot; the few
  style/scroll gaps it has are shimmed in `style.rs` (`FlexExt`).
  `thirdparty/block/` is a leftover vendored crate referenced by no manifest.
- The library package is **`guise-ui`** (crates.io name) with `[lib] name = "guise"`,
  so cargo commands use `-p guise-ui` while code imports `use guise::...`.

## The two component patterns

1. **Stateless `RenderOnce` builder** — `#[derive(IntoElement)]`, chainable
   `mut self -> Self` setters, parent owns all state. Most components. *Controlled*
   inputs (`Checkbox`, `Switch`, `Radio`) are this: parent holds the value and
   passes `.on_change(cx.listener(...))`.
2. **Stateful gpui entity** — `Render` + `EventEmitter<…>`, owns a
   `FocusHandle`/buffer/open-state. Built with `cx.new(...)`, parent subscribes to
   events. These are `TextInput`, `TextArea`, `NumberInput`, `PasswordInput`,
   `PinInput`, `Select`, `Combobox`, `SegmentedControl`, `Slider`, `RangeSlider`,
   `ColorInput`, `TagsInput`, `Menu`, `ContextMenu`, `HoverCard`, `Tabs`,
   `Accordion`, `Pagination`, `Editor`, `MarkdownEditor`, `TableView`, `DataView`, `TreeView`,
   `TabBar`, `SplitPanel`, `PaneGroup`, `UpdatePrompt`, `UpdateNotice`.

Both patterns can two-way bind to the reactive layer (`guise::reactive`):
`Signal<T>` is the store, `Binding<T>` the connection — controlled builders take
`.bind(signal.binding())`, entities take `X::bind(&entity, &signal, cx)`.

## Conventions (non-obvious — follow exactly)

- **Read every visual from the theme** via `theme(cx)` — never hardcode a color or
  size. This is what makes light/dark switching free. Semantic getters
  (`body`/`surface`/`text`/`dimmed`/`border`) and the `surface(theme, color, variant)`
  resolver in `style.rs` already encode the dark/light branches.
- **Resolve all theme values into locals BEFORE any `cx.listener(...)` or content
  closure.** `theme(cx)` borrows `cx` immutably; listeners need it mutably. A late
  `theme(cx)` read overlaps the borrow and won't compile.
- **Closures stored on elements (`.hover`, `.on_click`) must be `'static`** — capture
  resolved `Hsla`/`f32` values, not the `&Theme` borrow.
- **Tabs/Accordion panel content is a builder closure re-invoked every frame** so
  panels show live data, not a snapshot.
- **Overlays paint above siblings via `deferred()` + `occlude()`** (Modal, Menu,
  Select dropdown).
- Container components implement `ParentElement` (just `extend`); `.child`/`.children`
  come free.
- **Icons are Lucide**, drawn from an icon font embedded in the crate
  (`assets/lucide/`); it self-registers on first render, so consumers need no
  asset setup. `src/icon/lucide.rs` is generated — never hand-edit it;
  regenerate with `bun scripts/icons.ts`. Icon slots on components take
  `impl Into<Glyph>` (a Lucide `IconName` or literal text).

## Tailor

The visual interface builder that draws with this library lived here through
1.6.0 and is now its own project at [github.com/wess/tailor](https://github.com/wess/tailor). It depends on
`guise-ui` from crates.io.

One thing about it reaches back here: its catalog is checked against a record of
what this library ships, so **adding a component to guise without catalogueing
it there fails Tailor's tests**. That is deliberate — it is how a new component
gets picked up rather than quietly missed. Nothing in this repository needs to
change for it.

## File/naming conventions

- **One component per file**, lowercase, no `-`/`_`/spaces. Group with directories
  (`input/select.rs`), never concatenated names (`input-select.rs`).
- `flex/` is **not** glob-exported (names overlap with `layout/`); import via
  `use guise::flex::*`. `layout/` is token/`Size`-based; `flex/` is
  pixel-based Flutter-style (`Row`/`Column`/`Expanded`/`EdgeInsets`).
- `update/` (self-update: release check, in-place install, `UpdatePrompt`) is the
  one module that **shells out** — `curl`, and `hdiutil`/`rsync`/`codesign` on
  macOS — and the one that parses nested JSON, with its own reader
  (`theme/json.rs` is flat-only on purpose). Keep both confined there.

## Adding a component

1. New file under the right module (or crate root for a loose one).
2. `RenderOnce` builder, or `Render` + `EventEmitter` entity if it owns state.
   Resolve visuals from `theme(cx)`.
3. Re-export: module `mod.rs` → `lib.rs` → `prelude`.
4. Add a showcase to `crates/gallery/`.
5. Document it on the right `docs/*.md` page; a **new** page must also be
   registered in `site/render/nav.ts` or the site won't build it.
6. Unit-test pure logic (parsing, range math, editing models) with `#[cfg(test)]`
   next to the code. For wiring that needs a live app — signals, bindings, entity
   events, the theme global — use the gpui test harness:
   `#[gpui::test] fn x(cx: &mut TestAppContext)` in `src/apptests.rs`.
