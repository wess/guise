//! `Switch` — a controlled on/off toggle styled as a sliding track.

use gpui::prelude::*;
use gpui::{div, px, App, ClickEvent, ElementId, IntoElement, SharedString, Window};

use super::ClickHandler;
use crate::reactive::Binding;
use crate::theme::{theme, ColorName, Size};

/// A controlled switch. The Mantine `Switch`.
#[derive(IntoElement)]
pub struct Switch {
    id: ElementId,
    checked: bool,
    label: Option<SharedString>,
    size: Size,
    color: ColorName,
    disabled: bool,
    binding: Option<Binding<bool>>,
    on_change: Option<ClickHandler>,
}

impl Switch {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Switch {
            id: id.into(),
            checked: false,
            label: None,
            size: Size::Md,
            color: ColorName::Blue,
            disabled: false,
            binding: None,
            on_change: None,
        }
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    pub fn color(mut self, color: ColorName) -> Self {
        self.color = color;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Two-way bind the on/off state. Overrides `checked`; clicks write the
    /// toggled value back through the binding, then run any `on_change`.
    pub fn bind(mut self, binding: Binding<bool>) -> Self {
        self.binding = Some(binding);
        self
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }

    fn track_height(&self) -> f32 {
        match self.size {
            Size::Xs => 16.0,
            Size::Sm => 20.0,
            Size::Md => 24.0,
            Size::Lg => 30.0,
            Size::Xl => 36.0,
        }
    }
}

impl RenderOnce for Switch {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let height = self.track_height();
        let width = (height * 1.85).round();
        let knob = height - 4.0;
        let accent = t.color(self.color, t.primary_shade());
        let checked = self.binding.as_ref().map_or(self.checked, |b| b.get(cx));

        let track_bg = if checked {
            accent.hsla()
        } else {
            t.color(ColorName::Gray, if t.scheme.is_dark() { 6 } else { 4 })
                .hsla()
        };
        let knob_x = if checked { width - knob - 2.0 } else { 2.0 };

        let track = div()
            .w(px(width))
            .h(px(height))
            .rounded(px(height))
            .bg(track_bg)
            .relative()
            .child(
                div()
                    .absolute()
                    .top(px(2.0))
                    .left(px(knob_x))
                    .w(px(knob))
                    .h(px(knob))
                    .rounded(px(knob))
                    .bg(t.white.hsla()),
            );

        let mut row = div()
            .id(self.id)
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(track);
        if let Some(label) = self.label {
            row = row.child(
                div()
                    .text_size(px(t.font_size(self.size)))
                    .text_color(t.text().hsla())
                    .child(label),
            );
        }

        if self.disabled {
            row.opacity(0.5)
        } else {
            if self.binding.is_some() || self.on_change.is_some() {
                let binding = self.binding;
                let handler = self.on_change;
                let next = !checked;
                row = row.on_click(move |ev, window, cx| {
                    if let Some(binding) = &binding {
                        binding.set(cx, next);
                    }
                    if let Some(handler) = &handler {
                        handler(ev, window, cx);
                    }
                });
            }
            row
        }
    }
}
