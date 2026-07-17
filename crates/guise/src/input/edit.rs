//! Pure single-line text-editing model: a string plus a char-index cursor,
//! with the operations a text field needs. No UI — fully unit-testable; the
//! `TextInput` entity drives it from key events and renders from `split`.

/// An editable line of text with a cursor and an optional selection.
#[derive(Debug, Clone, Default)]
pub struct TextEdit {
    chars: Vec<char>,
    /// Cursor position as a char index in `0..=chars.len()`.
    cursor: usize,
    /// Selection anchor; a selection spans `anchor..cursor` in either order.
    /// `None` means no selection.
    anchor: Option<usize>,
}

impl TextEdit {
    /// Start editing `text` with the cursor at the end.
    pub fn new(text: &str) -> Self {
        let chars: Vec<char> = text.chars().collect();
        let cursor = chars.len();
        Self {
            chars,
            cursor,
            anchor: None,
        }
    }

    pub fn text(&self) -> String {
        self.chars.iter().collect()
    }

    pub fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    pub fn len(&self) -> usize {
        self.chars.len()
    }

    /// The selected span `(start, end)` as char indices, or `None` if the
    /// selection is empty/collapsed.
    pub fn selection(&self) -> Option<(usize, usize)> {
        let a = self.anchor?;
        (a != self.cursor).then(|| (a.min(self.cursor), a.max(self.cursor)))
    }

    pub fn has_selection(&self) -> bool {
        self.selection().is_some()
    }

    /// The selected text, or `None` when nothing is selected.
    pub fn selected_text(&self) -> Option<String> {
        let (s, e) = self.selection()?;
        Some(self.chars[s..e].iter().collect())
    }

    /// Select the whole line.
    pub fn select_all(&mut self) {
        if self.chars.is_empty() {
            self.anchor = None;
            return;
        }
        self.anchor = Some(0);
        self.cursor = self.chars.len();
    }

    /// Drop any selection, keeping the cursor put.
    pub fn clear_selection(&mut self) {
        self.anchor = None;
    }

    pub fn collapse_selection_start(&mut self) -> bool {
        let Some((start, _)) = self.selection() else {
            return false;
        };
        self.cursor = start;
        self.anchor = None;
        true
    }

    pub fn collapse_selection_end(&mut self) -> bool {
        let Some((_, end)) = self.selection() else {
            return false;
        };
        self.cursor = end;
        self.anchor = None;
        true
    }

    /// Delete the selected text (if any), leaving the cursor at its start.
    /// Returns whether anything was removed.
    pub fn delete_selection(&mut self) -> bool {
        let Some((s, e)) = self.selection() else {
            return false;
        };
        self.chars.drain(s..e);
        self.cursor = s;
        self.anchor = None;
        true
    }

    /// Prepare for a cursor move: with `extend` (Shift held) anchor a selection
    /// at the current cursor if one isn't already open; otherwise drop it.
    pub fn pre_move(&mut self, extend: bool) {
        if extend {
            if self.anchor.is_none() {
                self.anchor = Some(self.cursor);
            }
        } else {
            self.anchor = None;
        }
    }

    /// The text split around the selection: `(before, selected, after)`, or
    /// `None` when nothing is selected.
    pub fn split_selection(&self) -> Option<(String, String, String)> {
        let (s, e) = self.selection()?;
        Some((
            self.chars[..s].iter().collect(),
            self.chars[s..e].iter().collect(),
            self.chars[e..].iter().collect(),
        ))
    }

    /// Insert `s` at the cursor, replacing any selection, advancing past it.
    pub fn insert(&mut self, s: &str) {
        self.delete_selection();
        for c in s.chars() {
            self.chars.insert(self.cursor, c);
            self.cursor += 1;
        }
    }

    /// Delete the selection, or the char before the cursor. Returns whether
    /// anything changed.
    pub fn backspace(&mut self) -> bool {
        if self.delete_selection() {
            return true;
        }
        if self.cursor == 0 {
            return false;
        }
        self.cursor -= 1;
        self.chars.remove(self.cursor);
        true
    }

    /// Delete the selection, or the char at the cursor. Returns whether anything
    /// changed.
    pub fn delete(&mut self) -> bool {
        if self.delete_selection() {
            return true;
        }
        if self.cursor >= self.chars.len() {
            return false;
        }
        self.chars.remove(self.cursor);
        true
    }

    pub fn left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn right(&mut self) {
        if self.cursor < self.chars.len() {
            self.cursor += 1;
        }
    }

    pub fn home(&mut self) {
        self.cursor = 0;
    }

    pub fn end(&mut self) {
        self.cursor = self.chars.len();
    }

    pub fn line_home(&mut self) {
        while self.cursor > 0 && self.chars[self.cursor - 1] != '\n' {
            self.cursor -= 1;
        }
    }

    pub fn line_end(&mut self) {
        while self.cursor < self.chars.len() && self.chars[self.cursor] != '\n' {
            self.cursor += 1;
        }
    }

    /// Move left to the start of the previous word (Option+Left on macOS).
    pub fn word_left(&mut self) {
        while self.cursor > 0 && !is_word(self.chars[self.cursor - 1]) {
            self.cursor -= 1;
        }
        while self.cursor > 0 && is_word(self.chars[self.cursor - 1]) {
            self.cursor -= 1;
        }
    }

    /// Move right past the end of the next word (Option+Right on macOS).
    pub fn word_right(&mut self) {
        let n = self.chars.len();
        while self.cursor < n && !is_word(self.chars[self.cursor]) {
            self.cursor += 1;
        }
        while self.cursor < n && is_word(self.chars[self.cursor]) {
            self.cursor += 1;
        }
    }

    /// Delete the word before the cursor (Option+Backspace). Returns whether
    /// anything changed.
    pub fn delete_word_back(&mut self) -> bool {
        if self.delete_selection() {
            return true;
        }
        let end = self.cursor;
        self.word_left();
        if self.cursor < end {
            self.chars.drain(self.cursor..end);
            true
        } else {
            false
        }
    }

    /// Delete the word after the cursor (Option+Delete). Returns whether
    /// anything changed.
    pub fn delete_word_forward(&mut self) -> bool {
        if self.delete_selection() {
            return true;
        }
        let start = self.cursor;
        let n = self.chars.len();
        let mut end = self.cursor;
        while end < n && !is_word(self.chars[end]) {
            end += 1;
        }
        while end < n && is_word(self.chars[end]) {
            end += 1;
        }
        if end > start {
            self.chars.drain(start..end);
            true
        } else {
            false
        }
    }

    /// Delete from the cursor to the line start (Cmd+Backspace). Returns
    /// whether anything changed.
    pub fn delete_to_start(&mut self) -> bool {
        if self.delete_selection() {
            return true;
        }
        if self.cursor == 0 {
            return false;
        }
        self.chars.drain(0..self.cursor);
        self.cursor = 0;
        true
    }

    /// Delete from the cursor to the line end (Cmd+Delete / Ctrl+K). Returns
    /// whether anything changed.
    pub fn delete_to_end(&mut self) -> bool {
        if self.delete_selection() {
            return true;
        }
        if self.cursor >= self.chars.len() {
            return false;
        }
        self.chars.truncate(self.cursor);
        true
    }

    /// The text before and after the cursor, for rendering a caret between.
    pub fn split(&self) -> (String, String) {
        (
            self.chars[..self.cursor].iter().collect(),
            self.chars[self.cursor..].iter().collect(),
        )
    }

    /// Move the cursor up one line, keeping the column where possible. Multiline
    /// only (single-line text has nowhere to go).
    pub fn up(&mut self) {
        self.vmove(-1);
    }

    /// Move the cursor down one line, keeping the column where possible.
    pub fn down(&mut self) {
        self.vmove(1);
    }

    /// (line, column) of the cursor, counting `\n`-separated lines.
    fn line_col(&self) -> (usize, usize) {
        let mut line = 0;
        let mut col = 0;
        for &c in &self.chars[..self.cursor] {
            if c == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
        }
        (line, col)
    }

    /// (start char index, length excluding newline) for each line.
    fn line_bounds(&self) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        let mut start = 0;
        let mut len = 0;
        for (i, &c) in self.chars.iter().enumerate() {
            if c == '\n' {
                out.push((start, len));
                start = i + 1;
                len = 0;
            } else {
                len += 1;
            }
        }
        out.push((start, len));
        out
    }

    fn vmove(&mut self, dir: isize) {
        let (line, col) = self.line_col();
        let bounds = self.line_bounds();
        let target = line as isize + dir;
        if target < 0 || target as usize >= bounds.len() {
            return;
        }
        let (start, len) = bounds[target as usize];
        self.cursor = start + col.min(len);
    }
}

/// Word characters for word-wise navigation/deletion.
fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_places_cursor_at_end() {
        let e = TextEdit::new("abc");
        assert_eq!(e.split(), ("abc".into(), "".into()));
    }

    #[test]
    fn insert_at_cursor() {
        let mut e = TextEdit::new("ac");
        e.left();
        e.insert("b");
        assert_eq!(e.text(), "abc");
        assert_eq!(e.split(), ("ab".into(), "c".into()));
    }

    #[test]
    fn backspace_and_delete() {
        let mut e = TextEdit::new("abc");
        assert!(e.backspace());
        assert_eq!(e.text(), "ab");
        e.home();
        assert!(e.delete());
        assert_eq!(e.text(), "b");
        e.home();
        assert!(!e.backspace());
        e.end();
        assert!(!e.delete());
    }

    #[test]
    fn handles_unicode() {
        let mut e = TextEdit::new("café");
        assert!(e.backspace());
        assert_eq!(e.text(), "caf");
        e.insert("é");
        assert_eq!(e.text(), "café");
    }

    #[test]
    fn vertical_movement_keeps_column() {
        // Two lines: "hello" / "hi". Cursor starts at end ("hi").
        let mut e = TextEdit::new("hello\nhi");
        // Column 2 on line 1.
        e.up();
        // Same column (2) on line 0 → between "he" and "llo".
        assert_eq!(e.split().0, "he");
        e.down();
        // Back to line 1; column clamped to its length (2) → end.
        assert_eq!(e.split(), ("hello\nhi".into(), "".into()));
    }

    #[test]
    fn vertical_movement_stops_at_edges() {
        let mut e = TextEdit::new("a\nb");
        e.home(); // line 1 has only the final char; home goes to absolute start
        e.up(); // already on first line, no-op
        assert_eq!(e.split().0, "");
    }

    #[test]
    fn word_navigation() {
        let mut e = TextEdit::new("foo bar baz");
        e.word_left();
        assert_eq!(e.split(), ("foo bar ".into(), "baz".into()));
        e.word_left();
        assert_eq!(e.split(), ("foo ".into(), "bar baz".into()));
        e.word_right();
        assert_eq!(e.split(), ("foo bar".into(), " baz".into()));
    }

    #[test]
    fn delete_word_back_and_forward() {
        let mut e = TextEdit::new("foo bar baz");
        assert!(e.delete_word_back());
        assert_eq!(e.text(), "foo bar ");
        e.home();
        assert!(e.delete_word_forward());
        assert_eq!(e.text(), " bar ");
        // Nothing before the cursor at home: no-op.
        e.home();
        assert!(!e.delete_word_back());
    }

    #[test]
    fn select_all_then_type_replaces() {
        let mut e = TextEdit::new("hello");
        e.select_all();
        assert_eq!(e.selected_text().as_deref(), Some("hello"));
        e.insert("x");
        assert_eq!(e.text(), "x");
        assert!(!e.has_selection());
    }

    #[test]
    fn shift_arrow_extends_selection() {
        let mut e = TextEdit::new("abcd");
        e.pre_move(true);
        e.left(); // select "d"
        e.pre_move(true);
        e.left(); // select "cd"
        assert_eq!(e.selected_text().as_deref(), Some("cd"));
        let (before, sel, after) = e.split_selection().unwrap();
        assert_eq!(
            (before.as_str(), sel.as_str(), after.as_str()),
            ("ab", "cd", "")
        );
    }

    #[test]
    fn plain_move_clears_selection() {
        let mut e = TextEdit::new("abcd");
        e.select_all();
        e.pre_move(false);
        e.left();
        assert!(!e.has_selection());
    }

    #[test]
    fn backspace_deletes_selection() {
        let mut e = TextEdit::new("abcd");
        e.select_all();
        assert!(e.backspace());
        assert_eq!(e.text(), "");
    }

    #[test]
    fn modified_deletes_replace_the_selection() {
        for delete in [
            TextEdit::delete_word_back as fn(&mut TextEdit) -> bool,
            TextEdit::delete_word_forward,
            TextEdit::delete_to_start,
            TextEdit::delete_to_end,
        ] {
            let mut edit = TextEdit::new("abcd");
            edit.select_all();
            assert!(delete(&mut edit));
            assert_eq!(edit.text(), "");
        }
    }

    #[test]
    fn selection_collapses_to_the_requested_edge() {
        let mut edit = TextEdit::new("abcd");
        edit.select_all();
        assert!(edit.collapse_selection_start());
        assert_eq!(edit.split().0, "");
        edit.select_all();
        assert!(edit.collapse_selection_end());
        assert_eq!(edit.split().0, "abcd");
    }

    #[test]
    fn delete_to_line_edges() {
        let mut e = TextEdit::new("hello world");
        e.home();
        e.right();
        e.right();
        assert!(e.delete_to_start());
        assert_eq!(e.text(), "llo world");
        assert!(e.delete_to_end());
        assert_eq!(e.text(), "");
        assert!(!e.delete_to_end());
    }

    #[test]
    fn line_edges_stay_on_the_current_line() {
        let mut e = TextEdit::new("one\ntwo\nthree");
        e.up();
        e.line_home();
        assert_eq!(e.split().0, "one\n");
        e.line_end();
        assert_eq!(e.split().0, "one\ntwo");
    }
}
