//! `FileInput` — a stateful file-picking field (gpui entity).
//!
//! A field-styled trigger that opens the platform file dialog and holds the
//! chosen paths. Emits [`FileInputEvent`] on every selection change (an empty
//! vec means cleared).

use std::path::PathBuf;

use gpui::prelude::*;
use gpui::{
    div, px, Context, EventEmitter, FocusHandle, IntoElement, PathPromptOptions, SharedString,
    Window,
};

use super::accept::{filter_paths, normalize_ext};
use super::control_metrics;
use crate::icon::{Icon, IconName};
use crate::theme::{theme, Size};

/// Emitted when the selection changes. Empty means cleared.
#[derive(Debug, Clone)]
pub struct FileInputEvent(pub Vec<PathBuf>);

/// A file-picker field. Create with `cx.new(|cx| FileInput::new(cx))`.
pub struct FileInput {
    focus: FocusHandle,
    paths: Vec<PathBuf>,
    multiple: bool,
    directories: bool,
    accept: Vec<String>,
    placeholder: SharedString,
    label: Option<SharedString>,
    size: Size,
    disabled: bool,
}

impl EventEmitter<FileInputEvent> for FileInput {}

impl FileInput {
    pub fn new(cx: &mut Context<Self>) -> Self {
        FileInput {
            focus: cx.focus_handle(),
            paths: Vec::new(),
            multiple: false,
            directories: false,
            accept: Vec::new(),
            placeholder: SharedString::new_static("Choose a file"),
            label: None,
            size: Size::Sm,
            disabled: false,
        }
    }

    pub fn multiple(mut self) -> Self {
        self.multiple = true;
        self.placeholder = SharedString::new_static("Choose files");
        self
    }

    /// Pick directories instead of files.
    pub fn directories(mut self) -> Self {
        self.directories = true;
        self.placeholder = SharedString::new_static("Choose a folder");
        self
    }

    /// Allowed extensions ("png", ".jpg", case-insensitive). Empty = any.
    pub fn accept<I, S>(mut self, entries: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.accept = entries
            .into_iter()
            .map(|e| normalize_ext(e.as_ref()))
            .collect();
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

    pub fn paths(&self) -> &[PathBuf] {
        &self.paths
    }

    fn browse(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: !self.directories,
            directories: self.directories,
            multiple: self.multiple,
            prompt: None,
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = receiver.await {
                this.update(cx, |input, cx| input.set_paths(paths, cx)).ok();
            }
        })
        .detach();
    }

    fn set_paths(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        let kept = filter_paths(paths, &self.accept);
        if kept.is_empty() {
            return;
        }
        self.paths = kept;
        cx.emit(FileInputEvent(self.paths.clone()));
        cx.notify();
    }

    fn clear(&mut self, cx: &mut Context<Self>) {
        if !self.paths.is_empty() {
            self.paths.clear();
            cx.emit(FileInputEvent(Vec::new()));
            cx.notify();
        }
    }

    fn shown_text(&self) -> Option<SharedString> {
        match self.paths.as_slice() {
            [] => None,
            [single] => Some(
                single
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| single.display().to_string())
                    .into(),
            ),
            many => Some(format!("{} files", many.len()).into()),
        }
    }
}

impl Render for FileInput {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        let (height, pad_x, font) = control_metrics(self.size);
        let radius = t.radius(t.default_radius);
        let surface = t.surface().hsla();
        let surface_hover = t.surface_hover().hsla();
        let border = t.border().hsla();
        let text_color = t.text().hsla();
        let dimmed = t.dimmed().hsla();
        let font_sm = t.font_size(Size::Sm);

        let shown = self.shown_text();
        let has_value = shown.is_some();
        let value_text = shown.unwrap_or_else(|| self.placeholder.clone());

        let mut trigger = div()
            .id("guise-fileinput-trigger")
            .track_focus(&self.focus)
            .flex()
            .items_center()
            .gap(px(8.0))
            .h(px(height))
            .px(px(pad_x))
            .rounded(px(radius))
            .border_1()
            .border_color(border)
            .bg(surface)
            .text_size(px(font))
            .text_color(if has_value { text_color } else { dimmed })
            .hover(move |s| s.bg(surface_hover))
            .child(
                div()
                    .text_color(dimmed)
                    .child(Icon::new(IconName::Paperclip).size(Size::Sm)),
            )
            .child(div().flex_1().truncate().child(value_text))
            .on_click(cx.listener(|this, _ev, _window, cx| {
                if !this.disabled {
                    this.browse(cx);
                }
            }));

        if has_value {
            trigger = trigger.child(
                div()
                    .id("guise-fileinput-clear")
                    .flex()
                    .items_center()
                    .text_color(dimmed)
                    .child(Icon::new(IconName::X).size(Size::Xs))
                    .on_click(cx.listener(|this, _ev, _window, cx| {
                        // The trigger sits underneath; don't reopen the dialog.
                        cx.stop_propagation();
                        this.clear(cx);
                    })),
            );
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
        column = column.child(trigger);

        if self.disabled {
            column.opacity(0.6)
        } else {
            column
        }
    }
}
