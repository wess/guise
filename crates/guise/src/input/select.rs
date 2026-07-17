//! `Select` — a stateful dropdown picker (gpui entity).
//!
//! Owns its open state and selection; renders a trigger plus a deferred
//! dropdown list, and emits [`SelectEvent`] when the choice changes.

use gpui::prelude::*;
use gpui::{
    deferred, div, px, App, Context, Entity, EventEmitter, FocusHandle, IntoElement, SharedString,
    Window,
};

use super::control_metrics;
use crate::reactive::Signal;
use crate::theme::{theme, Size};

/// Emitted when the user picks an option. Carries the option index.
#[derive(Debug, Clone)]
pub struct SelectEvent(pub usize);

/// A dropdown picker. Create with `cx.new(|cx| Select::new(cx).data([...]))`.
pub struct Select {
    options: Vec<SharedString>,
    selected: Option<usize>,
    open: bool,
    focus: FocusHandle,
    placeholder: SharedString,
    label: Option<SharedString>,
    size: Size,
    disabled: bool,
}

impl EventEmitter<SelectEvent> for Select {}

impl Select {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Select {
            options: Vec::new(),
            selected: None,
            open: false,
            focus: cx.focus_handle(),
            placeholder: SharedString::new_static("Pick one"),
            label: None,
            size: Size::Sm,
            disabled: false,
        }
    }

    pub fn data<I, S>(mut self, options: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.options = options.into_iter().map(Into::into).collect();
        self.selected = self.selected.and_then(|index| {
            (!self.options.is_empty()).then(|| index.min(self.options.len() - 1))
        });
        self
    }

    pub fn selected(mut self, index: usize) -> Self {
        self.selected = Some(index);
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
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

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.selected
    }

    pub fn selected_value(&self) -> Option<SharedString> {
        self.selected.and_then(|i| self.options.get(i).cloned())
    }

    /// Two-way bind this picker's selection to a `Signal<usize>`. The signal
    /// is the source of truth: the picker adopts its index now, picks write
    /// back through [`Signal::set_if_changed`], and signal writes move the
    /// selection without emitting [`SelectEvent`]. Equality guards on both
    /// directions prevent update loops.
    pub fn bind(entity: &Entity<Select>, signal: &Signal<usize>, cx: &mut App) {
        let initial = signal.get(cx);
        entity.update(cx, |this, cx| this.sync_selected(initial, cx));
        let sink = signal.clone();
        cx.subscribe(entity, move |_select, event: &SelectEvent, cx| {
            sink.set_if_changed(cx, event.0);
        })
        .detach();
        let select = entity.downgrade();
        cx.observe(signal.entity(), move |observed, cx| {
            let index = *observed.read(cx);
            select
                .update(cx, |this, cx| this.sync_selected(index, cx))
                .ok();
        })
        .detach();
    }

    /// Programmatic set: repaint without emitting an event.
    fn sync_selected(&mut self, index: usize, cx: &mut Context<Self>) {
        let selected = (!self.options.is_empty()).then(|| index.min(self.options.len() - 1));
        if self.selected != selected {
            self.selected = selected;
            cx.notify();
        }
    }
}

impl Render for Select {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        let (height, pad_x, font) = control_metrics(self.size);
        let radius = t.radius(t.default_radius);
        let surface = t.surface().hsla();
        let surface_hover = t.surface_hover().hsla();
        let border = t.border().hsla();
        let text_color = t.text().hsla();
        let dimmed = t.dimmed().hsla();
        let selected_bg = t.primary().alpha(0.12);
        let font_sm = t.font_size(Size::Sm);

        let selected = self.selected;
        let chosen = selected.and_then(|i| self.options.get(i));
        let has_value = chosen.is_some();
        let value_text: SharedString = chosen.cloned().unwrap_or_else(|| self.placeholder.clone());

        let trigger = div()
            .id("guise-select-trigger")
            .track_focus(&self.focus)
            .flex()
            .items_center()
            .justify_between()
            .gap(px(8.0))
            .h(px(height))
            .px(px(pad_x))
            .rounded(px(radius))
            .border_1()
            .border_color(border)
            .bg(surface)
            .text_size(px(font))
            .text_color(if has_value { text_color } else { dimmed })
            .child(value_text)
            .child(
                div()
                    .text_color(dimmed)
                    .child(SharedString::new_static("\u{25be}")),
            )
            .on_click(cx.listener(|this, _ev, _window, cx| {
                if !this.disabled {
                    this.open = !this.open;
                    cx.notify();
                }
            }));

        let mut wrap = div().relative().child(trigger);

        if self.open && !self.disabled {
            let mut menu = div()
                .absolute()
                .top(px(height + 6.0))
                .left(px(0.0))
                .right(px(0.0))
                .flex()
                .flex_col()
                .gap(px(2.0))
                .p(px(4.0))
                .rounded(px(radius))
                .border_1()
                .border_color(border)
                .bg(surface)
                .shadow_md();

            for (i, option) in self.options.iter().enumerate() {
                let is_selected = Some(i) == selected;
                let mut row = div()
                    .id(("guise-select-option", i))
                    .px(px(10.0))
                    .py(px(6.0))
                    .rounded(px(4.0))
                    .text_size(px(font))
                    .text_color(text_color)
                    .hover(move |s| s.bg(surface_hover))
                    .child(option.clone())
                    .on_click(cx.listener(move |this, _ev, _window, cx| {
                        this.selected = Some(i);
                        this.open = false;
                        cx.emit(SelectEvent(i));
                        cx.notify();
                    }));
                if is_selected {
                    row = row.bg(selected_bg);
                }
                menu = menu.child(row);
            }

            wrap = wrap.child(deferred(menu));
        }

        let mut column = div().flex().flex_col().gap(px(4.0));
        if let Some(label) = self.label.clone() {
            column = column.child(
                div()
                    .text_size(px(font_sm))
                    .text_color(text_color)
                    .child(label),
            );
        }
        column = column.child(wrap);

        if self.disabled {
            column.opacity(0.6)
        } else {
            column
        }
    }
}
