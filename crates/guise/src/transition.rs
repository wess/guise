//! Mount transitions and `Collapse`, built on gpui's animation API.
//!
//! [`Transition`] plays a one-shot fade/slide as its child appears; [`Collapse`]
//! reveals gated content with a fade. Both wrap the child and drive a
//! `with_animation` pass — the same mechanism [`Loader`](crate::Loader) uses,
//! but non-repeating.
//!
//! gpui has no transform/scale on elements, so motion is expressed through
//! opacity and margin offsets. A true height-collapsing `Collapse` would need a
//! measured content height; this one fades.

use std::time::Duration;

use gpui::prelude::*;
use gpui::{div, px, Animation, AnimationExt, AnyElement, App, ElementId, IntoElement, Window};

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
    duration: u64,
    child: Option<AnyElement>,
}

impl Transition {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Transition {
            id: id.into(),
            kind: TransitionKind::Fade,
            duration: 200,
            child: None,
        }
    }

    pub fn kind(mut self, kind: TransitionKind) -> Self {
        self.kind = kind;
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
        let animation = Animation::new(Duration::from_millis(self.duration));
        div()
            .child(child)
            .with_animation(self.id, animation, move |el, delta| {
                let shift = (1.0 - delta) * 8.0;
                match kind {
                    TransitionKind::Fade => el.opacity(delta),
                    TransitionKind::SlideUp => el.opacity(delta).mt(px(shift)),
                    TransitionKind::SlideDown => el.opacity(delta).mt(px(-shift)),
                    TransitionKind::SlideLeft => el.opacity(delta).ml(px(shift)),
                    TransitionKind::SlideRight => el.opacity(delta).ml(px(-shift)),
                }
            })
    }
}

/// Reveals its child with a fade when `open`, renders nothing when closed.
#[derive(IntoElement)]
pub struct Collapse {
    id: ElementId,
    open: bool,
    duration: u64,
    child: Option<AnyElement>,
}

impl Collapse {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Collapse {
            id: id.into(),
            open: false,
            duration: 180,
            child: None,
        }
    }

    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
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
        if !self.open {
            return div().into_any_element();
        }
        let child = self.child.unwrap_or_else(|| div().into_any_element());
        let animation = Animation::new(Duration::from_millis(self.duration));
        div()
            .child(child)
            .with_animation(self.id, animation, |el, delta| el.opacity(delta))
            .into_any_element()
    }
}
