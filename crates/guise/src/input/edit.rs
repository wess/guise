//! Pure single-line text-editing model: a string plus a char-index cursor,
//! with the operations a text field needs. No UI — fully unit-testable; the
//! `TextInput` entity drives it from key events and renders from `split`.

/// An editable line of text with a cursor.
#[derive(Debug, Clone, Default)]
pub struct TextEdit {
    chars: Vec<char>,
    /// Cursor position as a char index in `0..=chars.len()`.
    cursor: usize,
}

impl TextEdit {
    /// Start editing `text` with the cursor at the end.
    pub fn new(text: &str) -> Self {
        let chars: Vec<char> = text.chars().collect();
        let cursor = chars.len();
        Self { chars, cursor }
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

    /// Insert `s` at the cursor, advancing past it.
    pub fn insert(&mut self, s: &str) {
        for c in s.chars() {
            self.chars.insert(self.cursor, c);
            self.cursor += 1;
        }
    }

    /// Delete the char before the cursor. Returns whether anything changed.
    pub fn backspace(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        self.cursor -= 1;
        self.chars.remove(self.cursor);
        true
    }

    /// Delete the char at the cursor. Returns whether anything changed.
    pub fn delete(&mut self) -> bool {
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
}
