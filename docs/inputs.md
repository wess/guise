# Inputs

Two groups, by how they hold state:

- **Controlled builders** — `Checkbox`, `Switch`, `Radio`, `Chip`, plus the
  group wrappers `RadioGroup` / `CheckboxGroup`. The parent owns the value; wire
  changes with `cx.listener`.
- **Stateful entities** — `TextInput`, `TextArea`, `NumberInput`, `Select`,
  `Combobox`, `SegmentedControl`, `Slider`. Created with `cx.new`, they own their
  buffer/selection and emit events.

`Field` is the shared label/description/error chrome that wraps a control;
`NumberInput`, `TextArea`, and `Combobox` compose it.

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

## RadioGroup

A controlled set of mutually-exclusive radios — the ergonomic layer over bare
`Radio`. The parent owns the selected index; the group wires exclusivity and
reports the new index.

```rust
RadioGroup::new()
    .label("Plan")
    .options(["Free", "Pro", "Enterprise"])
    .value(self.plan)
    .on_change(cx.listener(|this, index, _w, cx| { this.plan = index; cx.notify(); }))
```

Methods: `new()`, `options(iter)`, `value(usize)`, `label`, `color`, `size`,
`on_change(Fn(usize, &mut Window, &mut App))`.

## CheckboxGroup

A controlled set over a shared selection. The parent owns a sorted `Vec<usize>`;
each toggle reports the *next* full selection.

```rust
CheckboxGroup::new()
    .label("Notify me about")
    .options(["Mentions", "Replies", "Releases"])
    .value(self.notify.clone())
    .on_change(cx.listener(|this, next, _w, cx| { this.notify = next; cx.notify(); }))
```

Methods: `new()`, `options(iter)`, `value(iter)`, `label`, `color`, `size`,
`on_change(Fn(Vec<usize>, &mut Window, &mut App))`.

## Field

The label / description / error wrapper every input draws. Use it directly to
give any control the same chrome.

```rust
Field::new()
    .label("API key")
    .description("Found in your account settings.")
    .child(my_control)
```

Methods: `new()`, `label`, `description`, `error` (supersedes description),
`child(impl IntoElement)`.

## NumberInput (entity)

A numeric field with stepper buttons. Constrains input to digits/`.`/`-`, clamps
to `min`/`max`, and nudges by `step` (steppers or ↑/↓). Composes `Field`.

```rust
let qty = cx.new(|cx| {
    NumberInput::new(cx).label("Quantity").min(0.0).max(99.0).step(1.0).value(1.0)
});
```

Methods: `new(cx)`, `value(f64)`, `min`, `max`, `step`, `label`, `description`,
`error`, `size`, `disabled`. Read with `value_f64() -> Option<f64>`. Emits
`NumberInputEvent(f64)`.

## TextArea (entity)

A multiline field. Enter inserts a newline; ↑/↓ move between lines keeping the
column. Reuses the unicode-correct `TextEdit` model and composes `Field`.

```rust
let bio = cx.new(|cx| {
    TextArea::new(cx).label("Bio").placeholder("Tell us about yourself").rows(4)
});
```

Methods: `new(cx)`, `value(&str)`, `placeholder`, `label`, `description`,
`error`, `rows(usize)`, `size`, `disabled`. Read with `text()`; set with
`set_text(value, cx)`. Emits `TextAreaEvent(String)`.

## Combobox (entity)

A searchable `Select`. The trigger is an editable query; the deferred list
filters by case-insensitive substring. Single-select closes on choice;
`multiple(true)` keeps a set and stays open. Type to filter, Enter picks the
first match, Esc closes.

```rust
let city = cx.new(|cx| {
    Combobox::new(cx).label("City").data(["Austin", "Boston", "Chicago", "Denver"])
});
```

Methods: `new(cx)`, `data(iter)`, `multiple(bool)`, `selected(iter)`,
`placeholder`, `label`, `size`, `disabled`. Read with `selected_indices()`.
Emits `ComboboxEvent(usize)` (the toggled index).

## Slider (entity)

A horizontal value track in `min..=max` snapped to `step`. Click the track to
set a value; arrow keys nudge by one step. (gpui doesn't hand elements their own
bounds, so position is derived from discrete segment cells rather than the raw
pointer x.)

```rust
let volume = cx.new(|cx| Slider::new(cx).min(0.0).max(100.0).step(5.0).value(40.0));
```

Methods: `new(cx)`, `value(f64)`, `min`, `max`, `step`, `color`, `disabled`.
Read with `value_f64() -> f64`. Emits `SliderEvent(f64)`.
