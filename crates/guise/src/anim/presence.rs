//! `Presence` — mount/unmount with both enter *and* exit animations.
//!
//! Stateless conditionals (`if open { modal }`) can't animate out: the
//! element is gone the frame `open` flips. `Presence` latches it — closing
//! plays the exit animation, then emits [`PresenceEvent::Hidden`] and stops
//! rendering. Wrap any overlay or conditional chunk whose exit should be
//! seen.

use std::rc::Rc;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{div, px, AnimationExt, AnyElement, App, Context, EventEmitter, IntoElement, Window};

use super::Easing;
use crate::transition::TransitionKind;

/// Emitted at the ends of the enter/exit animations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresenceEvent {
    Shown,
    Hidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Closed,
    Open,
    Closing,
}

type ContentBuilder = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement + 'static>;

/// An animated mount/unmount gate. Create with `cx.new(...)`, give it a
/// content builder, drive it with [`Presence::set_open`].
pub struct Presence {
    phase: Phase,
    kind: TransitionKind,
    easing: Easing,
    duration_ms: u64,
    content: Option<ContentBuilder>,
    /// Bumped on every open/close so each run gets a fresh animation id and
    /// stale close timers know to give up.
    epoch: usize,
}

impl EventEmitter<PresenceEvent> for Presence {}

impl Presence {
    pub fn new(_cx: &mut Context<Self>) -> Self {
        Presence {
            phase: Phase::Closed,
            kind: TransitionKind::Fade,
            easing: Easing::default(),
            duration_ms: 180,
            content: None,
            epoch: 0,
        }
    }

    /// The element to gate. Re-invoked every frame while visible, so content
    /// shows live data.
    pub fn content(
        mut self,
        builder: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.content = Some(Rc::new(builder));
        self
    }

    pub fn kind(mut self, kind: TransitionKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }

    pub fn duration_ms(mut self, duration: u64) -> Self {
        self.duration_ms = duration.max(1);
        self
    }

    /// Whether the gate is logically open (still true while closing).
    pub fn is_open(&self) -> bool {
        self.phase != Phase::Closed
    }

    pub fn set_open(&mut self, open: bool, cx: &mut Context<Self>) {
        match (open, self.phase) {
            (true, Phase::Closed) | (true, Phase::Closing) => {
                self.phase = Phase::Open;
                self.epoch += 1;
                cx.emit(PresenceEvent::Shown);
                cx.notify();
            }
            (false, Phase::Open) => {
                self.phase = Phase::Closing;
                self.epoch += 1;
                let epoch = self.epoch;
                let wait = Duration::from_millis(self.duration_ms);
                cx.spawn(async move |this, cx| {
                    cx.background_executor().timer(wait).await;
                    this.update(cx, |presence, cx| {
                        if presence.epoch == epoch && presence.phase == Phase::Closing {
                            presence.phase = Phase::Closed;
                            cx.emit(PresenceEvent::Hidden);
                            cx.notify();
                        }
                    })
                    .ok();
                })
                .detach();
                cx.notify();
            }
            _ => {}
        }
    }

    pub fn toggle(&mut self, cx: &mut Context<Self>) {
        self.set_open(self.phase == Phase::Closed, cx);
    }
}

impl Render for Presence {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.phase == Phase::Closed {
            return div().into_any_element();
        }
        let Some(builder) = self.content.clone() else {
            return div().into_any_element();
        };
        let child = builder(window, cx);
        let kind = self.kind;
        let entering = self.phase == Phase::Open;
        // Linear clock + animator-side curve, so overshooting easings
        // (springs) never pass through gpui's 0..=1-asserted easing slot;
        // see `Transition::render`.
        let easing = self.easing;
        let animation = easing.clock(self.duration_ms);
        let id = (
            if entering {
                "guise-presence-in"
            } else {
                "guise-presence-out"
            },
            self.epoch,
        );

        div()
            .child(child)
            .with_animation(id, animation, move |el, t| {
                // Exit runs the same curve toward zero visibility.
                let delta = easing.apply(t);
                let d = if entering { delta } else { 1.0 - delta };
                let opacity = d.clamp(0.0, 1.0);
                let shift = (1.0 - d) * 8.0;
                match kind {
                    TransitionKind::Fade => el.opacity(opacity),
                    TransitionKind::SlideUp => el.opacity(opacity).mt(px(shift)),
                    TransitionKind::SlideDown => el.opacity(opacity).mt(px(-shift)),
                    TransitionKind::SlideLeft => el.opacity(opacity).ml(px(shift)),
                    TransitionKind::SlideRight => el.opacity(opacity).ml(px(-shift)),
                }
            })
            .into_any_element()
    }
}
