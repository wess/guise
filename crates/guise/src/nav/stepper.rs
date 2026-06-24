//! `Stepper` — a horizontal progress indicator across ordered steps.

use gpui::prelude::*;
use gpui::{div, px, App, FontWeight, IntoElement, SharedString, Window};

use crate::theme::{theme, ColorName, Size};

struct StepDef {
    label: SharedString,
    description: Option<SharedString>,
}

/// A horizontal stepper. The Mantine `Stepper`. `active` is the current step
/// index; earlier steps render as completed.
#[derive(IntoElement)]
pub struct Stepper {
    steps: Vec<StepDef>,
    active: usize,
    color: ColorName,
}

impl Stepper {
    pub fn new() -> Self {
        Stepper {
            steps: Vec::new(),
            active: 0,
            color: ColorName::Blue,
        }
    }

    pub fn step(mut self, label: impl Into<SharedString>) -> Self {
        self.steps.push(StepDef {
            label: label.into(),
            description: None,
        });
        self
    }

    pub fn step_desc(
        mut self,
        label: impl Into<SharedString>,
        description: impl Into<SharedString>,
    ) -> Self {
        self.steps.push(StepDef {
            label: label.into(),
            description: Some(description.into()),
        });
        self
    }

    pub fn active(mut self, active: usize) -> Self {
        self.active = active;
        self
    }

    pub fn color(mut self, color: ColorName) -> Self {
        self.color = color;
        self
    }
}

impl Default for Stepper {
    fn default() -> Self {
        Stepper::new()
    }
}

impl RenderOnce for Stepper {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let accent = t.color(self.color, t.primary_shade());
        let accent_hsla = accent.hsla();
        let on_accent = accent.contrasting().hsla();
        let line = t.border().hsla();
        let text = t.text().hsla();
        let dimmed = t.dimmed().hsla();
        let surface = t.surface().hsla();
        let font = t.font_size(Size::Sm);
        let font_xs = t.font_size(Size::Xs);
        let active = self.active;
        let count = self.steps.len();

        let mut row = div().flex().items_center().gap(px(8.0));
        for (i, step) in self.steps.into_iter().enumerate() {
            let completed = i < active;
            let is_active = i == active;
            let filled = completed || is_active;

            let circle = {
                let base = div()
                    .w(px(30.0))
                    .h(px(30.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(30.0))
                    .text_size(px(font))
                    .font_weight(FontWeight::SEMIBOLD);
                if filled {
                    let label = if completed {
                        SharedString::new_static("\u{2713}")
                    } else {
                        SharedString::from((i + 1).to_string())
                    };
                    base.bg(accent_hsla).text_color(on_accent).child(label)
                } else {
                    base.bg(surface)
                        .border_1()
                        .border_color(line)
                        .text_color(dimmed)
                        .child(SharedString::from((i + 1).to_string()))
                }
            };

            let mut labels = div().flex().flex_col().gap(px(1.0)).child(
                div()
                    .text_size(px(font))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(if filled { text } else { dimmed })
                    .child(step.label),
            );
            if let Some(description) = step.description {
                labels = labels.child(
                    div()
                        .text_size(px(font_xs))
                        .text_color(dimmed)
                        .child(description),
                );
            }

            row = row.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(circle)
                    .child(labels),
            );

            if i + 1 < count {
                row = row.child(
                    div()
                        .flex_1()
                        .min_w(px(24.0))
                        .h(px(2.0))
                        .rounded(px(1.0))
                        .bg(if completed { accent_hsla } else { line }),
                );
            }
        }
        row
    }
}
