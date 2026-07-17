//! `Dropzone` — a drag-and-drop file target (stateless builder).
//!
//! Accepts OS file drags (gpui `ExternalPaths`) and, by default, opens the
//! platform file dialog on click. The parent owns whatever happens with the
//! paths; wire `.on_files(...)`.

use std::path::PathBuf;
use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    div, px, App, ElementId, ExternalPaths, IntoElement, PathPromptOptions, SharedString, Window,
};

use super::accept::{filter_paths, normalize_ext};
use crate::icon::{Icon, IconName};
use crate::theme::{theme, Size};

type FilesHandler = Rc<dyn Fn(Vec<PathBuf>, &mut App) + 'static>;

/// A drop target for OS file drags. `Dropzone::new("dz").on_files(...)`.
#[derive(IntoElement)]
pub struct Dropzone {
    id: ElementId,
    label: SharedString,
    hint: Option<SharedString>,
    icon: IconName,
    accept: Vec<String>,
    multiple: bool,
    clickable: bool,
    height: f32,
    on_files: Option<FilesHandler>,
}

impl Dropzone {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Dropzone {
            id: id.into(),
            label: SharedString::new_static("Drop files here"),
            hint: None,
            icon: IconName::CloudUpload,
            accept: Vec::new(),
            multiple: true,
            clickable: true,
            height: 140.0,
            on_files: None,
        }
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }

    /// Secondary line under the label ("PNG or JPG, up to 10 MB").
    pub fn hint(mut self, hint: impl Into<SharedString>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = icon;
        self
    }

    /// Allowed extensions ("png", ".jpg", case-insensitive). Empty = any.
    /// Dropped paths that don't pass are silently ignored.
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

    /// Only take the first dropped/browsed file.
    pub fn single(mut self) -> Self {
        self.multiple = false;
        self
    }

    /// Disable the click-to-browse dialog (drop only).
    pub fn no_click(mut self) -> Self {
        self.clickable = false;
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Receives the accepted paths of every drop or dialog pick.
    pub fn on_files(mut self, handler: impl Fn(Vec<PathBuf>, &mut App) + 'static) -> Self {
        self.on_files = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Dropzone {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let radius = t.radius(t.default_radius);
        let surface = t.surface().hsla();
        let border = t.border().hsla();
        let text_color = t.text().hsla();
        let dimmed = t.dimmed().hsla();
        let accent = t.primary();
        let accent_border = accent.hsla();
        let accent_tint = accent.alpha(0.08);
        let font = t.font_size(Size::Sm);

        let accept = self.accept;
        let multiple = self.multiple;
        let handler = self.on_files;

        let deliver = {
            let accept = accept.clone();
            let handler = handler.clone();
            move |mut paths: Vec<PathBuf>, cx: &mut App| {
                paths = filter_paths(paths, &accept);
                if !multiple {
                    paths.truncate(1);
                }
                if let (Some(handler), false) = (&handler, paths.is_empty()) {
                    handler(paths, cx);
                }
            }
        };

        let mut zone = div()
            .id(self.id)
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(6.0))
            .w_full()
            .h(px(self.height))
            .rounded(px(radius))
            .border_1()
            .border_dashed()
            .border_color(border)
            .bg(surface)
            .drag_over::<ExternalPaths>(move |style, _paths, _window, _cx| {
                style.border_color(accent_border).bg(accent_tint)
            })
            .on_drop({
                let deliver = deliver.clone();
                move |dropped: &ExternalPaths, _window, cx| {
                    deliver(dropped.paths().to_vec(), cx);
                }
            })
            .child(
                div()
                    .text_color(dimmed)
                    .child(Icon::new(self.icon).size(Size::Lg)),
            )
            .child(
                div()
                    .text_size(px(font))
                    .text_color(text_color)
                    .child(self.label),
            );

        if let Some(hint) = self.hint {
            zone = zone.child(
                div()
                    .text_size(px(font - 2.0))
                    .text_color(dimmed)
                    .child(hint),
            );
        }

        if self.clickable {
            zone = zone.cursor_pointer().on_click(move |_ev, _window, cx| {
                let receiver = cx.prompt_for_paths(PathPromptOptions {
                    files: true,
                    directories: false,
                    multiple,
                    prompt: None,
                });
                let deliver = deliver.clone();
                cx.spawn(async move |cx| {
                    if let Ok(Ok(Some(paths))) = receiver.await {
                        cx.update(|cx| deliver(paths, cx)).ok();
                    }
                })
                .detach();
            });
        }

        zone
    }
}
