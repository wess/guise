//! `RadioGroup` — a controlled set of mutually-exclusive [`Radio`]s.
//!
//! The parent owns the selected index; the group wires exclusivity and reports
//! the new index through `on_change`. This is the ergonomic layer over the bare
//! `Radio`, which leaves grouping to the caller.

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{div, px, App, IntoElement, SharedString, Window};

use super::Radio;
use crate::theme::{theme, ColorName, Size};

type GroupHandler = Rc<dyn Fn(usize, &mut Window, &mut App) + 'static>;

/// A vertical group of radios with a single selected value.
#[derive(IntoElement)]
pub struct RadioGroup {
    options: Vec<SharedString>,
    value: Option<usize>,
    color: ColorName,
    size: Size,
    label: Option<SharedString>,
    on_change: Option<GroupHandler>,
}

impl RadioGroup {
    pub fn new() -> Self {
        RadioGroup {
            options: Vec::new(),
            value: None,
            color: ColorName::Blue,
            size: Size::Sm,
            label: None,
            on_change: None,
        }
    }

    pub fn options<I, S>(mut self, options: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.options = options.into_iter().map(Into::into).collect();
        self
    }

    /// The currently selected index.
    pub fn value(mut self, value: usize) -> Self {
        self.value = Some(value);
        self
    }

    pub fn color(mut self, color: ColorName) -> Self {
        self.color = color;
        self
    }

    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Called with the newly selected index when a radio is clicked.
    pub fn on_change(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl Default for RadioGroup {
    fn default() -> Self {
        RadioGroup::new()
    }
}

impl RenderOnce for RadioGroup {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let gap = t.spacing(Size::Xs);
        let text = t.text().hsla();
        let font = t.font_size(Size::Sm);

        let mut column = div().flex().flex_col().gap(px(gap));
        if let Some(label) = self.label.clone() {
            column = column.child(div().text_size(px(font)).text_color(text).child(label));
        }

        for (i, option) in self.options.iter().enumerate() {
            let mut radio = Radio::new(("guise-radiogroup", i))
                .label(option.clone())
                .checked(self.value == Some(i))
                .color(self.color)
                .size(self.size);
            if let Some(handler) = self.on_change.clone() {
                radio = radio.on_change(move |_ev, window, cx| handler(i, window, cx));
            }
            column = column.child(radio);
        }
        column
    }
}
