//! `Tour` — a step-by-step onboarding overlay (gpui entity).
//!
//! A sequence of titled steps shown as a centered card over a scrim, with
//! Back/Next/Skip and progress dots. Emits [`TourEvent`] as the user moves
//! through it. Anchoring to specific UI elements is left to the host (pair
//! a step's text with highlighting in your own chrome if needed).

use gpui::prelude::*;
use gpui::{
    deferred, div, px, Context, EventEmitter, FocusHandle, FontWeight, IntoElement, SharedString,
    Window,
};

use crate::theme::{theme, Size};

/// Tour progress events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TourEvent {
    /// Moved to this step index.
    Step(usize),
    /// Finished the last step.
    Finished,
    /// Dismissed early.
    Skipped,
}

struct TourStep {
    title: SharedString,
    body: SharedString,
}

/// An onboarding walkthrough. Create with
/// `cx.new(|cx| Tour::new(cx).step("Welcome", "…").step("Panels", "…"))`,
/// then `tour.update(cx, |t, cx| t.start(cx))`.
pub struct Tour {
    steps: Vec<TourStep>,
    current: usize,
    open: bool,
    focus: FocusHandle,
}

impl EventEmitter<TourEvent> for Tour {}

impl Tour {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Tour {
            steps: Vec::new(),
            current: 0,
            open: false,
            focus: cx.focus_handle(),
        }
    }

    pub fn step(mut self, title: impl Into<SharedString>, body: impl Into<SharedString>) -> Self {
        self.steps.push(TourStep {
            title: title.into(),
            body: body.into(),
        });
        self
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn current(&self) -> usize {
        self.current
    }

    /// Show the tour from the first step.
    pub fn start(&mut self, cx: &mut Context<Self>) {
        if !self.steps.is_empty() {
            self.current = 0;
            self.open = true;
            cx.emit(TourEvent::Step(0));
            cx.notify();
        }
    }

    pub fn next(&mut self, cx: &mut Context<Self>) {
        if self.current + 1 < self.steps.len() {
            self.current += 1;
            cx.emit(TourEvent::Step(self.current));
        } else {
            self.open = false;
            cx.emit(TourEvent::Finished);
        }
        cx.notify();
    }

    pub fn back(&mut self, cx: &mut Context<Self>) {
        if self.current > 0 {
            self.current -= 1;
            cx.emit(TourEvent::Step(self.current));
            cx.notify();
        }
    }

    pub fn skip(&mut self, cx: &mut Context<Self>) {
        if self.open {
            self.open = false;
            cx.emit(TourEvent::Skipped);
            cx.notify();
        }
    }
}

impl Render for Tour {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.open || self.steps.is_empty() {
            return div().into_any_element();
        }

        let t = theme(cx);
        let radius = t.radius(Size::Md);
        let surface = t.surface().hsla();
        let surface_hover = t.surface_hover().hsla();
        let border = t.border().hsla();
        let text_color = t.text().hsla();
        let dimmed = t.dimmed().hsla();
        let accent = t.primary();
        let accent_bg = accent.hsla();
        let accent_fg = accent.contrasting().hsla();
        let scrim = t.black.alpha(0.55);
        let font = t.font_size(Size::Sm);

        let step = &self.steps[self.current];
        let last = self.current + 1 == self.steps.len();
        let viewport = window.viewport_size();

        let mut dots = div().flex().gap(px(5.0));
        for i in 0..self.steps.len() {
            dots = dots.child(
                div()
                    .w(px(7.0))
                    .h(px(7.0))
                    .rounded_full()
                    .bg(if i == self.current { accent_bg } else { border }),
            );
        }

        let button = |id: &'static str, label: &'static str, filled: bool| {
            let mut b = div()
                .id(id)
                .px(px(12.0))
                .py(px(5.0))
                .rounded(px(t.radius(Size::Sm)))
                .text_size(px(font));
            if filled {
                b = b.bg(accent_bg).text_color(accent_fg);
            } else {
                b = b
                    .border_1()
                    .border_color(border)
                    .text_color(text_color)
                    .hover(move |s| s.bg(surface_hover));
            }
            b.child(SharedString::new_static(label))
        };

        let mut controls = div().flex().items_center().justify_between().pt(px(4.0));
        controls = controls.child(
            div()
                .id("guise-tour-skip")
                .text_size(px(font))
                .text_color(dimmed)
                .hover(move |s| s.text_color(text_color))
                .child(SharedString::new_static("Skip"))
                .on_click(cx.listener(|this, _ev, _window, cx| this.skip(cx))),
        );
        let mut actions = div().flex().gap(px(8.0));
        if self.current > 0 {
            actions = actions.child(
                button("guise-tour-back", "Back", false)
                    .on_click(cx.listener(|this, _ev, _window, cx| this.back(cx))),
            );
        }
        actions = actions.child(
            button("guise-tour-next", if last { "Finish" } else { "Next" }, true)
                .on_click(cx.listener(|this, _ev, _window, cx| this.next(cx))),
        );
        controls = controls.child(actions);

        let card = div()
            .id("guise-tour-card")
            .occlude()
            .flex()
            .flex_col()
            .gap(px(10.0))
            .w(px(360.0))
            .p(px(t.spacing(Size::Md)))
            .rounded(px(radius))
            .bg(surface)
            .border_1()
            .border_color(border)
            .shadow_xl()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .font_weight(FontWeight::BOLD)
                            .text_color(text_color)
                            .child(step.title.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(font - 1.0))
                            .text_color(dimmed)
                            .child(SharedString::from(format!(
                                "{} / {}",
                                self.current + 1,
                                self.steps.len()
                            ))),
                    ),
            )
            .child(
                div()
                    .text_size(px(font))
                    .text_color(dimmed)
                    .child(step.body.clone()),
            )
            .child(dots)
            .child(controls);

        let backdrop = div()
            .id("guise-tour-backdrop")
            .occlude()
            .absolute()
            .top(px(0.0))
            .left(px(0.0))
            .w(viewport.width)
            .h(viewport.height)
            .flex()
            .items_center()
            .justify_center()
            .bg(scrim)
            .track_focus(&self.focus)
            .child(card);

        deferred(backdrop).into_any_element()
    }
}
