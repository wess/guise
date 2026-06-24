//! `SegmentedControl` — a stateful single-choice segmented switch (gpui entity).

use gpui::prelude::*;
use gpui::{div, px, Context, EventEmitter, IntoElement, SharedString, Window};

use super::control_metrics;
use crate::theme::{theme, ColorName, Size};

/// Emitted when the selected segment changes. Carries the option index.
#[derive(Debug, Clone)]
pub struct SegmentedControlEvent(pub usize);

/// A segmented control. Create with
/// `cx.new(|cx| SegmentedControl::new(cx).data(["Day", "Week", "Month"]))`.
pub struct SegmentedControl {
    options: Vec<SharedString>,
    selected: usize,
    size: Size,
}

impl EventEmitter<SegmentedControlEvent> for SegmentedControl {}

impl SegmentedControl {
    pub fn new(_cx: &mut Context<Self>) -> Self {
        SegmentedControl {
            options: Vec::new(),
            selected: 0,
            size: Size::Sm,
        }
    }

    pub fn data<I, S>(mut self, options: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.options = options.into_iter().map(Into::into).collect();
        self
    }

    pub fn selected(mut self, index: usize) -> Self {
        self.selected = index;
        self
    }

    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    pub fn selected_index(&self) -> usize {
        self.selected
    }
}

impl Render for SegmentedControl {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        let (height, pad_x, font) = control_metrics(self.size);
        let radius = t.radius(Size::Sm);
        let track = t.color(ColorName::Gray, if t.scheme.is_dark() { 8 } else { 1 }).hsla();
        let active_bg = t.surface().hsla();
        let active_fg = t.text().hsla();
        let inactive_fg = t.dimmed().hsla();

        let count = self.options.len();
        let selected = if count == 0 { 0 } else { self.selected.min(count - 1) };

        let mut row = div()
            .flex()
            .items_center()
            .gap(px(2.0))
            .p(px(3.0))
            .rounded(px(radius + 2.0))
            .bg(track);

        for (i, option) in self.options.iter().enumerate() {
            let is_active = i == selected;
            let mut seg = div()
                .id(("guise-segment", i))
                .flex()
                .items_center()
                .justify_center()
                .h(px(height - 6.0))
                .px(px(pad_x))
                .rounded(px(radius))
                .text_size(px(font))
                .text_color(if is_active { active_fg } else { inactive_fg })
                .child(option.clone())
                .on_click(cx.listener(move |this, _ev, _window, cx| {
                    this.selected = i;
                    cx.emit(SegmentedControlEvent(i));
                    cx.notify();
                }));
            if is_active {
                seg = seg.bg(active_bg).shadow_sm();
            }
            row = row.child(seg);
        }
        row
    }
}
