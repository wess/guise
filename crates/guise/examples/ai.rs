//! A working chat UI in one file: `cargo run -p guise-ui --example ai`
//!
//! There is no model behind it. The "transport" is a background task that
//! dribbles a canned reply back a few characters at a time — which is exactly
//! the shape a real one has, and enough to exercise the streaming caret, the
//! stop button, and stick-to-bottom scrolling. Swap `send` for your own client
//! and the rest is unchanged.

use std::time::Duration;

use gpui::{
  div, prelude::*, px, size, App, Bounds, Context, Entity, Task, Window, WindowBounds,
  WindowOptions,
};
use guise::prelude::*;
use guise::theme::{Size, Theme};

/// What the pretend model says, whatever you ask it.
const REPLY: &str = "Here's the thing about **streaming**: the transcript is \
just a `String` you append to.\n\n\
1. `begin_reply` opens a turn.\n\
2. `push_delta` appends to it.\n\
3. `end_reply` closes it — or `fail_reply` if the connection dropped.\n\n\
```rust\nchat.update(cx, |chat, cx| chat.push_delta(&token, cx));\n```\n\n\
Everything else — the caret, the scroll, the markdown — follows from that.";

/// How fast the fake transport dribbles the reply back.
const TICK: Duration = Duration::from_millis(18);
const CHUNK: usize = 3;

struct Demo {
  chat: Entity<AIChatView>,
  composer: Entity<AIComposer>,
  model: Entity<AIModelPicker>,
  usage: AIUsage,
  /// The in-flight reply. Dropping it cancels the stream, which is all the
  /// stop button has to do.
  stream: Option<Task<()>>,
}

impl Demo {
  fn new(cx: &mut Context<Self>) -> Self {
    let chat = cx.new(|cx| {
      AIChatView::new(cx)
        .max_width(680.0)
        .empty_message("Ask anything — the reply is canned, the plumbing isn't.")
    });
    let composer = cx.new(|cx| {
      AIComposer::new(cx)
        .attachments(true)
        .hint("Enter sends \u{00b7} Shift+Enter for a new line")
    });
    cx.subscribe(
      &composer,
      |this, _composer, event: &AIComposerEvent, cx| match event {
        AIComposerEvent::Submit(text) => this.send(text.clone(), cx),
        AIComposerEvent::Stop => this.stop(cx),
        AIComposerEvent::Attach | AIComposerEvent::Change(_) => {}
      },
    )
    .detach();

    let model = cx.new(|cx| {
      AIModelPicker::new(cx)
        .label("Model")
        .models([
          AIModel::new("claude-opus-5", "Opus 5")
            .description("Deepest reasoning")
            .context(200_000)
            .pricing(AIPricing::new(15.0, 75.0)),
          AIModel::new("claude-sonnet-5", "Sonnet 5")
            .description("Balanced speed and depth")
            .context(200_000)
            .pricing(AIPricing::new(3.0, 15.0)),
        ])
        .selected_id("claude-sonnet-5")
    });

    Demo {
      chat,
      composer,
      model,
      usage: AIUsage::default(),
      stream: None,
    }
  }

  /// Push the prompt, open a reply, and start feeding it. A real client would
  /// spawn its request here instead.
  fn send(&mut self, prompt: String, cx: &mut Context<Self>) {
    self.chat.update(cx, |chat, cx| {
      chat.push(AITurn::user(prompt.clone()), cx);
      chat.set_pending(Some("Thinking"), cx);
    });
    self
      .composer
      .update(cx, |composer, cx| composer.set_busy(true, cx));
    self.usage = self.usage + AIUsage::new(prompt.len() as u64 / 4, 0);

    let chat = self.chat.clone();
    let composer = self.composer.clone();
    self.stream = Some(cx.spawn(async move |this, cx| {
      cx.background_executor().timer(TICK * 24).await;
      chat.update(cx, |chat, cx| {
        chat.set_pending(None::<&str>, cx);
        chat.begin_reply(cx);
      });
      // Chunked by bytes, on char boundaries: a delta that split a
      // multi-byte character would panic on the join.
      let mut at = 0;
      while at < REPLY.len() {
        let mut end = (at + CHUNK).min(REPLY.len());
        while !REPLY.is_char_boundary(end) {
          end += 1;
        }
        let chunk = &REPLY[at..end];
        at = end;
        chat.update(cx, |chat, cx| chat.push_delta(chunk, cx));
        cx.background_executor().timer(TICK).await;
      }
      chat.update(cx, |chat, cx| chat.end_reply(cx));
      composer.update(cx, |composer, cx| composer.set_busy(false, cx));
      let _ = this.update(cx, |this, cx| {
        this.usage = this.usage + AIUsage::new(0, REPLY.len() as u64 / 4);
        this.stream = None;
        cx.notify();
      });
    }));
  }

  /// Stop is just "drop the task" — plus closing the turn so it stops
  /// claiming to be streaming.
  fn stop(&mut self, cx: &mut Context<Self>) {
    self.stream = None;
    self.chat.update(cx, |chat, cx| {
      chat.set_pending(None::<&str>, cx);
      chat.end_reply(cx);
    });
    self
      .composer
      .update(cx, |composer, cx| composer.set_busy(false, cx));
    cx.notify();
  }
}

impl Render for Demo {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let t = cx.global::<Theme>();
    let body = t.body().hsla();
    let text = t.text().hsla();
    let border = t.border().hsla();

    let model = self.model.read(cx).selection();
    let pricing = model
      .and_then(|m| m.pricing)
      .unwrap_or(AIPricing::new(3.0, 15.0));
    let context = model.map(|m| m.context).unwrap_or(200_000);

    let sidebar = div()
      .flex()
      .flex_col()
      .gap(px(14.0))
      .w(px(280.0))
      .flex_none()
      .child(self.model.clone())
      .child(AITokenMeter::new(self.usage.total(), context).label("Context"))
      .child(
        AICost::new(self.usage, pricing)
          .label("Session")
          .breakdown(true),
      );

    let conversation = div()
      .flex()
      .flex_col()
      .gap(px(10.0))
      .flex_1()
      .min_w(px(0.0))
      .child(
        div()
          .flex_1()
          .min_h(px(0.0))
          .rounded(px(8.0))
          .border_1()
          .border_color(border)
          .child(self.chat.clone()),
      )
      .child(self.composer.clone());

    div()
      .flex()
      .flex_row()
      .gap(px(24.0))
      .p(px(20.0))
      .size_full()
      .bg(body)
      .text_color(text)
      .text_size(px(t.font_size(Size::Sm)))
      .child(conversation)
      .child(sidebar)
  }
}

fn main() {
  gpui_platform::application().run(|cx: &mut App| {
    Theme::light().init(cx);
    let bounds = Bounds::centered(None, size(px(1060.0), px(700.0)), cx);
    cx.open_window(
      WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        ..Default::default()
      },
      |window, cx| {
        let demo = cx.new(Demo::new);
        let focus = demo.read(cx).composer.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
        demo
      },
    )
    .unwrap();
    cx.activate(true);
  });
}
