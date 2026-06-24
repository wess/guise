# Reactive state (`guise::reactive`)

A small, React-flavored layer over gpui's reactivity. gpui already has
observable entities and globals; `reactive` wraps them in a familiar API:

- **`Signal<T>`** — an observable state cell (React's `useState` value).
- **`provide` / `use_context`** — the Context/Provider pattern.
- **`use_state` / `watch`** — hook-style helpers.

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
| `update(cx, f)` | `&mut App` | `f: FnOnce(&mut T)`, notifies |
| `entity()` | `-> &Entity<T>` | for manual `cx.observe` |

`cx` is `&mut App`, but a `&mut Context<V>` derefs to it, so you can call these
from any view method or event handler.

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
```

`watch` is the wiring behind a component "subscribing" to state. Call it once
per signal in a view's constructor (where `cx` is the view's `Context`).

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
- `watch` is `cx.observe(signal.entity(), |_, _, cx| cx.notify()).detach()`.
- `provide` / `use_context` store the value in a typed gpui global.

If you outgrow this layer, drop down to the entities and globals directly — it's
the same machinery.
