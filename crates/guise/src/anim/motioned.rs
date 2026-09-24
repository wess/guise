//! `Motioned::animate` — a clip on an element's own style.
//!
//! [`Animated`](super::Animated) wraps its child in a `div`, which is right
//! when the child isn't styleable but wrong when the animated thing has a
//! layout contract with its parent: a wrapper is a new flex item, and a
//! `w_full` child suddenly measures against it instead of the row it was in.
//!
//! For anything that is `Styled` — a `div`, a container, the box a component
//! already sits in — this puts the sampled values on that element directly
//! and adds nothing to the tree.

use std::time::Duration;

use gpui::prelude::*;
use gpui::{Animation, AnimationElement, AnimationExt, ElementId, SharedString};

use super::Clip;

pub trait Motioned: IntoElement + Styled + Sized + 'static {
  /// Play a clip once, from the moment this element first lays out.
  ///
  /// Changing the id replays it — that is the only way to restart a
  /// mounted one-shot, and what a "preview" button hands you.
  fn animate(self, id: impl Into<ElementId>, clip: impl Into<Clip>) -> AnimationElement<Self> {
    let clip = clip.into();
    let total = clip.total_ms();

    // An endless clip has no duration to give gpui, so it runs as a
    // repeating animation over one pass — two when it alternates, so the
    // there-and-back parity survives the wrap.
    let (span, repeats) = if total.is_finite() {
      // Zero would divide by zero inside gpui's animation element; a
      // millisecond lands on the final frame immediately, which is what
      // an empty clip should look like anyway.
      (total.max(1.0), false)
    } else {
      let passes = if clip.alternates() { 2.0 } else { 1.0 };
      ((clip.iteration_ms() * passes).max(1.0), true)
    };

    let mut animation = Animation::new(Duration::from_secs_f32(span / 1000.0));
    if repeats {
      animation = animation.repeat();
    }
    // The clock stays linear and the clip does its own easing: gpui
    // debug-asserts the easing slot into 0..=1, and half these curves
    // overshoot on purpose.
    self.with_animation(id, animation, move |el, t| clip.sample(t * span).apply(el))
  }

  /// Play the clip only while `condition` holds.
  ///
  /// `.when(cond, |el| el.animate(..))` cannot work: `animate` changes the
  /// element's type and `when` has to hand back the type it was given. This
  /// keeps the type stable by running an empty clip when the condition is
  /// false, which samples nothing and sets nothing.
  ///
  /// The two states get different element ids, so a clip starts from its
  /// beginning every time the condition turns on rather than resuming a
  /// clock that ran while nobody was looking.
  fn animate_when(
    self,
    condition: bool,
    id: impl Into<ElementId>,
    clip: impl Into<Clip>,
  ) -> AnimationElement<Self> {
    let id = ElementId::NamedChild(
      Box::new(id.into()).into(),
      SharedString::new_static(if condition { "on" } else { "off" }),
    );
    let clip = if condition {
      clip.into()
    } else {
      Clip::default()
    };
    self.animate(id, clip)
  }
}

impl<E: IntoElement + Styled + 'static> Motioned for E {}
