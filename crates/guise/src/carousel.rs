//! `Carousel` — a slide deck with arrows, dots, and optional autoplay
//! (gpui entity).
//!
//! Slides are content builders re-invoked every frame (live data, same rule
//! as Tabs panels). Emits [`CarouselEvent`] with the new slide index.

use std::rc::Rc;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{div, px, AnyElement, App, Context, EventEmitter, IntoElement, Window};

use crate::icon::{Icon, IconName};
use crate::theme::{theme, Size};

/// Emitted when the visible slide changes. Carries the slide index.
#[derive(Debug, Clone, Copy)]
pub struct CarouselEvent(pub usize);

type SlideBuilder = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement + 'static>;

/// The slide index after moving by `delta` from `current`, wrapping (or
/// clamping when `wrap` is off).
fn step(current: usize, len: usize, delta: isize, wrap: bool) -> usize {
    if len == 0 {
        return 0;
    }
    let last = len as isize - 1;
    let target = current as isize + delta;
    if wrap {
        target.rem_euclid(len as isize) as usize
    } else {
        target.clamp(0, last) as usize
    }
}

/// A slide deck. Create with `cx.new(|cx| Carousel::new(cx).slide(..).slide(..))`.
pub struct Carousel {
    slides: Vec<SlideBuilder>,
    current: usize,
    wrap: bool,
    height: f32,
    autoplay: Option<Duration>,
    /// Bumped on every slide change so stale autoplay timers give up.
    epoch: usize,
}

impl EventEmitter<CarouselEvent> for Carousel {}

impl Carousel {
    pub fn new(_cx: &mut Context<Self>) -> Self {
        Carousel {
            slides: Vec::new(),
            current: 0,
            wrap: true,
            height: 220.0,
            autoplay: None,
            epoch: 0,
        }
    }

    /// Append a slide (re-invoked every frame while visible).
    pub fn slide<E>(mut self, builder: impl Fn(&mut Window, &mut App) -> E + 'static) -> Self
    where
        E: IntoElement,
    {
        self.slides.push(Rc::new(move |window, cx| {
            builder(window, cx).into_any_element()
        }));
        self
    }

    /// Stop at the ends instead of wrapping.
    pub fn no_wrap(mut self) -> Self {
        self.wrap = false;
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height.max(40.0);
        self
    }

    /// Advance automatically on this interval (pauses forever on any manual
    /// navigation reset — each change reschedules).
    pub fn autoplay(mut self, every: Duration, cx: &mut Context<Self>) -> Self {
        self.autoplay = Some(every);
        self.schedule(cx);
        self
    }

    pub fn current(&self) -> usize {
        self.current
    }

    pub fn go_to(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.slides.len() && index != self.current {
            self.current = index;
            self.changed(cx);
        }
    }

    pub fn next(&mut self, cx: &mut Context<Self>) {
        let target = step(self.current, self.slides.len(), 1, self.wrap);
        if target != self.current {
            self.current = target;
            self.changed(cx);
        }
    }

    pub fn prev(&mut self, cx: &mut Context<Self>) {
        let target = step(self.current, self.slides.len(), -1, self.wrap);
        if target != self.current {
            self.current = target;
            self.changed(cx);
        }
    }

    fn changed(&mut self, cx: &mut Context<Self>) {
        cx.emit(CarouselEvent(self.current));
        self.schedule(cx);
        cx.notify();
    }

    fn schedule(&mut self, cx: &mut Context<Self>) {
        let Some(every) = self.autoplay else { return };
        self.epoch += 1;
        let epoch = self.epoch;
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(every).await;
            this.update(cx, |carousel, cx| {
                if carousel.epoch == epoch {
                    carousel.next(cx);
                }
            })
            .ok();
        })
        .detach();
    }
}

impl Render for Carousel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        let radius = t.radius(t.default_radius);
        let surface = t.surface().hsla();
        let surface_hover = t.surface_hover().hsla();
        let border = t.border().hsla();
        let dimmed = t.dimmed().hsla();
        let accent = t.primary().hsla();

        let count = self.slides.len();
        let content: AnyElement = match self.slides.get(self.current.min(count.saturating_sub(1))) {
            Some(builder) => builder.clone()(window, cx),
            None => div().into_any_element(),
        };

        let mut arrows = Vec::new();
        for (key, icon, forward) in [
            ("guise-carousel-prev", IconName::ChevronLeft, false),
            ("guise-carousel-next", IconName::ChevronRight, true),
        ] {
            arrows.push(
                div()
                    .id(key)
                    .flex()
                    .items_center()
                    .justify_center()
                    .w(px(28.0))
                    .h(px(28.0))
                    .rounded_full()
                    .bg(surface)
                    .border_1()
                    .border_color(border)
                    .text_color(dimmed)
                    .hover(move |s| s.bg(surface_hover))
                    .child(Icon::new(icon).size(Size::Sm))
                    .on_click(cx.listener(move |this, _ev, _window, cx| {
                        if forward {
                            this.next(cx);
                        } else {
                            this.prev(cx);
                        }
                    })),
            );
        }
        let mut arrows = arrows.into_iter();

        let stage = div()
            .relative()
            .w_full()
            .h(px(self.height))
            .rounded(px(radius))
            .border_1()
            .border_color(border)
            .bg(surface)
            .overflow_hidden()
            .child(content)
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(8.0))
                    .child(arrows.next().expect("prev arrow"))
                    .child(arrows.next().expect("next arrow")),
            );

        let mut dots = div().flex().justify_center().gap(px(6.0)).pt(px(8.0));
        for i in 0..count {
            let active = i == self.current;
            dots = dots.child(
                div()
                    .id(("guise-carousel-dot", i))
                    .w(px(if active { 18.0 } else { 8.0 }))
                    .h(px(8.0))
                    .rounded_full()
                    .bg(if active { accent } else { border })
                    .on_click(cx.listener(move |this, _ev, _window, cx| this.go_to(i, cx))),
            );
        }

        div().flex().flex_col().w_full().child(stage).child(dots)
    }
}

#[cfg(test)]
mod tests {
    use super::step;

    #[test]
    fn wrapping_steps_cycle() {
        assert_eq!(step(0, 3, 1, true), 1);
        assert_eq!(step(2, 3, 1, true), 0);
        assert_eq!(step(0, 3, -1, true), 2);
    }

    #[test]
    fn clamped_steps_stop_at_the_edges() {
        assert_eq!(step(2, 3, 1, false), 2);
        assert_eq!(step(0, 3, -1, false), 0);
        assert_eq!(step(1, 3, 1, false), 2);
    }

    #[test]
    fn empty_deck_is_safe() {
        assert_eq!(step(0, 0, 1, true), 0);
        assert_eq!(step(5, 0, -1, false), 0);
    }
}
