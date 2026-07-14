//! `Autocomplete` — a freeform text field with suggestions (gpui entity).
//!
//! Unlike [`Combobox`](super::Combobox) (pick one of the options), the value
//! here is whatever the user types — suggestions are shortcuts, not
//! constraints. Arrow keys walk the list, Enter adopts the highlighted
//! suggestion (or commits the typed text), Escape closes.

use gpui::prelude::*;
use gpui::{
    deferred, div, px, App, Context, Entity, EventEmitter, FocusHandle, IntoElement, KeyDownEvent,
    SharedString, Window,
};

use super::{control_metrics, Field, TextEdit};
use crate::reactive::Signal;
use crate::theme::{theme, Size};

/// Emitted as the value changes and when it's committed.
#[derive(Debug, Clone)]
pub enum AutocompleteEvent {
    /// Every edit; carries the current text.
    Change(String),
    /// Enter or a suggestion click; carries the final text.
    Commit(String),
}

/// Indices of suggestions matching `query` (case-insensitive substring).
/// An empty query matches nothing — the list is a typing aid, not a menu.
fn matches(suggestions: &[SharedString], query: &str) -> Vec<usize> {
    if query.is_empty() {
        return Vec::new();
    }
    let q = query.to_lowercase();
    suggestions
        .iter()
        .enumerate()
        .filter(|(_, s)| s.to_lowercase().contains(&q))
        .map(|(i, _)| i)
        .collect()
}

/// A text field with completion. Create with
/// `cx.new(|cx| Autocomplete::new(cx).suggestions([..]))`.
pub struct Autocomplete {
    suggestions: Vec<SharedString>,
    edit: TextEdit,
    open: bool,
    highlight: usize,
    max_shown: usize,
    focus: FocusHandle,
    placeholder: SharedString,
    label: Option<SharedString>,
    size: Size,
    disabled: bool,
}

impl EventEmitter<AutocompleteEvent> for Autocomplete {}

impl Autocomplete {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Autocomplete {
            suggestions: Vec::new(),
            edit: TextEdit::new(""),
            open: false,
            highlight: 0,
            max_shown: 8,
            focus: cx.focus_handle(),
            placeholder: SharedString::new_static("Type…"),
            label: None,
            size: Size::Sm,
            disabled: false,
        }
    }

    pub fn suggestions<I, S>(mut self, suggestions: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.suggestions = suggestions.into_iter().map(Into::into).collect();
        self
    }

    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.edit = TextEdit::new(&value.into());
        self
    }

    /// Cap the dropdown length (default 8).
    pub fn max_shown(mut self, max: usize) -> Self {
        self.max_shown = max.max(1);
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

    pub fn text(&self) -> String {
        self.edit.text().to_string()
    }

    /// Two-way bind the text to a `Signal<String>`. The signal is the source
    /// of truth; equality guards on both directions prevent loops.
    pub fn bind(entity: &Entity<Autocomplete>, signal: &Signal<String>, cx: &mut App) {
        let initial = signal.get(cx);
        entity.update(cx, |this, cx| this.sync_text(initial, cx));
        let sink = signal.clone();
        cx.subscribe(entity, move |_this, event: &AutocompleteEvent, cx| {
            if let AutocompleteEvent::Change(text) = event {
                sink.set_if_changed(cx, text.clone());
            }
        })
        .detach();
        let field = entity.downgrade();
        cx.observe(signal.entity(), move |observed, cx| {
            let text = observed.read(cx).clone();
            field.update(cx, |this, cx| this.sync_text(text, cx)).ok();
        })
        .detach();
    }

    fn sync_text(&mut self, text: String, cx: &mut Context<Self>) {
        if self.edit.text() != text {
            self.edit = TextEdit::new(&text);
            cx.notify();
        }
    }

    fn adopt(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(text) = self.suggestions.get(index).cloned() {
            self.edit = TextEdit::new(text.as_ref());
            self.open = false;
            cx.emit(AutocompleteEvent::Change(text.to_string()));
            cx.emit(AutocompleteEvent::Commit(text.to_string()));
            cx.notify();
        }
    }

    fn on_key(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        let ks = &event.keystroke;
        if ks.modifiers.platform || ks.modifiers.control {
            return;
        }
        let shown = matches(&self.suggestions, &self.edit.text())
            .len()
            .min(self.max_shown);
        match ks.key.as_str() {
            "escape" => self.open = false,
            "down" if self.open && shown > 0 => {
                self.highlight = (self.highlight + 1) % shown;
            }
            "up" if self.open && shown > 0 => {
                self.highlight = (self.highlight + shown - 1) % shown;
            }
            "enter" => {
                if self.open && shown > 0 {
                    let target = matches(&self.suggestions, &self.edit.text())[self.highlight];
                    self.adopt(target, cx);
                } else {
                    self.open = false;
                    cx.emit(AutocompleteEvent::Commit(self.text()));
                }
            }
            "backspace" => {
                self.edit.backspace();
                self.open = true;
                self.highlight = 0;
                cx.emit(AutocompleteEvent::Change(self.text()));
            }
            "left" => self.edit.left(),
            "right" => self.edit.right(),
            _ => {
                if let Some(text) = ks
                    .key_char
                    .as_deref()
                    .filter(|t| !t.is_empty() && !ks.modifiers.alt)
                {
                    self.edit.insert(text);
                    self.open = true;
                    self.highlight = 0;
                    cx.emit(AutocompleteEvent::Change(self.text()));
                }
            }
        }
        cx.notify();
        cx.stop_propagation();
    }
}

impl Render for Autocomplete {
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
        let highlight_bg = t.primary().alpha(0.12);

        let empty = self.edit.text().is_empty();
        let interior = if focused {
            let (before, after) = self.edit.split();
            div()
                .flex()
                .items_center()
                .text_color(text_color)
                .child(SharedString::from(before))
                .child(div().w(px(1.0)).h(px(font * 1.15)).bg(caret))
                .child(SharedString::from(after))
        } else if empty {
            div().text_color(dimmed).child(self.placeholder.clone())
        } else {
            div()
                .text_color(text_color)
                .child(SharedString::from(self.text()))
        };

        let trigger = div()
            .id("guise-autocomplete-trigger")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key))
            .on_click(cx.listener(|this, _ev, window, cx| {
                if !this.disabled {
                    window.focus(&this.focus, cx);
                    cx.notify();
                }
            }))
            .flex()
            .items_center()
            .h(px(height))
            .px(px(pad_x))
            .rounded(px(radius))
            .border_1()
            .border_color(border)
            .bg(surface)
            .text_size(px(font))
            .child(interior);

        let mut wrap = div().relative().child(trigger);

        let shown = matches(&self.suggestions, &self.edit.text());
        if self.open && focused && !shown.is_empty() {
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
                .shadow_md()
                .occlude();
            for (row_ix, &option_ix) in shown.iter().take(self.max_shown).enumerate() {
                let option = self.suggestions[option_ix].clone();
                let mut row = div()
                    .id(("guise-autocomplete-option", row_ix))
                    .px(px(10.0))
                    .py(px(6.0))
                    .rounded(px(4.0))
                    .text_size(px(font))
                    .text_color(text_color)
                    .child(option)
                    .on_click(cx.listener(move |this, _ev, _window, cx| {
                        this.adopt(option_ix, cx);
                    }));
                if row_ix == self.highlight {
                    row = row.bg(highlight_bg);
                } else {
                    row = row.hover(move |s| s.bg(surface_hover));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn suggestions() -> Vec<SharedString> {
        ["Rust", "Ruby", "Python", "TypeScript"]
            .into_iter()
            .map(SharedString::new_static)
            .collect()
    }

    #[test]
    fn matching_is_substring_and_case_insensitive() {
        let s = suggestions();
        assert_eq!(matches(&s, "ru"), vec![0, 1]);
        assert_eq!(matches(&s, "PY"), vec![2]);
        assert_eq!(matches(&s, "script"), vec![3]);
        assert_eq!(matches(&s, "zzz"), Vec::<usize>::new());
    }

    #[test]
    fn empty_query_matches_nothing() {
        assert_eq!(matches(&suggestions(), ""), Vec::<usize>::new());
    }
}
