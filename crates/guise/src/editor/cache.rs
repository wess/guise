//! Per-line token cache for the editor's syntax highlighting.
//!
//! [`HighlightCache::sync`] revalidates against the current document: a
//! line's tokens are reused when its text and the tokenizer state entering
//! it are both unchanged, so a keystroke re-tokenizes only the edited line
//! (and later lines only when a block construct actually changed the state
//! reaching them). Validation is self-contained — no edit notifications —
//! which keeps [`EditorModel`](super::EditorModel) pure. The remaining
//! per-frame cost is one string equality per line, which is a memcmp.

use std::ops::Range;

use super::highlight::{Highlighter, LineState, TokenKind};

/// One cached line: the text it was tokenized from, the tokenizer state
/// entering and leaving it, and the tokens produced.
struct Entry {
    text: String,
    state_in: LineState,
    state_out: LineState,
    tokens: Vec<(Range<usize>, TokenKind)>,
}

/// Caches per-line highlight tokens across renders. [`sync`](Self::sync)
/// once per frame, then read [`tokens`](Self::tokens) per line.
#[derive(Default)]
pub struct HighlightCache {
    entries: Vec<Entry>,
}

impl HighlightCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Drop every entry (the highlighter changed, so no line is reusable).
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Bring the cache in line with `lines`, re-tokenizing only lines whose
    /// text or entering state changed. Returns how many lines were
    /// re-tokenized.
    pub fn sync(&mut self, highlighter: &dyn Highlighter, lines: &[String]) -> usize {
        let mut recomputed = 0;
        let mut state = LineState::default();
        for (i, line) in lines.iter().enumerate() {
            let hit = self
                .entries
                .get(i)
                .is_some_and(|e| e.state_in == state && e.text == *line);
            if hit {
                state = self.entries[i].state_out;
                continue;
            }
            let state_in = state;
            let tokens = highlighter.line(line, &mut state);
            let entry = Entry {
                text: line.clone(),
                state_in,
                state_out: state,
                tokens,
            };
            match self.entries.get_mut(i) {
                Some(slot) => *slot = entry,
                None => self.entries.push(entry),
            }
            recomputed += 1;
        }
        self.entries.truncate(lines.len());
        recomputed
    }

    /// The cached tokens for line `i` (empty when out of range). Only
    /// meaningful after a [`sync`](Self::sync) against the same lines.
    pub fn tokens(&self, i: usize) -> &[(Range<usize>, TokenKind)] {
        self.entries.get(i).map(|e| e.tokens.as_slice()).unwrap_or(&[])
    }
}

#[cfg(test)]
mod tests {
    use super::super::highlight::Language;
    use super::*;

    fn doc(lines: &[&str]) -> Vec<String> {
        lines.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn unchanged_lines_are_not_retokenized() {
        let lines = doc(&["let a = 1;", "let b = 2;", "let c = 3;"]);
        let mut cache = HighlightCache::new();
        assert_eq!(cache.sync(&Language::Rust, &lines), 3);
        assert_eq!(cache.sync(&Language::Rust, &lines), 0);
        assert!(!cache.tokens(0).is_empty());
    }

    #[test]
    fn editing_one_line_retokenizes_only_that_line() {
        let mut lines = doc(&["let a = 1;", "let b = 2;", "let c = 3;"]);
        let mut cache = HighlightCache::new();
        cache.sync(&Language::Rust, &lines);
        lines[1] = "let b = 42;".to_string();
        assert_eq!(cache.sync(&Language::Rust, &lines), 1);
        let text = &lines[1];
        let has_42 = cache
            .tokens(1)
            .iter()
            .any(|(r, k)| &text[r.clone()] == "42" && *k == TokenKind::Number);
        assert!(has_42);
    }

    #[test]
    fn opening_a_block_comment_invalidates_the_lines_below() {
        let mut lines = doc(&["let a = 1;", "let b = 2;", "let c = 3;"]);
        let mut cache = HighlightCache::new();
        cache.sync(&Language::Rust, &lines);
        // `/*` on line 0 changes the state entering lines 1 and 2, so all
        // three recompute...
        lines[0] = "/* let a = 1;".to_string();
        assert_eq!(cache.sync(&Language::Rust, &lines), 3);
        assert_eq!(cache.tokens(2), &[(0..lines[2].len(), TokenKind::Comment)]);
        // ...and closing it recomputes them again.
        lines[0] = "let a = 1;".to_string();
        assert_eq!(cache.sync(&Language::Rust, &lines), 3);
        assert_ne!(cache.tokens(2), &[(0..lines[2].len(), TokenKind::Comment)]);
    }

    #[test]
    fn state_convergence_stops_the_recompute() {
        // Both closed and open block comments leave the same state after the
        // closing line, so lines below it stay cached.
        let mut lines = doc(&["/* a", "b */", "let c = 3;", "let d = 4;"]);
        let mut cache = HighlightCache::new();
        cache.sync(&Language::Rust, &lines);
        lines[0] = "/* aa".to_string();
        assert_eq!(cache.sync(&Language::Rust, &lines), 1);
    }

    #[test]
    fn line_count_changes_truncate_and_extend() {
        let mut lines = doc(&["let a = 1;", "let b = 2;"]);
        let mut cache = HighlightCache::new();
        cache.sync(&Language::Rust, &lines);
        lines.pop();
        assert_eq!(cache.sync(&Language::Rust, &lines), 0);
        assert!(cache.tokens(1).is_empty()); // truncated
        lines.push("let z = 9;".to_string());
        lines.push("let w = 8;".to_string());
        assert_eq!(cache.sync(&Language::Rust, &lines), 2);
        assert!(!cache.tokens(2).is_empty());
    }

    #[test]
    fn clear_forces_a_full_recompute() {
        let lines = doc(&["let a = 1;"]);
        let mut cache = HighlightCache::new();
        cache.sync(&Language::Rust, &lines);
        cache.clear();
        assert!(cache.tokens(0).is_empty());
        assert_eq!(cache.sync(&Language::Rust, &lines), 1);
    }

    #[test]
    fn out_of_range_lines_are_empty() {
        let cache = HighlightCache::new();
        assert!(cache.tokens(0).is_empty());
        assert!(cache.tokens(99).is_empty());
    }
}
