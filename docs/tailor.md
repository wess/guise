# Tailor

Tailor is a visual interface builder for gpui and guise, and it ships in this
repository. You lay out a screen by dragging real components onto a canvas, wire
the state and the actions, and export idiomatic Rust that has no dependency on
Tailor left in it.

**New here? Start with [the tutorial](tailortutorial.md)** — it builds a
complete app from an empty project to a running binary, and every code block in
it is output Tailor actually produced.

| Page | What is on it |
| --- | --- |
| [Tutorial](tailortutorial.md) | Build an app end to end, and run what comes out |
| [The canvas](tailorcanvas.md) | Modes, selecting, resizing, layout modes, snapping, the live window |
| [Components and slots](tailorcomponents.md) | The 101-component catalog, slots, the five drawn containers, your own components |
| [State, bindings and actions](tailorstate.md) | Signals, two-way binding, events, the lint pass |
| [What gets generated](tailorcodegen.md) | The output, the flavours, export, the file format, the theme |
| [The MCP server](tailormcp.md) | Driving the same document from an agent |
| [Zed & other editors](tailorzed.md) | Jumping between a component and its code, in both directions |

## Getting it

Every [release](https://github.com/wess/guise/releases) attaches **`Tailor.dmg`**
— drag it to Applications. The bundle carries the app as `tailor` and the MCP
server as `tailor-mcp` beside it, and it is signed and notarized when the
release was built with a Developer ID.

From a checkout:

```sh
cargo run -p tailor-app                     # open Tailor
cargo run -p tailor-app -- demo.tailor      # open a project
cargo build -p tailor-app                   # the binary is `tailordev`
```

The binary is named `tailordev` so a development build never collides with an
installed `tailor`. To build the bundle yourself, see
[releasing](release.md#building-the-app-locally).

## What it is, and what it is not

The canvas is not a drawing of your interface — it is your interface. A `Button`
on the canvas is a `guise::Button`, reading the same theme, laid out by the same
flexbox. That is the whole design: there is no second rendering path to keep in
step with the real one, so a component cannot look right in the builder and
wrong in the app.

What it is not is a runtime. The `.tailor` file is a design document, not
something your app loads. The output is a Rust file you own — and the ending
Tailor is built for is the one where you take that file and stop opening the
builder.

## The window

Five regions, laid out the way Interface Builder and Android Studio's layout
editor lay theirs out.

| Region | What it is |
| --- | --- |
| **Library** (left) | Every component you can place, searchable, grouped by category, plus the components you built in this project. |
| **Outline** | The node tree. Rows are drag sources and drop targets; named slots appear as their own rows. |
| **Canvas** (centre) | The artboard, at the device size you picked, with the document rendered inside it. |
| **Inspector** (right) | Six tabs: Attributes, Size, Style, Motion, Connections, Identity. |
| **Problems** (bottom) | What will not generate, and what probably was not meant. |

Every panel resizes and folds away, and the layout persists. ⌥⌘1 – ⌥⌘4 toggle
the four panels. The [canvas page](tailorcanvas.md) covers the rest, including
the full shortcut list.

## Right-click

Every surface has a menu, and each acts on the row under the pointer rather than
on whatever happened to be selected:

| Surface | What it offers |
| --- | --- |
| A component on the canvas, or a row in the outline | Rename, Duplicate, Cut/Copy/Paste, Embed, Unwrap, Move, Lock, Hide, Select parent, Extract to a component, Delete |
| The canvas itself | Paste, Select all, add a screen or component |
| A document tab | Rename…, Duplicate, add a screen or component, open the live window, Delete |
| A component in the Library | Insert into the selection, insert at the top level |
| One of *your* components | the same, plus Edit, Rename…, Duplicate, Delete |
| A row in Problems | Reveal — open the document and select the node — and copy the message |
| The generated code | Copy the file, export the project, hide the code pane |
| A project in Recent, on the start screen | Open, reveal in Finder, copy the path, remove from the list |
| The live window | Inspect element, show or hide the inspector |

A node's menu also has **Open in Editor**, which puts your cursor on the line
that node generated. The reverse — cursor on a line, component selected in
Tailor — is a Zed task away. See [Zed](tailorzed.md).

Nothing in any of them is a command that exists only there. **Rename…** on a tab
opens the document and puts the cursor in the name field the inspector already
had; the menu is a shortcut to the app, not a second way to drive it.

## Settings

⌘, opens preferences, built out of guise's own `SettingsView`, `SettingsSection`
and `SettingsRow` — the settings screen the library ships is the settings screen
its builder uses.

| Page | What is on it |
| --- | --- |
| **General** | Autosave, whether the live window opens with the inspector, which editor **Jump to** opens, what new projects generate as, the start screen's scheme (`dark`, `light`, or **system** to follow the OS) |
| **Canvas** | Mode, show the grid, grid spacing, snap to grid, snap to objects, nudge distance, new frames are free form, show layout bounds |
| **Panels** | Panel sizes, and a reset for the whole layout |
| **About** | Version, and where the settings file lives |

The canvas options are also on the Arrange menu, where a layout program puts
them.

## The workspace

```
crates/tailor/
├── model/     # the document: catalog, node tree, tokens, state, file format
├── codegen/   # document -> idiomatic guise Rust
├── store/     # project files, recents, editor settings, export, the editor bridge
├── render/    # document -> live guise components (the canvas)
├── app/       # the gpui workbench
└── mcp/       # an MCP server over the same document model

extensions/zed/  # a Zed context server for tailor-mcp — its own cargo
                 # workspace, because it targets wasm32-wasip2
```

`model`, `codegen`, and `store` are free of gpui and carry the tests: the
reparent rules, the cycle checks, the undo stack, the generated output, and the
file round-trip are all plain-data logic, and that is where a builder actually
goes wrong.

```sh
cargo test -p tailor-model -p tailor-codegen -p tailor-store
```

All six crates are `publish = false`. Nothing about Tailor reaches crates.io,
and nothing about it is in `guise-ui` — `cargo package -p guise-ui --list` is
the proof, and it is why the library still depends on nothing but gpui and std
while Tailor is free to use serde.

## Adding a component to the catalog

If you are working on Tailor itself, the catalog in `tailor-model` is read by
four consumers: the Library lists it, the inspector builds a control per prop,
the renderer builds the real component, and the generator prints it.

1. Add an entry to the right file under `crates/tailor/model/src/catalog/`,
   using the `comp!` macro: seven positional facts, then any field that differs
   from the defaults.
2. Add an arm to `crates/tailor/render/src/nodes/build.rs`.
3. That is all — unless the constructor is not one chained call, in which case
   add an arm to `Emitter::special` in `crates/tailor/codegen/src/node.rs` too.

`PropSpec::emit` is what keeps the three in step: `Emit::Method("size")`
generates `.size(..)`, `Emit::Flag("fill")` generates `.fill()` when the bool is
true, `Emit::None` means the constructor already consumed it.

Editing the catalog without the renderer — or the other way round — is how a
canvas and an export drift apart, which is the one failure this whole design
exists to prevent.
