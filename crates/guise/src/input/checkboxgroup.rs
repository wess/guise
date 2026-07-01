//! `CheckboxGroup` — a controlled set of [`Checkbox`]es over a shared value.
//!
//! The parent owns the selected indices (a sorted `Vec<usize>`); each toggle
//! reports the *next* full selection through `on_change`.

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{div, px, App, IntoElement, SharedString, Window};

use super::Checkbox;
use crate::reactive::Binding;
use crate::theme::{theme, ColorName, Size};

type GroupHandler = Rc<dyn Fn(Vec<usize>, &mut Window, &mut App) + 'static>;

/// A vertical group of checkboxes sharing one selection set.
#[derive(IntoElement)]
pub struct CheckboxGroup {
    options: Vec<SharedString>,
    value: Vec<usize>,
    color: ColorName,
    size: Size,
    label: Option<SharedString>,
    binding: Option<Binding<Vec<usize>>>,
    on_change: Option<GroupHandler>,
}

impl CheckboxGroup {
    pub fn new() -> Self {
        CheckboxGroup {
            options: Vec::new(),
            value: Vec::new(),
            color: ColorName::Blue,
            size: Size::Sm,
            label: None,
            binding: None,
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

    /// The currently selected indices.
    pub fn value(mut self, value: impl IntoIterator<Item = usize>) -> Self {
        self.value = value.into_iter().collect();
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

    /// Two-way bind the selection set. Overrides `value`; each toggle writes
    /// the full next selection back through the binding, then runs any
    /// `on_change`.
    pub fn bind(mut self, binding: Binding<Vec<usize>>) -> Self {
        self.binding = Some(binding);
        self
    }

    /// Called with the full next selection (sorted) when any box is toggled.
    pub fn on_change(
        mut self,
        handler: impl Fn(Vec<usize>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl Default for CheckboxGroup {
    fn default() -> Self {
        CheckboxGroup::new()
    }
}

impl RenderOnce for CheckboxGroup {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let gap = t.spacing(Size::Xs);
        let text = t.text().hsla();
        let font = t.font_size(Size::Sm);
        let value = self
            .binding
            .as_ref()
            .map_or_else(|| self.value.clone(), |b| b.get(cx));

        let mut column = div().flex().flex_col().gap(px(gap));
        if let Some(label) = self.label.clone() {
            column = column.child(div().text_size(px(font)).text_color(text).child(label));
        }

        let current = Rc::new(value.clone());
        for (i, option) in self.options.iter().enumerate() {
            let mut checkbox = Checkbox::new(("guise-checkboxgroup", i))
                .label(option.clone())
                .checked(value.contains(&i))
                .color(self.color)
                .size(self.size);
            if self.binding.is_some() || self.on_change.is_some() {
                let binding = self.binding.clone();
                let handler = self.on_change.clone();
                let current = current.clone();
                checkbox = checkbox.on_change(move |_ev, window, cx| {
                    let mut next = (*current).clone();
                    if let Some(pos) = next.iter().position(|x| *x == i) {
                        next.remove(pos);
                    } else {
                        next.push(i);
                        next.sort_unstable();
                    }
                    if let Some(binding) = &binding {
                        binding.set(cx, next.clone());
                    }
                    if let Some(handler) = &handler {
                        handler(next, window, cx);
                    }
                });
            }
            column = column.child(checkbox);
        }
        column
    }
}
