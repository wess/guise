# Architecture

## Workspace

```
guise/
├── Cargo.toml            # workspace + [patch.crates-io] (mirrors zed)
├── thirdparty/block/     # vendored patch for the macOS gpui build
└── crates/
    ├── guise/            # the component library
    └── gallery/          # a live showcase (cargo run -p gallery)
```

## The gpui dependency

gpui is a git dependency pinned to a zed rev. Two things are required and easy to
get wrong:

1. **Rust stable ≥ 1.96.** Earlier toolchains fail on library features zed uses.
2. **Mirror zed's `[patch.crates-io]`.** Cargo patch entries do not propagate
   through git dependencies, so the consuming workspace must declare them itself.
   The root `Cargo.toml` carries the `async-process` / `async-task` forks and a
   vendored `block` crate (a transitive cocoa dep with a one-line fix).

If you bump the zed rev, re-check zed's root `Cargo.toml` patch section and match
it. This recipe is borrowed from the sibling `prompt` project.

## Library module map (`crates/guise/src`)

| Module | Contents |
| --- | --- |
| `theme/` | `Theme`, `Color`, `Palette`, `Scale`, `Size`, `ColorScheme` |
| `style.rs` | the `Variant` system and `surface()` resolver |
| `layout/` | themed `Stack`, `Group`, `Center`, `SimpleGrid` |
| `flex/` | Flutter-style `Row`, `Column`, `Container`, `Expanded`, … |
| `input/` | `TextInput`, `TextArea`, `NumberInput`, `Select`, `Combobox`, `Checkbox`, `Switch`, `Radio`, `RadioGroup`, `CheckboxGroup`, `SegmentedControl`, `Slider`, `Field`, the `TextEdit` model |
| `data/` | `Avatar`, `AvatarGroup`, `List`, `Table`, `Timeline`, `Tabs`, `Accordion` |
| `feedback/` | `Alert`, `Loader`, `Progress`, `RingProgress`, `Notification`, `ToastStack` |
| `overlay/` | `Modal`, `Drawer`, `Menu`, `Popover`, `Spotlight`, `Tooltip` |
| `nav/` | `Breadcrumbs`, `NavLink`, `Stepper`, `Pagination`, `StatusBar` |
| `reactive/` | `Signal`, Context/Provider, hooks, `FormState` |
| `macros.rs` | the `row!`/`col!`/… layout macros |
| `transition.rs` | `Transition` / `Collapse` mount animations |
| `webview.rs` | `WebView` — native embedded web view via `wry` (default-on `webview` feature) |
| root files | `Button`, `Badge`, `Card`, `Paper`, `Text`, `Title`, `Anchor`, `Code`, `Kbd`, `Icon`, `ActionIcon`, `ThemeIcon`, `CloseButton`, `Chip`, `Indicator`, `Skeleton`, `Divider`, `ScrollArea` |

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
5. For pure logic (parsing, range math, an editing model), add `#[cfg(test)]`
   tests next to the code.

See the [component model](components.md) for the two patterns in detail.

## Commands

```sh
cargo run -p gallery        # launch the showcase
cargo check -p guise        # fast type-check
cargo test -p guise         # unit tests
cargo build -p gallery      # full build of the binary
```
