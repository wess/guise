//! The finished app from the motion tutorial:
//! `cargo run -p guise-ui --example checklist`
//!
//! A release checklist that runs itself. Every animation idea in
//! `docs/motiontutorial.md` is in here and nothing else is — the panel slides
//! in, its rows stagger, one playhead drives the whole run, the working row
//! breathes, and the badge at the end animates *out* as well as in.

use gpui::prelude::*;
use gpui::{
  div, px, size, AnimationElement, App, Bounds, Context, Div, Entity, Hsla, IntoElement,
  SharedString, Window, WindowBounds, WindowOptions,
};
use guise::prelude::*;
use guise::theme::{theme, ColorName, Theme};

const STEPS: [&str; 5] = [
  "Run the test suite",
  "Build the universal binary",
  "Notarize and staple",
  "Attach the DMG",
  "Publish the release",
];

/// Milliseconds per step. The whole run is this times `STEPS.len()`.
const PER_STEP: f32 = 900.0;

/// How wide the progress track is, so the bar has something to fill.
const TRACK: f32 = 280.0;

struct Checklist {
  /// One playhead for the whole run. Every row reads it; none of them owns
  /// a clock of its own.
  run: Entity<Animator>,
  /// The "shipped" badge, latched so it can animate out again.
  shipped: Entity<Presence>,
  /// Bumped to replay the one-shot entrances.
  epoch: usize,
}

impl Checklist {
  fn new(cx: &mut Context<Self>) -> Self {
    // Two tracks on one clock: a step counter nothing styles, and the bar
    // that fills. `Custom` is the escape hatch — the frame carries the
    // number and the rows decide what it means.
    let run = cx.new(|cx| {
      Animator::new(
        motion! {
            duration: PER_STEP * STEPS.len() as f32;
            ease: linear;
            custom("step"): 0 => STEPS.len() as i32;
            w: 0 => TRACK;
        },
        cx,
      )
    });

    let shipped = cx.new(|cx| {
      Presence::new(cx)
        .kind(TransitionKind::SlideLeft)
        .duration_ms(220)
        .content(|_window, cx| {
          let color = theme(cx).color(ColorName::Teal, 5).hsla();
          div()
            .px(px(9.0))
            .py(px(3.0))
            .rounded(px(999.0))
            .bg(color)
            .text_size(px(11.0))
            .child("shipped")
            .into_any_element()
        })
    });

    // The badge follows the playhead rather than a timer, so scrubbing
    // backwards takes it away again.
    cx.subscribe(&run, |this: &mut Self, _run, event: &AnimatorEvent, cx| {
      let done = matches!(event, AnimatorEvent::Complete);
      this
        .shipped
        .update(cx, |badge, cx| badge.set_open(done, cx));
    })
    .detach();

    Checklist {
      run,
      shipped,
      epoch: 0,
    }
  }

  /// One row: waiting, working, or done, decided by the shared playhead.
  fn row(
    &self,
    index: usize,
    reached: f32,
    colors: (Hsla, Hsla, Hsla),
    delay: f32,
  ) -> AnimationElement<Div> {
    let (surface, accent, dimmed) = colors;
    let done = reached >= index as f32 + 1.0;
    let working = !done && reached > index as f32;

    let dot = div()
      .w(px(8.0))
      .h(px(8.0))
      .rounded(px(999.0))
      .bg(if done || working { accent } else { dimmed })
      // Only the working row breathes, and only while it is working —
      // an endless clip asks for a frame forever, so it should never
      // outlive the thing it is describing.
      .animate_when(
        working,
        ("pulse", index),
        motion! {
            duration: 700;
            ease: in_out sine;
            repeat: forever;
            alternate;
            opacity: 1 => 0.3;
        },
      );

    div()
      .flex()
      .items_center()
      .gap(px(9.0))
      .px(px(10.0))
      .py(px(7.0))
      .rounded(px(7.0))
      .bg(surface)
      .child(dot)
      .child(SharedString::from(STEPS[index]))
      .when(!done && !working, |el| el.text_color(dimmed))
      // The stagger: same clip, one delay per index.
      .animate(
        ("row", self.epoch * STEPS.len() + index),
        motion! {
            enter: slide_left 18;
            duration: 380;
            ease: out back;
            delay: delay;
        },
      )
  }
}

impl Render for Checklist {
  fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let t = theme(cx);
    let body = t.body().hsla();
    let surface = t.surface().hsla();
    let raised = t.surface_hover().hsla();
    let accent = t.color(ColorName::Teal, 5).hsla();
    let text = t.text().hsla();
    let dimmed = t.dimmed().hsla();

    // One sample, read by everything below. Asking for it here is also
    // what keeps the window repainting while the run is moving.
    let frame = self.run.read(cx).frame(window);
    let reached = frame.number_or(Prop::Custom("step"), 0.0);
    let filled = frame.number_or(Prop::Width, 0.0);
    let playing = self.run.read(cx).is_playing();

    let rise = Stagger::new(70.0).start(120.0);
    let rows = (0..STEPS.len())
      .map(|index| {
        self.row(
          index,
          reached,
          (raised, accent, dimmed),
          rise.at(index, STEPS.len()),
        )
      })
      .collect::<Vec<_>>();

    let bar = div()
      .w(px(TRACK))
      .h(px(6.0))
      .rounded(px(999.0))
      .bg(raised)
      .child(div().h(px(6.0)).rounded(px(999.0)).bg(accent).w(px(filled)));

    let controls = div()
      .flex()
      .gap(px(8.0))
      .items_center()
      .child(
        Button::new("run", if playing { "Pause" } else { "Run" })
          .variant(Variant::Filled)
          .color(ColorName::Teal)
          .on_click(cx.listener(|this, _ev, _window, cx| {
            this.run.update(cx, |run, cx| run.toggle(cx));
          })),
      )
      .child(
        Button::new("rewind", "Rewind")
          .variant(Variant::Light)
          .on_click(cx.listener(|this, _ev, _window, cx| {
            this.run.update(cx, |run, cx| run.stop(cx));
            this
              .shipped
              .update(cx, |badge, cx| badge.set_open(false, cx));
          })),
      )
      .child(
        Button::new("replay", "Replay entrance")
          .variant(Variant::Subtle)
          .on_click(cx.listener(|this, _ev, _window, cx| {
            this.epoch += 1;
            cx.notify();
          })),
      )
      .child(self.shipped.clone());

    // The panel itself: one entrance, on its own box rather than in a
    // wrapper, so turning it on moved nothing.
    let panel = div()
      .w(px(360.0))
      .flex()
      .flex_col()
      .gap(px(10.0))
      .p(px(18.0))
      .rounded(px(12.0))
      .bg(surface)
      .text_color(text)
      .child(Title::new("Release checklist").order(4))
      .children(rows)
      .child(bar)
      .child(controls)
      .animate(
        ("panel", self.epoch),
        motion! {
            enter: slide_up 20;
            duration: 420;
            ease: out back;
        },
      );

    div()
      .size_full()
      .bg(body)
      .flex()
      .items_center()
      .justify_center()
      .child(panel)
  }
}

fn main() {
  gpui_platform::application().run(|cx: &mut App| {
    Theme::dark().init(cx);
    let bounds = Bounds::centered(None, size(px(640.0), px(560.0)), cx);
    cx.open_window(
      WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        ..Default::default()
      },
      |_window, cx| cx.new(Checklist::new),
    )
    .unwrap();
    cx.activate(true);
  });
}
