# Tailor: components and slots

Tailor's Library is guise's component set, one entry per component, grouped the
way the docs group them. There are **124** of them.

## The catalog

| Category | Count | Some of what is in it |
| --- | --- | --- |
| Layout | 25 | Frame, Absolute frame, Stack, Group, Center, Grid, Container, Card, Paper, Panel, Scroll area, App shell, Split panel, Expanded, Settings screen, Settings section, Settings row, About, Window controls |
| Typography | 9 | Text, Title, Anchor, Code, Kbd, Mark, Blockquote, Markdown, Spoiler |
| Controls | 13 | Button, Action icon, Close button, Copy button, Badge, Chip, Icon, Theme icon, Theme picker, Indicator, Rating, WebView |
| Inputs | 27 | Text input, Text area, Number, Password, PIN, Select, Combobox, Autocomplete, Checkbox, Switch, Radio, Segmented, Slider, Range slider, Colour, Tags, Date, Time, Calendar, File, Dropzone, Transfer, Field, Editor, Markdown editor |
| Data | 11 | Avatar, Avatar group, List, Table, Tabs, Accordion, Tab bar, Timeline, Tree view, Carousel, Virtual list |
| Feedback | 11 | Alert, Notification, Loader, Progress, Ring progress, Skeleton, Modal, Drawer, Tooltip, Loading overlay, Confirm |
| Navigation | 8 | Breadcrumbs, Nav link, Stepper, Pagination, Status bar, Navigation menu, Menu, Context menu |
| Charts | 6 | Sparkline, Line, Area, Bar, Pie, Scatter |
| Media | 1 | Image |
| AI | 13 | Chat view, Message, Composer, Streaming text, Thinking, Reasoning, Tool call, Token meter, Cost, Citation, Sources, Model picker, AI settings |

### What is deliberately not in it

The catalog is checked against guise's own source by a test
(`catalog::coverage` in `tailor-model`), so a component added to the library is
either catalogued or listed as an exclusion with a reason. Adding one to guise
and forgetting Tailor now fails `cargo test` rather than going unnoticed for a
release.

The standing exclusions, and why:

| What | Why |
| --- | --- |
| The `flex/` primitives (`Align`, `Padding`, `Spacer`, `SizedBox`, `Wrap`, …) | pixel-based twins of `layout/`, which the catalog offers instead |
| `Animated`, `Presence`, `Transition`, `Collapse` | motion is a *property* of a node, not a component you drop |
| `Popover`, `HoverCard` | trigger *and* content are `'static` element closures; `Tooltip` covers what fits in a prop |
| `Spotlight`, `Tour` | steps point at element ids that only exist in the running app |
| `MenuBar` | app-level menus dispatch actions rather than lay anything out |
| `PaneGroup` | the host owns the items; the component owns only the layout over them |
| `Draggable`, `DropTarget`, `SortableList` | generic over a payload or a collection the host owns |
| `GpuView` | draws a `GpuScene` the host assembles in code |
| `OverlayHost`, `ToastStack`, `DevTools` | one per window, installed by the host |
| `UpdatePrompt`, `UpdateNotice` | driven by a live `Updater`; there is nothing to preview |
| `ResizeHandles` | window chrome, meaningful only against a real borderless window |

The Library searches across names and blurbs, and the category pills filter it.
Drag an entry onto the canvas, or click it to drop it into the selection —
click is faster once you know where a thing is going, because it lands inside
whatever is selected rather than wherever you let go.

Right-clicking an entry offers **Insert into the selection** and **Insert at the
top level**.

## Two kinds of component

The catalog marks which is which, and it decides the shape of the generated
file:

- **Stateless builders.** A `Button` is a value: you hand it a label, a variant
  and a click handler, and it draws. Most of the catalog is this.
- **Stateful entities.** A `TextInput` owns a buffer, a caret and an IME. It is
  a gpui entity, created with `cx.new`, and it emits events rather than taking
  handlers.

You do not pick between them; the component is one or the other. What it changes
for you is that a document holding *any* entity generates as a `Render` entity
with a constructor, rather than a `RenderOnce` builder. See
[what gets generated](tailorcodegen.md).

The entities are every text-ish input, every picker, the overlays that own
open-state, and the big surfaces — Editor, Markdown editor, Tabs, Accordion,
Pagination, Carousel, Table view, Tree view, Tab bar, Split panel.

## Slots

Children live in *slots*, not in one list. Most components have only the default
`children` slot, but the ones you build a screen out of have named regions:

| Component | Slots |
| --- | --- |
| `AppShell` | children, header, navbar, aside, footer |
| `Panel` | children, icon, action, footer |
| `Button` | left section, right section |
| `SplitPanel` | first, second |
| `StatusBar` | left, center, right |
| `Field` | children |
| `Tabs` / `Accordion` | one per tab or section, from the labels you typed |

Drop into a slot from the canvas or from the Outline. An empty slot draws a
dashed placeholder so there is something to aim at, and the Outline shows slots
as their own rows — which is the easier target when a region is a 64px strip.

Some slots are **single**: a shell has one header, not a list of them. Dropping
a second node into a single slot replaces what was there.

## The containers Tailor draws itself

`Tabs`, `Accordion`, `SplitPanel`, `AppShell`, `Carousel`, `SettingsView` and
`VirtualList` take their regions as `'static` closures. A closure is opaque — a
designer cannot drop a node into one — so Tailor draws those from the theme
instead of instantiating them.

That is not a downgrade, it is the point: drawing them is what lets you click a
tab to reveal the slot behind it and drop into it. A real `Tabs` would show you
one panel and hide the rest behind a closure you cannot open.

**Generated code uses the real component.** The drawing is a canvas affordance
and never leaves the canvas. `SettingsView` is the one whose generated shape
differs most from its drawn one: the canvas shows a sidebar and one page's
slot, and the export declares `.page(id, title)` per page and fills them from a
single `.content(|page, ..| match page { .. })` closure, because that is the
API the real component has.

One consequence, and the Problems panel will tell you about it: a *stateful*
component inside one of these is a problem. Their regions are `'static`, so
a `TextInput` in an `AppShell` header cannot be a field of the screen — it would
have to be created inside the closure, on every frame. Extract that part into
its own component, which generates its own entity, and place that instead.

Event handlers inside those regions are fine — Tailor routes them through a weak
handle rather than a borrowed context, which is what a hand-written host does.

## Components you build

A document is either a **screen** (generates a `Render` entity) or a
**component** (generates a `RenderOnce` builder). **File → New Component**, or
the *Add a component* item on a tab's right-click menu.

Your components appear at the top of the Library under **This project**. Place
one and you get a `@Name` node: it renders inline on the canvas, so the screen
looks like the screen, and it generates as `Name::new()`.

A placed component is a **reference, not a copy**. Change the component and
every placement changes.

Right-clicking one in the Library offers Insert, **Edit the component** (opens
its document), Rename…, Duplicate and Delete.

### Extract to a component

Select some nodes on the canvas, right-click, **Extract to a component…**.
Tailor lifts them into a new component document and leaves a reference behind in
their place.

The selection has to share one parent, because that is where the reference goes
back.

### Cycles

A component that would contain itself is refused when you drop it, and again
when the file is loaded — a hand-edited `.tailor` with a loop in it comes back
repaired rather than hanging the app that opened it.

That check is on *names*, because a `@Name` reference carries a name. It is also
why duplicating a component gives the copy a new name rather than a new id
alone: two documents called the same thing would generate the same Rust type,
which the Problems panel reports as an error.

## Props

The inspector's **Attributes** tab is generated from the catalog, one row per
prop, with the control the prop's type asks for: a text field, a number, a size
or variant picker, a colour swatch, an icon picker with search, a list editor.

Defaults are shown greyed. Tailor does not restate a default in the generated
code — the file is what you would have written, not a dump of every prop — so a
prop you never touched costs nothing in the output.

Any text, number or boolean prop can be **bound** to a state variable instead of
holding a literal. See [state, bindings and actions](tailorstate.md).

## Adding a component to the catalog

If you are working on Tailor itself: the catalog is the single source of truth,
and adding a component is two edits that have to happen together.

1. One `comp!` entry in `crates/tailor/model/src/catalog/`: the kind, the Rust
   type, the category, the blurb, the constructor shape, the props with their
   types and defaults, the slots, the events.
2. One arm in `crates/tailor/render/src/nodes/build.rs`, which turns a node of
   that kind into a live guise component for the canvas.

`PropSpec::emit` decides what the generator prints for each prop — a method
call, a bare flag, or something custom. Editing the catalog without the renderer
(or the other way round) is how a canvas and an export drift apart, which is the
one failure this design exists to prevent.

Two tests hold that together, and both fail loudly rather than quietly:

- **`catalog::coverage`** (`tailor-model`) reads guise's own source and asserts
  every `RenderOnce` builder and `Render` entity is either in the catalog or in
  `EXCLUDED` with a reason. Adding a component to guise and forgetting Tailor
  is a failing test, not a missing library entry noticed a release later.
- **`every_catalog_kind_generates_something`** (`tailor-codegen`) generates one
  document per catalog kind and asserts each names its own type. A `comp!` entry
  with no generator support fails here.

Constructor shapes live in `Ctor`: `Unit`, `Id`, `IdAnd(prop)`, `Arg(prop)`,
`Args(&[prop, ..])` for the ones that take two facts (`AIMessage::new(role,
body)`), the three `Entity` variants, and `Special` for the handful the emitter
writes by hand.
