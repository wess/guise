# Inputs

Two groups, by how they hold state:

- **Controlled builders** — `Checkbox`, `Switch`, `Radio`, `Chip`, `Rating`,
  plus the group wrappers `RadioGroup` / `CheckboxGroup`. The parent owns the
  value; wire changes with `cx.listener` (or a [binding](#binding-inputs)).
- **Stateful entities** — `TextInput`, `TextArea`, `NumberInput`,
  `PasswordInput`, `PinInput`, `ColorInput`, `TagsInput`, `Select`, `Combobox`,
  `SegmentedControl`, `Slider`, `RangeSlider`. Created with `cx.new`, they own
  their buffer/selection and emit events.

`Field` is the shared label/description/error chrome that wraps a control;
`NumberInput`, `TextArea`, and `Combobox` compose it.

## Binding inputs

Every input can skip the hand-written change handler and two-way bind to a
[`Signal`](reactive.md#signal) instead. The two shapes mirror the two component
patterns:

- **Controlled builders** take a [`Binding`](reactive.md#bindings) —
  `.bind(signal.binding())` for the whole value, or
  `.bind(signal.lens(...))` for one field of a struct signal. The binding
  overrides the plain value setter; user actions write back through it, then
  run any `on_change`.
- **Stateful entities** bind once after creation with the associated function
  `X::bind(&entity, &signal, cx)` — the entity adopts the signal's value now,
  edits write back, and signal writes update the entity.

```rust
// Controlled: the binding replaces `checked` + `on_change`.
let dark = use_state(cx, false);
Switch::new("dark-mode").label("Dark mode").bind(dark.binding())
```

```rust
// Entity: bind after cx.new; edits and signal writes stay in sync.
let name = use_state(cx, String::new());
let input = cx.new(|cx| TextInput::new(cx).label("Name"));
TextInput::bind(&input, &name, cx);
```

Writes land in `set_if_changed` on both directions, so an echoed value is a
no-op and updates can't loop. The full story — lenses, `map`, `constant` — is
in [Reactive → Bindings](reactive.md#bindings).

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

## Rating

A row of clickable stars (controlled). Clicking star *i* sets the value to
`i`; hovering an unfilled star previews it in the accent color. The `f32`
value is rounded to whole stars for display.

```rust
Rating::new("stars")
    .value(self.stars)
    .color(ColorName::Yellow)
    .on_change(cx.listener(|this, value: &f32, _w, cx| {
        this.stars = *value;
        cx.notify();
    }))
```

| Method | Default | Notes |
| --- | --- | --- |
| `new(id)` | — | |
| `value(f32)` | `0.0` | rounded to whole stars, clamped to `count` |
| `count(usize)` | `5` | how many stars |
| `color(impl Into<ColorValue>)` | `Yellow` | |
| `size(Size)` | `Md` | glyph sizes: xs 14 … xl 36 |
| `readonly(bool)` | `false` | display-only — no hover preview, no clicks |
| `on_change(handler)` | — | `Fn(&f32, &mut Window, &mut App)` |
| `bind(Binding<f32>)` | — | overrides `value`; clicks write back, then run `on_change` |

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

Editing follows the macOS/Linux conventions: Option+←/→ moves by word,
Cmd+←/→ to line start/end, Option+Backspace deletes a word, Cmd+Backspace /
Cmd+Delete clears to the line edge, plus Ctrl+A / Ctrl+E / Ctrl+K. It is
unicode-correct (the underlying `TextEdit` model is unit-tested). Escape and
Tab are left to bubble so a host (a dialog, a form) can cancel or move focus.

### Driving a field yourself

The single-line model and its key map are public, for hosts that render their
own chrome (a search bar, a command palette) instead of embedding the full
`TextInput`:

```rust
use guise::{apply_key, KeyOutcome, TextEdit};

// state: edit: TextEdit
match apply_key(&mut self.edit, &keystroke) {
    KeyOutcome::Submit => { /* commit */ }
    KeyOutcome::Cancel => { /* dismiss */ }
    KeyOutcome::Edited => { /* redraw; read self.edit.split() for the caret */ }
    KeyOutcome::Pass   => { /* not ours — Tab, Cmd+W, … */ }
}
```

`apply_key` is exactly what `TextInput` uses internally, so an inline field and
the full component stay keystroke-for-keystroke identical.

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

## RangeSlider (entity)

A two-thumb slider holding a `(low, high)` pair in `min..=max`. Each thumb is
a real gpui drag source (`on_drag` + `on_drag_move`), so dragging tracks the
pointer even outside the control; clicking the track jumps the nearest thumb;
arrow keys (and Home/End) nudge the last active thumb. Values snap to `step`
and keep at least `min_gap` apart.

```rust
let range = cx.new(|cx| {
    RangeSlider::new(cx).min(0.0).max(100.0).min_gap(10.0).value((20.0, 80.0))
});
cx.subscribe(&range, |_this, _slider, event: &RangeSliderEvent, _cx| {
    let (low, high) = event.0;
}).detach();
```

| Method | Default | Notes |
| --- | --- | --- |
| `new(cx)` | — | construct inside `cx.new(\|cx\| ...)` |
| `value((f64, f64))` | `(25.0, 75.0)` | set `min`/`max`/`step`/`min_gap` first — the pair is normalized against them |
| `min(f64)` / `max(f64)` | `0.0` / `100.0` | |
| `step(f64)` | `1.0` | |
| `min_gap(f64)` | `0.0` | minimum distance between the thumbs |
| `color(ColorName)` | `Blue` | |
| `size(Size)` | `Md` | thumb and track dimensions |
| `disabled(bool)` | `false` | |
| `value_pair()` | — | read the current `(low, high)` |
| `RangeSlider::bind(&entity, &Signal<(f64, f64)>, cx)` | — | two-way [binding](reactive.md#bindings) |

Emits `RangeSliderEvent((f64, f64))` on change. (Unlike `Slider`, click
positions are hit-tested against real track bounds — an invisible canvas
captures them each frame, since gpui doesn't hand elements their own bounds.)

## PasswordInput (entity)

A masked text field with an eye toggle that reveals the plain text. Same
buffer, focus, and key handling as `TextInput` in password mode; the eye flips
visibility without losing the value.

```rust
let secret = cx.new(|cx| {
    PasswordInput::new(cx)
        .label("Password")
        .placeholder("At least 8 characters")
});
cx.subscribe(&secret, |_this, _input, event, _cx| {
    if let PasswordInputEvent::Submit(value) = event { /* log in */ }
}).detach();
```

| Method | Default | Notes |
| --- | --- | --- |
| `new(cx)` | — | construct inside `cx.new(\|cx\| ...)` |
| `value(&str)` | `""` | initial text |
| `placeholder` / `label` / `description` / `error` | none | chrome; error turns the border red |
| `visible(bool)` | `false` | start revealed; the eye still toggles |
| `size(Size)` | `Sm` | |
| `disabled(bool)` | `false` | |
| `text()` / `set_text(&str, cx)` | — | read / write at runtime |
| `PasswordInput::bind(&entity, &Signal<String>, cx)` | — | two-way [binding](reactive.md#bindings) |

Emits `PasswordInputEvent::{Change(String), Submit(String)}` — `Submit` fires
on Enter. Escape and Tab bubble to the host, as in `TextInput`.

## PinInput (entity)

Segmented one-character code boxes — the one-time-code field. Typing advances,
Backspace clears and retreats, arrows move between boxes, and Cmd+V fills them
from the clipboard (whitespace stripped, extra characters dropped).

```rust
let pin = cx.new(|cx| PinInput::new(cx).length(6).mask(true));
cx.subscribe(&pin, |_this, _pin, event, _cx| {
    if let PinInputEvent::Complete(code) = event { /* verify */ }
}).detach();
```

| Method | Default | Notes |
| --- | --- | --- |
| `new(cx)` | — | construct inside `cx.new(\|cx\| ...)` |
| `length(usize)` | `4` | number of boxes |
| `mask(bool)` | `false` | render filled boxes as bullets |
| `value(&str)` | `""` | initial code; characters beyond `length` are dropped |
| `size(Size)` | `Sm` | box dimensions |
| `disabled(bool)` | `false` | |
| `text()` / `set_text(&str, cx)` | — | read / write at runtime |
| `PinInput::bind(&entity, &Signal<String>, cx)` | — | two-way [binding](reactive.md#bindings) |

Emits `PinInputEvent::{Change(String), Complete(String)}` — `Complete` fires
when every box is filled (its `Change` fires first). `text()` returns the
filled characters in slot order.

## ColorInput (entity)

A swatch plus an editable hex/CSS text field. Clicking the swatch opens the
full theme palette (14 colors × 10 shades) in a deferred dropdown; typing any
`css()`-parsable color — `#40c057`, `rgb(64, 192, 87)`, `teal` — updates the
swatch live. Enter normalizes the buffer to hex; Escape closes the dropdown.

```rust
let brand = cx.new(|cx| {
    ColorInput::new(cx).label("Brand color").value(rgb(34, 139, 230))
});
cx.subscribe(&brand, |_this, _input, event: &ColorInputEvent, _cx| {
    let color: Hsla = event.0;
}).detach();
```

| Method | Default | Notes |
| --- | --- | --- |
| `new(cx)` | — | construct inside `cx.new(\|cx\| ...)` |
| `value(impl Into<Hsla>)` | black | also rewrites the buffer as hex |
| `label` / `description` / `error` | none | chrome; error turns the border red |
| `size(Size)` | `Sm` | |
| `disabled(bool)` | `false` | |
| `color_value()` | — | read the current `Hsla` |
| `ColorInput::bind(&entity, &Signal<Hsla>, cx)` | — | two-way [binding](reactive.md#bindings) |

Emits `ColorInputEvent(Hsla)` whenever the color changes (typed or picked).

> **Note** Alpha is dropped when the buffer is normalized — the field holds
> opaque hex.

## TagsInput (entity)

A pill list with an inline editor. Enter or comma commits the query as a tag
(trimmed, non-empty, unique); Backspace in an empty query pops the last pill;
each pill has a remove button.

```rust
let topics = cx.new(|cx| {
    TagsInput::new(cx)
        .label("Topics")
        .placeholder("Type and press Enter…")
        .max_tags(5)
});
cx.subscribe(&topics, |_this, _input, event: &TagsInputEvent, _cx| {
    let tags: &Vec<String> = &event.0;
}).detach();
```

| Method | Default | Notes |
| --- | --- | --- |
| `new(cx)` | — | construct inside `cx.new(\|cx\| ...)` |
| `tags(impl IntoIterator)` | `[]` | initial tags |
| `placeholder` / `label` / `description` / `error` | none | chrome; error turns the border red |
| `max_tags(usize)` | unlimited | commits beyond the cap are ignored |
| `size(Size)` | `Sm` | |
| `disabled(bool)` | `false` | |
| `tag_values()` / `set_tags(Vec<String>, cx)` | — | read / write at runtime (`set_tags` emits no event) |
| `TagsInput::bind(&entity, &Signal<Vec<String>>, cx)` | — | two-way [binding](reactive.md#bindings) |

Emits `TagsInputEvent(Vec<String>)` with the full list on every add/remove.
Committing a duplicate clears the query without emitting.

## Autocomplete (entity)

A freeform text field with suggestions. Unlike [Combobox](#combobox-entity)
(pick one of the options), the value here is whatever the user types —
suggestions are shortcuts, not constraints. Typing opens the list
(case-insensitive substring match, empty query shows nothing), ↑/↓ walk it,
Enter adopts the highlighted suggestion or commits the typed text, Escape
closes.

```rust
let field = cx.new(|cx| {
    Autocomplete::new(cx)
        .suggestions(["Rust", "Ruby", "Python", "TypeScript"])
        .label("Language")
});
cx.subscribe(&field, |_this, _f, event: &AutocompleteEvent, _cx| match event {
    AutocompleteEvent::Change(text) => { /* every edit */ }
    AutocompleteEvent::Commit(text) => { /* Enter or a suggestion click */ }
})
.detach();
```

Methods: `suggestions(iter)`, `value(text)`, `max_shown(n)` (default 8),
`placeholder` / `label` / `size` / `disabled`, `text()` reads back. Two-way
binding: `Autocomplete::bind(&entity, &Signal<String>, cx)`.

## Transfer (entity)

A dual-list membership editor: one item pool, two panes (available/chosen).
Click rows to check them, move checked items with the middle buttons. Emits
`TransferEvent(Vec<usize>)` — the chosen side's item indices — after every
move.

```rust
let transfer = cx.new(|cx| {
    Transfer::new(cx)
        .data(["Alpha", "Bravo", "Charlie", "Delta"])
        .chosen([1])
        .titles("Bench", "Team")
});
cx.subscribe(&transfer, |_this, _t, TransferEvent(chosen), _cx| {
    // chosen: indices into the original data, ascending
})
.detach();
```

Methods: `data(iter)`, `chosen(indices)`, `titles(left, right)`,
`height(px)` (default 200), `disabled(bool)`, `chosen_indices()` reads back.
Both panes scroll independently.
