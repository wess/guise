# guise documentation

A Mantine-inspired component library for [gpui](https://github.com/zed-industries/zed).

`guise` gives you a themed palette, sizing tokens, ~90 ready-made components,
a Flutter-style flexbox layer, terse layout macros, mount transitions, and a
lightweight React-style state layer (with form validation and two-way
bindings) — all on top of gpui's retained-mode renderer.

## Start here

- [Getting started](gettingstarted.md) — add the crate, install a theme, render your first window.
- [Tutorial](tutorial.md) — build a complete app step by step, from an empty window to bound data views.
- [Theming](theming.md) — the palette, scales, color scheme, and semantic colors.
- [Component model](components.md) — how components are built (`RenderOnce` builders vs. stateful entities), variants, sizes, and event handlers.

## Components

- [Buttons](buttons.md) — `Button`, `ActionIcon`, `CloseButton`, `ThemeIcon`, `CopyButton`
- [Icons](icons.md) — `Icon`, `IconName`
- [Inputs](inputs.md) — `TextInput`, `TextArea`, `NumberInput`, `PasswordInput`, `PinInput`, `Checkbox`, `Switch`, `Radio`, `RadioGroup`, `CheckboxGroup`, `Select`, `Combobox`, `SegmentedControl`, `Slider`, `RangeSlider`, `Rating`, `ColorInput`, `TagsInput`, `Chip`, `Field`
- [Typography](typography.md) — `Text`, `Title`, `Anchor`, `Code`, `Kbd`, `Mark`, `Blockquote`, `Spoiler`
- [Layout](layout.md) — `Stack`, `Group`, `Center`, `SimpleGrid`, `ScrollArea`, `Paper`, `Card`, `Divider`, `AppShell`, `Container`, `Space`
- [Panels](panels.md) — `Panel` (titled, collapsible surface), `SplitPanel` (draggable two-pane split)
- [Feedback](feedback.md) — `Alert`, `Loader`, `Progress`, `RingProgress`, `Notification`, `ToastStack`, `Skeleton`
- [Data display](data.md) — `Avatar`, `AvatarGroup`, `Badge`, `Indicator`, `Image`, `List`, `Table`, `TableView`, `DataView`, `TreeView`, `TabBar`, `Timeline`, `Tabs`, `Accordion`
- [Charts](charts.md) — `Sparkline`, `LineChart`, `BarChart`, `PieChart`
- [Editor](editor.md) — `Editor`, a code editor entity with Rust / SQL / JSON highlighting
- [Overlays](overlays.md) — `Modal`, `Drawer`, `ConfirmModal`, `Menu`, `MenuBar`, `ContextMenu`, `Popover`, `HoverCard`, `LoadingOverlay`, `Spotlight`, `Tooltip`
- [Navigation](navigation.md) — `Breadcrumbs`, `NavLink`, `Stepper`, `Pagination`, `StatusBar`
- [WebView](webview.md) — `WebView`, a native embedded web view (`wry`)

## Systems

- [Flex layout](flex.md) — `guise::flex`: `Row`, `Column`, `Container`, `Expanded`, `Stack`, …
- [Layout macros](macros.md) — `row!`, `col!`, `zstack!`, `wrap!`, `vstack!`, `hstack!`
- [Transitions](transitions.md) — `Transition`, `Collapse` mount animations
- [Reactive state](reactive.md) — `Signal`, `Binding` (two-way `.bind`), `provide`/`use_context`, `use_state`/`watch`/`use_memo`/`use_effect`, `use_form`/`FormState`
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
