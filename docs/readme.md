# guise documentation

A Mantine-inspired component library for [gpui](https://github.com/zed-industries/zed).

`guise` gives you a themed palette, sizing tokens, 120+ ready-made components,
a Flutter-style flexbox layer, terse layout macros, an animation toolkit
(easings, springs, exit transitions), typed drag & drop, and a lightweight
React-style state layer (signals, two-way bindings, and a reactive form) —
all on top of gpui's retained-mode renderer.

## Start here

- [Getting started](gettingstarted.md) — add the crate, install a theme, render your first window.
- [Tutorial](tutorial.md) — build a complete app step by step, from an empty window to bound data views.
- [App walkthrough](appguide.md) — a project tracker wired the way a real guise app fits together: forms, overlays, reordering, motion.
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
- [Editor](editor.md) — `Editor`, a code editor entity with 10-language highlighting and a diagnostics API
- [Markdown editor](markdowneditor.md) — `MarkdownEditor`, an Obsidian-style live-preview markdown editor
- [Overlays](overlays.md) — `Modal`, `Drawer`, `ConfirmModal`, `Menu`, `MenuBar`, `ContextMenu`, `Popover`, `HoverCard`, `LoadingOverlay`, `Spotlight`, `Tooltip`, `Tour`, and `OverlayHost` (window-level modals + toasts)
- [Navigation](navigation.md) — `Breadcrumbs`, `NavLink`, `NavigationMenu`, `Stepper`, `Pagination`, `StatusBar`
- [WebView](webview.md) — `WebView`, a native embedded web view (`wry`)

## Systems

- [Flex layout](flex.md) — `guise::flex`: `Row`, `Column`, `Container`, `Expanded`, `Stack`, …
- [Layout macros](macros.md) — `row!`, `col!`, `zstack!`, `wrap!`, `vstack!`, `hstack!`
- [Transitions & animation](transitions.md) — `Easing` curves, `Spring` physics, `Transition`, `Collapse` (true height), `Presence` (exit animations)
- [Drag & drop](dnd.md) — `Draggable`, `DropTarget`, `SortableList` with typed payloads
- [Reactive state](reactive.md) — `Signal`, `Binding` (two-way `.bind`), `provide`/`use_context`, `use_state`/`watch`/`use_memo`/`use_effect`, and the reactive `Form`
- [Window menu](windowmenu.md) — wiring the native application menu
- [Architecture](architecture.md) — workspace layout, the gpui dependency, and how to add a component

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
