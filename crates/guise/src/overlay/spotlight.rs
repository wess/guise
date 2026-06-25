//! `Spotlight` — a command palette (gpui entity).
//!
//! A centered overlay with a search field and a keyboard-navigable command list.
//! Type to filter, ↑/↓ to move the highlight, Enter to run, Esc to dismiss.
//! Built on the same deferred-backdrop pattern as [`Modal`](super::Modal) plus
//! the [`TextEdit`](crate::TextEdit) query model.

use gpui::prelude::*;
use gpui::{
    deferred, div, px, App, Context, FocusHandle, IntoElement, KeyDownEvent, SharedString, Window,
};

use crate::icon::{Icon, IconName};
use crate::input::TextEdit;
use crate::theme::{theme, Size};

type CommandHandler = Box<dyn Fn(&mut Window, &mut App) + 'static>;

struct Command {
    label: SharedString,
    hint: Option<SharedString>,
    handler: Option<CommandHandler>,
}

/// A command palette. Create with `cx.new(|cx| Spotlight::new(cx))`, register
/// commands with [`Spotlight::item`], and open it from an action.
pub struct Spotlight {
    open: bool,
    focus: FocusHandle,
    query: TextEdit,
    commands: Vec<Command>,
    selected: usize,
}

impl Spotlight {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Spotlight {
            open: false,
            focus: cx.focus_handle(),
            query: TextEdit::new(""),
            commands: Vec::new(),
            selected: 0,
        }
    }

    /// Register a command with the action to run when it is chosen.
    pub fn item(
        mut self,
        label: impl Into<SharedString>,
        handler: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.commands.push(Command {
            label: label.into(),
            hint: None,
            handler: Some(Box::new(handler)),
        });
        self
    }

    /// Register a command with a trailing hint (e.g. a shortcut).
    pub fn item_hint(
        mut self,
        label: impl Into<SharedString>,
        hint: impl Into<SharedString>,
        handler: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.commands.push(Command {
            label: label.into(),
            hint: Some(hint.into()),
            handler: Some(Box::new(handler)),
        });
        self
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open = true;
        self.query = TextEdit::new("");
        self.selected = 0;
        window.focus(&self.focus);
        cx.notify();
    }

    pub fn close(&mut self, cx: &mut Context<Self>) {
        self.open = false;
        cx.notify();
    }

    fn filtered(&self) -> Vec<usize> {
        let q = self.query.text().to_lowercase();
        self.commands
            .iter()
            .enumerate()
            .filter(|(_, c)| q.is_empty() || c.label.to_lowercase().contains(&q))
            .map(|(i, _)| i)
            .collect()
    }

    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let ks = &event.keystroke;
        if ks.modifiers.platform || ks.modifiers.control {
            return;
        }
        match ks.key.as_str() {
            "escape" => self.open = false,
            "up" => self.selected = self.selected.saturating_sub(1),
            "down" => {
                let n = self.filtered().len();
                if n > 0 {
                    self.selected = (self.selected + 1).min(n - 1);
                }
            }
            "enter" => {
                let filtered = self.filtered();
                if let Some(&idx) = filtered.get(self.selected) {
                    self.open = false;
                    if let Some(handler) = &self.commands[idx].handler {
                        handler(window, cx);
                    }
                }
            }
            "backspace" => {
                self.query.backspace();
                self.selected = 0;
            }
            _ => {
                if let Some(text) = ks
                    .key_char
                    .as_deref()
                    .filter(|t| !t.is_empty() && !ks.modifiers.alt)
                {
                    self.query.insert(text);
                    self.selected = 0;
                }
            }
        }
        cx.notify();
        cx.stop_propagation();
    }
}

impl Render for Spotlight {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut root = div();
        if !self.open {
            return root;
        }

        let t = theme(cx);
        let radius = t.radius(Size::Md);
        let surface = t.surface().hsla();
        let border = t.border().hsla();
        let text_color = t.text().hsla();
        let dimmed = t.dimmed().hsla();
        let caret = t.primary().hsla();
        let selected_bg = t.primary().alpha(0.14);
        let scrim = t.black.alpha(0.45);
        let font = t.font_size(Size::Md);
        let font_sm = t.font_size(Size::Sm);
        let viewport = window.viewport_size();

        let (before, after) = self.query.split();
        let query_text = self.query.text();
        let search = div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .px(px(14.0))
            .h(px(48.0))
            .border_b_1()
            .border_color(border)
            .child(Icon::new(IconName::Search).color(crate::theme::ColorName::Gray))
            .child(if query_text.is_empty() {
                div()
                    .flex()
                    .items_center()
                    .text_color(dimmed)
                    .child(div().w(px(1.0)).h(px(font * 1.15)).bg(caret))
                    .child(SharedString::new_static("Type a command…"))
            } else {
                div()
                    .flex()
                    .items_center()
                    .text_color(text_color)
                    .child(SharedString::from(before))
                    .child(div().w(px(1.0)).h(px(font * 1.15)).bg(caret))
                    .child(SharedString::from(after))
            });

        let filtered = self.filtered();
        let mut list = div().flex().flex_col().gap(px(2.0)).p(px(6.0));
        if filtered.is_empty() {
            list = list.child(
                div()
                    .px(px(10.0))
                    .py(px(8.0))
                    .text_size(px(font_sm))
                    .text_color(dimmed)
                    .child(SharedString::new_static("No commands")),
            );
        }
        for (j, idx) in filtered.iter().enumerate() {
            let idx = *idx;
            let is_active = j == self.selected;
            let command = &self.commands[idx];
            let mut row = div()
                .id(("guise-spotlight-item", idx))
                .flex()
                .items_center()
                .justify_between()
                .px(px(10.0))
                .py(px(8.0))
                .rounded(px(6.0))
                .text_size(px(font_sm))
                .text_color(text_color)
                .child(command.label.clone())
                .on_click(cx.listener(move |this, _ev, window, cx| {
                    this.open = false;
                    if let Some(handler) = &this.commands[idx].handler {
                        handler(window, cx);
                    }
                    cx.notify();
                }));
            if let Some(hint) = command.hint.clone() {
                row = row.child(div().text_size(px(font_sm)).text_color(dimmed).child(hint));
            }
            if is_active {
                row = row.bg(selected_bg);
            }
            list = list.child(row);
        }

        let panel = div()
            .id("guise-spotlight-panel")
            .occlude()
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key))
            .on_click(|_ev, _window, cx| cx.stop_propagation())
            .mt(px(80.0))
            .w(px(560.0))
            .flex()
            .flex_col()
            .bg(surface)
            .rounded(px(radius))
            .border_1()
            .border_color(border)
            .shadow_xl()
            .child(search)
            .child(list);

        let backdrop = div()
            .id("guise-spotlight-backdrop")
            .occlude()
            .absolute()
            .top(px(0.0))
            .left(px(0.0))
            .w(viewport.width)
            .h(viewport.height)
            .flex()
            .justify_center()
            .items_start()
            .bg(scrim)
            .on_click(cx.listener(|this, _ev, _window, cx| {
                this.open = false;
                cx.notify();
            }))
            .child(panel);

        root = root.child(deferred(backdrop));
        root
    }
}
