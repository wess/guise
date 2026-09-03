# Changelog

Notable changes to [`guise-ui`](https://crates.io/crates/guise-ui). Versions
follow [semver](https://semver.org): from 1.0 on, a breaking change means a
major release, and is called out under **Breaking**. Releases before 1.0 landed
breaking changes in minor versions.

## 1.6.0 — 2026-09-03

### A theme manager

Switching a theme was always a one-liner — write the `Theme` global, ask for a
redraw, and every component restyles because it reads `theme(cx)` at paint time.
That is why guise shipped this long without a manager. What the one-liner never
answered is everything *around* the switch: which themes exist, which one the
user picked last time, and what "follow the system" means in an app that ships
four dark themes. Every consumer wrote that part again.

`ThemeManager` owns exactly that and nothing about how a theme looks. A registry
of `id -> ThemeEntry` seeded with `light` and `dark`, widened by `with_presets()`,
by host themes in code, or by `with_dir(..)` over a folder of JSON theme files.
A `ThemeChoice` that is either one theme by id or *follow the system*, resolved
through the registry's light/dark pair. And `ThemeManager::watch(window, cx)`,
which keeps that resolution honest when the OS flips at sundown.

Persistence stays with the host, the way `settings` leaves the schema with the
host: `ThemeChoice` is `Display` + `FromStr`, so it round trips through one
string in whatever config file the app already writes. Reading a themes folder
is the only thing here that touches the disk — an app that would rather bring
its own bytes uses `register_json`. `ThemeSource` says where an entry came from,
so a picker can group the app's own themes apart from a user's.

### `ThemePicker`

The list of installed themes and the click that wears one. It reads the manager
out of the global and writes the choice straight back, so there is no state to
hold and nothing to wire: drop it in a settings page and it is done. Without a
manager installed it draws nothing rather than guessing at a theme list.

Each row previews the theme it offers, painted from *that* theme's body,
surface, border and primary — the only honest way to show a theme you are not
currently wearing. `List` fits a settings pane, `Grid` a gallery.

New in the crate root and the prelude: `ThemeManager`, `ThemeChoice`,
`ThemeEntry`, `ThemeSource`, `ThemeLoadError`, `ThemePicker`,
`ThemePickerLayout`, `manager`, and `scheme_of`. The gallery installs a manager
at launch, so its Themes section is the real list rather than a mock.

## 1.5.4 — 2026-09-03

### Text no longer collapses to zero width

`Text` and `Title` carried `min_w(0)` on their own div from 1.5.1. Taffy clamps
the available space it hands a measured leaf by that leaf's own minimum, so a
min-content pass measured the string at `Definite(0)` and gpui wrapped it after
every character. Any flex item whose content was guise text then reported a
content size of zero and laid out one glyph wide — a heading rendered as a
vertical column of letters, and the row beside it collapsed to its gaps.

`min_w(0)` is right on a *container* — and stays where guise already had it, in
`Field`, `Select`, `AppShell`, `TableView` and the rest — but on a wrapping text
leaf it is the whole layout escape hatch, with nothing left to hold the box
open. Nothing in guise depended on it: every component that ellipsizes uses
`truncate_text()`, which pairs the zero minimum with `whitespace_nowrap`, so the
text never wraps and the minimum only permits clipping.

A caller that wants shrinkable text in a flex row can still wrap it:
`div().min_w(px(0.0)).child(Text::new(..))`.

This release is the fix and nothing else: it was cut from the published
1.5.3 source, so no other in-flight work rode along with it.

## 1.5.3 — 2026-08-26

### Installation docs

The README, getting-started guide, tutorial, WebView guide, crate README, and
website now point at the current 1.5 release line. The optional git pin now
targets `v1.5.3`.

## 1.5.2 — 2026-08-26

### Settings sidebar

`SettingsView::sidebar_matches_body(true)` lets an app use the content background
for the page list instead of the raised surface. Sidebar pages now participate in
the tab order and show a theme-derived focus border.

The workspace now checks in its two-space `rustfmt.toml`, making the existing
formatting convention reproducible instead of editor-specific.

## 1.5.1 — 2026-08-26

### Text overflow

Single-line inputs now clip only across their horizontal viewport instead of
masking the text's line box on both axes. Accents, fallback fonts, emoji, and
other glyphs whose ink extends beyond their advance metrics are no longer cut
off at the top or bottom.

Labels, descriptions, picker values, table cells, and DevTools rows can now
shrink inside flex layouts and ellipsize without the same vertical mask.
`TextArea` rows wrap instead of clipping, and a textarea capped with
`max_rows` scrolls vertically rather than hiding content.

## 1.5.0 — 2026-08-26

### GPU View

`GpuView` adds a small retained-scene API for maps, simulations, diagrams, and
sprite-heavy status surfaces. `GpuScene` submits ordered quads and encoded
textures through gpui's native GPU paint pipeline, with contain/cover/stretch
fitting, full-parent sizing, pixel-boundary snapping, and normalized sprite-atlas
cropping. It embeds no browser or web canvas and leaves simulation time, frame
selection, pause behavior, and reduced-motion policy with the caller.

### Keyboard and icon-action names

`Button` and `ActionIcon` now participate in GPUI's tab order, show a primary-color
focus border, and activate through GPUI's Enter/Space keyboard-click path.
`ActionIcon::label` gives icon-only actions a descriptive tooltip and probe attribute.

### Rendering performance

The gallery now virtualizes its variable-height sections, so layout and paint work
stay proportional to the visible demos instead of the full component catalog.
Native web views are hidden as soon as their section leaves the viewport.

`Loader`, `Skeleton`, and the streaming-text caret now schedule coalesced frames only
while visible. Idle DevTools instances also release their retained probe tree instead
of recording frames they cannot display. Together these changes remove persistent
offscreen animation work and prevent the recorder from growing between snapshots.

The release lockfile also updates `crossbeam-epoch`, `h2`, `quick-xml`, XCB, and
Wayland scanner dependencies to clear the current RustSec vulnerability set.

## 1.4.0 — 2026-08-24

### ScrollArea fills

`ScrollArea` had one way to be bounded — `max_height(f32)` — which is right for
a list that takes a known slice of a larger layout and wrong for the other
common desktop shape: a pane as tall as whatever the window gives it. There was
no way to say that, so apps dropped the component and hand-rolled
`div().id(..).size_full().overflow_y_scroll()`, leaving two scrolling idioms in
one codebase.

`ScrollArea::new("settings").fill()` is that mode. It grows into the leftover
main axis under a flex parent and takes the height under a plain block one
(gpui's default display, and what a route body usually is), so it does not care
which shape it is mounted in. `max_height` is unchanged, and the two compose:
`.fill().max_height(600.0)` grows with the window but never past 600px.

Tailor's scroll area gets a **Fill parent** checkbox for the same thing.

## 1.3.0 — 2026-08-24

An animation system, in the shape of [anime.js](https://animejs.com), and
Tailor learns to use it.

### Motion

`guise::anim` grew from "easing curves plus `Presence`" into the whole thing,
and it splits in two.

The **description** is pure. A `Motion` is keyframed tracks over a duration; a
`Sequence` places motions on one clock (anime.js's timeline, with absolute,
relative, alongside-the-previous and label positions); a `Stagger` maps an
index to a delay, with `from`, grid distance, axis, easing and range spreading.
`sample(t)` maps a millisecond offset to a `Frame` — the properties that have a
value at that instant — with no state, no window, and nothing to tick, which is
why all of it is unit-tested without a gpui app.

The **clock** is a thin shell over it. `Animated::new(id).motion(..)` plays a
clip once when its element mounts; `Motioned::animate(id, clip)` does the same
straight onto anything already `Styled`, which is what a layout wants — a
wrapper is a new flex item, and a `w_full` child would start measuring against
it. `Animator` is an entity that owns a playhead: play, pause, reverse, seek,
speed, with `Begin`/`Complete` events. It holds a clock *anchor* rather than
ticking, so a paused animation costs nothing, seeking is one assignment, and
sampling is pure enough to test.

gpui has no transform matrix on an element, so `Prop` names what there actually
is: opacity, the relative inset (an element moves, the layout does not), the
box, and colours — plus `Rotate`, `Scale` and `Custom("name")`, which a frame
carries for you to read back and apply yourself.

`cargo run -p guise-ui --example motion` is all of it in one window, and
`--example checklist` is the app the new [motion
tutorial](docs/motiontutorial.md) builds over nine chapters.

### Two macros, in the shape of the layout ones

`motion!` is to an animation what `style!` is to a box — timing and tracks as
one block instead of a chain of setters:

```rust
div().child(card).animate("card", motion! {
    enter: slide_up 12;
    duration: 420;
    ease: out back;
    opacity: 0 => 1;
})
```

A track is `prop: from => to`, or `prop: from => [a, b]` for more than two
states. Easing is a direction and a shape (`out back`, `in_out sine`), with
`linear`, `spring` and `steps(n)` standing alone. `custom("name")` tweens a
number that is not a style at all.

`sequence!` is the variadic one — what `col!` does for children, for motions on
a clock. A position in front of an entry places it: `rel(-120) => slide_up`
overlaps the tail, `with(0) => tint` runs alongside the previous one.

Both expand to the builder, so the two forms mix and anything the block does
not cover still chains.

Two smaller things fell out of writing the tutorial with them.
`Motioned::animate_when(cond, id, clip)` exists because
`.when(cond, |el| el.animate(..))` cannot compile — `animate` changes the
element's type and `when` must return the type it was given. And a keyframe
list now takes bare values *or* built `Keyframe`s, through a small
`IntoKeyframe` trait.

### Easing

Sixteen curves became the full anime.js matrix without thirty enum variants: a
direction and a shape. `Easing::In(Curve::Quad)`, `Easing::Out(Curve::Elastic)`,
`Easing::InOut(Curve::Sine)`, over `Quad`, `Cubic`, `Quart`, `Quint`, `Sine`,
`Expo`, `Circ`, `Back`, `Elastic` and `Bounce`. Plus `Easing::Steps(n)`.
`Back`, `Elastic` and `Bounce` are defined as reflections of the existing
`ease_out_*` functions, so the two spellings cannot drift apart.

### Tailor animates

A node carries an entrance, edited in the inspector's new **Motion** tab:
entrance, easing, duration, delay, distance, repeat and alternate. It plays on
the canvas the moment you change it, plays in the live window, and generates as
`.animate(..)` on the box the node already had — the same `Motion`, from the
same settings, so the preview is the export.

**Stagger** moves the animation off a container and onto its children, one
delay per index; a child with its own entrance keeps it and drops out of the
wave. The MCP server takes the same settings as a `motion` object on `add_node`
and `set_node`, merged rather than replaced, with `"enter": null` to take one
away.

### What it costs

Sampling is pure and, in the common case, allocation-free: a `Frame` carries up
to four properties inline and only spills to the heap for a sequence layering
more than that. Measured on an M-series laptop, release build — a two-track
motion samples in **19ns**, a three-entry sequence in **71ns**. Both roughly
halved over the first cut of this release, which walked every track twice per
sample and allocated for every one.

What costs something is the repaint. gpui's `request_animation_frame` notifies
the whole **view**, so a looping animation re-renders every component beside it,
every frame. A settled one-shot asks for nothing: the two new examples idle at
0.5% CPU, while the gallery — 130 components in one view, three of which loop
by their nature — sits at 84%. [Size and
performance](docs/performance.md#per-frame-work) has the numbers and what to do
about them.

### Also

- Zero clippy warnings and zero rustdoc warnings across the workspace, for the
  first time. Nine pre-existing lints went with them.
- A `NaN` tweened into a layout property now lands as zero rather than reaching
  taffy, where it would corrupt a layout silently and permanently.

### Breaking

`Easing` gained four variants (`In`, `Out`, `InOut`, `Steps`), so a downstream
`match` on it without a `_` arm will stop compiling. Nothing in guise matched it
exhaustively and nothing on crates.io depends on it that way, which is why this
is a minor release and not a major one — but it is the one thing to know before
upgrading.

## 1.2.1 — 2026-08-21

Documentation and the website. The library's code is unchanged from 1.2.0 —
what ships here is the reading material around it, which on crates.io means the
front page of the crate.

### The site leads with both things this repository ships

The landing page made one claim, and there are two. The hero now carries the
pair side by side — the Tailor workbench, an arrow, and the guise interface that
falls out of it — under a headline that says what the choice actually is: write
it, or draw it. The annotated component exhibit moved down into the components
band, where its callouts have the horizontal room they need.

Tailor is a top-level nav item now, lit across all eight of its pages, with a
footer column of its own. Three feature cards were missing entirely — the
builder, the AI chat kit, and the data views that window their rows — and so
were three systems rows: drag and drop, DevTools, and self-update.

Twenty-one cards on the documentation index were rendering with an empty
description. That was every page added since the blurb list was last touched:
`appguide`, `dates`, `files`, `ai`, `markdowneditor`, `dnd`, `update`,
`devtools`, `settings`, `release`, `performance`, and all eight Tailor pages.

### The README says what the repository is

Tailor was a paragraph at the bottom of a library README. It is a section now,
with the workbench, direct manipulation, what comes out, the live window, the
MCP server and the editor jump each accounted for — and the workspace list names
the Tailor crates, the Zed extension and the site generator rather than stopping
at the library and the gallery.

`guise::ai` was not in the README at all, despite being a whole component
family: a transcript, a composer, streaming text, reasoning blocks, tool calls,
citations and cost meters, none of which opens a socket. It has a section now,
and a row in the component table, as does the read-only `Markdown` renderer and
`PaneGroup`.

In the docs, Tailor moved out of a nested bullet under *Systems* into a section
of its own listing all eight pages; `architecture.md` gained the `mcp/` crate,
`extensions/zed/`, `panegroup/` and `icon/`, and its *adding a component* list
gained the two steps that were missing — the `.probe("Name")` call that makes a
component visible to DevTools, and the catalog entry that lets Tailor place it.

### Corrections

Things that had quietly stopped being true:

- `docs/tutorial.md` asked for `guise-ui = "0.2"`, and `gettingstarted.md` for
  `"1.0"` at tag `v1.0.0`.
- The test count was given as 300+ in the README and 320+ in `CLAUDE.md`. It is
  526.
- `AGENTS.md` described the workspace as the library plus the gallery, which it
  has not been since 1.1.0.
- `release.md` counted five Tailor crates; there are six.
- Five source comments still called the menu item *Open in Zed*. It has been
  *Open in Editor* since 1.2.0, because it opens any of six editors.

## 1.2.0 — 2026-08-21

The library itself is unchanged from 1.1.0 — everything here is Tailor, its
documentation, and the editor bridge between them. Note that 1.1.0 never
reached crates.io, so for anyone installing from there this release also carries
1.1.0's library work: the window-scoped DevTools recorder and the Elements tree
rework.

### Generated code that binds or handles an event now compiles

Writing Tailor's tutorial meant exporting a real app and building it, which is
how three bugs surfaced that every test had walked past.

- **A bound prop said `self` inside `new`.** `.value(self.query.get(cx))` is
  fine in `render` and impossible in a constructor — the thing being built *is*
  what `self` will be made of. State variables are locals at the top of `new`
  now, and a binding reads the local while it is one.
- **A binding was one-way.** guise binds two ways in two shapes: entities take
  `X::bind(&entity, &signal, cx)` after both exist, controlled builders take
  `.bind(signal.binding())` in the chain. Tailor emitted a read for both, so a
  bound switch showed its variable and then refused to change it.
- **An event inside a drawn container's region did not compile at all.** Those
  regions take `'static` closures, and the handler was `cx.listener(..)`, which
  borrows a context that cannot outlive the method. It goes through a weak
  handle now, cloned in ahead of the closure and upgraded when the event fires.

### Tailor and Zed jump to each other

What Interface Builder gives you inside Xcode is a loop: click a control, land
on its code; sit on a line of code, find the control. Tailor and Zed now do that
across two apps, with no extension, agent or network in it.

**Open in Zed** (⌥⌘O, or a node's right-click menu) puts your cursor on the line
that node generated. The generator tags each node's expression while it writes
the file and records the line the tag came off, which ships as
`Generated::lines`. Tagging rather than searching the finished text is what
makes it cover everything: a `Button` carries its node id into the output and
could be found by searching, but `Text::new("Ada Whitfield")` carries nothing.

**`tailordev --reveal <file>:<line>`** goes the other way, which a Zed task binds
to a key. It resolves the file to a project, the line to a node, and leaves a
request the open window picks up on the poll it already runs — so Tailor comes
forward with that component selected. Exporting records which project owns which
directory, in Tailor's own config rather than in your source tree: generated code
stays code, with no dotfiles or absolute local paths committed beside it.

**View → Set Up Editor Jump…** writes that task into Zed's global task file and
copies the keybinding. It never overwrites — a file that will not parse is an
error rather than something to replace — and it does not touch your keymap,
because claiming a key in somebody's keymap is not a thing to do quietly.

**Settings → General → Jump to** picks the editor: Zed, VS Code, Sublime,
IntelliJ, Emacs or Neovim. They all take a path and a position, so it is a table
of command lines rather than a plugin per editor, and the menu item is *Open in
Editor* rather than *Open in Zed*.

Separately, and not part of that loop, `extensions/zed/` registers `tailor-mcp`
as a Zed context server for building a design from the agent panel. A Tailor
canvas *inside* a Zed pane is not on the table at all — extensions are
WebAssembly with no UI API, so it is absent rather than difficult.

### Tailor's documentation

One page became seven, and it grew a tutorial. `docs/tailortutorial.md` builds a
complete app — app shell, a component of its own placed three times, two bound
controls, a wired action — then exports it and runs it. Every code block in it
is output Tailor actually produced: the project was built through the MCP server
so the tutorial cannot drift from the generator.

The reference splits into [the canvas](docs/tailorcanvas.md),
[components and slots](docs/tailorcomponents.md),
[state, bindings and actions](docs/tailorstate.md),
[what gets generated](docs/tailorcodegen.md) and
[the MCP server](docs/tailormcp.md), with the overview keeping the map.

## 1.1.0 — 2026-08-21

### Tailor, a visual interface builder

The repository now ships a second binary: **Tailor**, a drag-and-drop interface
builder for gpui and guise, in `crates/tailor/`. It is five crates — a gpui-free
document model with the component catalog, a Rust generator, a file layer, a
renderer, and the workbench — and it is `publish = false` throughout, so nothing
about `guise-ui` on crates.io changes.

The canvas renders the real components against the real theme rather than a
second drawing of them, which is what stops a builder from showing you one thing
and generating another. Five containers are the exception and are drawn from the
theme (`Tabs`, `Accordion`, `SplitPanel`, `AppShell`, `Carousel`): their regions
are `'static` closures, and drawing them is what lets you click a tab and drop
into the slot behind it.

Output is a Rust file you own — a `Render` entity when the document holds state,
a `RenderOnce` builder when it does not — with state variables as `Signal<T>`
fields, events wired through `cx.listener` or `cx.subscribe`, and every resolved
colour hoisted into a `let` at the top of `render` the way guise's own
conventions require.

Also in the box: a live second window that follows every edit, a Split mode that
regenerates the Rust as you drag, an Interface-Builder-shaped five-tab
inspector, a Problems panel, embed/unwrap, align and distribute, and four
project templates.

Every panel resizes from the divider beside it and folds away from the chevron
in its header, leaving a rail to click it back; the inspector's sections fold
individually. All of it persists, so the layout you left is the one you come
back to. Right-clicking a component on the canvas or a row in the outline
selects it and opens a menu on it — including **Extract to a component**, which
lifts the selection into a new component document and leaves a reference behind.

Selection puts eight resize knobs around a node, Interface Builder's way:
drag a knob to resize, drag an absolutely placed node's body to move, with
snapping to the grid and to siblings' edges, guides drawn where it caught, a
live size readout, and arrow-key nudging. A component that carries its own
pixel size — an image, a chart — resizes through those props rather than
through the box around it.

Tailor ships as an app, not only as a cargo target: every release from here
attaches **`Tailor.dmg`**, built and notarized by `release.yml` from
`scripts/bundle.sh`. It carries the MCP server beside the executable, and it
takes the workspace version — one repository, one version, one set of notes.

Also shipping: **`tailor-mcp`**, an MCP server over the same document model, so
an agent can place components, wire state and actions, and generate or export
Rust without opening the app. It edits the `.tailor` file, and the app watches
the file it has open — so a screen built over MCP appears on the canvas as it
is built. See [`docs/tailor.md`](docs/tailor.md).

The project is now shared behind an `Arc` rather than copied: an undo snapshot
and the canvas's view of it are refcount bumps, and editing goes through
`Arc::make_mut`, which pays for one copy per edit instead of one per commit and
one per frame — an idle frame copies nothing. Regenerating the Rust, the lint
pass, autosave, export, and the file watcher all moved to gpui's background
executor, debounced, with a revision guard so a stale result never overwrites a
newer one and a held `Task` so superseded work is cancelled. On a debug build of
a 3,744-node project a keystroke went from about 7.9 ms of main-thread work to
about 2.4 ms. Two hundred undo entries used to be two hundred whole projects;
they now share.

Preferences (⌘,) are built out of guise's own `SettingsView`, `SettingsSection`
and `SettingsRow`, across General, Canvas, Panels and About pages — the settings
screen the library ships is the one its builder uses. The canvas page carries
what any layout program carries: grid spacing, snap to grid and snap to objects
as separate toggles, the nudge distance, and whether new frames are free form.
The same options are on the Arrange menu, with ⇧⌘G to flip the selected
container between flow and free form.

`View → Developer Tools` (⌥⌘I) opens guise's own inspector along the bottom of
the live window, so a design can be drilled into where it is running: the
Elements tree, the box model, and each resolved style with the source line it
came from. It goes in the live window rather than on the canvas because that
window renders the document alone — the tree is your interface, with none of
Tailor's own panels in it. It is closed until asked for, and closing it drops
it, which is what stops the recorder; a preference opens the live window with
it already showing. Right-clicking the design gives you **Inspect element**, the
browser move: it selects the deepest component under the pointer, scrolls the
tree to it, and boxes it in the window. The inspector takes its room from the
window rather than from the design — opening it makes the window bigger and
leaves the document at its device size, because showing the design at that size
is the whole reason the window exists.

Every surface now has a right-click menu, each acting on the row under the
pointer: document tabs (rename, duplicate, delete), both kinds of Library row
(insert into the selection or at the top level; edit, rename, duplicate or
delete your own components), Problems rows (reveal, copy the message), the
generated code (copy, export, hide), and the start screen's recents (open,
reveal in Finder, copy the path, forget). Nothing in them is a command that
exists only there.

Two bugs worth naming. Every node's wrapper reused its component's `ElementId`,
so gpui aliased their element state and some components — `Switch` among them —
never appeared on the canvas at all. And nothing in the app ever took focus, so
gpui had no dispatch path: every action registered on an element was
unreachable, which greyed out the whole menu bar and swallowed most keyboard
shortcuts. Focus now lives on the canvas, and returns to the window root
whenever it would otherwise go nowhere.

A `.tailor` file is text somebody can edit, so loading one assumes nothing:
`Document::repair` now makes a document a real tree — a root that exists, one
parent per node, no loops, bounded depth — and the traversals underneath it are
cycle-safe regardless, so a file with a loop in it comes back with a short
answer instead of hanging the app that opened it. Saving refuses a project
holding an infinity or a NaN rather than writing the `null` serde would produce
and leaving a file that no longer loads, and every field those could come from
rejects them on the way in. Generated string literals escape every control
character, a document whose name would collide with a guise component is an
error rather than a mystery compile failure, and an export only writes below the
directory it was given.

### The inspector's Elements tree is a tree, not markup

It printed `<Button variant="filled" size="sm" />` with the closing rows a
browser prints. But these are components built from builder calls: there is no
attributes-versus-children distinction to draw, the self-closing form promises a
model gpui does not have, and a `</Card>` row says nothing the next row's
indentation has not already said — while costing every container half the panel.

It is now an indented tree, one row per component, with props read as a YAML flow
mapping the way the Styles pane reads a declaration:

```
▾ Card
    Text        size: sm, dimmed
    Title       order: 2
    Sparkline
```

`ElementsPanel::reveal` also scrolls the tree to what it revealed. Expanding a
node's ancestors is not much use when the node is still below the fold, which is
what happens every time the element picker selects something.

### Fixed

- **The DevTools element recorder now records one window.** It is thread-local,
  and every window a thread draws was recording into the same tree — so an app
  with a second window open showed an inspector a tree of both, its own window
  included. An inspector now claims the frame when it renders, and elements
  prepainting in any other window that frame are skipped. `probe::begin_frame`
  takes the window it is claiming for and is exported alongside `set_enabled`
  for hosts driving the recorder by hand.

### Settings screens, from sinclair

Three apps had built the same settings screen separately, and the copies had
already drifted — one marked an overridden key with a reset arrow, another with
a dot. `guise::settings` is the part they shared.

```rust
SettingsView::new(cx)
    .page_icon("appearance", "Appearance", IconName::Palette)
    .searchable(true)
    .content(|page, query, _window, cx| appearance_page(page, query, cx))
```

- **`SettingsView`** — the shell: page list, content pane, optional search and
  footer. `content` is re-invoked every frame with the active page and the live
  query, the same contract `Tabs` and `Accordion` use.
- **`SettingsSection`** — a titled group of rows, a plain `ParentElement`.
- **`SettingsRow`** — `Field`'s horizontal sibling: name and description on the
  left, control on the right. Exactly one "modified" marker, never two — a reset
  control when you offer `on_reset`, a dot when you don't.

**No schema type, and that is the point.** Every app types its settings against
its own config struct, and a component generic enough to hold those would push
the cost back onto the caller as type parameters or stringly-typed values. The
schema is the product surface; it stays in the app. Search works the same way:
the view has nothing to search, so it reports the query and the host matches.

### App chrome, also from sinclair

- **`About`** — the small centred card, with `BuildKind` behind it. A build made
  from some commit that merely carries the version number is not the release, and
  printing "Released 2026-08-18" on one is a small lie that costs a bug report,
  so a development build says what it is.
- **`WindowControls`** and **`ResizeHandles`** — the minimise/maximise/close
  buttons and the resize border a client-side-decorated window has to draw
  itself. `needed()` carries the `cfg`, not the components, because a
  `cfg!(target_os)` buried in a component cannot be previewed from the other
  side. `TRAFFIC_LIGHT_INSET` comes with them.

## 1.0.0 — 2026-08-18

The API is stable. Everything below 1.0 moved breaking changes through minor
versions; from here a break means a major release.

That is the whole meaning of this number — it is not a rewrite. `guise` has
been carrying real applications for months: 130+ components, a reactive layer, a
pane system, an editor, a markdown editor, an AI component set, a self-updater,
515 tests, and a documented page for every module. What changes today is the
promise, not the code.

One caveat worth stating plainly: `guise` builds against `gpui = "0.2.2"`, which
is itself pre-1.0. A breaking change in gpui forces a breaking change here, so
2.0 may well arrive on gpui's schedule rather than this crate's. The alternative
— staying at 0.x forever because a dependency is — helps nobody.


### New in 1.0: DevTools — Safari's Web Inspector, aimed at your own app

`guise::devtools` adds an in-app inspector with eight tools across the top:
Elements, Network, Sources, Timelines, Storage, Layers, Logs and Audit.

```rust
DevToolsState::new().init(cx);          // once at startup
let devtools = cx.new(DevTools::new);   // then put it wherever you like
```

`cargo run -p guise-ui --example devtools` opens it beside a small app.

- **Elements is real introspection, not a mock.** Every component now ends its
  `render` with `.probe("Name")`, which snapshots the element's
  `StyleRefinement` and brackets `prepaint` to rebuild the tree. So the outline
  is the live component hierarchy — `<Button variant="filled" size="sm" />`,
  foldable, with closing tags — and the Styles sidebar shows the element's
  actual declarations with color swatches, the Computed sidebar its real box
  model, and the Node sidebar the source location it was constructed at. gpui
  exposes only the element under the pointer and no way to enumerate a tree,
  which is why the recording exists.
- **Logs, Network, Storage and Timelines are reported by the host**, the same
  arrangement `ai/` uses: `log`, `network_begin`/`network_update`,
  `storage_set`, `measure`. Nothing in `guise` opens a socket. `log` is
  `#[track_caller]`, so a line knows where it came from without being told.
- **Sources** reads the files the tree points at off disk, resolving the
  workspace-relative paths `#[track_caller]` produces against the working
  directory and its ancestors.
- **Audit** runs rules over the recorded tree — WCAG text contrast, hit target
  size, collapsed containers, children escaping their parent — each finding
  selecting the node it came from.
- **Cost.** A probe is one boolean check per element per frame while the
  inspector is closed, and allocates nothing in that state. Recording starts
  with the first `DevTools` and stops with the last. An app that never
  constructs one links none of the panels.

Named Logs rather than Console on purpose: half of Safari's Console tab is a
JavaScript evaluator, and a compiled binary has nothing to evaluate.

### Also

- `Size::label()` and `Variant::label()` — the token names the docs already
  used, now available to code.
- New guide: [DevTools](docs/devtools.md).

## 0.13.0 — 2026-08-17

The release that makes text fields behave like the ones people already know,
adds a component set for putting a model in front of a person, and takes 43%
off the binary.

### Text fields work the way an `<input>` does

Every single-line field — `TextInput`, `PasswordInput`, `NumberInput`,
`ColorInput`, `Combobox`, `Autocomplete`, and the query box in `TagsInput` —
is now built on one shared core (`input/line.rs`) that shapes the line through
gpui's text system instead of drawing it as three sibling divs.

- **Tab moves to the next field.** It used to type a literal tab character:
  the platform reports a `\t` for the key, and the printable-input path took
  it. Shift+Tab goes back. Ordering follows render order, like `tabindex="0"`.
- **The mouse works.** Click to place the caret, drag to select, double-click a
  word, triple-click the value, Shift+click to extend. None of this existed —
  which is the real reason copying felt broken, since there was no way to
  select anything to copy.
- **Clipboard everywhere.** Cut, copy, and paste were on `TextInput` and
  `TextArea` only; the other six had none. A multi-line paste is flattened to
  one line, the way `<input>` flattens it.
- **Undo and redo**, coalesced by word rather than by keystroke.
- **IME, dead keys, press-and-hold accents, and the macOS character palette.**
  Painting a field now registers an `ElementInputHandler`, which is the only
  way to see any of them. Text entry therefore no longer runs through key
  handling — the platform delivers it after the key handler declines it.
- **Long values scroll horizontally** to keep the caret in view instead of
  disappearing under the border.
- The caret sits on a glyph boundary and blinks.

New on the fields: `read_only`, `max_length`, `tab_index`, `tab_stop`, and
`focus_handle` — the last three were on three fields and missing from four
that were nonetheless in the Tab ring.

`TextArea` gains Tab-moves-focus, undo, `max_rows`, `submit_on_enter` (with a
separate `TextAreaSubmit` event), `is_blank`, and a placeholder that stays
visible while the field is focused and empty.

### AI components

A new `guise::ai` module: a transcript, a prompt box, streaming feedback, tool
calls, citations, and the controls and meters around a request. See
[`docs/ai.md`](docs/ai.md) and `cargo run -p guise-ui --example ai`.

- `AIChatView` — the transcript, with stick-to-bottom scrolling that follows
  the tail only while you are already at the tail, and per-turn disclosure
  state. `AITurn` / `AITurnTool` are what go in it.
- `AIMessage` — one turn, if you would rather lay the list out yourself.
- `AIComposer` — Enter sends, Shift+Enter breaks the line, the box grows to a
  ceiling, and the send button becomes a stop button while a reply streams.
- `AIStreamingText`, `AIThinking`, `AIReasoning`, `AIToolCall`, `AICitation`,
  `AISources`, `AIModelPicker`, `AITokenMeter`, `AICost`, `AISettings`.

None of it opens a socket or holds a key — the host owns the request, so the
same transcript drives a local model, a hosted API, or a replayed log.

Also new: **`markdown::Markdown`**, a read-only markdown renderer over the same
pure passes `MarkdownEditor` uses. It is what message bodies draw with, and it
works anywhere text does.

### Security

- **Linux updates are verifiable.** `update::appimage` downloaded a file,
  marked it executable, and renamed it over the running binary with nothing but
  a byte count vouching for it — macOS had a pinned `codesign` requirement and
  Linux had no equivalent. A published SHA-256 is now checked when a release
  ships one, on both platforms, and `UpdateConfig::require_checksum(true)`
  turns a *missing* digest from a silent pass into a refusal. Recognises
  `<asset>.sha256` and `SHA256SUMS`-style listings; the hash comes from
  `shasum`, `sha256sum`, or `openssl`.
- **Unbounded recursion in the pane-layout decoder.** A corrupted or hostile
  snapshot of `"h0.5("` repeated recursed until the stack ran out, which
  aborts the process rather than unwinding. Capped at 64 levels, matching the
  cap the JSON reader already had.
- A stale IME composition range could produce text runs longer than the string
  they cover, which the text system slices by — a panic, not a mis-draw.
- `WebView`'s local-file handler no longer builds responses through `unwrap`
  on wry's request thread, where a panic takes the process with it.

### Size and performance

The gallery went from **13.86 MB to 7.82 MB** — see
[`docs/performance.md`](docs/performance.md) for the breakdown and the release
profile to copy.

- The bundled Lucide font is **78 KB smaller**: GSUB and the v2 `post` table
  are stripped, since glyphs are addressed by codepoint and neither is ever
  read (`scripts/stripfont.py`, run by the icon generator).
- `IconName`'s `Debug` is written out instead of derived — a derived one is a
  match with 1991 arms to print a string the name table already holds.
- The icon tables are `static` rather than `const`, so they are not
  materialised at each use site.
- `AIChatView` virtualizes: a turn more than a screen away is drawn as a spacer
  of its measured height, because building one re-parses its markdown. That
  was 1.25 ms per frame on a 46 KB transcript and grew with the conversation;
  it is now proportional to the viewport. `.virtualize(false)` opts out.
- Undo history is bounded by the text it retains (256k chars), not just by step
  count — 128 steps of a 200 KB `TextArea` was 100 MB of snapshots.
- `TextEdit::insert` and `replace_range` use one `splice` instead of shifting
  the tail per character, which was quadratic on a large paste.
- Assorted per-frame allocations removed: a masked field no longer builds the
  cleartext buffer to throw it away, collapsed reasoning blocks and folded tool
  cards no longer copy text nothing draws, and the composer no longer
  materialises the whole draft to test whether it is blank.

### Fixed

- The read-only markdown renderer and `MarkdownEditor` had drifted apart on
  heading sizes (h2 1.4 vs 1.45, h3 1.25 vs 1.28, code 0.92 vs 0.88), so the
  same document rendered at different sizes depending on whether you were
  reading or editing it. One table now serves both, in `markdown::layout`.
- `AIToolCall` asked for the `"monospace"` family, which the text system does
  not resolve — tool arguments and results rendered in the prose font.
- `AIModelPicker` sized itself ad-hoc and came out 44px tall where every other
  control at `Size::Sm` is 36, so it would not line up in a toolbar.
- The text-selection tint was open-coded in eight files and had already drifted
  from the editor's. It is now `Theme::selection()`.

### Breaking

- `theme::mantine()` is now **`theme::open_color()`**. The palette is
  open-color; the old name described where it was borrowed from rather than
  what it is.
- `AIChatViewEvent::Retry` removed — it was never emitted.
- `AIModelPicker::selected(index)` removed; use `selected_id(&str)`, which
  covers both the build-time and runtime cases.
- `IconName`'s `Debug` now prints the kebab-case name (`arrow-up`) rather than
  the variant name (`ArrowUp`), matching what lucide.dev lists it under.
- `apply_key` no longer types control characters, so Tab and Enter cannot be
  inserted as text. Fields use the new `apply_nav`, which leaves text entry to
  the platform's input handler; `apply_key` remains for hosts driving a
  `TextEdit` from raw key events.
- `Progress::color` and `Loader::color` take `impl Into<ColorValue>` rather
  than `ColorName`. Existing calls still compile.

### Added API

`Theme::selection()`, `markdown::layout::{metrics, RowMetrics}`,
`TextEdit::{chars, replace_range, undo, redo, break_undo, cursor, set_cursor,
set_selection, extend_to, word_at, byte_of, char_of, set_text}`,
`NumberInput::{set_value, set_min, set_max}`, `Slider::set_value`,
`TextArea::{is_blank, set_placeholder}`, `AIToolCall::expandable`,
`input::{LineEditor, LineState}`.

## Earlier releases

0.12.0 and before predate this changelog; see the
[git history](https://github.com/wess/guise/commits/main) and the
[release tags](https://github.com/wess/guise/releases).
