# guise documentation

A component library for [gpui](https://github.com/zed-industries/zed), and the
visual interface builder that draws with it.

`guise` gives you a themed palette, sizing tokens, 130+ ready-made components,
a Flutter-style flexbox layer, terse macros for layout and motion, an
animation system (keyframes, sequences, stagger, springs, and a playhead you
can scrub), typed drag & drop, and a lightweight React-style state layer
(signals, two-way bindings, and a reactive form) — all on top of gpui's
retained-mode renderer.

**[Tailor](https://github.com/wess/tailor)** is the visual interface builder that draws with
them: drag those components onto a canvas, wire the state and the actions, and
export idiomatic Rust. Write the interface or draw it — you end up with the same
components either way. It is its own project, and consumes this library from
crates.io.

## Start here

- [Getting started](gettingstarted.md) — add the crate, install a theme, render your first window.
- [Tutorial](tutorial.md) — build a complete app step by step, from an empty window to bound data views.
- [App walkthrough](appguide.md) — a project tracker wired the way a real guise app fits together: forms, overlays, reordering, motion.
- [Motion tutorial](motiontutorial.md) — one animated panel, nine chapters: entrances, stagger, keyframes, a playhead, and exits.
- [Theming](theming.md) — the palette, scales, semantic colors, JSON theme files, and prebuilt presets.
- [Component model](components.md) — how components are built (`RenderOnce` builders vs. stateful entities), variants, sizes, and event handlers.

## Components

- [Buttons](buttons.md) — `Button`, `ActionIcon`, `CloseButton`, `ThemeIcon`, `CopyButton`
- [Icons](icons.md) — `Icon`, `IconName`, `Glyph` (the full Lucide set, embedded)
- [Inputs](inputs.md) — `TextInput`, `TextArea`, `NumberInput`, `PasswordInput`, `PinInput`, `Checkbox`, `Switch`, `Radio`, `RadioGroup`, `CheckboxGroup`, `Select`, `Combobox`, `Autocomplete`, `SegmentedControl`, `Slider`, `RangeSlider`, `Rating`, `ColorInput`, `TagsInput`, `Transfer`, `Chip`, `Field`
- [Dates & times](dates.md) — `Calendar`, `DatePicker`, `TimePicker`, and the pure `Date`/`Time` models
- [File handling](files.md) — `FileInput` (native dialog), `Dropzone` (OS drag-drop)
- [Typography](typography.md) — `Text`, `Title`, `Anchor`, `Code`, `Kbd`, `Mark`, `Blockquote`, `Spoiler`
- [Layout](layout.md) — `Stack`, `Group`, `Center`, `SimpleGrid`, `ScrollArea`, `Paper`, `Card`, `Divider`, `AppShell`, `Container`, `Space`, plus `Breakpoint`/`Responsive`
- [Panels](panels.md) — `Panel`, `SplitPanel`, and `PaneGroup` (splits-with-tabs with layout persistence)
- [Feedback](feedback.md) — `Alert`, `Loader`, `Progress`, `RingProgress`, `Notification`, `ToastStack`, `Skeleton`
- [Data display](data.md) — `Avatar`, `AvatarGroup`, `Badge`, `Indicator`, `Image`, `List`, `VirtualList`, `Table`, `TableView`, `DataView`, `TreeView`, `TabBar`, `Timeline`, `Tabs`, `Accordion`, `Carousel`
- [Charts](charts.md) — `Sparkline`, `LineChart`, `AreaChart`, `BarChart`, `ScatterChart`, `PieChart` — with optional axes, legends, and hover readouts
- [GPU View](gpuview.md) — `GpuView`, `GpuScene`, and `GpuTexture` for native scene, map, and simulation surfaces
- [Editor](editor.md) — `Editor`, a code editor entity with 10-language highlighting and a diagnostics API
- [AI](ai.md) — `AIChatView`, `AIComposer`, streaming text, reasoning, tool calls, citations and cost meters — transport-agnostic
- [Markdown editor](markdowneditor.md) — `MarkdownEditor`, an Obsidian-style live-preview markdown editor
- [Overlays](overlays.md) — `Modal`, `Drawer`, `ConfirmModal`, `Menu`, `MenuBar`, `ContextMenu`, `Popover`, `HoverCard`, `LoadingOverlay`, `Spotlight`, `Tooltip`, `Tour`, and `OverlayHost` (window-level modals + toasts)
- [Navigation](navigation.md) — `Breadcrumbs`, `NavLink`, `NavigationMenu`, `Stepper`, `Pagination`, `StatusBar`
- [WebView](webview.md) — `WebView`, a native embedded web view (`wry`)

## Systems

- [Flex layout](flex.md) — `guise::flex`: `Row`, `Column`, `Container`, `Expanded`, `Stack`, …
- [Macros](macros.md) — `row!`, `col!`, `zstack!`, `vstack!`, `hstack!`, plus `style!`, `color!`, `motion!` and `sequence!`
- [Motion & transitions](transitions.md) — keyframed `Motion`, `Sequence`, `Stagger`, the `Animator` playhead, `Easing`/`Spring` curves, `Transition`, `Collapse` (true height), `Presence` (exit animations)
- [Drag & drop](dnd.md) — `Draggable`, `DropTarget`, `SortableList` with typed payloads
- [Reactive state](reactive.md) — `Signal`, `Binding` (two-way `.bind`), `provide`/`use_context`, `use_state`/`watch`/`use_memo`/`use_effect`, and the reactive `Form`
- [Software update](update.md) — `Updater`, `UpdatePrompt`, `UpdateNotice`: release check, in-place install, and the prompt that runs it
- [Settings](settings.md) — `SettingsView`, `SettingsSection`, `SettingsRow`: the settings screen chrome, without a schema you have to adopt
- [DevTools](devtools.md) — `DevTools`: an in-app Safari-style inspector, with an Elements tree read back out of what actually rendered
- [Window menu & chrome](windowmenu.md) — the native application menu, plus `About`, `WindowControls` and `ResizeHandles`

## Tailor — the interface builder

A drag-and-drop builder for guise interfaces, downloadable as an app. Its canvas
renders the real components against the real theme, so what you lay out is what
it generates. It lives at [github.com/wess/tailor](https://github.com/wess/tailor), where its
documentation does too.

## Reference

- [Architecture](architecture.md) — workspace layout, the gpui dependency, and how to add a component
- [Releasing](release.md) — cutting a version and publishing the crate
- [Size & performance](performance.md) — what the crate costs to compile, link and render

## A taste

```rust
use guise::prelude::*;

Card::new().child(
    Stack::new()
        .gap(Size::Sm)
        .child(Title::new("Welcome").order(3))
        .child(Text::new("Build native UIs with a familiar component API.").dimmed())
        .child(
            Group::new()
                .justify(Justify::End)
                .child(Button::new("cancel", "Cancel").variant(Variant::Default))
                .child(Button::new("ok", "Get started")),
        ),
)
```
