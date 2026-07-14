//! Mount transitions and `Collapse`, built on gpui's animation API.
//!
//! [`Transition`] plays a one-shot fade/slide as its child appears;
//! [`Collapse`] reveals gated content — give it the content height and it
//! animates that height open *and* closed (overflow clipped), falling back
//! to a fade when the height is unknown. Both take an [`Easing`], including
//! springs. For exit animations on arbitrary conditionals, see
//! [`Presence`](crate::anim::Presence).

use gpui::prelude::*;
use gpui::{div, px, AnimationExt, AnyElement, App, ElementId, IntoElement, Window};

use crate::anim::Easing;

/// The kind of entrance motion [`Transition`] plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionKind {
    Fade,
    SlideUp,
    SlideDown,
    SlideLeft,
    SlideRight,
}

/// Plays a one-shot entrance animation around its child.
#[derive(IntoElement)]
pub struct Transition {
    id: ElementId,
    kind: TransitionKind,
    easing: Easing,
    duration: u64,
    child: Option<AnyElement>,
}

impl Transition {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Transition {
            id: id.into(),
            kind: TransitionKind::Fade,
            easing: Easing::default(),
            duration: 200,
            child: None,
        }
    }

    pub fn kind(mut self, kind: TransitionKind) -> Self {
        self.kind = kind;
        self
    }

    /// Timing curve, including `Easing::Spring(..)`.
    pub fn easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }

    pub fn duration_ms(mut self, duration: u64) -> Self {
        self.duration = duration;
        self
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.child = Some(child.into_any_element());
        self
    }
}

impl RenderOnce for Transition {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let child = self.child.unwrap_or_else(|| div().into_any_element());
        let kind = self.kind;
        // Linear clock + animator-side curve: overshooting easings (springs)
        // return deltas past 1.0, which gpui's easing slot debug-asserts
        // against but the animator accepts — margins overshoot and settle,
        // opacity clamps to its legal range.
        let easing = self.easing;
        div()
            .child(child)
            .with_animation(self.id, easing.clock(self.duration), move |el, t| {
                let delta = easing.apply(t);
                let opacity = delta.clamp(0.0, 1.0);
                let shift = (1.0 - delta) * 8.0;
                match kind {
                    TransitionKind::Fade => el.opacity(opacity),
                    TransitionKind::SlideUp => el.opacity(opacity).mt(px(shift)),
                    TransitionKind::SlideDown => el.opacity(opacity).mt(px(-shift)),
                    TransitionKind::SlideLeft => el.opacity(opacity).ml(px(shift)),
                    TransitionKind::SlideRight => el.opacity(opacity).ml(px(-shift)),
                }
            })
    }
}

/// Reveals gated content. With a known content `height`, the box height
/// animates open and closed (a real collapse, clipped while moving); without
/// one it fades in and unmounts instantly on close.
#[derive(IntoElement)]
pub struct Collapse {
    id: ElementId,
    open: bool,
    height: Option<f32>,
    easing: Easing,
    duration: u64,
    child: Option<AnyElement>,
}

impl Collapse {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Collapse {
            id: id.into(),
            open: false,
            height: None,
            easing: Easing::default(),
            duration: 180,
            child: None,
        }
    }

    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// The content's height in px. Unlocks the true height animation — the
    /// closed state keeps the child mounted at height 0 so it can animate
    /// back open.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height.max(0.0));
        self
    }

    pub fn easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }

    pub fn duration_ms(mut self, duration: u64) -> Self {
        self.duration = duration;
        self
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.child = Some(child.into_any_element());
        self
    }
}

impl RenderOnce for Collapse {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        // Linear clock + animator-side curve; see `Transition::render`.
        let easing = self.easing;
        let animation = easing.clock(self.duration);

        let Some(height) = self.height else {
            // No measured height: fade in on open, vanish on close.
            if !self.open {
                return div().into_any_element();
            }
            let child = self.child.unwrap_or_else(|| div().into_any_element());
            return div()
                .child(child)
                .with_animation(self.id, animation, move |el, t| {
                    el.opacity(easing.apply(t).clamp(0.0, 1.0))
                })
                .into_any_element();
        };

        let child = self.child.unwrap_or_else(|| div().into_any_element());
        let open = self.open;
        // Swapping the animation id replays the animation: one id per
        // direction gives a real two-way collapse from stateless renders.
        let direction = if open {
            "guise-collapse-open"
        } else {
            "guise-collapse-close"
        };
        div()
            .id(self.id)
            .overflow_hidden()
            .child(child)
            .with_animation(direction, animation, move |el, t| {
                let d = if open {
                    easing.apply(t)
                } else {
                    1.0 - easing.apply(t)
                };
                // A springy open overshoots the height and settles back;
                // opacity and the closing height stay in legal range.
                el.h(px(height * d.max(0.0))).opacity(d.clamp(0.0, 1.0))
            })
            .into_any_element()
    }
}
