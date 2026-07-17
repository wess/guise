//! Live-preview markdown editor demo:
//! `cargo run -p guise-ui --example markdown`

use gpui::prelude::*;
use gpui::{
    div, px, size, App, Bounds, Context, Entity, IntoElement, Window, WindowBounds, WindowOptions,
};
use guise::markdown::{MarkdownEditor, MarkdownEditorEvent};
use guise::theme::Theme;

const SAMPLE: &str = r#"# Meeting notes

A **bold** plan with *emphasis*, `inline code`, ~~dropped ideas~~, and a
==highlight== for the important part. See [the docs](https://example.com)
or [[Roadmap|the roadmap page]].

## Todo

- [x] draft the outline
- [ ] review with the team
    - [ ] schedule the call
- regular bullet item

1. first step
2. second step

> Simplicity is the ultimate sophistication.
> > and quotes nest.

```rust
fn main() {
    println!("fenced code, highlighted");
}
```

---

That's the whole demo. Click a checkbox, Cmd+click a link, and move the
caret around to watch lines reveal their markdown.
"#;

struct Demo {
    editor: Entity<MarkdownEditor>,
}

impl Demo {
    fn new(cx: &mut Context<Self>) -> Self {
        let editor = cx.new(|cx| MarkdownEditor::new(cx).value(SAMPLE));
        cx.subscribe(
            &editor,
            |_this, _editor, event: &MarkdownEditorEvent, _cx| {
                if let MarkdownEditorEvent::LinkClick(target) = event {
                    println!("link: {target}");
                }
            },
        )
        .detach();
        Demo { editor }
    }
}

impl Render for Demo {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = cx.global::<Theme>();
        div()
            .size_full()
            .bg(t.body().hsla())
            .text_color(t.text().hsla())
            .p(px(24.0))
            .child(self.editor.clone())
    }
}

fn main() {
    gpui::Application::new().run(|cx: &mut App| {
        Theme::dark().init(cx);
        let bounds = Bounds::centered(None, size(px(760.0), px(820.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_window, cx| cx.new(Demo::new),
        )
        .expect("open window");
        cx.activate(true);
    });
}
