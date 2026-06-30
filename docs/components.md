# Component model

`guise` components come in two flavors, chosen to match how each one behaves in
gpui's retained-mode renderer.

## 1. Stateless builders (`RenderOnce`)

Most components are `RenderOnce` builder structs with `#[derive(IntoElement)]`.
You construct one, chain configuration methods, and hand it to `.child(...)`.
The parent view owns any state.

```rust
Button::new("save", "Save")
    .variant(Variant::Filled)
    .color(ColorName::Blue)
    .size(Size::Md)
```

This is the same pattern zed's own `ui` crate uses, and the closest match to
Mantine's prop API. Builders are cheap values — create them fresh each render.

### Event handlers compose with `cx.listener`

Click/change handlers take `Fn(&ClickEvent, &mut Window, &mut App)`. Inside a
view, `cx.listener(...)` produces exactly that signature while giving you
`&mut Self`, so controlled components drive your view's state directly:

```rust
Checkbox::new("agree")
    .checked(self.agree)
    .on_change(cx.listener(|this, _ev, _window, cx| {
        this.agree = !this.agree;
        cx.notify();
    }))
```

"Controlled" means the component renders from a value you pass in (`checked`,
`active`, `value`) and reports changes through a handler — the value lives in
your view, exactly like React's controlled inputs.

## 2. Stateful entities (`Render` + events)

Components that own intrinsic, frame-to-frame state are gpui *entities* instead.
These are: `TextInput`, `Select`, `SegmentedControl`, `Tabs`, `Accordion`,
`Pagination`, `Menu`, `MenuBar`. You create them with `cx.new(...)`, store the `Entity`,
and add it as a child:

```rust
struct MyView {
    name: Entity<TextInput>,
}

impl MyView {
    fn new(cx: &mut Context<Self>) -> Self {
        let name = cx.new(|cx| TextInput::new(cx).label("Name").placeholder("Ada"));
        MyView { name }
    }
}

impl Render for MyView {
    fn render(&mut self, _w: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.name.clone()   // Entity<V: Render> is itself an element
    }
}
```

Entities emit typed events (`TextInputEvent`, `SelectEvent`, …). Subscribe with
`cx.subscribe(&entity, ...)` to react to them, or just read the entity's state
(`self.name.read(cx).text()`).

## Shared conventions

Almost every component supports the same vocabulary:

- **`Variant`** — `Filled`, `Light`, `Outline`, `Subtle`, `Default`,
  `Transparent`, `White`. Resolved against `(color, variant)` by `guise::surface`.
- **`ColorName`** — a named palette color (defaults to `Blue`, or the relevant
  neutral). Filled components pick a readable foreground automatically.
- **`Size`** — `Xs..Xl` controls height, padding, and font size together.
- **`radius`** — falls back to the theme's `default_radius` when unset.
- **`disabled`** — dims the control and drops its handler.

Everything visual is resolved from the theme at render time, so all components
restyle automatically when you switch light/dark or change the palette.

## The `Variant` system

`guise::surface(theme, color, variant)` returns a `Surface { bg, bg_hover, fg,
border }`. This is the shared resolver behind `Button`, `Badge`, `Alert`,
`ActionIcon`, `Avatar`, `Chip`, and more — so a `(color, variant)` pair looks
identical across components.

```rust
use guise::{surface, Variant};

let s = surface(theme(cx), ColorName::Teal, Variant::Light);
div().bg(s.bg).text_color(s.fg)
```

| Variant | Background | Foreground | Border |
| --- | --- | --- | --- |
| `Filled` | solid color | contrasting | — |
| `Light` | tinted | colored | — |
| `Outline` | transparent | colored | colored |
| `Subtle` | transparent (fills on hover) | colored | — |
| `Default` | surface | text | border |
| `Transparent` | none | colored | — |
| `White` | white | colored | — |
