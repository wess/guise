//! `Timeline` — a vertical sequence of events with bullets and connectors.

use gpui::prelude::*;
use gpui::{div, px, App, FontWeight, IntoElement, SharedString, Window};

use crate::theme::{theme, ColorName, Size};

struct Item {
    title: SharedString,
    description: Option<SharedString>,
}

/// A vertical timeline. Items up to and including `active` are highlighted.
#[derive(IntoElement)]
pub struct Timeline {
    items: Vec<Item>,
    active: usize,
    color: ColorName,
}

impl Timeline {
    pub fn new() -> Self {
        Timeline {
            items: Vec::new(),
            active: 0,
            color: ColorName::Blue,
        }
    }

    pub fn item(mut self, title: impl Into<SharedString>) -> Self {
        self.items.push(Item {
            title: title.into(),
            description: None,
        });
        self
    }

    pub fn item_desc(
        mut self,
        title: impl Into<SharedString>,
        description: impl Into<SharedString>,
    ) -> Self {
        self.items.push(Item {
            title: title.into(),
            description: Some(description.into()),
        });
        self
    }

    /// Index of the last highlighted item.
    pub fn active(mut self, active: usize) -> Self {
        self.active = active;
        self
    }

    pub fn color(mut self, color: ColorName) -> Self {
        self.color = color;
        self
    }
}

impl Default for Timeline {
    fn default() -> Self {
        Timeline::new()
    }
}

impl RenderOnce for Timeline {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let accent = t.color(self.color, t.primary_shade()).hsla();
        let border = t.border().hsla();
        let surface = t.surface().hsla();
        let text = t.text().hsla();
        let dimmed = t.dimmed().hsla();
        let font = t.font_size(Size::Sm);
        let last = self.items.len().saturating_sub(1);

        let mut column = div().flex().flex_col();
        for (i, item) in self.items.iter().enumerate() {
            let reached = i <= self.active;
            let connector_done = i < self.active;

            let mut rail = div().flex().flex_col().items_center().w(px(20.0));
            let mut dot = div().w(px(14.0)).h(px(14.0)).rounded(px(7.0));
            dot = if reached {
                dot.bg(accent)
            } else {
                dot.bg(surface).border_1().border_color(border)
            };
            rail = rail.child(dot);
            if i != last {
                rail = rail.child(
                    div()
                        .w(px(2.0))
                        .flex_grow(1.0)
                        .min_h(px(12.0))
                        .bg(if connector_done { accent } else { border }),
                );
            }

            let mut content = div()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .pb(px(if i == last { 0.0 } else { 18.0 }))
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_size(px(font))
                        .text_color(text)
                        .child(item.title.clone()),
                );
            if let Some(description) = item.description.clone() {
                content = content.child(
                    div()
                        .text_size(px(t.font_size(Size::Xs)))
                        .text_color(dimmed)
                        .child(description),
                );
            }

            column = column.child(div().flex().gap(px(10.0)).child(rail).child(content));
        }
        column
    }
}
