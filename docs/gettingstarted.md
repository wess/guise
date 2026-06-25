# Getting started

## Requirements

- Rust stable **≥ 1.96** (gpui uses recently-stabilized library features).
- The workspace pins gpui to a specific zed git rev and mirrors zed's
  `[patch.crates-io]` entries, because cargo patches don't propagate through git
  dependencies. See [Architecture](architecture.md) for the full recipe.

## Add the dependency

`guise` lives in this Cargo workspace as `crates/guise`. From another crate in
the workspace:

```toml
[dependencies]
guise-ui = { path = "crates/guise" }   # published as `guise-ui`, imported as `guise`
gpui = { git = "https://github.com/zed-industries/zed", rev = "96285fc1" }
gpui_platform = { git = "https://github.com/zed-industries/zed", rev = "96285fc1", features = ["font-kit", "wayland", "x11"] }
```

Your binary crate needs `gpui_platform` to start the app; library crates that
only build components need `gpui` alone.

## The smallest app

```rust
use gpui::prelude::*;
use gpui::{div, px, size, App, Bounds, Context, IntoElement, Window, WindowBounds, WindowOptions};
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
    gpui_platform::application().run(|cx: &mut App| {
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

- Learn the [component model](components.md) (builders vs. stateful entities).
- Browse the component pages from the [index](readme.md).
- The `gallery` crate (`cargo run -p gallery`) is a live showcase of everything.
