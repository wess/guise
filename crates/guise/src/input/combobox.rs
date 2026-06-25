//! `Combobox` — a searchable [`Select`](super::Select) (gpui entity).
//!
//! The trigger is an editable query field; the deferred list filters by a
//! case-insensitive substring match. Single-select closes on choice; with
//! [`Combobox::multiple`] it keeps a selection set and stays open. Emits
//! [`ComboboxEvent`] with the toggled option index.

use gpui::prelude::*;
use gpui::{
    deferred, div, px, Context, EventEmitter, FocusHandle, IntoElement, KeyDownEvent, SharedString,
    Window,
};

use super::{control_metrics, Field, TextEdit};
use crate::icon::{Icon, IconName};
use crate::theme::{theme, Size};

/// Emitted when an option is chosen/toggled. Carries the option index.
#[derive(Debug, Clone, Copy)]
pub struct ComboboxEvent(pub usize);

/// A searchable picker. Create with `cx.new(|cx| Combobox::new(cx).data([..]))`.
pub struct Combobox {
    options: Vec<SharedString>,
    selected: Vec<usize>,
    query: TextEdit,
    open: bool,
    multiple: bool,
    focus: FocusHandle,
    placeholder: SharedString,
    label: Option<SharedString>,
    size: Size,
    disabled: bool,
}

impl EventEmitter<ComboboxEvent> for Combobox {}

impl Combobox {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Combobox {
            options: Vec::new(),
            selected: Vec::new(),
            query: TextEdit::new(""),
            open: false,
            multiple: false,
            focus: cx.focus_handle(),
            placeholder: SharedString::new_static("Search…"),
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
        self
    }

    pub fn multiple(mut self, multiple: bool) -> Self {
        self.multiple = multiple;
        self
    }

    pub fn selected(mut self, indices: impl IntoIterator<Item = usize>) -> Self {
        self.selected = indices.into_iter().collect();
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

    pub fn selected_indices(&self) -> &[usize] {
        &self.selected
    }

    /// Indices of options matching the current query.
    fn filtered(&self) -> Vec<usize> {
        let q = self.query.text().to_lowercase();
        self.options
            .iter()
            .enumerate()
            .filter(|(_, o)| q.is_empty() || o.to_lowercase().contains(&q))
            .map(|(i, _)| i)
            .collect()
    }

    fn choose(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.multiple {
            if let Some(pos) = self.selected.iter().position(|x| *x == index) {
                self.selected.remove(pos);
            } else {
                self.selected.push(index);
                self.selected.sort_unstable();
            }
        } else {
            self.selected = vec![index];
            self.open = false;
            self.query = TextEdit::new("");
        }
        cx.emit(ComboboxEvent(index));
        cx.notify();
    }

    fn on_key(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        let ks = &event.keystroke;
        if ks.modifiers.platform || ks.modifiers.control {
            return;
        }
        match ks.key.as_str() {
            "escape" => {
                self.open = false;
            }
            "enter" => {
                if let Some(&first) = self.filtered().first() {
                    self.choose(first, cx);
                }
            }
            "backspace" => {
                self.query.backspace();
                self.open = true;
            }
            "left" => self.query.left(),
            "right" => self.query.right(),
            _ => {
                if let Some(text) = ks
                    .key_char
                    .as_deref()
                    .filter(|t| !t.is_empty() && !ks.modifiers.alt)
                {
                    self.query.insert(text);
                    self.open = true;
                }
            }
        }
        cx.notify();
        cx.stop_propagation();
    }

    fn value_text(&self) -> SharedString {
        match (self.multiple, self.selected.len()) {
            (_, 0) => self.placeholder.clone(),
            (true, n) => SharedString::from(format!("{n} selected")),
            (false, _) => self
                .selected
                .first()
                .and_then(|i| self.options.get(*i))
                .cloned()
                .unwrap_or_else(|| self.placeholder.clone()),
        }
    }
}

impl Render for Combobox {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        let (height, pad_x, font) = control_metrics(self.size);
        let radius = t.radius(t.default_radius);
        let focused = self.focus.is_focused(window) && !self.disabled;
        let surface = t.surface().hsla();
        let surface_hover = t.surface_hover().hsla();
        let border = if focused { t.primary() } else { t.border() }.hsla();
        let text_color = t.text().hsla();
        let dimmed = t.dimmed().hsla();
        let caret = t.primary().hsla();
        let selected_bg = t.primary().alpha(0.12);

        let has_value = !self.selected.is_empty();
        let interior = if self.open && focused {
            let (before, after) = self.query.split();
            div()
                .flex()
                .items_center()
                .text_color(text_color)
                .child(SharedString::from(before))
                .child(div().w(px(1.0)).h(px(font * 1.15)).bg(caret))
                .child(SharedString::from(after))
        } else {
            div()
                .text_color(if has_value { text_color } else { dimmed })
                .child(self.value_text())
        };

        let trigger = div()
            .id("guise-combobox-trigger")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key))
            .on_click(cx.listener(|this, _ev, window, cx| {
                if !this.disabled {
                    this.open = !this.open;
                    window.focus(&this.focus);
                    cx.notify();
                }
            }))
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
            .child(interior)
            .child(Icon::new(IconName::ChevronDown).size(Size::Xs).color(crate::theme::ColorName::Gray));

        let mut wrap = div().relative().child(trigger);

        if self.open && !self.disabled {
            let filtered = self.filtered();
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

            if filtered.is_empty() {
                menu = menu.child(
                    div()
                        .px(px(10.0))
                        .py(px(6.0))
                        .text_size(px(font))
                        .text_color(dimmed)
                        .child(SharedString::new_static("No matches")),
                );
            }
            for i in filtered {
                let is_selected = self.selected.contains(&i);
                let option = self.options[i].clone();
                let mut row = div()
                    .id(("guise-combobox-option", i))
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(10.0))
                    .py(px(6.0))
                    .rounded(px(4.0))
                    .text_size(px(font))
                    .text_color(text_color)
                    .hover(move |s| s.bg(surface_hover))
                    .child(option)
                    .on_click(cx.listener(move |this, _ev, _window, cx| this.choose(i, cx)));
                if is_selected {
                    row = row
                        .bg(selected_bg)
                        .child(Icon::new(IconName::Check).size(Size::Xs));
                }
                menu = menu.child(row);
            }

            wrap = wrap.child(deferred(menu));
        }

        let mut chrome = Field::new().child(if self.disabled {
            wrap.opacity(0.6)
        } else {
            wrap
        });
        if let Some(label) = self.label.clone() {
            chrome = chrome.label(label);
        }
        chrome
    }
}
