//! `Editor` — a multiline code editor (gpui entity).
//!
//! Renders an [`EditorModel`] with a line-number gutter, syntax highlighting
//! (a built-in [`Language`] tokenizer, or a whole-document
//! [`DocumentHighlighter`] such as the `treesitter` feature's adapter),
//! selection, caret, and the full macOS-convention keyboard map. Emits
//! [`EditorEvent::Change`] on every edit and [`EditorEvent::Run`] on
//! Cmd+Enter, so a host can execute the buffer (a query console, a REPL).
//!
//! ```ignore
//! let editor = cx.new(|cx| {
//!     Editor::new(cx)
//!         .language(Language::Rust)
//!         .rows(8)
//!         .value("fn main() {\n    println!(\"hi\");\n}")
//! });
//! cx.subscribe(&editor, |_this, _editor, event: &EditorEvent, _cx| {
//!     if let EditorEvent::Run(source) = event {
//!         // Cmd+Enter — run `source`
//!     }
//! })
//! .detach();
//! ```

use std::ops::Range;

use gpui::prelude::*;
use gpui::{
    canvas, div, point, px, App, Bounds, ClipboardItem, Context, Div, DragMoveEvent, Empty, Entity,
    EntityId, EventEmitter, FocusHandle, Font, Hsla, IntoElement, KeyDownEvent, MouseButton,
    MouseDownEvent, Pixels, Point, ScrollHandle, ShapedLine, SharedString, StyledText, TextRun,
    Window,
};

use super::cache::HighlightCache;
use super::diagnostic::{line_message, line_severity, Diagnostic};
use super::highlight::{token_color, DocumentHighlighter, Language, TokenKind};
use super::model::{EditorModel, Pos};
use crate::reactive::Signal;
use crate::theme::theme;

/// The monospace family used for the buffer, gutter, and placeholder.
const MONO_FAMILY: &str = "Menlo";
/// Horizontal padding around the text content, in px.
const PAD_X: f32 = 10.0;
/// Vertical padding above and below the lines, in px.
const PAD_Y: f32 = 8.0;
/// Padding inside the gutter on each side of the line numbers, in px.
const GUTTER_PAD: f32 = 10.0;

/// Emitted as the user edits or asks to run the buffer.
#[derive(Debug, Clone)]
pub enum EditorEvent {
    /// The document changed. Carries the full new text.
    Change(String),
    /// Cmd+Enter. Carries the current text, for hosts that execute it.
    Run(String),
}

/// The drag payload for selection-by-mouse; tagged with the owning entity so
/// two editors in one window never react to each other's drags.
struct EditorDrag(EntityId);

/// Per-editor visual overrides. Unset fields fall back to the theme-derived
/// defaults, so an empty style changes nothing.
#[derive(Clone, Copy, Default)]
pub struct EditorStyle {
    /// Paint no frame border and no corner radius (an embedded strip).
    pub bare: bool,
    pub bg: Option<Hsla>,
    pub text: Option<Hsla>,
    pub caret: Option<Hsla>,
    pub selection: Option<Hsla>,
    pub active_line: Option<Hsla>,
    pub gutter_fg: Option<Hsla>,
    pub gutter_fg_active: Option<Hsla>,
    pub placeholder: Option<Hsla>,
}

/// A multiline code editor. Create with `cx.new(|cx| Editor::new(cx))`.
///
/// The text model is the unit-tested [`EditorModel`] (char-index cursor,
/// anchor selection, coalesced undo). Read-only editors still support
/// selection and copy — only mutations are blocked.
pub struct Editor {
    model: EditorModel,
    language: Language,
    /// Whole-document highlighter; overrides `language` while set.
    doc_highlighter: Option<Box<dyn DocumentHighlighter>>,
    /// Whether `doc_highlighter` must reparse before the next paint.
    doc_dirty: bool,
    /// Per-line tokens for the `language` path, revalidated each frame and
    /// re-tokenized only for lines that changed.
    hl_cache: HighlightCache,
    placeholder: SharedString,
    read_only: bool,
    line_numbers: bool,
    font_size: f32,
    rows: Option<usize>,
    token_palette: Option<[Hsla; 8]>,
    style: EditorStyle,
    highlights: Vec<(Pos, Pos, Hsla)>,
    diagnostics: Vec<Diagnostic>,
    focus: FocusHandle,
    scroll: ScrollHandle,
    hscroll: ScrollHandle,
    /// Window-space bounds of the text content, captured at prepaint. Mouse
    /// positions minus this origin give content coordinates directly (the
    /// captured origin already moves with both scroll axes).
    text_bounds: Bounds<Pixels>,
    /// Measured monospace cell advance ('0'), refreshed every render. Only a
    /// rough unit (gutter width, scroll margins) — never per-glyph math.
    cell_w: f32,
    /// Line height in px, refreshed every render.
    line_h: f32,
    /// The resolved mono font, refreshed every render so shaping for mouse
    /// math uses exactly the font the glyphs are painted with.
    mono_font: Font,
}

impl EventEmitter<EditorEvent> for Editor {}

impl Editor {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Editor {
            model: EditorModel::new(""),
            language: Language::None,
            doc_highlighter: None,
            doc_dirty: true,
            hl_cache: HighlightCache::new(),
            placeholder: SharedString::default(),
            read_only: false,
            line_numbers: true,
            font_size: 13.0,
            rows: None,
            token_palette: None,
            style: EditorStyle::default(),
            highlights: Vec::new(),
            diagnostics: Vec::new(),
            focus: cx.focus_handle(),
            scroll: ScrollHandle::new(),
            hscroll: ScrollHandle::new(),
            text_bounds: Bounds::default(),
            cell_w: 13.0 * 0.6,
            line_h: 20.0,
            mono_font: gpui::font(MONO_FAMILY),
        }
    }

    // ---- builders ----

    /// Initial text (named like [`TextInput::value`](crate::input::TextInput::value);
    /// `text()` is the getter).
    pub fn value(mut self, text: &str) -> Self {
        self.model.set_text(text);
        self
    }

    /// Syntax highlighting language (default [`Language::None`]).
    pub fn language(mut self, language: Language) -> Self {
        self.language = language;
        self.hl_cache.clear();
        self
    }

    /// Highlight with a whole-document backend (a tree-sitter adapter)
    /// instead of the line-based `language` tokenizer. Takes precedence over
    /// [`language`](Self::language) while set.
    pub fn highlighter(mut self, highlighter: impl DocumentHighlighter + 'static) -> Self {
        self.doc_highlighter = Some(Box::new(highlighter));
        self.doc_dirty = true;
        self
    }

    /// Dimmed hint shown while the buffer is empty and unfocused.
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Block edits. Selection, copy, and Cmd+Enter still work.
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// Show the line-number gutter (default true).
    pub fn line_numbers(mut self, show: bool) -> Self {
        self.line_numbers = show;
        self
    }

    /// Spaces per tab stop (default 4).
    pub fn tab_size(mut self, n: usize) -> Self {
        self.model.set_tab_size(n);
        self
    }

    /// Buffer font size in px (default 13.0).
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Minimum height, as a number of visible lines.
    pub fn rows(mut self, rows: usize) -> Self {
        self.rows = Some(rows);
        self
    }

    /// Override the syntax palette — one color per [`TokenKind`], in
    /// [`TokenKind::ALL`] order. Defaults to the theme mapping
    /// ([`token_color`]).
    pub fn token_colors(mut self, colors: [Hsla; 8]) -> Self {
        self.token_palette = Some(colors);
        self
    }

    /// Per-editor visual overrides (see [`EditorStyle`]).
    pub fn style(mut self, style: EditorStyle) -> Self {
        self.style = style;
        self
    }

    /// Replace the style at runtime (theme switches).
    pub fn set_style(&mut self, style: EditorStyle, cx: &mut Context<Self>) {
        self.style = style;
        cx.notify();
    }

    /// Switch the highlighting language at runtime (a file-type change).
    /// Ignored while a document highlighter is set.
    pub fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        self.language = language;
        self.hl_cache.clear();
        cx.notify();
    }

    /// Install or replace the document highlighter at runtime; `None` falls
    /// back to the line-based `language` tokenizer.
    pub fn set_highlighter(
        &mut self,
        highlighter: Option<Box<dyn DocumentHighlighter>>,
        cx: &mut Context<Self>,
    ) {
        self.doc_highlighter = highlighter;
        self.doc_dirty = true;
        cx.notify();
    }

    /// Background rectangles painted under the text — search matches,
    /// occurrence highlights. Document-position ranges; multi-line ranges
    /// paint like selections.
    pub fn set_highlights(&mut self, highlights: Vec<(Pos, Pos, Hsla)>, cx: &mut Context<Self>) {
        self.highlights = highlights;
        cx.notify();
    }

    /// Attach diagnostics (compiler/linter/LSP output). Affected lines get a
    /// severity-colored gutter dot and range underline, and the active
    /// line's message shows in a strip under the buffer.
    pub fn set_diagnostics(&mut self, diagnostics: Vec<Diagnostic>, cx: &mut Context<Self>) {
        self.diagnostics = diagnostics;
        cx.notify();
    }

    pub fn clear_diagnostics(&mut self, cx: &mut Context<Self>) {
        if !self.diagnostics.is_empty() {
            self.diagnostics.clear();
            cx.notify();
        }
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    // ---- runtime API ----

    /// The current document text.
    pub fn text(&self) -> String {
        self.model.text()
    }

    /// Replace the document, resetting cursor, selection, and history.
    pub fn set_text(&mut self, value: &str, cx: &mut Context<Self>) {
        self.model.set_text(value);
        self.doc_dirty = true;
        cx.notify();
    }

    /// The editor's focus handle, so a host can focus it on open.
    pub fn focus_handle(&self) -> FocusHandle {
        self.focus.clone()
    }

    /// Read access to the underlying [`EditorModel`] — cursor, selection,
    /// lines — for hosts that build features over the buffer (completion,
    /// modal keymaps, search).
    pub fn model(&self) -> &EditorModel {
        &self.model
    }

    /// Mutate the [`EditorModel`] directly. Emits [`EditorEvent::Change`]
    /// when the text changed, keeps the caret visible, and repaints — the
    /// same bookkeeping every built-in edit goes through.
    pub fn edit<R>(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        f: impl FnOnce(&mut EditorModel) -> R,
    ) -> R {
        let before = self.model.text();
        let result = f(&mut self.model);
        let after = self.model.text();
        if after != before {
            self.doc_dirty = true;
            cx.emit(EditorEvent::Change(after));
        }
        self.ensure_cursor_visible(window);
        cx.notify();
        result
    }

    /// Window-space origin of the caret's cell — where a completion popup or
    /// search bar anchors. Tracks both scroll axes; meaningless before the
    /// first paint (returns the content origin).
    pub fn caret_origin(&self, window: &Window) -> Point<Pixels> {
        let cursor = self.model.cursor();
        let x = match self.model.line(cursor.line) {
            Some(line) => self.caret_x(line, cursor.col, window),
            None => 0.0,
        };
        point(
            self.text_bounds.origin.x + px(x),
            self.text_bounds.origin.y + px(cursor.line as f32 * self.line_h),
        )
    }

    /// The pixel height of one buffer line, as painted last frame.
    pub fn line_height(&self) -> f32 {
        self.line_h
    }

    /// Two-way bind this editor's text to a `Signal<String>`. The signal is
    /// the source of truth: the editor adopts its value now, edits write back
    /// through [`Signal::set_if_changed`], and signal writes replace the text.
    /// Equality guards on both directions prevent update loops.
    pub fn bind(entity: &Entity<Editor>, signal: &Signal<String>, cx: &mut App) {
        let initial = signal.get(cx);
        entity.update(cx, |this, cx| {
            if this.text() != initial {
                this.set_text(&initial, cx);
            }
        });
        let sink = signal.clone();
        cx.subscribe(entity, move |_editor, event: &EditorEvent, cx| {
            if let EditorEvent::Change(text) = event {
                sink.set_if_changed(cx, text.clone());
            }
        })
        .detach();
        // Weak handle: a strong clone would form a retain cycle with the
        // subscription above and leak both the editor and the signal.
        let editor = entity.downgrade();
        cx.observe(signal.entity(), move |observed, cx| {
            let value = observed.read(cx).clone();
            editor
                .update(cx, |this, cx| {
                    if this.text() != value {
                        this.set_text(&value, cx);
                    }
                })
                .ok();
        })
        .detach();
    }

    // ---- input handling ----

    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let ks = &event.keystroke;
        let m = ks.modifiers;
        let shift = m.shift;
        match ks.key.as_str() {
            "left" => {
                if m.platform {
                    self.model.home(shift);
                } else if m.alt {
                    self.model.word_left(shift);
                } else {
                    self.model.move_left(shift);
                }
                self.after_move(window, cx);
            }
            "right" => {
                if m.platform {
                    self.model.end(shift);
                } else if m.alt {
                    self.model.word_right(shift);
                } else {
                    self.model.move_right(shift);
                }
                self.after_move(window, cx);
            }
            "up" => {
                if m.platform {
                    self.model.doc_start(shift);
                } else {
                    self.model.move_up(shift);
                }
                self.after_move(window, cx);
            }
            "down" => {
                if m.platform {
                    self.model.doc_end(shift);
                } else {
                    self.model.move_down(shift);
                }
                self.after_move(window, cx);
            }
            "home" => {
                if m.platform {
                    self.model.doc_start(shift);
                } else {
                    self.model.home(shift);
                }
                self.after_move(window, cx);
            }
            "end" => {
                if m.platform {
                    self.model.doc_end(shift);
                } else {
                    self.model.end(shift);
                }
                self.after_move(window, cx);
            }
            "backspace" => {
                if self.read_only {
                    return;
                }
                let changed = if self.model.selection().is_some() {
                    self.model.delete_selection()
                } else if m.platform {
                    self.model.home(true);
                    self.model.delete_selection()
                } else if m.alt {
                    self.model.word_left(true);
                    self.model.delete_selection()
                } else {
                    self.model.backspace()
                };
                if changed {
                    self.after_edit(window, cx);
                } else {
                    cx.stop_propagation();
                }
            }
            "delete" => {
                if self.read_only {
                    return;
                }
                let changed = if self.model.selection().is_some() {
                    self.model.delete_selection()
                } else if m.platform {
                    self.model.end(true);
                    self.model.delete_selection()
                } else if m.alt {
                    self.model.word_right(true);
                    self.model.delete_selection()
                } else {
                    self.model.delete()
                };
                if changed {
                    self.after_edit(window, cx);
                } else {
                    cx.stop_propagation();
                }
            }
            "enter" if m.platform => {
                cx.emit(EditorEvent::Run(self.model.text()));
                cx.stop_propagation();
            }
            "enter" => {
                if self.read_only {
                    return;
                }
                self.model.newline();
                self.after_edit(window, cx);
            }
            "tab" => {
                // Cmd+Tab (and read-only Tab) bubbles so hosts keep focus moves.
                if m.platform || self.read_only {
                    return;
                }
                self.model.tab();
                self.after_edit(window, cx);
            }
            // Escape bubbles (dialogs close on it) but still drops the selection.
            "escape" => {
                if self.model.selection().is_some() {
                    self.model.clear_selection();
                    cx.notify();
                }
            }
            "a" if m.platform => {
                self.model.select_all();
                cx.notify();
                cx.stop_propagation();
            }
            "c" if m.platform => {
                if let Some(text) = self.model.copy() {
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                }
                cx.stop_propagation();
            }
            "x" if m.platform => {
                if self.read_only {
                    // Selection stays; degrade cut to copy.
                    if let Some(text) = self.model.copy() {
                        cx.write_to_clipboard(ClipboardItem::new_string(text));
                    }
                } else if let Some(text) = self.model.cut() {
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                    self.after_edit(window, cx);
                    return;
                }
                cx.stop_propagation();
            }
            "v" if m.platform => {
                if !self.read_only {
                    if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                        if !text.is_empty() {
                            self.model.insert(&text);
                            self.after_edit(window, cx);
                            return;
                        }
                    }
                }
                cx.stop_propagation();
            }
            "z" if m.platform => {
                if !self.read_only {
                    let changed = if m.shift {
                        self.model.redo()
                    } else {
                        self.model.undo()
                    };
                    if changed {
                        self.after_edit(window, cx);
                        return;
                    }
                }
                cx.stop_propagation();
            }
            _ => {
                // Printable input: never on Cmd/Ctrl chords; Option+key is
                // allowed so composed glyphs land (matches `input::apply_key`).
                if !self.read_only && !m.platform && !m.control {
                    if let Some(text) = ks.key_char.as_deref().filter(|t| !t.is_empty()) {
                        self.model.insert(text);
                        self.after_edit(window, cx);
                    }
                }
                // Everything else bubbles to the host.
            }
        }
    }

    fn on_mouse_down(&mut self, ev: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus, cx);
        let (line, col) = self.hit(ev.position, window);
        match ev.click_count {
            2 => {
                self.model.move_to(line, col, false);
                self.model.select_word();
            }
            n if n > 2 => {
                self.model.move_to(line, col, false);
                self.model.select_line();
            }
            _ => self.model.move_to(line, col, ev.modifiers.shift),
        }
        cx.notify();
    }

    fn on_drag_move(
        &mut self,
        ev: &DragMoveEvent<EditorDrag>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if ev.drag(cx).0 != cx.entity_id() {
            return;
        }
        let (line, col) = self.hit(ev.event.position, window);
        self.model.move_to(line, col, true);
        self.ensure_cursor_visible(window);
        cx.notify();
    }

    /// Shape one line with the editor's mono font. The resulting layout maps
    /// char boundaries to painted x positions (and back), so mouse math, the
    /// caret, and selection agree with the glyphs `StyledText` actually paints
    /// — including double-width CJK/emoji fallback glyphs and literal tabs.
    fn shape(&self, line: &str, window: &Window) -> ShapedLine {
        let text = SharedString::from(line.to_string());
        let run = TextRun {
            len: text.len(),
            font: self.mono_font.clone(),
            color: Hsla::default(),
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        window
            .text_system()
            .shape_line(text, px(self.font_size), &[run], None)
    }

    /// Window position -> (line, col): the line from the fixed row height,
    /// the column from the shaped line's closest char boundary. The model
    /// clamps out-of-range values.
    fn hit(&self, position: Point<Pixels>, window: &Window) -> (usize, usize) {
        let x = f32::from(position.x) - f32::from(self.text_bounds.origin.x);
        let y = f32::from(position.y) - f32::from(self.text_bounds.origin.y);
        let line = hit_line(y, self.line_h).min(self.model.line_count().saturating_sub(1));
        let Some(text) = self.model.line(line) else {
            return (line, 0);
        };
        let byte = self.shape(text, window).closest_index_for_x(px(x.max(0.0)));
        (line, col_for_byte(text, byte))
    }

    /// Painted x of the caret at char column `col` on `line`.
    fn caret_x(&self, line: &str, col: usize, window: &Window) -> f32 {
        f32::from(
            self.shape(line, window)
                .x_for_index(byte_for_col(line, col)),
        )
    }

    fn after_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.doc_dirty = true;
        cx.emit(EditorEvent::Change(self.model.text()));
        self.ensure_cursor_visible(window);
        cx.notify();
        cx.stop_propagation();
    }

    fn after_move(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.ensure_cursor_visible(window);
        cx.notify();
        cx.stop_propagation();
    }

    /// Nudge both scroll axes so the caret (plus a padding margin) is inside
    /// the viewport. No-op before the first paint.
    fn ensure_cursor_visible(&mut self, window: &Window) {
        let cursor = self.model.cursor();
        let view_h = f32::from(self.scroll.bounds().size.height);
        if view_h > 0.0 {
            let top = cursor.line as f32 * self.line_h;
            let bottom = top + self.line_h + 2.0 * PAD_Y;
            let offset = self.scroll.offset();
            let y = scroll_adjust(f32::from(offset.y), view_h, top, bottom);
            if y != f32::from(offset.y) {
                self.scroll.set_offset(point(offset.x, px(y)));
            }
        }
        let view_w = f32::from(self.hscroll.bounds().size.width);
        if view_w > 0.0 {
            let left = match self.model.line(cursor.line) {
                Some(line) => self.caret_x(line, cursor.col, window),
                None => cursor.col as f32 * self.cell_w,
            };
            let right = left + self.cell_w + 2.0 * PAD_X;
            let offset = self.hscroll.offset();
            let x = scroll_adjust(f32::from(offset.x), view_w, left, right);
            if x != f32::from(offset.x) {
                self.hscroll.set_offset(point(px(x), offset.y));
            }
        }
    }
}

impl Render for Editor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.focus.is_focused(window);

        let t = theme(cx);
        let style = self.style;
        let frame_border = if focused {
            t.primary().hsla()
        } else {
            t.border().hsla()
        };
        let edge = t.border().hsla();
        let bg = style.bg.unwrap_or_else(|| t.surface().hsla());
        let text_color = style.text.unwrap_or_else(|| t.text().hsla());
        let dimmed = style.placeholder.unwrap_or_else(|| t.dimmed().hsla());
        let caret_color = style.caret.unwrap_or_else(|| t.primary().hsla());
        let selection_bg = style.selection.unwrap_or_else(|| t.primary().alpha(0.25));
        let active_bg = style.active_line.unwrap_or_else(|| t.surface_hover().alpha(0.55));
        let gutter_fg = style.gutter_fg.unwrap_or_else(|| t.dimmed().alpha(0.7));
        let gutter_fg_active = style.gutter_fg_active.unwrap_or(text_color);
        let radius = t.radius(t.default_radius);
        let token_colors: [Hsla; 8] = self
            .token_palette
            .unwrap_or_else(|| TokenKind::ALL.map(|kind| token_color(kind, t)));

        // Resolve the mono font once per render and keep it on self: mouse
        // math shapes lines with the same font the glyphs are painted with.
        let font_size = self.font_size;
        let line_h = (font_size * 1.5).round();
        let font = Font {
            family: MONO_FAMILY.into(),
            ..window.text_style().font()
        };
        let cell_w = {
            let ts = window.text_system();
            let font_id = ts.resolve_font(&font);
            ts.ch_advance(font_id, px(font_size))
                .map(f32::from)
                .unwrap_or(font_size * 0.6)
        };
        self.line_h = line_h;
        self.cell_w = cell_w;
        self.mono_font = font.clone();

        let cursor = self.model.cursor();
        let selection = self.model.selection();
        let line_count = self.model.line_count();
        let show_placeholder = self.model.is_empty() && !focused && !self.placeholder.is_empty();
        let show_caret = focused && !self.read_only;

        let digits = line_count.to_string().len().max(2);
        let gutter_w = digits as f32 * cell_w + 2.0 * GUTTER_PAD;
        let mut max_line_w: f32 = 0.0;

        // Re-highlight only what changed, never per frame: the document
        // highlighter reparses on its dirty flag; the line cache revalidates
        // against the lines and re-tokenizes only the ones that moved.
        match &mut self.doc_highlighter {
            Some(doc) => {
                if self.doc_dirty {
                    let text = self.model.text();
                    doc.update(&text);
                    self.doc_dirty = false;
                }
            }
            None => {
                self.hl_cache.sync(&self.language, self.model.lines());
            }
        }

        let mut gutter_rows: Vec<Div> = Vec::with_capacity(line_count);
        let mut text_rows: Vec<Div> = Vec::with_capacity(line_count);
        for (i, line) in self.model.lines().iter().enumerate() {
            let is_active = i == cursor.line;

            if self.line_numbers {
                let mut gutter_row = div()
                    .relative()
                    .h(px(line_h))
                    .flex()
                    .items_center()
                    .justify_end()
                    .text_color(if is_active && focused {
                        gutter_fg_active
                    } else {
                        gutter_fg
                    })
                    .child(SharedString::from((i + 1).to_string()));
                if let Some(severity) = line_severity(&self.diagnostics, i) {
                    gutter_row = gutter_row.child(
                        div()
                            .absolute()
                            .left(px(1.0))
                            .top(px((line_h - 6.0) / 2.0))
                            .w(px(6.0))
                            .h(px(6.0))
                            .rounded_full()
                            .bg(severity.color(t)),
                    );
                }
                gutter_rows.push(gutter_row);
            }

            // Shaped once per line (cached across frames by gpui): painted
            // width, selection rects, and the caret all read real glyph
            // positions instead of assuming one uniform cell per char.
            let shaped = self.shape(line, window);
            max_line_w = max_line_w.max(f32::from(shaped.width));

            let mut row = div().relative().h(px(line_h)).w_full();
            if focused && is_active {
                row = row.bg(active_bg);
            }
            for (start, end, color) in &self.highlights {
                if let Some((s, e, newline)) =
                    line_selection(*start, *end, i, line.chars().count())
                {
                    let sx = f32::from(shaped.x_for_index(byte_for_col(line, s)));
                    let ex = f32::from(shaped.x_for_index(byte_for_col(line, e)));
                    let w = (ex - sx) + if newline { cell_w } else { 0.0 };
                    row = row.child(
                        div()
                            .absolute()
                            .top_0()
                            .bottom_0()
                            .left(px(sx))
                            .w(px(w))
                            .bg(*color),
                    );
                }
            }
            if let Some((start, end)) = selection {
                if let Some((s, e, newline)) = line_selection(start, end, i, line.chars().count()) {
                    let sx = f32::from(shaped.x_for_index(byte_for_col(line, s)));
                    let ex = f32::from(shaped.x_for_index(byte_for_col(line, e)));
                    let w = (ex - sx) + if newline { cell_w } else { 0.0 };
                    row = row.child(
                        div()
                            .absolute()
                            .top_0()
                            .bottom_0()
                            .left(px(sx))
                            .w(px(w))
                            .bg(selection_bg),
                    );
                }
            }
            // Diagnostic underlines: a 2px severity-colored bar under the
            // char range (empty range = the whole line).
            for diag in self.diagnostics.iter().filter(|d| d.line == i) {
                let chars = line.chars().count();
                let (s, e) = if diag.cols.is_empty() || diag.cols.start >= chars {
                    (0, chars)
                } else {
                    (diag.cols.start, diag.cols.end.min(chars))
                };
                let sx = f32::from(shaped.x_for_index(byte_for_col(line, s)));
                let ex = f32::from(shaped.x_for_index(byte_for_col(line, e)));
                row = row.child(
                    div()
                        .absolute()
                        .bottom(px(1.0))
                        .left(px(sx))
                        .w(px((ex - sx).max(cell_w * 0.75)))
                        .h(px(2.0))
                        .rounded(px(1.0))
                        .bg(diag.severity.color(t)),
                );
            }
            let tokens = match &self.doc_highlighter {
                Some(doc) => doc.tokens(i),
                None => self.hl_cache.tokens(i),
            };
            if !line.is_empty() {
                let runs: Vec<TextRun> = spans(line.len(), tokens)
                    .into_iter()
                    .map(|(len, kind)| TextRun {
                        len,
                        font: font.clone(),
                        color: kind.map_or(text_color, |k| token_colors[k.index()]),
                        background_color: None,
                        underline: None,
                        strikethrough: None,
                    })
                    .collect();
                row = row.child(StyledText::new(SharedString::from(line.clone())).with_runs(runs));
            } else if i == 0 && show_placeholder {
                row = row.child(div().text_color(dimmed).child(self.placeholder.clone()));
            }
            if show_caret && is_active {
                let x = f32::from(shaped.x_for_index(byte_for_col(line, cursor.col)));
                row = row.child(
                    div()
                        .absolute()
                        .top_0()
                        .h(px(line_h))
                        .left(px((x - 1.0).max(0.0)))
                        .w(px(2.0))
                        .bg(caret_color),
                );
            }
            text_rows.push(row);
        }
        let content_w = max_line_w + cell_w;

        // Invisible bounds probe: its painted origin (which moves with both
        // scroll axes) is exactly where content cell (0, 0) is, so mouse
        // hit-testing is a subtraction.
        let entity = cx.entity();
        let probe = canvas(
            move |bounds, _window, cx| {
                entity.update(cx, |this, _| this.text_bounds = bounds);
            },
            |_, _, _, _| {},
        )
        .absolute()
        .size_full();

        let lines_col = div()
            .relative()
            .flex()
            .flex_col()
            .flex_grow(1.0)
            .min_w(px(content_w))
            .whitespace_nowrap()
            .child(probe)
            .children(text_rows);

        let text_area = div()
            .id("guise-editor-text")
            .flex_1()
            .overflow_x_scroll()
            .track_scroll(&self.hscroll)
            .px(px(PAD_X))
            .child(lines_col);

        let mut content_row = div().flex().items_start().w_full().py(px(PAD_Y));
        if self.line_numbers {
            content_row = content_row.child(
                div()
                    .flex()
                    .flex_col()
                    .flex_none()
                    .w(px(gutter_w))
                    .pr(px(GUTTER_PAD))
                    .border_r_1()
                    .border_color(edge)
                    .children(gutter_rows),
            );
        }
        content_row = content_row.child(text_area);

        let mut body = div()
            .id("guise-editor-body")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_drag(EditorDrag(cx.entity_id()), |_, _, _, cx| cx.new(|_| Empty))
            .on_drag_move(cx.listener(Self::on_drag_move))
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .w_full()
            .max_h_full()
            .cursor_text()
            .child(content_row);
        if let Some(rows) = self.rows {
            body = body.min_h(px(rows as f32 * line_h + 2.0 * PAD_Y));
        }

        let mut frame = div().flex().flex_col().w_full().h_full();
        if !style.bare {
            frame = frame.rounded(px(radius)).border_1().border_color(frame_border);
        }
        frame = frame
            .bg(bg)
            .overflow_hidden()
            .font_family(MONO_FAMILY)
            .text_size(px(font_size))
            .line_height(px(line_h))
            .text_color(text_color)
            .child(body);

        // The active line's diagnostic, in a strip under the buffer (no
        // hover needed, no hit-test interference with text selection).
        if let Some(diag) = line_message(&self.diagnostics, cursor.line) {
            let accent = diag.severity.color(t);
            frame = frame.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .px(px(PAD_X))
                    .py(px(3.0))
                    .border_t_1()
                    .border_color(edge)
                    .bg(Hsla { a: 0.06, ..accent })
                    .text_size(px(font_size - 2.0))
                    .text_color(dimmed)
                    .child(div().w(px(6.0)).h(px(6.0)).rounded_full().bg(accent))
                    .child(diag.message.clone()),
            );
        }
        frame
    }
}

// ---- pure geometry helpers (unit-tested) -----------------------------------

/// Cover `len` bytes with contiguous span lengths: token ranges keep their
/// kind, gaps get `None`. Clamps overlapping or out-of-range tokens so the
/// lengths always sum to exactly `len` (a gpui `StyledText` requirement).
fn spans(len: usize, tokens: &[(Range<usize>, TokenKind)]) -> Vec<(usize, Option<TokenKind>)> {
    let mut out = Vec::new();
    let mut at = 0;
    for (range, kind) in tokens {
        let start = range.start.max(at).min(len);
        let end = range.end.max(start).min(len);
        if start > at {
            out.push((start - at, None));
        }
        if end > start {
            out.push((end - start, Some(*kind)));
        }
        at = end.max(at);
    }
    if at < len {
        out.push((len - at, None));
    }
    out
}

/// The selected char-column range on `line` for a normalized selection
/// `(start, end)`, or `None` when the selection misses the line entirely.
/// The `bool` is whether the selection continues past this line's end (draw
/// the newline as one extra cell).
fn line_selection(
    start: Pos,
    end: Pos,
    line: usize,
    line_len: usize,
) -> Option<(usize, usize, bool)> {
    if line < start.line || line > end.line {
        return None;
    }
    let s = if line == start.line {
        start.col.min(line_len)
    } else {
        0
    };
    let e = if line == end.line {
        end.col.min(line_len)
    } else {
        line_len
    };
    let e = e.max(s);
    let newline = line < end.line;
    if e == s && !newline {
        return None;
    }
    Some((s, e, newline))
}

/// Content-space y -> line row, clamping negatives to zero. Rows share one
/// fixed height, so this stays uniform math; columns go through shaping.
fn hit_line(y: f32, line_h: f32) -> usize {
    (y / line_h).floor().max(0.0) as usize
}

/// Byte offset of char column `col` in `line`, clamped to the line end.
fn byte_for_col(line: &str, col: usize) -> usize {
    line.char_indices()
        .nth(col)
        .map(|(i, _)| i)
        .unwrap_or(line.len())
}

/// Char column of byte offset `byte` in `line`. Boundary-safe: offsets inside
/// a multi-byte char count as the column of that char.
fn col_for_byte(line: &str, byte: usize) -> usize {
    line.char_indices().take_while(|&(i, _)| i < byte).count()
}

/// Adjust a scroll offset (0 or negative, more negative = scrolled further)
/// so the content range `top..bottom` is inside a `view`-long viewport.
fn scroll_adjust(offset: f32, view: f32, top: f32, bottom: f32) -> f32 {
    let mut adjusted = offset;
    if bottom + adjusted > view {
        adjusted = view - bottom;
    }
    if top + adjusted < 0.0 {
        adjusted = -top;
    }
    adjusted
}

#[cfg(test)]
mod tests {
    use super::*;

    fn total(spans: &[(usize, Option<TokenKind>)]) -> usize {
        spans.iter().map(|(len, _)| len).sum()
    }

    #[test]
    fn spans_cover_the_line_exactly() {
        let tokens = vec![(2..5, TokenKind::Keyword), (7..9, TokenKind::Number)];
        let s = spans(10, &tokens);
        assert_eq!(
            s,
            vec![
                (2, None),
                (3, Some(TokenKind::Keyword)),
                (2, None),
                (2, Some(TokenKind::Number)),
                (1, None),
            ]
        );
        assert_eq!(total(&s), 10);
    }

    #[test]
    fn spans_with_no_tokens_is_one_gap() {
        assert_eq!(spans(4, &[]), vec![(4, None)]);
        assert!(spans(0, &[]).is_empty());
    }

    #[test]
    fn spans_clamp_overlap_and_overflow() {
        // Overlapping ranges never double-cover bytes...
        let tokens = vec![(0..6, TokenKind::Keyword), (4..8, TokenKind::Number)];
        let s = spans(10, &tokens);
        assert_eq!(total(&s), 10);
        // ...and ranges past the end clamp to it.
        let tokens = vec![(8..20, TokenKind::Comment)];
        let s = spans(10, &tokens);
        assert_eq!(s, vec![(8, None), (2, Some(TokenKind::Comment))]);
    }

    #[test]
    fn spans_token_flush_to_both_edges() {
        let tokens = vec![(0..10, TokenKind::Comment)];
        assert_eq!(spans(10, &tokens), vec![(10, Some(TokenKind::Comment))]);
    }

    fn at(line: usize, col: usize) -> Pos {
        Pos::new(line, col)
    }

    #[test]
    fn selection_on_a_single_line() {
        let sel = line_selection(at(1, 2), at(1, 5), 1, 8);
        assert_eq!(sel, Some((2, 5, false)));
        assert_eq!(line_selection(at(1, 2), at(1, 5), 0, 8), None);
        assert_eq!(line_selection(at(1, 2), at(1, 5), 2, 8), None);
    }

    #[test]
    fn selection_across_lines() {
        // First line: from start.col to the end, plus the newline cell.
        assert_eq!(line_selection(at(0, 3), at(2, 2), 0, 6), Some((3, 6, true)));
        // Middle line: everything, plus the newline cell.
        assert_eq!(line_selection(at(0, 3), at(2, 2), 1, 4), Some((0, 4, true)));
        // Last line: from col 0 to end.col.
        assert_eq!(
            line_selection(at(0, 3), at(2, 2), 2, 6),
            Some((0, 2, false))
        );
    }

    #[test]
    fn selection_on_an_empty_middle_line_shows_the_newline() {
        assert_eq!(line_selection(at(0, 0), at(2, 1), 1, 0), Some((0, 0, true)));
    }

    #[test]
    fn selection_cols_clamp_to_line_len() {
        assert_eq!(line_selection(at(0, 10), at(0, 20), 0, 5), None); // both past end
        assert_eq!(
            line_selection(at(0, 2), at(0, 20), 0, 5),
            Some((2, 5, false))
        );
    }

    #[test]
    fn hit_line_maps_and_clamps() {
        assert_eq!(hit_line(0.0, 20.0), 0);
        assert_eq!(hit_line(45.0, 20.0), 2);
        assert_eq!(hit_line(-10.0, 20.0), 0); // padding clicks above the text
    }

    #[test]
    fn byte_for_col_handles_multibyte_chars() {
        assert_eq!(byte_for_col("abc", 0), 0);
        assert_eq!(byte_for_col("abc", 2), 2);
        assert_eq!(byte_for_col("abc", 9), 3); // clamps to the line end
                                               // "日本語abc": each CJK char is 3 bytes.
        assert_eq!(byte_for_col("日本語abc", 1), 3);
        assert_eq!(byte_for_col("日本語abc", 3), 9);
        assert_eq!(byte_for_col("日本語abc", 4), 10);
    }

    #[test]
    fn col_for_byte_inverts_byte_for_col() {
        let line = "日本語abc";
        for col in 0..=6 {
            assert_eq!(col_for_byte(line, byte_for_col(line, col)), col);
        }
        assert_eq!(col_for_byte(line, 999), 6); // past the end
        assert_eq!(col_for_byte("", 0), 0);
    }

    #[test]
    fn scroll_adjust_reveals_the_target() {
        // Already visible: unchanged.
        assert_eq!(scroll_adjust(-10.0, 100.0, 20.0, 40.0), -10.0);
        // Above the viewport: scroll up to the top edge.
        assert_eq!(scroll_adjust(-50.0, 100.0, 20.0, 40.0), -20.0);
        // Below the viewport: scroll down to the bottom edge.
        assert_eq!(scroll_adjust(0.0, 100.0, 150.0, 170.0), -70.0);
        // Taller than the viewport: the top wins.
        assert_eq!(scroll_adjust(0.0, 50.0, 100.0, 200.0), -100.0);
    }
}
