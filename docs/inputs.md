# Inputs

Two groups, by how they hold state:

- **Controlled builders** — `Checkbox`, `Switch`, `Radio`, `Chip`. The parent
  owns the value; wire changes with `cx.listener`.
- **Stateful entities** — `TextInput`, `Select`, `SegmentedControl`. Created with
  `cx.new`, they own their buffer/selection and emit events.

## Checkbox

```rust
Checkbox::new("agree")
    .label("I agree to the terms")
    .checked(self.agree)
    .color(ColorName::Blue)
    .on_change(cx.listener(|this, _ev, _w, cx| {
        this.agree = !this.agree;
        cx.notify();
    }))
```

| Method | Default |
| --- | --- |
| `new(id)` | — |
| `checked(bool)` | `false` |
| `indeterminate(bool)` | `false` |
| `label(impl Into<SharedString>)` | none |
| `size(Size)` | `Sm` |
| `color(ColorName)` | `Blue` |
| `disabled(bool)` | `false` |
| `on_change(handler)` | — |

## Switch

Same controlled API as `Checkbox`, rendered as a sliding track.

```rust
Switch::new("notify")
    .label("Enable notifications")
    .checked(self.notify)
    .on_change(cx.listener(|this, _, _, cx| { this.notify = !this.notify; cx.notify(); }))
```

Methods: `new(id)`, `checked`, `label`, `size` (default `Md`), `color`,
`disabled`, `on_change`.

## Radio

A single radio button. Exclusivity is the parent's job — give each one a
`checked` derived from your selection and an `on_change` that sets it.

```rust
let plans = ["Free", "Pro", "Enterprise"];
let group = plans.iter().enumerate().fold(Group::new(), |g, (i, label)| {
    g.child(
        Radio::new(("plan", i))
            .label(*label)
            .checked(self.plan == i)
            .on_change(cx.listener(move |this, _, _, cx| { this.plan = i; cx.notify(); })),
    )
});
```

Methods: `new(id)`, `checked`, `label`, `size` (default `Sm`), `color`,
`disabled`, `on_change`.

## Chip

A selectable pill (controlled), good for tag-style multi/single selection.

```rust
Chip::new("chip", "Notifications")
    .checked(self.on)
    .color(ColorName::Blue)
    .on_change(cx.listener(|this, _, _, cx| { this.on = !this.on; cx.notify(); }))
```

Methods: `new(id, label)`, `checked`, `color`, `size` (default `Md`), `on_change`.

## TextInput (entity)

A single-line text field that owns its buffer, focus, and caret. Create it with
`cx.new` and keep the `Entity`.

```rust
let name = cx.new(|cx| {
    TextInput::new(cx)
        .label("Name")
        .placeholder("Ada Lovelace")
        .description("Click to focus, then type.")
});
```

| Builder method | Notes |
| --- | --- |
| `new(cx)` | construct inside `cx.new(\|cx\| ...)` |
| `value(&str)` | initial text |
| `placeholder` / `label` / `description` / `error` | chrome (error supersedes description, turns the border red) |
| `size(Size)` | default `Sm` |
| `radius(Size)` | |
| `disabled(bool)` | |
| `password(bool)` | masks characters |

Runtime: `input.read(cx).text()` reads the value; `input.update(cx, |ti, cx| ti.set_text("…", cx))` sets it. It emits:

```rust
pub enum TextInputEvent { Change(String), Submit(String) } // Submit fires on Enter
```

```rust
cx.subscribe(&name, |_this, _input, event, cx| {
    if let TextInputEvent::Submit(value) = event {
        // ...
    }
}).detach();
```

Editing supports insert, backspace/delete, arrows, home/end, and is
unicode-correct (the underlying `TextEdit` model is unit-tested).

## Select (entity)

A dropdown picker that owns its open state and selection, rendered as a trigger
plus a deferred (overlaid) list.

```rust
let framework = cx.new(|cx| {
    Select::new(cx)
        .label("Framework")
        .placeholder("Choose one…")
        .data(["gpui", "Mantine", "SwiftUI", "Flutter"])
        .selected(0)
});
```

Methods: `new(cx)`, `data(iter)`, `selected(usize)`, `placeholder`, `label`,
`size`, `disabled`. Read with `selected_index()` / `selected_value()`. Emits
`SelectEvent(usize)`.

## SegmentedControl (entity)

A single-choice segmented switch.

```rust
let range = cx.new(|cx| {
    SegmentedControl::new(cx).data(["Day", "Week", "Month"]).selected(1)
});
```

Methods: `new(cx)`, `data(iter)`, `selected(usize)`, `size`. Read with
`selected_index()`. Emits `SegmentedControlEvent(usize)`.
