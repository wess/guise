//! Pure multiline text-editing model: the document as lines, a char-index
//! cursor, an anchor-based selection, and snapshot undo/redo with coalesced
//! typing. No UI and no gpui — fully unit-testable; the `Editor` entity
//! drives it from key/mouse events and renders from `lines()`/`selection()`.
//!
//! The multiline successor to [`TextEdit`](crate::input::TextEdit): same
//! char-index cursor and word-boundary semantics, plus selections and history.

use std::borrow::Cow;

/// A position in the document: `line` index plus `col` as a **char** index
/// (not bytes) in `0..=line_len`. Ordering is document order.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pos {
    pub line: usize,
    pub col: usize,
}

impl Pos {
    pub fn new(line: usize, col: usize) -> Self {
        Self { line, col }
    }
}

/// One undo/redo step: the document and cursor as they were before an edit.
#[derive(Debug, Clone)]
struct Snapshot {
    lines: Vec<String>,
    cursor: Pos,
}

/// A multiline editing model with cursor, selection, and undo history.
///
/// The document is a `Vec<String>` of lines (no trailing `\n` stored); an
/// empty document is one empty line. All columns are char indices, so
/// multibyte text (é, 日本語) edits correctly. Runs of single-char typing
/// coalesce into one undo step; any other edit or movement breaks the run.
#[derive(Debug, Clone)]
pub struct EditorModel {
    lines: Vec<String>,
    cursor: Pos,
    /// Selection anchor; the selection spans anchor..cursor in either order.
    anchor: Option<Pos>,
    /// Sticky column for vertical movement through short lines.
    goal_col: Option<usize>,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    /// Whether the last edit was a coalescable single-char insert.
    coalescing: bool,
    /// Spaces per tab stop for [`tab`](Self::tab).
    tab_size: usize,
}

impl Default for EditorModel {
    fn default() -> Self {
        Self::new("")
    }
}

impl EditorModel {
    /// Start editing `text` with the cursor at the document start.
    pub fn new(text: &str) -> Self {
        Self {
            lines: split_lines(text),
            cursor: Pos::default(),
            anchor: None,
            goal_col: None,
            undo: Vec::new(),
            redo: Vec::new(),
            coalescing: false,
            tab_size: 4,
        }
    }

    // ---- document access ----

    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    /// Replace the whole document, resetting cursor, selection, and history.
    pub fn set_text(&mut self, text: &str) {
        self.lines = split_lines(text);
        self.cursor = Pos::default();
        self.anchor = None;
        self.goal_col = None;
        self.undo.clear();
        self.redo.clear();
        self.coalescing = false;
    }

    pub fn is_empty(&self) -> bool {
        self.lines.len() == 1 && self.lines[0].is_empty()
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// The text of line `i`, if it exists.
    pub fn line(&self, i: usize) -> Option<&str> {
        self.lines.get(i).map(String::as_str)
    }

    /// All lines, for rendering.
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    pub fn cursor(&self) -> Pos {
        self.cursor
    }

    pub fn tab_size(&self) -> usize {
        self.tab_size
    }

    /// Spaces per tab stop (min 1, default 4).
    pub fn set_tab_size(&mut self, n: usize) {
        self.tab_size = n.max(1);
    }

    // ---- editing ----

    /// Insert `s` at the cursor, replacing the selection if there is one.
    /// Embedded newlines split lines; CRLF is normalized to `\n`.
    pub fn insert(&mut self, s: &str) {
        let s: Cow<str> = if s.contains('\r') {
            Cow::Owned(s.replace('\r', ""))
        } else {
            Cow::Borrowed(s)
        };
        if s.is_empty() {
            return;
        }
        let coalesce = self.selection().is_none() && s.chars().count() == 1 && !s.contains('\n');
        self.push_undo(coalesce);
        self.remove_selection();
        self.insert_at_cursor(&s);
        self.goal_col = None;
    }

    /// Delete the selection, or the char before the cursor (joining lines at
    /// a line start). Returns whether anything changed.
    pub fn backspace(&mut self) -> bool {
        if self.selection().is_some() {
            return self.delete_selection();
        }
        if self.cursor == Pos::default() {
            return false;
        }
        self.push_undo(false);
        let start = self.prev_pos(self.cursor);
        self.remove_range(start, self.cursor);
        self.goal_col = None;
        true
    }

    /// Delete the selection, or the char at the cursor (joining lines at a
    /// line end). Returns whether anything changed.
    pub fn delete(&mut self) -> bool {
        if self.selection().is_some() {
            return self.delete_selection();
        }
        let end = self.next_pos(self.cursor);
        if end == self.cursor {
            return false;
        }
        self.push_undo(false);
        self.remove_range(self.cursor, end);
        self.goal_col = None;
        true
    }

    /// Split the line at the cursor, auto-indenting the new line with the
    /// current line's leading whitespace (capped at the cursor column, so
    /// splitting inside the indent doesn't over-indent).
    pub fn newline(&mut self) {
        self.push_undo(false);
        self.remove_selection();
        let indent: String = self.lines[self.cursor.line]
            .chars()
            .take(self.cursor.col)
            .take_while(|c| c.is_whitespace())
            .collect();
        self.insert_at_cursor("\n");
        self.insert_at_cursor(&indent);
        self.goal_col = None;
    }

    /// Insert spaces up to the next tab stop (see [`set_tab_size`](Self::set_tab_size)).
    pub fn tab(&mut self) {
        self.push_undo(false);
        self.remove_selection();
        let n = self.tab_size - (self.cursor.col % self.tab_size);
        self.insert_at_cursor(&" ".repeat(n));
        self.goal_col = None;
    }

    // ---- movement (extend = shift held: grow the selection) ----

    pub fn move_left(&mut self, extend: bool) {
        self.goal_col = None;
        if !extend {
            if let Some((start, _)) = self.selection() {
                self.coalescing = false;
                self.anchor = None;
                self.cursor = start;
                return;
            }
        }
        self.start_move(extend);
        self.cursor = self.prev_pos(self.cursor);
    }

    pub fn move_right(&mut self, extend: bool) {
        self.goal_col = None;
        if !extend {
            if let Some((_, end)) = self.selection() {
                self.coalescing = false;
                self.anchor = None;
                self.cursor = end;
                return;
            }
        }
        self.start_move(extend);
        self.cursor = self.next_pos(self.cursor);
    }

    /// Move up one line, keeping the goal column through shorter lines.
    pub fn move_up(&mut self, extend: bool) {
        self.start_move(extend);
        let goal = self.goal_col.unwrap_or(self.cursor.col);
        if self.cursor.line > 0 {
            self.cursor.line -= 1;
            self.cursor.col = goal.min(self.line_len(self.cursor.line));
        }
        self.goal_col = Some(goal);
    }

    /// Move down one line, keeping the goal column through shorter lines.
    pub fn move_down(&mut self, extend: bool) {
        self.start_move(extend);
        let goal = self.goal_col.unwrap_or(self.cursor.col);
        if self.cursor.line + 1 < self.lines.len() {
            self.cursor.line += 1;
            self.cursor.col = goal.min(self.line_len(self.cursor.line));
        }
        self.goal_col = Some(goal);
    }

    pub fn home(&mut self, extend: bool) {
        self.start_move(extend);
        self.cursor.col = 0;
        self.goal_col = None;
    }

    pub fn end(&mut self, extend: bool) {
        self.start_move(extend);
        self.cursor.col = self.line_len(self.cursor.line);
        self.goal_col = None;
    }

    pub fn doc_start(&mut self, extend: bool) {
        self.start_move(extend);
        self.cursor = Pos::default();
        self.goal_col = None;
    }

    pub fn doc_end(&mut self, extend: bool) {
        self.start_move(extend);
        let line = self.lines.len() - 1;
        self.cursor = Pos::new(line, self.line_len(line));
        self.goal_col = None;
    }

    /// Move left to the start of the previous word (Option+Left), crossing
    /// line boundaries.
    pub fn word_left(&mut self, extend: bool) {
        self.start_move(extend);
        let mut p = self.cursor;
        while let Some(c) = self.char_before(p) {
            if is_word(c) {
                break;
            }
            p = self.prev_pos(p);
        }
        while let Some(c) = self.char_before(p) {
            if !is_word(c) {
                break;
            }
            p = self.prev_pos(p);
        }
        self.cursor = p;
        self.goal_col = None;
    }

    /// Move right past the end of the next word (Option+Right), crossing
    /// line boundaries.
    pub fn word_right(&mut self, extend: bool) {
        self.start_move(extend);
        let mut p = self.cursor;
        while let Some(c) = self.char_at(p) {
            if is_word(c) {
                break;
            }
            p = self.next_pos(p);
        }
        while let Some(c) = self.char_at(p) {
            if !is_word(c) {
                break;
            }
            p = self.next_pos(p);
        }
        self.cursor = p;
        self.goal_col = None;
    }

    // ---- mouse ----

    /// Clamp a raw (line, col) from mouse hit-testing to a valid position:
    /// line into the document, col to that line's char length.
    pub fn pos_for_click(&self, line: usize, col: usize) -> Pos {
        let line = line.min(self.lines.len() - 1);
        Pos::new(line, col.min(self.line_len(line)))
    }

    /// Move the cursor to a clicked position (clamped), extending the
    /// selection when `extend` (shift-click or drag).
    pub fn move_to(&mut self, line: usize, col: usize, extend: bool) {
        let pos = self.pos_for_click(line, col);
        self.start_move(extend);
        self.cursor = pos;
        self.goal_col = None;
    }

    // ---- selection ----

    /// The selection as a normalized (start, end) pair in document order, or
    /// `None` when there is no selection (or it is empty).
    pub fn selection(&self) -> Option<(Pos, Pos)> {
        let anchor = self.anchor?;
        if anchor == self.cursor {
            return None;
        }
        Some(if anchor < self.cursor {
            (anchor, self.cursor)
        } else {
            (self.cursor, anchor)
        })
    }

    pub fn clear_selection(&mut self) {
        self.anchor = None;
    }

    pub fn select_all(&mut self) {
        self.coalescing = false;
        self.goal_col = None;
        self.anchor = Some(Pos::default());
        let line = self.lines.len() - 1;
        self.cursor = Pos::new(line, self.line_len(line));
    }

    /// Select the word around the cursor (double-click). On a non-word char,
    /// selects just that char.
    pub fn select_word(&mut self) {
        self.coalescing = false;
        self.goal_col = None;
        let line = self.cursor.line;
        let chars: Vec<char> = self.lines[line].chars().collect();
        if chars.is_empty() {
            return;
        }
        let col = self.cursor.col.min(chars.len());
        // The word char under the cursor, or the one before at a word end.
        let seed = if col < chars.len() && is_word(chars[col]) {
            col
        } else if col > 0 && is_word(chars[col - 1]) {
            col - 1
        } else {
            let start = if col < chars.len() { col } else { col - 1 };
            self.anchor = Some(Pos::new(line, start));
            self.cursor = Pos::new(line, start + 1);
            return;
        };
        let mut start = seed;
        while start > 0 && is_word(chars[start - 1]) {
            start -= 1;
        }
        let mut end = seed + 1;
        while end < chars.len() && is_word(chars[end]) {
            end += 1;
        }
        self.anchor = Some(Pos::new(line, start));
        self.cursor = Pos::new(line, end);
    }

    /// Select the cursor's whole line (triple-click).
    pub fn select_line(&mut self) {
        self.coalescing = false;
        self.goal_col = None;
        let line = self.cursor.line;
        self.anchor = Some(Pos::new(line, 0));
        self.cursor = Pos::new(line, self.line_len(line));
    }

    /// The selected text, `None` when nothing is selected.
    pub fn selected_text(&self) -> Option<String> {
        let (start, end) = self.selection()?;
        if start.line == end.line {
            let line = &self.lines[start.line];
            return Some(line[byte_idx(line, start.col)..byte_idx(line, end.col)].to_string());
        }
        let first = &self.lines[start.line];
        let mut out = first[byte_idx(first, start.col)..].to_string();
        for line in &self.lines[start.line + 1..end.line] {
            out.push('\n');
            out.push_str(line);
        }
        let last = &self.lines[end.line];
        out.push('\n');
        out.push_str(&last[..byte_idx(last, end.col)]);
        Some(out)
    }

    /// Delete the selected text. Returns whether anything changed.
    pub fn delete_selection(&mut self) -> bool {
        if self.selection().is_none() {
            return false;
        }
        self.push_undo(false);
        self.remove_selection();
        self.goal_col = None;
        true
    }

    // ---- clipboard (the entity layer talks to the OS) ----

    /// Remove and return the selected text; `None` when nothing is selected.
    pub fn cut(&mut self) -> Option<String> {
        let text = self.selected_text()?;
        self.push_undo(false);
        self.remove_selection();
        self.goal_col = None;
        Some(text)
    }

    /// The selected text without modifying the document.
    pub fn copy(&self) -> Option<String> {
        self.selected_text()
    }

    // ---- history ----

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Revert the last edit (a coalesced typing run counts as one). Returns
    /// whether anything changed.
    pub fn undo(&mut self) -> bool {
        let Some(snap) = self.undo.pop() else {
            return false;
        };
        let lines = std::mem::replace(&mut self.lines, snap.lines);
        self.redo.push(Snapshot {
            lines,
            cursor: self.cursor,
        });
        self.cursor = snap.cursor;
        self.anchor = None;
        self.goal_col = None;
        self.coalescing = false;
        true
    }

    /// Re-apply the last undone edit. Returns whether anything changed.
    pub fn redo(&mut self) -> bool {
        let Some(snap) = self.redo.pop() else {
            return false;
        };
        let lines = std::mem::replace(&mut self.lines, snap.lines);
        self.undo.push(Snapshot {
            lines,
            cursor: self.cursor,
        });
        self.cursor = snap.cursor;
        self.anchor = None;
        self.goal_col = None;
        self.coalescing = false;
        true
    }

    // ---- internals ----

    fn line_len(&self, i: usize) -> usize {
        self.lines[i].chars().count()
    }

    /// The position one char before `p` (line boundaries count as one char).
    fn prev_pos(&self, p: Pos) -> Pos {
        if p.col > 0 {
            Pos::new(p.line, p.col - 1)
        } else if p.line > 0 {
            Pos::new(p.line - 1, self.line_len(p.line - 1))
        } else {
            p
        }
    }

    /// The position one char after `p` (line boundaries count as one char).
    fn next_pos(&self, p: Pos) -> Pos {
        if p.col < self.line_len(p.line) {
            Pos::new(p.line, p.col + 1)
        } else if p.line + 1 < self.lines.len() {
            Pos::new(p.line + 1, 0)
        } else {
            p
        }
    }

    /// The char before `p`, with `\n` at line starts; `None` at doc start.
    fn char_before(&self, p: Pos) -> Option<char> {
        if p.col > 0 {
            self.lines[p.line].chars().nth(p.col - 1)
        } else if p.line > 0 {
            Some('\n')
        } else {
            None
        }
    }

    /// The char at `p`, with `\n` at line ends; `None` at doc end.
    fn char_at(&self, p: Pos) -> Option<char> {
        if p.col < self.line_len(p.line) {
            self.lines[p.line].chars().nth(p.col)
        } else if p.line + 1 < self.lines.len() {
            Some('\n')
        } else {
            None
        }
    }

    /// Movement prologue: break undo coalescing and set/clear the anchor.
    fn start_move(&mut self, extend: bool) {
        self.coalescing = false;
        if extend {
            if self.anchor.is_none() {
                self.anchor = Some(self.cursor);
            }
        } else {
            self.anchor = None;
        }
    }

    /// Record the current state for undo unless coalescing with the previous
    /// single-char insert. Any edit invalidates the redo stack.
    fn push_undo(&mut self, coalesce: bool) {
        if !(coalesce && self.coalescing) {
            self.undo.push(Snapshot {
                lines: self.lines.clone(),
                cursor: self.cursor,
            });
        }
        self.coalescing = coalesce;
        self.redo.clear();
    }

    /// Remove the selected range if any, leaving the cursor at its start.
    fn remove_selection(&mut self) {
        if let Some((start, end)) = self.selection() {
            self.remove_range(start, end);
        }
        self.anchor = None;
    }

    /// Remove `start..end` (document order), leaving the cursor at `start`.
    fn remove_range(&mut self, start: Pos, end: Pos) {
        if start.line == end.line {
            let line = &mut self.lines[start.line];
            let a = byte_idx(line, start.col);
            let b = byte_idx(line, end.col);
            line.replace_range(a..b, "");
        } else {
            let last = &self.lines[end.line];
            let tail = last[byte_idx(last, end.col)..].to_string();
            let first = &mut self.lines[start.line];
            first.truncate(byte_idx(first, start.col));
            first.push_str(&tail);
            self.lines.drain(start.line + 1..=end.line);
        }
        self.cursor = start;
        self.anchor = None;
    }

    /// Insert already-normalized text at the cursor, advancing past it.
    fn insert_at_cursor(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        let Pos { line, col } = self.cursor;
        let at = byte_idx(&self.lines[line], col);
        if !s.contains('\n') {
            self.lines[line].insert_str(at, s);
            self.cursor.col += s.chars().count();
            return;
        }
        let tail = self.lines[line].split_off(at);
        let mut parts = s.split('\n');
        if let Some(first) = parts.next() {
            self.lines[line].push_str(first);
        }
        let mut last = line;
        for part in parts {
            last += 1;
            self.lines.insert(last, part.to_string());
        }
        self.cursor = Pos::new(last, self.line_len(last));
        self.lines[last].push_str(&tail);
    }
}

/// Byte offset of char index `col` in `line` (`line.len()` past the end).
fn byte_idx(line: &str, col: usize) -> usize {
    line.char_indices()
        .nth(col)
        .map(|(i, _)| i)
        .unwrap_or(line.len())
}

/// Document text as lines, normalizing CRLF.
fn split_lines(text: &str) -> Vec<String> {
    let text = text.replace('\r', "");
    text.split('\n').map(str::to_string).collect()
}

/// Word characters for word-wise navigation — mirrors `input::edit`.
fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(line: usize, col: usize) -> Pos {
        Pos::new(line, col)
    }

    #[test]
    fn empty_doc_is_one_empty_line() {
        let m = EditorModel::new("");
        assert_eq!(m.line_count(), 1);
        assert!(m.is_empty());
        assert_eq!(m.text(), "");
        assert_eq!(m.cursor(), at(0, 0));
    }

    #[test]
    fn text_roundtrip() {
        let m = EditorModel::new("a\nb\n\nc");
        assert_eq!(m.line_count(), 4);
        assert_eq!(m.line(2), Some(""));
        assert_eq!(m.line(4), None);
        assert_eq!(m.text(), "a\nb\n\nc");
    }

    #[test]
    fn new_normalizes_crlf() {
        let m = EditorModel::new("a\r\nb");
        assert_eq!(m.text(), "a\nb");
        assert_eq!(m.line_count(), 2);
    }

    #[test]
    fn set_text_resets_cursor_selection_and_history() {
        let mut m = EditorModel::new("abc");
        m.doc_end(false);
        m.insert("d");
        m.select_all();
        m.set_text("xyz");
        assert_eq!(m.text(), "xyz");
        assert_eq!(m.cursor(), at(0, 0));
        assert_eq!(m.selection(), None);
        assert!(!m.can_undo());
        assert!(!m.can_redo());
    }

    #[test]
    fn insert_advances_cursor() {
        let mut m = EditorModel::new("ac");
        m.move_right(false);
        m.insert("b");
        assert_eq!(m.text(), "abc");
        assert_eq!(m.cursor(), at(0, 2));
    }

    #[test]
    fn insert_multiline_splits_lines() {
        let mut m = EditorModel::new("hello world");
        m.move_to(0, 5, false);
        m.insert("\nmid\n");
        assert_eq!(m.text(), "hello\nmid\n world");
        assert_eq!(m.cursor(), at(2, 0));
    }

    #[test]
    fn insert_replaces_selection() {
        let mut m = EditorModel::new("one two three");
        m.move_to(0, 4, false);
        m.move_to(0, 7, true);
        m.insert("2");
        assert_eq!(m.text(), "one 2 three");
        assert_eq!(m.cursor(), at(0, 5));
        assert_eq!(m.selection(), None);
    }

    #[test]
    fn insert_replaces_multiline_selection() {
        let mut m = EditorModel::new("aaa\nbbb\nccc");
        m.move_to(0, 1, false);
        m.move_to(2, 2, true);
        m.insert("X");
        assert_eq!(m.text(), "aXc");
        assert_eq!(m.cursor(), at(0, 2));
    }

    #[test]
    fn backspace_within_line_and_join() {
        let mut m = EditorModel::new("ab\ncd");
        m.move_to(1, 1, false);
        assert!(m.backspace());
        assert_eq!(m.text(), "ab\nd");
        // At the line start: joins with the previous line.
        assert!(m.backspace());
        assert_eq!(m.text(), "abd");
        assert_eq!(m.cursor(), at(0, 2));
    }

    #[test]
    fn backspace_at_doc_start_is_noop() {
        let mut m = EditorModel::new("ab");
        assert!(!m.backspace());
        assert_eq!(m.text(), "ab");
        assert!(!m.can_undo());
    }

    #[test]
    fn delete_within_line_and_join() {
        let mut m = EditorModel::new("ab\ncd");
        m.move_to(0, 1, false);
        assert!(m.delete());
        assert_eq!(m.text(), "a\ncd");
        // At the line end: joins with the next line.
        assert!(m.delete());
        assert_eq!(m.text(), "acd");
        assert_eq!(m.cursor(), at(0, 1));
    }

    #[test]
    fn delete_at_doc_end_is_noop() {
        let mut m = EditorModel::new("ab");
        m.doc_end(false);
        assert!(!m.delete());
        assert!(!m.can_undo());
    }

    #[test]
    fn newline_copies_leading_whitespace() {
        let mut m = EditorModel::new("    foo");
        m.end(false);
        m.newline();
        assert_eq!(m.text(), "    foo\n    ");
        assert_eq!(m.cursor(), at(1, 4));
    }

    #[test]
    fn newline_inside_indent_caps_the_copy() {
        let mut m = EditorModel::new("    foo");
        m.move_to(0, 2, false);
        m.newline();
        assert_eq!(m.text(), "  \n    foo");
        assert_eq!(m.cursor(), at(1, 2));
    }

    #[test]
    fn newline_replaces_selection() {
        let mut m = EditorModel::new("  ab cd");
        m.move_to(0, 4, false);
        m.move_to(0, 5, true);
        m.newline();
        assert_eq!(m.text(), "  ab\n  cd");
        assert_eq!(m.cursor(), at(1, 2));
    }

    #[test]
    fn tab_advances_to_next_stop() {
        let mut m = EditorModel::new("");
        m.tab();
        assert_eq!(m.text(), "    ");
        let mut m = EditorModel::new("ab");
        m.end(false);
        m.tab();
        assert_eq!(m.text(), "ab  ");
        assert_eq!(m.cursor(), at(0, 4));
        let mut m = EditorModel::new("x");
        m.set_tab_size(2);
        m.end(false);
        m.tab();
        assert_eq!(m.text(), "x ");
    }

    #[test]
    fn horizontal_movement_crosses_lines() {
        let mut m = EditorModel::new("ab\ncd");
        m.move_to(0, 2, false);
        m.move_right(false);
        assert_eq!(m.cursor(), at(1, 0));
        m.move_left(false);
        assert_eq!(m.cursor(), at(0, 2));
        // Doc edges are no-ops.
        m.doc_start(false);
        m.move_left(false);
        assert_eq!(m.cursor(), at(0, 0));
        m.doc_end(false);
        m.move_right(false);
        assert_eq!(m.cursor(), at(1, 2));
    }

    #[test]
    fn vertical_movement_keeps_goal_column() {
        let mut m = EditorModel::new("hello\nhi\nworld!");
        m.move_to(0, 4, false);
        m.move_down(false);
        assert_eq!(m.cursor(), at(1, 2)); // clamped by the short line
        m.move_down(false);
        assert_eq!(m.cursor(), at(2, 4)); // goal column restored
        m.move_up(false);
        m.move_up(false);
        assert_eq!(m.cursor(), at(0, 4));
    }

    #[test]
    fn horizontal_movement_resets_goal_column() {
        let mut m = EditorModel::new("hello\nhi\nworld!");
        m.move_to(0, 4, false);
        m.move_down(false); // (1, 2), goal 4
        m.move_left(false); // (1, 1), goal cleared
        m.move_down(false);
        assert_eq!(m.cursor(), at(2, 1));
    }

    #[test]
    fn vertical_movement_stops_at_edges() {
        let mut m = EditorModel::new("a\nb");
        m.move_up(false);
        assert_eq!(m.cursor(), at(0, 0));
        m.doc_end(false);
        m.move_down(false);
        assert_eq!(m.cursor(), at(1, 1));
    }

    #[test]
    fn home_end_and_doc_edges() {
        let mut m = EditorModel::new("abc\ndef");
        m.move_to(1, 1, false);
        m.home(false);
        assert_eq!(m.cursor(), at(1, 0));
        m.end(false);
        assert_eq!(m.cursor(), at(1, 3));
        m.doc_start(false);
        assert_eq!(m.cursor(), at(0, 0));
        m.doc_end(false);
        assert_eq!(m.cursor(), at(1, 3));
    }

    #[test]
    fn word_movement_within_line() {
        let mut m = EditorModel::new("foo bar baz");
        m.doc_end(false);
        m.word_left(false);
        assert_eq!(m.cursor(), at(0, 8));
        m.word_left(false);
        assert_eq!(m.cursor(), at(0, 4));
        m.word_right(false);
        assert_eq!(m.cursor(), at(0, 7));
    }

    #[test]
    fn word_movement_crosses_lines() {
        let mut m = EditorModel::new("foo\nbar");
        m.move_to(1, 0, false);
        m.word_left(false);
        assert_eq!(m.cursor(), at(0, 0));
        m.word_right(false);
        assert_eq!(m.cursor(), at(0, 3));
        m.word_right(false);
        assert_eq!(m.cursor(), at(1, 3));
    }

    #[test]
    fn extend_grows_and_normalizes_selection() {
        let mut m = EditorModel::new("abcdef");
        m.move_to(0, 2, false);
        m.move_right(true);
        m.move_right(true);
        assert_eq!(m.selection(), Some((at(0, 2), at(0, 4))));
        // Extending left of the anchor still yields a normalized range.
        let mut m = EditorModel::new("abcdef");
        m.move_to(0, 4, false);
        m.move_left(true);
        m.move_left(true);
        assert_eq!(m.selection(), Some((at(0, 2), at(0, 4))));
        assert_eq!(m.selected_text().as_deref(), Some("cd"));
    }

    #[test]
    fn empty_selection_is_none() {
        let mut m = EditorModel::new("abc");
        m.move_right(true);
        m.move_left(true); // back to the anchor
        assert_eq!(m.selection(), None);
        assert_eq!(m.selected_text(), None);
    }

    #[test]
    fn plain_move_collapses_selection_to_edge() {
        let mut m = EditorModel::new("abcdef");
        m.move_to(0, 2, false);
        m.move_to(0, 4, true);
        m.move_left(false);
        assert_eq!(m.cursor(), at(0, 2));
        assert_eq!(m.selection(), None);
        let mut m = EditorModel::new("abcdef");
        m.move_to(0, 2, false);
        m.move_to(0, 4, true);
        m.move_right(false);
        assert_eq!(m.cursor(), at(0, 4));
        assert_eq!(m.selection(), None);
    }

    #[test]
    fn selected_text_multiline() {
        let mut m = EditorModel::new("aaa\nbbb\nccc");
        m.move_to(0, 1, false);
        m.move_to(2, 2, true);
        assert_eq!(m.selected_text().as_deref(), Some("aa\nbbb\ncc"));
    }

    #[test]
    fn delete_selection_joins_lines() {
        let mut m = EditorModel::new("aaa\nbbb\nccc");
        m.move_to(0, 2, false);
        m.move_to(2, 1, true);
        assert!(m.delete_selection());
        assert_eq!(m.text(), "aacc");
        assert_eq!(m.cursor(), at(0, 2));
        assert!(!m.delete_selection());
    }

    #[test]
    fn cut_and_copy() {
        let mut m = EditorModel::new("hello world");
        assert_eq!(m.copy(), None);
        assert_eq!(m.cut(), None);
        m.move_to(0, 0, false);
        m.word_right(true);
        assert_eq!(m.copy().as_deref(), Some("hello"));
        assert_eq!(m.text(), "hello world"); // copy leaves the doc alone
        assert_eq!(m.cut().as_deref(), Some("hello"));
        assert_eq!(m.text(), " world");
        assert!(m.undo());
        assert_eq!(m.text(), "hello world");
    }

    #[test]
    fn select_all_spans_document() {
        let mut m = EditorModel::new("ab\ncd");
        m.select_all();
        assert_eq!(m.selection(), Some((at(0, 0), at(1, 2))));
        assert_eq!(m.selected_text().as_deref(), Some("ab\ncd"));
        m.clear_selection();
        assert_eq!(m.selection(), None);
    }

    #[test]
    fn select_word_variants() {
        // Mid-word.
        let mut m = EditorModel::new("foo bar_baz qux");
        m.move_to(0, 6, false);
        m.select_word();
        assert_eq!(m.selected_text().as_deref(), Some("bar_baz"));
        // At a word end (cursor just past the last char).
        m.move_to(0, 3, false);
        m.select_word();
        assert_eq!(m.selected_text().as_deref(), Some("foo"));
        // On a non-word char (with no word char adjacent on the left):
        // selects just that char.
        let mut m = EditorModel::new("foo .. bar");
        m.move_to(0, 5, false);
        m.select_word();
        assert_eq!(m.selection(), Some((at(0, 5), at(0, 6))));
        // Empty line: nothing to select.
        let mut m = EditorModel::new("");
        m.select_word();
        assert_eq!(m.selection(), None);
    }

    #[test]
    fn select_line_spans_line() {
        let mut m = EditorModel::new("abc\ndef");
        m.move_to(1, 1, false);
        m.select_line();
        assert_eq!(m.selection(), Some((at(1, 0), at(1, 3))));
        assert_eq!(m.selected_text().as_deref(), Some("def"));
    }

    #[test]
    fn undo_redo_roundtrip() {
        let mut m = EditorModel::new("ab");
        m.doc_end(false);
        m.newline();
        m.insert("cd");
        assert_eq!(m.text(), "ab\ncd");
        assert!(m.undo());
        assert_eq!(m.text(), "ab\n");
        assert!(m.undo());
        assert_eq!(m.text(), "ab");
        assert_eq!(m.cursor(), at(0, 2));
        assert!(!m.undo());
        assert!(m.redo());
        assert_eq!(m.text(), "ab\n");
        assert!(m.redo());
        assert_eq!(m.text(), "ab\ncd");
        assert!(!m.redo());
    }

    #[test]
    fn typing_coalesces_into_one_undo_step() {
        let mut m = EditorModel::new("");
        m.insert("a");
        m.insert("b");
        m.insert("c");
        assert!(m.undo());
        assert_eq!(m.text(), "");
        assert!(!m.undo());
        assert!(m.redo());
        assert_eq!(m.text(), "abc");
    }

    #[test]
    fn movement_breaks_coalescing() {
        let mut m = EditorModel::new("");
        m.insert("a");
        m.insert("b");
        m.move_left(false);
        m.move_right(false);
        m.insert("c");
        assert!(m.undo());
        assert_eq!(m.text(), "ab");
        assert!(m.undo());
        assert_eq!(m.text(), "");
    }

    #[test]
    fn structural_edits_break_coalescing() {
        let mut m = EditorModel::new("");
        m.insert("a");
        m.newline();
        m.insert("b");
        assert!(m.undo());
        assert_eq!(m.text(), "a\n");
        assert!(m.undo());
        assert_eq!(m.text(), "a");
        assert!(m.undo());
        assert_eq!(m.text(), "");
        // Backspace also breaks a run.
        let mut m = EditorModel::new("");
        m.insert("a");
        m.insert("b");
        m.backspace();
        m.insert("c");
        assert!(m.undo());
        assert_eq!(m.text(), "a");
        assert!(m.undo());
        assert_eq!(m.text(), "ab");
        assert!(m.undo());
        assert_eq!(m.text(), "");
    }

    #[test]
    fn selection_replacement_is_its_own_undo_step() {
        let mut m = EditorModel::new("");
        m.insert("a");
        m.select_all();
        m.insert("b"); // single char, but it replaced a selection
        assert!(m.undo());
        assert_eq!(m.text(), "a");
        assert!(m.undo());
        assert_eq!(m.text(), "");
    }

    #[test]
    fn new_edit_clears_redo() {
        let mut m = EditorModel::new("");
        m.insert("a");
        m.undo();
        assert!(m.can_redo());
        m.insert("b");
        assert!(!m.can_redo());
        assert!(!m.redo());
        assert_eq!(m.text(), "b");
    }

    #[test]
    fn pos_for_click_clamps() {
        let m = EditorModel::new("ab\ncdef");
        assert_eq!(m.pos_for_click(0, 99), at(0, 2)); // past the line end
        assert_eq!(m.pos_for_click(9, 1), at(1, 1)); // past the last line
        assert_eq!(m.pos_for_click(9, 99), at(1, 4)); // past both
    }

    #[test]
    fn move_to_extend_selects_like_a_drag() {
        let mut m = EditorModel::new("hello\nworld");
        m.move_to(0, 1, false);
        m.move_to(1, 3, true);
        assert_eq!(m.selected_text().as_deref(), Some("ello\nwor"));
        // Dragging backwards past the anchor flips the range.
        m.move_to(0, 0, true);
        assert_eq!(m.selection(), Some((at(0, 0), at(0, 1))));
    }

    #[test]
    fn utf8_editing() {
        let mut m = EditorModel::new("café");
        m.doc_end(false);
        assert_eq!(m.cursor(), at(0, 4)); // chars, not bytes
        assert!(m.backspace());
        assert_eq!(m.text(), "caf");
        m.insert("é");
        assert_eq!(m.text(), "café");
    }

    #[test]
    fn utf8_cjk_positions() {
        let mut m = EditorModel::new("日本語\nhello");
        m.move_to(0, 1, false);
        m.insert("!");
        assert_eq!(m.text(), "日!本語\nhello");
        assert_eq!(m.cursor(), at(0, 2));
        assert!(m.delete());
        assert_eq!(m.text(), "日!語\nhello");
        // Vertical movement counts chars, and clamps clicks per line.
        m.end(false);
        m.move_down(false);
        assert_eq!(m.cursor(), at(1, 3));
        assert_eq!(m.pos_for_click(0, 99), at(0, 3));
    }

    #[test]
    fn utf8_selection_and_words() {
        let mut m = EditorModel::new("voilà café");
        m.doc_end(false);
        m.word_left(false);
        assert_eq!(m.cursor(), at(0, 6)); // é is a word char
        m.word_left(true);
        assert_eq!(m.selected_text().as_deref(), Some("voilà "));
        m.move_to(0, 8, false);
        m.select_word();
        assert_eq!(m.selected_text().as_deref(), Some("café"));
    }
}
