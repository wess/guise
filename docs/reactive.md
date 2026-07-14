# Reactive state (`guise::reactive`)

A small, React-flavored layer over gpui's reactivity. gpui already has
observable entities and globals; `reactive` wraps them in a familiar API:

- **`Signal<T>`** — an observable state cell (React's `useState` value).
- **`Binding<T>`** — a two-way connection to a value (SwiftUI's `Binding`);
  see [Bindings](#bindings).
- **`provide` / `use_context`** — the Context/Provider pattern.
- **`use_state` / `watch` / `use_memo` / `use_effect`** — hook-style helpers.

Everything is in the prelude.

## Signal

A clonable handle to a piece of state. All clones share one backing cell, so
passing a `Signal` around — or providing it as context — gives every holder the
same live value. Writes notify observers.

```rust
let count = Signal::new(cx, 0i32);  // cx: &mut App or &mut Context<_>

count.get(cx);                 // T: Clone — copy the value out
count.read(cx);                // &T — borrow it
count.set(cx, 5);              // replace + notify
count.update(cx, |n| *n += 1); // mutate in place + notify
```

| Method | Signature | Notes |
| --- | --- | --- |
| `Signal::new(cx, value)` | `&mut App` | create |
| `get(cx)` | `&App -> T` | requires `T: Clone` |
| `read(cx)` | `&App -> &T` | borrow |
| `set(cx, value)` | `&mut App` | replace, notifies |
| `set_if_changed(cx, value)` | `&mut App` | replace + notify, unless equal — then nothing happens (`T: PartialEq`) |
| `update(cx, f)` | `&mut App` | `f: FnOnce(&mut T)`, notifies |
| `binding()` | `-> Binding<T>` | the whole signal as a two-way [binding](#bindings) |
| `lens(get, set)` | `-> Binding<U>` | one field as a [binding](#bindings) |
| `entity()` | `-> &Entity<T>` | for manual `cx.observe` |

`cx` is `&mut App`, but a `&mut Context<V>` derefs to it, so you can call these
from any view method or event handler.

## Bindings

The macOS-style value-binding story, in three parts: a [`Signal`](#signal) is
the **store**, a `Binding<T>` is the **connection** — a getter and a setter
over `App` — and a component's `.bind(...)` / `X::bind(...)` is the
**wiring**. Once wired, the value flows both ways with no hand-written change
handlers:

- **down** — the component reads the current value through the binding (or
  adopts the signal's value at bind time), and every signal write repaints it;
- **up** — a user edit writes back through the setter, which lands in
  [`set_if_changed`](#signal) and notifies every other observer.

Equality guards on both directions make an echoed write a no-op, so the
round trip terminates instead of looping.

### Binding<T>

Cheap to clone (both accessors are `Rc`-shared) and `'static`, so element
closures (`.on_click`, `.hover`) can capture it.

| Method | Notes |
| --- | --- |
| `Binding::new(get, set)` | from raw accessors — `Fn(&App) -> T` + `Fn(&mut App, T)` |
| `get(cx)` | read the current value |
| `set(cx, value)` | write a new value |
| `map(from, into)` | bidirectional transform, e.g. `Binding<f64>` ⇄ `Binding<String>`; `from` converts on read, `into` back on write |
| `Binding::constant(value)` | read-only over a fixed value; writes are a no-op (disabled or demo states) |

You rarely call `Binding::new` yourself — build one from a signal:

```rust
let dark = use_state(cx, false);
Switch::new("dark-mode").bind(dark.binding());       // the whole signal

let settings = use_state(cx, Settings::default());   // one field (a lens)
Checkbox::new("mute").bind(settings.lens(|s| s.muted, |s, v| s.muted = v));
```

`Signal::binding` requires `T: Clone + PartialEq`; `lens` projects a
`Signal<T>` to a `Binding<U>` with a getter and a field setter, and skips the
notify when a write leaves the field unchanged.

### Wiring components

The two [component patterns](components.md) bind differently — see
[Inputs → Binding inputs](inputs.md#binding-inputs) for the per-component
surface:

- **Controlled builders** (`Checkbox`, `Switch`, `Rating`, …) take a
  `Binding` via `.bind(...)`; it overrides the plain value setter, and user
  actions write back through it before running any `on_change`.
- **Stateful entities** (`TextInput`, `Slider`, `Editor`, …) own their state,
  so they bind to the `Signal` itself, once, after creation:
  `X::bind(&entity, &signal, cx)`. Under the hood that's a `cx.subscribe`
  (entity events → signal) plus a `cx.observe` (signal writes → entity).

### Worked example: one Signal, two editors

Bind a single `Signal<String>` to a `TextInput` and an `Editor` at once.
Type in either — the edit lands in the signal via `set_if_changed`, the
observer pushes it into the other view, and the equality guard stops the echo
there.

```rust
struct Scratch {
    source: Signal<String>,
    input: Entity<TextInput>,
    editor: Entity<Editor>,
}

impl Scratch {
    fn new(cx: &mut Context<Self>) -> Self {
        let source = use_state(cx, String::from("fn main() {}"));

        let input = cx.new(|cx| TextInput::new(cx).label("One-liner"));
        TextInput::bind(&input, &source, cx);

        let editor = cx.new(|cx| Editor::new(cx).language(Language::Rust).rows(6));
        Editor::bind(&editor, &source, cx);

        watch(cx, &source); // only needed if this view renders the value too
        Scratch { source, input, editor }
    }
}

impl Render for Scratch {
    fn render(&mut self, _w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Stack::new()
            .child(self.input.clone())
            .child(self.editor.clone())
            .child(Text::new(format!("{} chars", self.source.read(cx).len())).dimmed())
    }
}
```

Programmatic writes work the same way: `source.set(cx, "reset".into())`
updates both views, no component-specific code required.

> **Tip** For collections there's a dedicated component:
> [`DataView`](data.md#dataview-entity) observes a `Signal<Vec<T>>` and
> repaints the list/grid on every write — filtering and sorting are render-time
> projections, so the source vector is never touched.

## Context / Provider

Share a value across the whole app, keyed by its Rust **type** — the gpui global
idiom (one value per type). This is how a child reads state it wasn't handed.

```rust
provide(cx, count.clone());          // <Context.Provider value={count}>
let count = use_context::<Signal<i32>>(cx).unwrap();  // useContext
```

| Function | Notes |
| --- | --- |
| `provide::<T>(cx, value)` | set/replace the provided value of type `T` |
| `use_context::<T: Clone>(cx) -> Option<T>` | cloned read |
| `use_context_ref::<T>(cx) -> Option<&T>` | borrowed read |
| `has_context::<T>(cx) -> bool` | presence check |

> One value **per type**. To provide several values of the same shape, wrap each
> in a distinct newtype (`struct UserId(Signal<u64>)`).

## Hooks

```rust
let count = use_state(cx, 0i32);   // = Signal::new
watch(cx, &count);                 // re-render this view when `count` changes

let label = use_memo(cx, &count, |n| format!("Count: {n}"));  // derived signal
use_effect(cx, &count, |n, _cx| println!("count -> {n}"));    // side effect
```

`watch` is the wiring behind a component "subscribing" to state. Call it once
per signal in a view's constructor (where `cx` is the view's `Context`).

- **`use_memo(cx, &source, f)`** — derived state (React's `useMemo`). Returns
  a new `Signal<U>` that recomputes `f(&T) -> U` on every `source` change;
  `watch` the returned signal like any other.
- **`use_effect(cx, &source, f)`** — run `f(&value, &mut App)` with the
  current value whenever `source` changes (React's `useEffect` with one
  dependency).

> **Caution** `use_effect` clones the value out of the signal before running
> your closure, so the effect may freely read or write any signal — including
> `source` itself. A write to `source` re-triggers the effect, so guard it
> with a condition (or the effect loops forever); to derive a value from the
> source, use `use_memo` instead.

## Forms

Two layers. **`Form`** is the one to reach for: every field is its own
`Signal<String>`, so inputs bind to fields directly — no change handlers, no
copying values in and out. **`FormState`** underneath is the plain
unit-testable map (values + validators + errors) for when you don't need the
wiring.

### Form — fields as signals

```rust
use guise::reactive::validators;

let form = Form::new(cx)
    .field(cx, "email", "")
    .rule("email", validators::required())
    .rule("email", validators::email())
    .field(cx, "password", "")
    .rule("password", validators::min_len(8))
    .field(cx, "confirm", "")
    .rule_form("confirm", validators::equals_field("password", "Passwords must match"));

// Each field IS a Signal<String> — bind inputs straight to it:
TextInput::bind(&email_input, &form.signal("email"), cx);
PasswordInput::bind(&password_input, &form.signal("password"), cx);

// Submit is one line — validate, get the values on success:
if let Some(values) = form.submit(cx) { /* values["email"] … */ }

// Render errors (watch the errors signal to re-render on validation):
watch(cx, &form.errors());
let email_error = form.error(cx, "email"); // Option<String>
```

Behavior worth knowing: `Form` is `Clone + 'static` (`Rc`-shared) so handlers
can capture it; rules run in order and the first failure wins; edits mark a
field *touched* (`form.touched("email")`); and a field that failed validation
**re-validates live** as it's edited — the error clears the moment the value
becomes valid, the standard validate-on-submit-then-live pattern.

Methods: `field(cx, name, initial)`, `rule(name, Validator)`,
`rule_form(name, Rule)` (cross-field), `signal(name)`, `value(cx, name)`,
`set(cx, name, v)`, `values(cx)`, `validate(cx)`, `validate_field(cx, name)`,
`error(cx, name)`, `errors()` (the signal), `touched(name)`, `is_valid(cx)`,
`submit(cx)`.

### Validators

`required()`, `min_len(n)`, `max_len(n)`, `email()`, `numeric()`,
`min_value(f64)`, `max_value(f64)`, `one_of(&["a", "b"])`,
`matches(pred, message)` (custom predicate), and the cross-field
`equals_field(other, message)` (a `Rule` — attach with `rule_form`). A
`Validator` is `Box<dyn Fn(&str) -> Option<String>>` and a `Rule` also sees a
`FormValues` snapshot of the whole form, so custom rules are just closures.

### FormState — the pure layer

```rust
let form = use_form(cx, FormState::new()
    .field("email", "")
    .validator("email", validators::email()));

form.update(cx, |f| {
    if f.validate() { /* f.value("email") … */ }
});
let email_error = form.read(cx).error("email"); // Option<&str>
```

`FormState` methods: `new()`, `field(name, initial)`, `validator(name, v)`,
`value(name)`, `set(name, value)`, `validate_field(name)`, `validate()`,
`error(name)`, `is_valid()`.

## Worked example: two views, one shared Signal

The pattern is exactly React's "lift state up, share via context": create a
`Signal` at the root, `provide` it, and let descendants read it back with
`use_context` and `watch` it.

```rust
use gpui::prelude::*;
use gpui::{Context, Entity, Window, IntoElement};
use guise::prelude::*;

// A child that never receives the signal directly — it reads it from context.
struct Counter { count: Signal<i32> }

impl Counter {
    fn new(cx: &mut Context<Self>) -> Self {
        let count = use_context::<Signal<i32>>(cx).expect("counter provided");
        watch(cx, &count);                 // re-render when it changes
        Counter { count }
    }
}

impl Render for Counter {
    fn render(&mut self, _w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Text::new(format!("Count: {}", self.count.get(cx))).bold()
    }
}

struct App { count: Signal<i32>, counter: Entity<Counter> }

impl App {
    fn new(cx: &mut Context<Self>) -> Self {
        let count = use_state(cx, 0);
        provide(cx, count.clone());        // share it
        let counter = cx.new(Counter::new);
        App { count, counter }
    }
}

impl Render for App {
    fn render(&mut self, _w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Stack::new()
            .child(self.counter.clone())   // reflects the shared value
            .child(Button::new("inc", "+").on_click(
                cx.listener(|this, _, _, cx| this.count.update(cx, |n| *n += 1)),
            ))
    }
}
```

Press `+` and the `Counter` view updates, even though it only ever knew about the
`Signal` through context. Mutating the signal notifies every `watch`er.

## How it maps to gpui

- `Signal<T>` is a thin wrapper over `Entity<T>`; `update` calls
  `entity.update(cx, |v, cx| { …; cx.notify() })`.
- `Binding<T>` is a pair of `Rc`'d closures over `App` — no entity of its own.
- `X::bind(entity, signal, cx)` is `cx.subscribe` (component events → signal)
  plus `cx.observe` (signal writes → component), both detached, with equality
  guards at each end.
- `watch` is `cx.observe(signal.entity(), |_, _, cx| cx.notify()).detach()`.
- `provide` / `use_context` store the value in a typed gpui global.

If you outgrow this layer, drop down to the entities and globals directly — it's
the same machinery.
