# Getting started

## Requirements

Rust stable. Both `guise` and `gpui` come straight from crates.io — there is
no git pin and no `[patch.crates-io]` recipe anymore. See
[Architecture](architecture.md#the-gpui-dependency).

## Add the dependency

```sh
cargo add guise-ui gpui
```

or in `Cargo.toml`:

```toml
[dependencies]
guise-ui = "0.2"
gpui = "0.2"
```

> **Note** The crate is published as **`guise-ui`** (the `guise` name was taken
> on crates.io), but its library is named `guise` — so you still write
> `use guise::prelude::*;`.

## The smallest app

```rust
use gpui::prelude::*;
use gpui::{
    div, px, size, App, Application, Bounds, Context, IntoElement, Window, WindowBounds,
    WindowOptions,
};
use guise::prelude::*;

struct Hello;

impl Render for Hello {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Read the active theme for window-level colors.
        let t = cx.global::<Theme>();
        div()
            .size_full()
            .bg(t.body().hsla())
            .text_color(t.text().hsla())
            .p(px(24.0))
            .child(
                Stack::new()
                    .gap(Size::Md)
                    .child(Title::new("Hello, guise").order(1))
                    .child(Button::new("ok", "Click me")),
            )
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        // 1. Install a theme exactly once, before opening any window.
        Theme::dark().init(cx);

        // 2. Open a window hosting your root view.
        let bounds = Bounds::centered(None, size(px(640.0), px(480.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_window, cx| cx.new(|_cx| Hello),
        )
        .expect("open window");
        cx.activate(true);
    });
}
```

Two things make this work:

1. **`Theme::dark().init(cx)`** installs the theme as a gpui *global*. Every
   `guise` component reads it during render, so colors, spacing and radius are
   consistent everywhere. Calling components before a theme is installed will
   panic — install it first. See [Theming](theming.md).
2. Components are values you build with a fluent API and hand to `.child(...)`.
   They render themselves; you never manually resolve a color or size.

## The prelude

`use guise::prelude::*;` brings in every component, the theme types
(`Theme`, `Size`, `ColorName`, `Variant`, …), the layout macros, and the
reactive helpers. The one thing it intentionally leaves out is the
[`guise::flex`](flex.md) module, whose names (`Row`, `Column`, `Stack`,
`Center`) overlap with the themed layout components — import that explicitly:

```rust
use guise::flex::*;
```

## Next

- Follow the [tutorial](tutorial.md) — it grows this smallest app into a
  complete one, one component pattern at a time.
- Learn the [component model](components.md) (builders vs. stateful entities).
- Browse the component pages from the [index](readme.md).
- The `gallery` crate (`cargo run -p gallery`) is a live showcase of everything.
