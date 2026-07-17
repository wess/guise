//! `MarkdownEditor` — an Obsidian-style live-preview markdown editor
//! (gpui entity).
//!
//! Every line renders formatted — headings sized, emphasis styled, list
//! markers replaced by bullets/checkboxes, fenced code highlighted — while
//! the cursor line (and any line the selection touches) *reveals* its
//! markdown syntax for editing. Text soft-wraps; list items keep a hanging
//! indent; checkboxes toggle on click; links open on Cmd+click (plain click
//! when read-only) via [`MarkdownEditorEvent::LinkClick`].
//!
//! ```ignore
//! let editor = cx.new(|cx| {
//!     MarkdownEditor::new(cx).value("# Notes\n\n- [ ] try **guise**")
//! });
//! cx.subscribe(&editor, |_this, _editor, event: &MarkdownEditorEvent, _cx| {
//!     match event {
//!         MarkdownEditorEvent::Change(text) => { /* persist */ }
//!         MarkdownEditorEvent::LinkClick(target) => { /* open */ }
//!     }
//! })
//! .detach();
//! ```

use gpui::prelude::*;
use gpui::{
    canvas, div, point, px, App, Bounds, ClipboardItem, Context, Div, DragMoveEvent, Empty, Entity,
    EntityId, EventEmitter, FocusHandle, Font, FontStyle, FontWeight, Hsla, IntoElement,
    KeyDownEvent, MouseButton, MouseDownEvent, Pixels, ScrollHandle, SharedString,
    StrikethroughStyle, TextAlign, TextRun, UnderlineStyle, Window, WrappedLine,
};

use super::block::{classify, Block, DocState};
use super::layout::{byte_for_col, col_for_byte, plan, src_for_vis, vis_for_src, RowKind, RowPlan};
use crate::editor::{token_color, EditorModel, Highlighter, Language, LineState, Pos, TokenKind};
use crate::reactive::Signal;
use crate::theme::theme;
use crate::{Glyph, IconName};

/// The monospace family for code spans and code blocks.
const MONO_FAMILY: &str = "Menlo";
/// Horizontal padding around the document, in px.
const PAD_X: f32 = 16.0;
/// Vertical padding above and below the document, in px.
const PAD_Y: f32 = 12.0;
/// Wrap width used before the first layout pass has measured the content.
const DEFAULT_WRAP: f32 = 640.0;

/// Emitted as the user edits or activates a link.
#[derive(Debug, Clone)]
pub enum MarkdownEditorEvent {
    /// The document changed. Carries the full new text.
    Change(String),
    /// A link was activated (Cmd+click, or plain click when read-only).
    /// Carries the target — a url or a wikilink page name.
    LinkClick(String),
}

/// The drag payload for selection-by-mouse; tagged with the owning entity so
/// two editors in one window never react to each other's drags.
struct MarkdownDrag(EntityId);

/// Per-editor visual overrides. Unset fields fall back to the theme-derived
/// defaults, so an empty style changes nothing.
#[derive(Clone, Copy, Default)]
pub struct MarkdownStyle {
    /// Paint no frame border and no corner radius (an embedded surface).
    pub bare: bool,
    pub bg: Option<Hsla>,
    pub text: Option<Hsla>,
    pub caret: Option<Hsla>,
    pub selection: Option<Hsla>,
    /// Links, bullets, checked boxes, quote bars.
    pub accent: Option<Hsla>,
    pub code_bg: Option<Hsla>,
    pub placeholder: Option<Hsla>,
}

/// One laid-out source line: its plan, shaped text, and geometry. Rebuilt
/// every render; mouse and caret math read the copy from the last frame.
struct Row {
    plan: RowPlan,
    /// `Rc` so the paint closure can share the shaped line with the row.
    text: Option<std::rc::Rc<WrappedLine>>,
    line_h: f32,
    pad_top: f32,
    inset: f32,
    height: f32,
    y: f32,
}

impl Row {
    fn visual_rows(&self) -> usize {
        self.text
            .as_ref()
            .map_or(1, |t| t.wrap_boundaries().len() + 1)
    }

    /// Byte offsets where the shaped line wraps, ascending.
    fn boundaries(&self) -> Vec<usize> {
        let Some(text) = &self.text else {
            return Vec::new();
        };
        text.wrap_boundaries()
            .iter()
            .map(|b| text.runs()[b.run_ix].glyphs[b.glyph_ix].index)
            .collect()
    }

    /// End-semantics position for a visible byte: a wrap-boundary byte maps
    /// to the end of the earlier visual row.
    fn pos_end(&self, vis: usize) -> (f32, f32) {
        let Some(text) = &self.text else {
            return (0.0, 0.0);
        };
        match text.position_for_index(vis.min(text.len()), px(self.line_h)) {
            Some(p) => (f32::from(p.x), f32::from(p.y)),
            None => (0.0, 0.0),
        }
    }

    /// Caret position for a visible byte: (x, visual row). Start semantics —
    /// a wrap-boundary byte belongs to the row it starts.
    fn caret(&self, vis: usize) -> (f32, usize) {
        let bounds = self.boundaries();
        let row = bounds.iter().filter(|&&b| b <= vis).count();
        if bounds.contains(&vis) {
            return (0.0, row);
        }
        let (x, y) = self.pos_end(vis);
        (
            x,
            if self.line_h > 0.0 {
                (y / self.line_h).round() as usize
            } else {
                0
            },
        )
    }

    /// Selection rectangles for the visible byte range: (x, visual row,
    /// width). `newline` extends the last rect to show a selected line end.
    fn sel_rects(&self, vs: usize, ve: usize, newline: bool, cell: f32) -> Vec<(f32, usize, f32)> {
        let bounds = self.boundaries();
        let (sr, er) = split_visual(&bounds, vs, ve);
        let (sx, _) = if bounds.contains(&vs) {
            (0.0, 0.0)
        } else {
            self.pos_end(vs)
        };
        let (ex, _) = self.pos_end(ve);
        let row_end = |r: usize| -> f32 {
            match bounds.get(r) {
                Some(&b) => self.pos_end(b).0,
                None => self.pos_end(usize::MAX).0,
            }
        };
        let mut rects = Vec::new();
        if sr == er {
            rects.push((sx, sr, (ex - sx).max(0.0)));
        } else {
            rects.push((sx, sr, (row_end(sr) - sx).max(0.0)));
            for r in sr + 1..er {
                rects.push((0.0, r, row_end(r).max(0.0)));
            }
            rects.push((0.0, er, ex.max(0.0)));
        }
        if newline {
            if let Some(last) = rects.last_mut() {
                last.2 += cell;
            }
        }
        rects.retain(|r| r.2 > 0.0);
        rects
    }
}

/// An Obsidian-style live-preview markdown editor. Create with
/// `cx.new(|cx| MarkdownEditor::new(cx))`.
///
/// The text model is the unit-tested [`EditorModel`] (char-index cursor,
/// anchor selection, coalesced undo); markdown structure comes from the pure
/// [`block`](super::block)/[`inline`](super::inline)/[`layout`](super::layout)
/// passes. Read-only editors render pure preview — selection, copy, and
/// plain-click links still work.
pub struct MarkdownEditor {
    model: EditorModel,
    placeholder: SharedString,
    read_only: bool,
    font_size: f32,
    rows: Option<usize>,
    style: MarkdownStyle,
    focus: FocusHandle,
    scroll: ScrollHandle,
    /// Window-space bounds of the document content, captured at prepaint.
    text_bounds: Bounds<Pixels>,
    /// Content width from the last frame — the wrap width for this one.
    wrap_w: f32,
    /// Measured prose cell advance, for indent units and newline cells.
    cell_w: f32,
    /// Last frame's per-line layout, for mouse/caret/scroll math.
    layout: Vec<Row>,
    /// Bring the caret into view on the next render.
    scroll_to_cursor: bool,
    /// Sticky x (content space) for visual-row vertical movement.
    goal_x: Option<f32>,
}

impl EventEmitter<MarkdownEditorEvent> for MarkdownEditor {}

impl MarkdownEditor {
    pub fn new(cx: &mut Context<Self>) -> Self {
        MarkdownEditor {
            model: EditorModel::new(""),
            placeholder: SharedString::default(),
            read_only: false,
            font_size: 15.0,
            rows: None,
            style: MarkdownStyle::default(),
            focus: cx.focus_handle(),
            scroll: ScrollHandle::new(),
            text_bounds: Bounds::default(),
            wrap_w: DEFAULT_WRAP,
            cell_w: 15.0 * 0.55,
            layout: Vec::new(),
            scroll_to_cursor: false,
            goal_x: None,
        }
    }

    // ---- builders ----

    /// Initial markdown text (`text()` is the getter).
    pub fn value(mut self, text: &str) -> Self {
        self.model.set_text(text);
        self
    }

    /// Dimmed hint shown while the document is empty and unfocused.
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Render pure preview: no caret, no edits, no syntax reveal. Selection,
    /// copy, and plain-click link activation still work.
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// Base font size in px (default 15.0). Headings scale from it.
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Minimum height, as a number of paragraph lines.
    pub fn rows(mut self, rows: usize) -> Self {
        self.rows = Some(rows);
        self
    }

    /// Spaces inserted per list indent level (default 4).
    pub fn tab_size(mut self, n: usize) -> Self {
        self.model.set_tab_size(n);
        self
    }

    /// Per-editor visual overrides (see [`MarkdownStyle`]).
    pub fn style(mut self, style: MarkdownStyle) -> Self {
        self.style = style;
        self
    }

    /// Replace the style at runtime (theme switches).
    pub fn set_style(&mut self, style: MarkdownStyle, cx: &mut Context<Self>) {
        self.style = style;
        cx.notify();
    }

    // ---- runtime API ----

    /// The current markdown text.
    pub fn text(&self) -> String {
        self.model.text()
    }

    /// Replace the document, resetting cursor, selection, and history.
    pub fn set_text(&mut self, value: &str, cx: &mut Context<Self>) {
        self.model.set_text(value);
        cx.notify();
    }

    /// The editor's focus handle, so a host can focus it on open.
    pub fn focus_handle(&self) -> FocusHandle {
        self.focus.clone()
    }

    /// Read access to the underlying [`EditorModel`] — cursor, selection,
    /// lines — for hosts building features over the buffer.
    pub fn model(&self) -> &EditorModel {
        &self.model
    }

    /// Mutate the [`EditorModel`] directly. Emits
    /// [`MarkdownEditorEvent::Change`] when the text changed, keeps the
    /// caret visible, and repaints.
    pub fn edit<R>(&mut self, cx: &mut Context<Self>, f: impl FnOnce(&mut EditorModel) -> R) -> R {
        let before = self.model.text();
        let result = f(&mut self.model);
        let after = self.model.text();
        if after != before {
            cx.emit(MarkdownEditorEvent::Change(after));
        }
        self.scroll_to_cursor = true;
        cx.notify();
        result
    }

    /// Two-way bind this editor's text to a `Signal<String>`. The signal is
    /// the source of truth; equality guards on both directions prevent
    /// update loops.
    pub fn bind(entity: &Entity<MarkdownEditor>, signal: &Signal<String>, cx: &mut App) {
        let initial = signal.get(cx);
        entity.update(cx, |this, cx| {
            if this.text() != initial {
                this.set_text(&initial, cx);
            }
        });
        let sink = signal.clone();
        cx.subscribe(entity, move |_editor, event: &MarkdownEditorEvent, cx| {
            if let MarkdownEditorEvent::Change(text) = event {
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

    // ---- markdown editing commands ----

    /// Toggle the checkbox on `line` if it is a task item, preserving the
    /// cursor. Returns whether the line was a task.
    pub fn toggle_task(&mut self, line: usize, cx: &mut Context<Self>) -> bool {
        let Some(text) = self.model.line(line) else {
            return false;
        };
        let Block::Task { checked, state, .. } = classify_alone(text) else {
            return false;
        };
        let cursor = self.model.cursor();
        self.edit(cx, |m| {
            // The marker prefix is ASCII, so `state` is also a char column.
            m.move_to(line, state, false);
            m.move_to(line, state + 1, true);
            m.insert(if checked { " " } else { "x" });
            m.move_to(cursor.line, cursor.col, false);
        });
        true
    }

    /// Wrap the selection (or the word at the cursor) in `marker`, or unwrap
    /// it when already wrapped. Single-line selections only.
    fn toggle_wrap(&mut self, marker: &str, cx: &mut Context<Self>) {
        if self.read_only {
            return;
        }
        let chars = marker.chars().count();
        if self.model.selection().is_none() {
            self.model.select_word();
        }
        let Some((start, end)) = self.model.selection() else {
            // Empty spot: insert a pair and park the cursor inside it.
            self.edit(cx, |m| {
                m.insert(&format!("{marker}{marker}"));
                for _ in 0..chars {
                    m.move_left(false);
                }
            });
            return;
        };
        if start.line != end.line {
            return;
        }
        // Selection just inside an existing pair: widen to include it.
        let line = self.model.line(start.line).unwrap_or("");
        let (sb, eb) = (byte_for_col(line, start.col), byte_for_col(line, end.col));
        if line[..sb].ends_with(marker) && line[eb..].starts_with(marker) {
            self.model.move_to(start.line, start.col - chars, false);
            self.model.move_to(end.line, end.col + chars, true);
        }
        let Some(sel) = self.model.selected_text() else {
            return;
        };
        let unwrap =
            sel.starts_with(marker) && sel.ends_with(marker) && sel.len() >= 2 * marker.len();
        let replacement = if unwrap {
            sel[marker.len()..sel.len() - marker.len()].to_string()
        } else {
            format!("{marker}{sel}{marker}")
        };
        self.edit(cx, |m| m.insert(&replacement));
    }

    /// Wrap the selection as a markdown link and park the cursor where the
    /// missing half goes (url for a selection, text otherwise).
    fn insert_link(&mut self, cx: &mut Context<Self>) {
        if self.read_only {
            return;
        }
        let single_line = matches!(self.model.selection(), Some((s, e)) if s.line == e.line);
        self.edit(cx, |m| {
            if single_line {
                let sel = m.selected_text().unwrap_or_default();
                m.insert(&format!("[{sel}]()"));
                m.move_left(false);
            } else {
                m.insert("[]()");
                for _ in 0..3 {
                    m.move_left(false);
                }
            }
        });
    }

    /// Enter: continue lists and quotes; an empty item exits the list.
    fn on_enter(&mut self, cx: &mut Context<Self>) {
        let cursor = self.model.cursor();
        let line = self.model.line(cursor.line).unwrap_or("").to_string();
        let in_code = matches!(
            self.layout.get(cursor.line).map(|r| &r.plan.kind),
            Some(RowKind::Code { .. } | RowKind::Fence { .. } | RowKind::FrontMatter)
        );
        let marker = if in_code || self.model.selection().is_some() {
            None
        } else {
            continuation(&line)
        };
        match marker {
            Some(_) if line[prefix_end(&line)..].trim().is_empty() => {
                // Enter on an empty item clears the marker instead of
                // continuing the list.
                let cols = line.chars().count();
                self.edit(cx, |m| {
                    m.move_to(cursor.line, 0, false);
                    m.move_to(cursor.line, cols, true);
                    m.delete_selection();
                });
            }
            Some(marker) => self.edit(cx, |m| {
                m.newline();
                m.insert(&marker);
            }),
            None => self.edit(cx, |m| m.newline()),
        }
    }

    /// Tab / Shift+Tab: indent or outdent list items; plain Tab elsewhere.
    fn on_tab(&mut self, outdent: bool, cx: &mut Context<Self>) {
        let cursor = self.model.cursor();
        let line = self.model.line(cursor.line).unwrap_or("").to_string();
        let is_item = matches!(
            classify_alone(&line),
            Block::Bullet { .. } | Block::Ordered { .. } | Block::Task { .. }
        );
        if !is_item {
            if !outdent {
                self.edit(cx, |m| m.tab());
            }
            return;
        }
        let n = self.model.tab_size();
        if outdent {
            let lead = line.chars().take_while(|&c| c == ' ').count().min(n);
            if lead == 0 {
                return;
            }
            self.edit(cx, |m| {
                m.move_to(cursor.line, 0, false);
                m.move_to(cursor.line, lead, true);
                m.delete_selection();
                m.move_to(cursor.line, cursor.col.saturating_sub(lead), false);
            });
        } else {
            self.edit(cx, |m| {
                m.move_to(cursor.line, 0, false);
                m.insert(&" ".repeat(n));
                m.move_to(cursor.line, cursor.col + n, false);
            });
        }
    }

    /// Backspace at the start of an item's content removes the whole marker
    /// (list or quote) instead of eating into it invisibly.
    fn backspace_marker(&mut self, cx: &mut Context<Self>) -> bool {
        let cursor = self.model.cursor();
        let line = self.model.line(cursor.line).unwrap_or("").to_string();
        let content = match classify_alone(&line) {
            Block::Bullet { content, .. }
            | Block::Ordered { content, .. }
            | Block::Task { content, .. }
            | Block::Quote { content, .. } => content,
            _ => return false,
        };
        // Marker prefixes are ASCII, so `content` is also a char column.
        if content == 0 || cursor.col != content {
            return false;
        }
        self.edit(cx, |m| {
            m.move_to(cursor.line, 0, false);
            m.move_to(cursor.line, content, true);
            m.delete_selection();
        });
        true
    }

    // ---- input handling ----

    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let ks = &event.keystroke;
        let m = ks.modifiers;
        let shift = m.shift;
        if !matches!(ks.key.as_str(), "up" | "down") {
            self.goal_x = None;
        }
        match ks.key.as_str() {
            "left" => {
                if m.platform {
                    self.model.home(shift);
                } else if m.alt {
                    self.model.word_left(shift);
                } else {
                    self.model.move_left(shift);
                }
                self.after_move(cx);
            }
            "right" => {
                if m.platform {
                    self.model.end(shift);
                } else if m.alt {
                    self.model.word_right(shift);
                } else {
                    self.model.move_right(shift);
                }
                self.after_move(cx);
            }
            "up" => {
                if m.platform {
                    self.model.doc_start(shift);
                    self.after_move(cx);
                } else {
                    self.move_visual(false, shift, cx);
                }
            }
            "down" => {
                if m.platform {
                    self.model.doc_end(shift);
                    self.after_move(cx);
                } else {
                    self.move_visual(true, shift, cx);
                }
            }
            "home" => {
                if m.platform {
                    self.model.doc_start(shift);
                } else {
                    self.model.home(shift);
                }
                self.after_move(cx);
            }
            "end" => {
                if m.platform {
                    self.model.doc_end(shift);
                } else {
                    self.model.end(shift);
                }
                self.after_move(cx);
            }
            "backspace" => {
                if self.read_only {
                    return;
                }
                if self.model.selection().is_none() && !m.platform && !m.alt {
                    if self.backspace_marker(cx) {
                        cx.stop_propagation();
                        return;
                    }
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
                    self.after_edit(cx);
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
                    self.after_edit(cx);
                } else {
                    cx.stop_propagation();
                }
            }
            "enter" if m.platform => {
                if !self.read_only && self.toggle_task(self.model.cursor().line, cx) {
                    cx.stop_propagation();
                }
            }
            "enter" => {
                if self.read_only {
                    return;
                }
                self.on_enter(cx);
                cx.stop_propagation();
            }
            "tab" => {
                if m.platform || self.read_only {
                    return;
                }
                self.on_tab(shift, cx);
                cx.stop_propagation();
            }
            // Escape bubbles (dialogs close on it) but drops the selection.
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
            "b" if m.platform => {
                self.toggle_wrap("**", cx);
                cx.stop_propagation();
            }
            "i" if m.platform => {
                self.toggle_wrap("*", cx);
                cx.stop_propagation();
            }
            "k" if m.platform => {
                self.insert_link(cx);
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
                    self.after_edit(cx);
                    return;
                }
                cx.stop_propagation();
            }
            "v" if m.platform => {
                if !self.read_only {
                    if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                        if !text.is_empty() {
                            self.model.insert(&text);
                            self.after_edit(cx);
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
                        self.after_edit(cx);
                        return;
                    }
                }
                cx.stop_propagation();
            }
            _ => {
                // Printable input: never on Cmd/Ctrl chords; Option+key is
                // allowed so composed glyphs land.
                if !self.read_only && !m.platform && !m.control {
                    if let Some(text) = ks.key_char.as_deref().filter(|t| !t.is_empty()) {
                        self.model.insert(text);
                        self.after_edit(cx);
                    }
                }
                // Everything else bubbles to the host.
            }
        }
        let _ = window;
    }

    fn on_mouse_down(&mut self, ev: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus);
        self.goal_x = None;
        let x = f32::from(ev.position.x) - f32::from(self.text_bounds.origin.x);
        let y = f32::from(ev.position.y) - f32::from(self.text_bounds.origin.y);
        let (line, col) = self.hit(x, y);

        // A click on a task's checkbox toggles it in place.
        if !self.read_only && ev.click_count == 1 {
            if let Some(row) = self.layout.get(line) {
                if let RowKind::Task { .. } = row.plan.kind {
                    let in_slot = x < row.inset && x >= 0.0;
                    let in_first_row = y >= row.y && y < row.y + row.pad_top + row.line_h;
                    if in_slot && in_first_row && self.toggle_task(line, cx) {
                        return;
                    }
                }
            }
        }
        // Cmd+click (plain click when read-only) follows links.
        if ev.modifiers.platform || self.read_only {
            if let (Some(row), Some(text)) = (self.layout.get(line), self.model.line(line)) {
                if let Some(target) = row.plan.link_at(byte_for_col(text, col)) {
                    cx.emit(MarkdownEditorEvent::LinkClick(target.to_string()));
                    return;
                }
            }
        }
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
        ev: &DragMoveEvent<MarkdownDrag>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if ev.drag(cx).0 != cx.entity_id() {
            return;
        }
        let x = f32::from(ev.event.position.x) - f32::from(self.text_bounds.origin.x);
        let y = f32::from(ev.event.position.y) - f32::from(self.text_bounds.origin.y);
        let (line, col) = self.hit(x, y);
        self.model.move_to(line, col, true);
        self.scroll_to_cursor = true;
        cx.notify();
    }

    /// Content-space position → (line, col), through the last frame's
    /// layout and each row's source↔visible mapping.
    fn hit(&self, x: f32, y: f32) -> (usize, usize) {
        if self.layout.is_empty() {
            return (0, 0);
        }
        let mut line = self.layout.len() - 1;
        for (i, row) in self.layout.iter().enumerate() {
            if y < row.y + row.height {
                line = i;
                break;
            }
        }
        line = line.min(self.model.line_count().saturating_sub(1));
        let row = &self.layout[line];
        let Some(text) = self.model.line(line) else {
            return (line, 0);
        };
        let text_h = (row.visual_rows() as f32 * row.line_h).max(row.line_h);
        let local = point(
            px((x - row.inset).max(0.0)),
            px((y - row.y - row.pad_top).clamp(0.0, text_h - 1.0)),
        );
        let vis = match &row.text {
            Some(shaped) => shaped
                .closest_index_for_position(local, px(row.line_h))
                .unwrap_or_else(|near| near),
            None => 0,
        };
        let src = src_for_vis(&row.plan.segs, vis);
        (line, col_for_byte(text, src))
    }

    /// Arrow up/down across *visual* rows, holding a sticky goal x through
    /// shorter rows — soft wrap means source-line movement would skip rows.
    fn move_visual(&mut self, down: bool, extend: bool, cx: &mut Context<Self>) {
        let cursor = self.model.cursor();
        if self.layout.len() != self.model.line_count() {
            // Stale layout (edit since last frame): plain line movement.
            if down {
                self.model.move_down(extend);
            } else {
                self.model.move_up(extend);
            }
            self.after_move(cx);
            return;
        }
        let row = &self.layout[cursor.line];
        let text = self.model.line(cursor.line).unwrap_or("");
        let vis = vis_for_src(&row.plan.segs, byte_for_col(text, cursor.col));
        let (x, vrow) = row.caret(vis);
        let goal = self.goal_x.unwrap_or(row.inset + x);
        self.goal_x = Some(goal);

        let target = if down {
            if vrow + 1 < row.visual_rows() {
                Some((cursor.line, vrow + 1))
            } else if cursor.line + 1 < self.layout.len() {
                Some((cursor.line + 1, 0))
            } else {
                None
            }
        } else if vrow > 0 {
            Some((cursor.line, vrow - 1))
        } else if cursor.line > 0 {
            Some((
                cursor.line - 1,
                self.layout[cursor.line - 1].visual_rows() - 1,
            ))
        } else {
            None
        };
        let Some((tline, tvrow)) = target else {
            if down {
                self.model.doc_end(extend);
            } else {
                self.model.doc_start(extend);
            }
            self.after_move(cx);
            return;
        };
        let trow = &self.layout[tline];
        let local = point(
            px((goal - trow.inset).max(0.0)),
            px((tvrow as f32 + 0.5) * trow.line_h),
        );
        let vis = match &trow.text {
            Some(shaped) => shaped
                .closest_index_for_position(local, px(trow.line_h))
                .unwrap_or_else(|near| near),
            None => 0,
        };
        let src = src_for_vis(&trow.plan.segs, vis);
        let col = col_for_byte(self.model.line(tline).unwrap_or(""), src);
        self.model.move_to(tline, col, extend);
        self.after_move(cx);
    }

    fn after_edit(&mut self, cx: &mut Context<Self>) {
        cx.emit(MarkdownEditorEvent::Change(self.model.text()));
        self.scroll_to_cursor = true;
        cx.notify();
        cx.stop_propagation();
    }

    fn after_move(&mut self, cx: &mut Context<Self>) {
        self.scroll_to_cursor = true;
        cx.notify();
        cx.stop_propagation();
    }

    /// Nudge the scroll offset so the caret's visual row is inside the
    /// viewport. Runs during render, right after the fresh layout is built.
    fn ensure_cursor_visible(&mut self) {
        let cursor = self.model.cursor();
        let Some(row) = self.layout.get(cursor.line) else {
            return;
        };
        let view_h = f32::from(self.scroll.bounds().size.height);
        if view_h <= 0.0 {
            return;
        }
        let text = self.model.line(cursor.line).unwrap_or("");
        let vis = vis_for_src(&row.plan.segs, byte_for_col(text, cursor.col));
        let (_, vrow) = row.caret(vis);
        let top = row.y + row.pad_top + vrow as f32 * row.line_h;
        let bottom = top + row.line_h + 2.0 * PAD_Y;
        let offset = self.scroll.offset();
        let y = scroll_adjust(f32::from(offset.y), view_h, top, bottom);
        if y != f32::from(offset.y) {
            self.scroll.set_offset(point(offset.x, px(y)));
        }
    }
}

impl Render for MarkdownEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.focus.is_focused(window);
        let base = self.font_size;

        let t = theme(cx);
        let style = self.style;
        let is_dark = t.scheme.is_dark();
        let frame_border = if focused {
            t.primary().hsla()
        } else {
            t.border().hsla()
        };
        let bg = style.bg.unwrap_or_else(|| t.surface().hsla());
        let text_color = style.text.unwrap_or_else(|| t.text().hsla());
        let dimmed = t.dimmed().hsla();
        let marker_color = t.dimmed().alpha(0.75);
        let accent = style.accent.unwrap_or_else(|| t.primary().hsla());
        let caret_color = style.caret.unwrap_or(accent);
        let selection_bg = style.selection.unwrap_or_else(|| t.primary().alpha(0.25));
        let code_bg = style
            .code_bg
            .unwrap_or_else(|| t.surface_hover().alpha(if is_dark { 0.45 } else { 0.6 }));
        let highlight_bg = t
            .color(crate::theme::ColorName::Yellow, if is_dark { 7 } else { 2 })
            .alpha(if is_dark { 0.45 } else { 0.7 });
        let placeholder_color = style.placeholder.unwrap_or_else(|| t.dimmed().hsla());
        let rule_color = t.border().hsla();
        let quote_bar = t.primary().alpha(0.55);
        let radius = t.radius(t.default_radius);
        let token_colors: [Hsla; 8] = TokenKind::ALL.map(|kind| token_color(kind, t));

        let prose = window.text_style().font();
        let mono = Font {
            family: MONO_FAMILY.into(),
            ..prose.clone()
        };
        let cell_w = {
            let ts = window.text_system();
            let font_id = ts.resolve_font(&prose);
            ts.ch_advance(font_id, px(base))
                .map(f32::from)
                .unwrap_or(base * 0.55)
        };
        self.cell_w = cell_w;
        let base_line_h = (base * 1.6).round();

        let cursor = self.model.cursor();
        let selection = self.model.selection();
        let reveal_range = match selection {
            _ if !focused || self.read_only => None,
            Some((s, e)) => Some((s.line, e.line)),
            None => Some((cursor.line, cursor.line)),
        };
        let show_caret = focused && !self.read_only;
        let show_placeholder = self.model.is_empty() && !focused && !self.placeholder.is_empty();

        // ---- build rows: classify, plan, shape ----
        let wrap_total = self.wrap_w.max(120.0);
        let mut rows: Vec<Row> = Vec::with_capacity(self.model.line_count());
        let mut doc_state = DocState::default();
        let mut hl_state = LineState::default();
        let mut y = 0.0;
        for (i, line) in self.model.lines().iter().enumerate() {
            let lang = doc_state.fence_lang().map(str::to_string);
            let block = classify(line, &mut doc_state);
            if matches!(block, Block::Fence { open: true, .. }) {
                hl_state = LineState::default();
            }
            let reveal = reveal_range.is_some_and(|(s, e)| i >= s && i <= e);
            let plan = plan(line, &block, lang.as_deref(), reveal);

            let (scale, lh, pt, pb) = metrics(&plan.kind);
            let size = (base * scale).round();
            let line_h = (size * lh).round();
            let pad_top = (base * pt).round();
            let pad_bottom = (base * pb).round();

            let inset = match &plan.kind {
                RowKind::Bullet { cols } | RowKind::Task { cols, .. } => {
                    *cols as f32 * cell_w + (base * 1.6).round()
                }
                RowKind::Ordered { cols, number } => {
                    let shaped = window.text_system().shape_line(
                        SharedString::from(format!("{number}.")),
                        px(size),
                        &[TextRun {
                            len: format!("{number}.").len(),
                            font: prose.clone(),
                            color: Hsla::default(),
                            background_color: None,
                            underline: None,
                            strikethrough: None,
                        }],
                        None,
                    );
                    *cols as f32 * cell_w + f32::from(shaped.width) + (base * 0.55).round()
                }
                RowKind::Quote { depth } => *depth as f32 * (base * 1.1).round(),
                RowKind::Code { .. } | RowKind::Fence { .. } => (base * 0.75).round(),
                _ => 0.0,
            };
            let right_pad = match &plan.kind {
                RowKind::Code { .. } | RowKind::Fence { .. } => inset,
                _ => 0.0,
            };
            let wrap = (wrap_total - inset - right_pad).max(60.0);

            let runs = if let RowKind::Code { lang } = &plan.kind {
                let language = fence_language(lang.as_deref());
                let tokens = language.line(&plan.visible, &mut hl_state);
                cover(plan.visible.len(), &tokens)
                    .into_iter()
                    .map(|(len, kind)| TextRun {
                        len,
                        font: mono.clone(),
                        color: kind.map_or(text_color, |k| token_colors[k.index()]),
                        background_color: None,
                        underline: None,
                        strikethrough: None,
                    })
                    .collect::<Vec<_>>()
            } else {
                let heading = matches!(plan.kind, RowKind::Heading(_));
                plan.runs
                    .iter()
                    .map(|run| {
                        let s = run.style;
                        let font = Font {
                            family: if s.code {
                                MONO_FAMILY.into()
                            } else {
                                prose.family.clone()
                            },
                            weight: if heading || s.bold {
                                FontWeight::BOLD
                            } else {
                                prose.weight
                            },
                            style: if s.italic {
                                FontStyle::Italic
                            } else {
                                prose.style
                            },
                            ..prose.clone()
                        };
                        let color = if run.marker {
                            marker_color
                        } else if run.dim {
                            dimmed
                        } else if s.link {
                            accent
                        } else {
                            text_color
                        };
                        TextRun {
                            len: run.len,
                            font,
                            color,
                            background_color: if s.highlight {
                                Some(highlight_bg)
                            } else if s.code {
                                Some(code_bg)
                            } else {
                                None
                            },
                            underline: (s.link && !run.marker).then(|| UnderlineStyle {
                                thickness: px(1.0),
                                color: Some(accent),
                                wavy: false,
                            }),
                            strikethrough: s.strike.then(|| StrikethroughStyle {
                                thickness: px(1.0),
                                color: Some(dimmed),
                            }),
                        }
                    })
                    .collect::<Vec<_>>()
            };

            let shaped = window
                .text_system()
                .shape_text(
                    SharedString::from(plan.visible.clone()),
                    px(size),
                    &runs,
                    Some(px(wrap)),
                    None,
                )
                .ok()
                .and_then(|mut lines| {
                    if lines.is_empty() {
                        None
                    } else {
                        Some(std::rc::Rc::new(lines.swap_remove(0)))
                    }
                });
            let visual_rows = shaped
                .as_ref()
                .map_or(1, |s| s.wrap_boundaries().len() + 1)
                .max(1);
            let height = pad_top + visual_rows as f32 * line_h + pad_bottom;
            rows.push(Row {
                plan,
                text: shaped,
                line_h,
                pad_top,
                inset,
                height,
                y,
            });
            y += height;
        }
        self.layout = rows;
        if self.scroll_to_cursor {
            self.scroll_to_cursor = false;
            self.ensure_cursor_visible();
        }

        // ---- build elements ----
        let mut row_divs: Vec<Div> = Vec::with_capacity(self.layout.len());
        for (i, row) in self.layout.iter().enumerate() {
            let size_of_row = row
                .text
                .as_ref()
                .map(|t| f32::from(t.font_size()))
                .unwrap_or(base);
            let mut el = div().relative().w_full().h(px(row.height));

            match &row.plan.kind {
                RowKind::Code { .. } => el = el.bg(code_bg),
                RowKind::Fence { open } => {
                    el = el.bg(code_bg);
                    el = if *open {
                        el.rounded_t(px(radius))
                    } else {
                        el.rounded_b(px(radius))
                    };
                }
                RowKind::Rule if !row.plan.revealed => {
                    el = el.child(
                        div()
                            .absolute()
                            .left_0()
                            .right_0()
                            .top(px((row.height / 2.0 - 1.0).max(0.0)))
                            .h(px(2.0))
                            .rounded(px(1.0))
                            .bg(rule_color),
                    );
                }
                RowKind::Quote { depth } => {
                    let step = (base * 1.1).round();
                    for k in 0..*depth {
                        el = el.child(
                            div()
                                .absolute()
                                .left(px(k as f32 * step + 1.0))
                                .top_0()
                                .bottom_0()
                                .w(px(3.0))
                                .rounded(px(1.5))
                                .bg(quote_bar),
                        );
                    }
                }
                RowKind::Bullet { .. } => {
                    let dot = (base * 0.36).round().max(4.0);
                    el = el.child(
                        div()
                            .absolute()
                            .left(px(row.inset - dot - (base * 0.6).round()))
                            .top(px(row.pad_top + (row.line_h - dot) / 2.0))
                            .size(px(dot))
                            .rounded_full()
                            .bg(marker_color),
                    );
                }
                RowKind::Ordered { number, .. } => {
                    el = el.child(
                        div()
                            .absolute()
                            .left_0()
                            .top(px(row.pad_top))
                            .w(px((row.inset - (base * 0.35)).max(0.0)))
                            .h(px(row.line_h))
                            .flex()
                            .items_center()
                            .justify_end()
                            .text_size(px(size_of_row))
                            .text_color(marker_color)
                            .child(SharedString::from(format!("{number}."))),
                    );
                }
                RowKind::Task { checked, .. } => {
                    let box_s = (size_of_row * 1.05).round();
                    let boxed = div()
                        .absolute()
                        .left(px(row.inset - box_s - (base * 0.45).round()))
                        .top(px(row.pad_top + (row.line_h - box_s) / 2.0))
                        .size(px(box_s))
                        .rounded(px(4.0))
                        .flex()
                        .items_center()
                        .justify_center();
                    el = el.child(if *checked {
                        boxed
                            .bg(accent)
                            .text_size(px(box_s * 0.8))
                            .text_color(gpui::white())
                            .child(Glyph::Lucide(IconName::Check))
                    } else {
                        boxed.border_1().border_color(dimmed)
                    });
                }
                _ => {}
            }

            // Selection rectangles.
            if let Some((start, end)) = selection {
                if let Some(text) = self.model.line(i) {
                    if let Some((s_col, e_col, newline)) =
                        line_selection(start, end, i, text.chars().count())
                    {
                        let vs = vis_for_src(&row.plan.segs, byte_for_col(text, s_col));
                        let ve = vis_for_src(&row.plan.segs, byte_for_col(text, e_col));
                        for (x, vrow, w) in row.sel_rects(vs, ve, newline, cell_w) {
                            el = el.child(
                                div()
                                    .absolute()
                                    .left(px(row.inset + x))
                                    .top(px(row.pad_top + vrow as f32 * row.line_h))
                                    .w(px(w.max(2.0)))
                                    .h(px(row.line_h))
                                    .bg(selection_bg),
                            );
                        }
                    }
                }
            }

            // The shaped text itself, painted directly so caret/selection
            // math and the glyphs always agree.
            if let Some(shaped) = row.text.clone() {
                if !row.plan.visible.is_empty() {
                    let line_h = px(row.line_h);
                    let text_h = row.visual_rows() as f32 * row.line_h;
                    el = el.child(
                        div()
                            .absolute()
                            .left(px(row.inset))
                            .top(px(row.pad_top))
                            .w(px((wrap_total - row.inset).max(60.0)))
                            .h(px(text_h))
                            .child(canvas(
                                |_, _, _| (),
                                move |bounds, _, window, cx| {
                                    let origin = bounds.origin;
                                    shaped
                                        .paint_background(
                                            origin,
                                            line_h,
                                            TextAlign::Left,
                                            None,
                                            window,
                                            cx,
                                        )
                                        .ok();
                                    shaped
                                        .paint(origin, line_h, TextAlign::Left, None, window, cx)
                                        .ok();
                                },
                            )),
                    );
                }
            }

            // Caret.
            if show_caret && i == cursor.line {
                if let Some(text) = self.model.line(i) {
                    let vis = vis_for_src(&row.plan.segs, byte_for_col(text, cursor.col));
                    let (x, vrow) = row.caret(vis);
                    el = el.child(
                        div()
                            .absolute()
                            .left(px((row.inset + x - 1.0).max(0.0)))
                            .top(px(row.pad_top + vrow as f32 * row.line_h))
                            .w(px(2.0))
                            .h(px(row.line_h))
                            .bg(caret_color),
                    );
                }
            }

            row_divs.push(el);
        }

        // Invisible bounds probe: its painted origin is content cell (0, 0)
        // and its width is next frame's wrap width.
        let entity = cx.entity();
        let probe = canvas(
            move |bounds, _window, cx| {
                entity.update(cx, |this, cx| {
                    this.text_bounds = bounds;
                    let w = f32::from(bounds.size.width);
                    if (w - this.wrap_w).abs() > 0.5 {
                        this.wrap_w = w;
                        cx.notify();
                    }
                });
            },
            |_, _, _, _| {},
        )
        .absolute()
        .size_full();

        let mut lines_col = div()
            .relative()
            .flex()
            .flex_col()
            .w_full()
            .child(probe)
            .children(row_divs);
        if show_placeholder {
            lines_col = lines_col.child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .text_color(placeholder_color)
                    .child(self.placeholder.clone()),
            );
        }

        let content = div().w_full().py(px(PAD_Y)).px(px(PAD_X)).child(lines_col);

        let mut body = div()
            .id("guise-markdown-body")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_drag(MarkdownDrag(cx.entity_id()), |_, _, _, cx| {
                cx.new(|_| Empty)
            })
            .on_drag_move(cx.listener(Self::on_drag_move))
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .w_full()
            .max_h_full()
            .cursor_text()
            .child(content);
        if let Some(rows) = self.rows {
            body = body.min_h(px(rows as f32 * base_line_h + 2.0 * PAD_Y));
        }

        let mut frame = div().flex().flex_col().w_full().h_full();
        if !style.bare {
            frame = frame
                .rounded(px(radius))
                .border_1()
                .border_color(frame_border);
        }
        frame
            .bg(bg)
            .overflow_hidden()
            .text_size(px(base))
            .line_height(px(base_line_h))
            .text_color(text_color)
            .child(body)
    }
}

// ---- pure helpers (unit-tested) ---------------------------------------------

/// Per-kind sizing: (font scale, line-height factor, pad-top ems,
/// pad-bottom ems), all relative to the base font size.
fn metrics(kind: &RowKind) -> (f32, f32, f32, f32) {
    match kind {
        RowKind::Heading(1) => (1.6, 1.3, 0.7, 0.3),
        RowKind::Heading(2) => (1.45, 1.3, 0.6, 0.25),
        RowKind::Heading(3) => (1.28, 1.3, 0.5, 0.2),
        RowKind::Heading(4) => (1.15, 1.3, 0.4, 0.15),
        RowKind::Heading(5) => (1.05, 1.3, 0.35, 0.1),
        RowKind::Heading(_) => (0.95, 1.3, 0.35, 0.1),
        RowKind::Code { .. } | RowKind::Fence { .. } | RowKind::FrontMatter | RowKind::Table => {
            (0.88, 1.55, 0.0, 0.0)
        }
        _ => (1.0, 1.6, 0.0, 0.0),
    }
}

/// Map a fence info string onto a highlighter language.
fn fence_language(lang: Option<&str>) -> Language {
    match lang {
        Some("rust" | "rs") => Language::Rust,
        Some("sql") => Language::Sql,
        Some("json" | "jsonc") => Language::Json,
        _ => Language::None,
    }
}

/// Classify a single line out of document context (fences and frontmatter
/// need the document pass; lists, quotes, and headings don't).
fn classify_alone(line: &str) -> Block {
    let mut state = DocState::default();
    classify("", &mut state);
    classify(line, &mut state)
}

/// The list/quote marker that continues `line` onto the next one, ready to
/// insert after the auto-copied indent — `None` when the line isn't a
/// continuable item.
fn continuation(line: &str) -> Option<String> {
    match classify_alone(line) {
        Block::Task { indent, .. } => {
            let ch = line.as_bytes()[indent] as char;
            Some(format!("{ch} [ ] "))
        }
        Block::Bullet { indent, .. } => {
            let ch = line.as_bytes()[indent] as char;
            Some(format!("{ch} "))
        }
        Block::Ordered { indent, number, .. } => {
            let delim = line[indent..]
                .bytes()
                .find(|b| *b == b'.' || *b == b')')
                .unwrap_or(b'.') as char;
            Some(format!("{}{delim} ", number + 1))
        }
        Block::Quote { depth, .. } => Some("> ".repeat(depth as usize)),
        _ => None,
    }
}

/// Where a continuable item's marker prefix ends (its content offset).
fn prefix_end(line: &str) -> usize {
    match classify_alone(line) {
        Block::Task { content, .. }
        | Block::Bullet { content, .. }
        | Block::Ordered { content, .. }
        | Block::Quote { content, .. } => content,
        _ => 0,
    }
}

/// The visual rows a visible byte range spans, given the wrap-boundary
/// bytes: (start row, end row). Range starts at a boundary belong to the
/// later row; range ends at a boundary belong to the earlier one.
fn split_visual(bounds: &[usize], vs: usize, ve: usize) -> (usize, usize) {
    let sr = bounds.iter().filter(|&&b| b <= vs).count();
    let er = bounds.iter().filter(|&&b| b < ve).count();
    (sr, er.max(sr))
}

/// Cover `len` bytes with contiguous span lengths: token ranges keep their
/// kind, gaps get `None`. Clamps overlap/overflow so lengths sum to `len`.
fn cover(
    len: usize,
    tokens: &[(std::ops::Range<usize>, TokenKind)],
) -> Vec<(usize, Option<TokenKind>)> {
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
/// `(start, end)`, or `None` when the selection misses the line. The `bool`
/// is whether the selection continues past this line's end.
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

    #[test]
    fn continuation_markers() {
        assert_eq!(continuation("- item"), Some("- ".into()));
        assert_eq!(continuation("* item"), Some("* ".into()));
        assert_eq!(continuation("  - item"), Some("- ".into()));
        assert_eq!(continuation("- [x] done"), Some("- [ ] ".into()));
        assert_eq!(continuation("3. third"), Some("4. ".into()));
        assert_eq!(continuation("3) third"), Some("4) ".into()));
        assert_eq!(continuation("> quoted"), Some("> ".into()));
        assert_eq!(continuation("> > deep"), Some("> > ".into()));
        assert_eq!(continuation("plain"), None);
        assert_eq!(continuation("# heading"), None);
    }

    #[test]
    fn prefix_end_finds_content() {
        assert_eq!(prefix_end("- item"), 2);
        assert_eq!(prefix_end("  - [ ] x"), 8);
        assert_eq!(prefix_end("> q"), 2);
        assert_eq!(prefix_end("plain"), 0);
    }

    #[test]
    fn split_visual_rows() {
        // No wrapping: everything is row 0.
        assert_eq!(split_visual(&[], 0, 10), (0, 0));
        // One boundary at 10: [0..10) is row 0, [10..) is row 1.
        assert_eq!(split_visual(&[10], 2, 8), (0, 0));
        assert_eq!(split_visual(&[10], 2, 15), (0, 1));
        assert_eq!(split_visual(&[10], 12, 15), (1, 1));
        // Starts at the boundary → later row; ends at it → earlier row.
        assert_eq!(split_visual(&[10], 10, 15), (1, 1));
        assert_eq!(split_visual(&[10], 2, 10), (0, 0));
        // Degenerate empty range never inverts.
        assert_eq!(split_visual(&[10], 10, 10), (1, 1));
    }

    #[test]
    fn cover_spans_exactly() {
        let tokens = vec![(2..5, TokenKind::Keyword)];
        let s = cover(10, &tokens);
        let total: usize = s.iter().map(|(len, _)| len).sum();
        assert_eq!(total, 10);
        assert_eq!(s[1], (3, Some(TokenKind::Keyword)));
        assert_eq!(cover(4, &[]), vec![(4, None)]);
    }

    #[test]
    fn heading_metrics_scale_down() {
        let (h1, ..) = metrics(&RowKind::Heading(1));
        let (h3, ..) = metrics(&RowKind::Heading(3));
        let (p, ..) = metrics(&RowKind::Paragraph);
        assert!(h1 > h3 && h3 > p);
        let (code, ..) = metrics(&RowKind::Code { lang: None });
        assert!(code < p);
    }

    #[test]
    fn fence_language_mapping() {
        assert_eq!(fence_language(Some("rust")), Language::Rust);
        assert_eq!(fence_language(Some("rs")), Language::Rust);
        assert_eq!(fence_language(Some("json")), Language::Json);
        assert_eq!(fence_language(Some("python")), Language::None);
        assert_eq!(fence_language(None), Language::None);
    }

    #[test]
    fn scroll_adjust_reveals_target() {
        assert_eq!(scroll_adjust(-10.0, 100.0, 20.0, 40.0), -10.0);
        assert_eq!(scroll_adjust(-50.0, 100.0, 20.0, 40.0), -20.0);
        assert_eq!(scroll_adjust(0.0, 100.0, 150.0, 170.0), -70.0);
    }

    fn at(line: usize, col: usize) -> Pos {
        Pos::new(line, col)
    }

    #[test]
    fn line_selection_matches_editor_semantics() {
        assert_eq!(
            line_selection(at(1, 2), at(1, 5), 1, 8),
            Some((2, 5, false))
        );
        assert_eq!(line_selection(at(0, 3), at(2, 2), 1, 4), Some((0, 4, true)));
        assert_eq!(line_selection(at(0, 3), at(2, 2), 3, 4), None);
        assert_eq!(line_selection(at(0, 0), at(2, 1), 1, 0), Some((0, 0, true)));
    }
}
