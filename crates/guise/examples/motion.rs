//! The animation system, end to end:
//! `cargo run -p guise-ui --example motion`
//!
//! Four things in one window — a keyframed one-shot, a staggered list, a
//! sequence on a playhead you control, and the exact shape Tailor generates
//! for a node with an entrance on it.

use gpui::prelude::*;
use gpui::{
  div, px, size, App, Bounds, Context, Entity, IntoElement, SharedString, Window, WindowBounds,
  WindowOptions,
};
use guise::prelude::*;
use guise::theme::{theme, ColorName, Theme};

struct Demo {
  player: Entity<Animator>,
  /// Bumped to hand the one-shots fresh element ids, which is what replays
  /// them: a mounted animation has already run.
  epoch: usize,
}

impl Demo {
  fn new(cx: &mut Context<Self>) -> Self {
    // Slide out, drop and round off, then come back — one clip, three
    // motions, the third after a beat of stillness.
    let orbit = sequence![
        motion! {
            duration: 620;
            ease: out cubic;
            x: 0 => 220;
        },
        motion! {
            duration: 520;
            ease: out elastic;
            y: 0 => 30;
            radius: 10 => 28;
        },
        rel(140) => motion! {
            duration: 720;
            ease: in_out sine;
            x: 220 => 0;
            y: 30 => 0;
            radius: 28 => 10;
        },
    ]
    .repeat_forever();

    Demo {
      player: cx.new(|cx| Animator::new(orbit, cx)),
      epoch: 0,
    }
  }
}

impl Render for Demo {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let t = theme(cx);
    let surface = t.surface().hsla();
    let stage_bg = t.surface_hover().hsla();
    let accent = t.color(ColorName::Blue, 5).hsla();
    let soft = t.color(ColorName::Grape, 5).hsla();
    let text = t.text().hsla();
    let epoch = self.epoch;
    let playing = self.player.read(cx).is_playing();

    // keyframes: three legs, the middle one held longer
    let swatch = Animated::new(("swatch", epoch))
      .motion(motion! {
          duration: 1100;
          ease: in_out quad;
          bg: soft => [
              Keyframe::to(accent).duration(500.0),
              Keyframe::to(soft).ease(Easing::Out(Curve::Expo)),
          ];
          radius: 6 => [30, 6];
      })
      .child(div().w(px(56.0)).h(px(56.0)));

    // stagger: one clip per row, offset by index
    let rows = ["Keyframes", "Springs", "Stagger", "Sequences", "Playheads"];
    let rise = Stagger::new(60.0).from(StaggerFrom::First);
    let list = div().flex().flex_col().gap(px(6.0)).children(
      rows
        .iter()
        .enumerate()
        .map(|(index, row)| {
          div()
            .px(px(10.0))
            .py(px(6.0))
            .rounded(px(6.0))
            .bg(stage_bg)
            .text_color(text)
            .child(SharedString::from(*row))
            // On the element's own box: no wrapper, no change to
            // how the row sits in the column.
            .animate(
              ("row", epoch * rows.len() + index),
              motion! {
                  enter: slide_left 24;
                  duration: 420;
                  ease: out back;
                  delay: rise.at(index, rows.len());
              },
            )
        })
        .collect::<Vec<_>>(),
    );

    // the playhead
    let stage = div()
      .w_full()
      .h(px(120.0))
      .p(px(12.0))
      .rounded(px(10.0))
      .bg(stage_bg)
      .overflow_hidden()
      .child(
        Animated::new("stage")
          .animator(&self.player)
          .child(div().w(px(56.0)).h(px(56.0)).bg(accent)),
      );

    let controls = div()
      .flex()
      .gap(px(8.0))
      .child(
        Button::new("play", if playing { "Pause" } else { "Play" })
          .variant(Variant::Light)
          .on_click(cx.listener(|this, _ev, _window, cx| {
            this.player.update(cx, |player, cx| player.toggle(cx));
          })),
      )
      .child(
        Button::new("reverse", "Reverse")
          .variant(Variant::Light)
          .on_click(cx.listener(|this, _ev, _window, cx| {
            this.player.update(cx, |player, cx| player.reverse(cx));
          })),
      )
      .child(
        Button::new("half", "0.5x")
          .variant(Variant::Subtle)
          .on_click(cx.listener(|this, _ev, _window, cx| {
            this
              .player
              .update(cx, |player, cx| player.set_speed(0.5, cx));
          })),
      )
      .child(
        Button::new("replay", "Replay one-shots")
          .variant(Variant::Subtle)
          .on_click(cx.listener(|this, _ev, _window, cx| {
            this.epoch += 1;
            cx.notify();
          })),
      );

    div()
      .size_full()
      .bg(surface)
      .text_color(text)
      .flex()
      .flex_col()
      .gap(px(16.0))
      .p(px(24.0))
      .child(Title::new("Motion").order(3))
      .child(
        div()
          .flex()
          .gap(px(24.0))
          .items_center()
          .child(swatch)
          .child(list),
      )
      .child(stage)
      .child(controls)
  }
}

fn main() {
  gpui::Application::new().run(|cx: &mut App| {
    Theme::dark().init(cx);
    let bounds = Bounds::centered(None, size(px(720.0), px(560.0)), cx);
    cx.open_window(
      WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        ..Default::default()
      },
      |_window, cx| cx.new(Demo::new),
    )
    .unwrap();
    cx.activate(true);
  });
}
