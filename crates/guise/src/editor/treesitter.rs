//! Tree-sitter adapter for the editor (the `treesitter` cargo feature).
//!
//! [`TreeSitterHighlighter`] implements [`DocumentHighlighter`] over
//! `tree-sitter-highlight`: the document is parsed once per edit and served
//! back as per-line tokens. guise ships no grammars — the consumer passes a
//! grammar crate's language and highlight query, so only the languages an
//! app actually uses get compiled in:
//!
//! ```ignore
//! let rust = TreeSitterHighlighter::new(
//!     tree_sitter_rust::LANGUAGE.into(),
//!     tree_sitter_rust::HIGHLIGHTS_QUERY,
//! )?;
//! let editor = cx.new(|cx| Editor::new(cx).highlighter(rust).value(source));
//! ```
//!
//! Capture names from the query (`keyword`, `string`, `function.method`, …)
//! map onto [`TokenKind`], so the theme palette and
//! [`token_colors`](super::Editor::token_colors) overrides apply unchanged.

use std::ops::Range;

use tree_sitter::{Language, QueryError};
use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

use super::highlight::{coalesce, DocumentHighlighter, TokenKind};

/// A [`DocumentHighlighter`] backed by a tree-sitter grammar.
pub struct TreeSitterHighlighter {
    config: HighlightConfiguration,
    parser: Highlighter,
    /// `TokenKind` per capture index in the query; `None` renders unstyled.
    kinds: Vec<Option<TokenKind>>,
    /// Per-line tokens from the last [`update`](DocumentHighlighter::update).
    lines: Vec<Vec<(Range<usize>, TokenKind)>>,
}

impl TreeSitterHighlighter {
    /// Build from a grammar and its highlight query, both from the
    /// language's grammar crate (`tree_sitter_rust::LANGUAGE.into()`,
    /// `tree_sitter_rust::HIGHLIGHTS_QUERY`). Errors mean the query does not
    /// compile against the grammar — mismatched crate versions, usually.
    pub fn new(language: Language, highlights_query: &str) -> Result<Self, QueryError> {
        let mut config = HighlightConfiguration::new(language, "source", highlights_query, "", "")?;
        let names: Vec<String> = config
            .query
            .capture_names()
            .iter()
            .map(|n| n.to_string())
            .collect();
        config.configure(&names);
        let kinds = names.iter().map(|n| kind_for(n)).collect();
        Ok(Self {
            config,
            parser: Highlighter::new(),
            kinds,
            lines: Vec::new(),
        })
    }
}

impl DocumentHighlighter for TreeSitterHighlighter {
    fn update(&mut self, text: &str) {
        let starts = line_starts(text);
        let mut lines: Vec<Vec<(Range<usize>, TokenKind)>> = vec![Vec::new(); starts.len()];
        // On any parse/query failure the document renders unstyled — same
        // fallback as `Language::None`.
        let Ok(events) = self
            .parser
            .highlight(&self.config, text.as_bytes(), None, |_| None)
        else {
            self.lines = lines;
            return;
        };
        // Highlights nest (a string inside a macro body): the innermost
        // capture that maps to a kind wins.
        let mut stack: Vec<Option<TokenKind>> = Vec::new();
        let mut cur = 0;
        for event in events {
            let Ok(event) = event else { break };
            match event {
                HighlightEvent::HighlightStart(h) => {
                    stack.push(self.kinds.get(h.0).copied().flatten());
                }
                HighlightEvent::HighlightEnd => {
                    stack.pop();
                }
                HighlightEvent::Source { start, end } => {
                    let Some(kind) = stack.iter().rev().find_map(|k| *k) else {
                        continue;
                    };
                    // Source ranges arrive ascending and may span newlines;
                    // split them into per-line ranges, skipping the `\n`s.
                    let mut s = start;
                    while s < end {
                        while cur + 1 < starts.len() && starts[cur + 1] <= s {
                            cur += 1;
                        }
                        let line_start = starts[cur];
                        let line_end = starts.get(cur + 1).map_or(text.len(), |n| n - 1);
                        let e = end.min(line_end);
                        if e > s {
                            lines[cur].push((s - line_start..e - line_start, kind));
                        }
                        s = if end > line_end { line_end + 1 } else { end };
                    }
                }
            }
        }
        self.lines = lines.into_iter().map(coalesce).collect();
    }

    fn tokens(&self, line: usize) -> &[(Range<usize>, TokenKind)] {
        self.lines.get(line).map(Vec::as_slice).unwrap_or(&[])
    }
}

/// Byte offset where each line begins (always at least one line).
fn line_starts(text: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (i, b) in text.bytes().enumerate() {
        if b == b'\n' {
            starts.push(i + 1);
        }
    }
    starts
}

/// Map a highlight capture name onto the token palette. Matches on the root
/// segment (`function.method` → `function`) — the tree-sitter convention for
/// theme fallback — so one arm covers a family of captures.
fn kind_for(name: &str) -> Option<TokenKind> {
    match name.split('.').next().unwrap_or(name) {
        "keyword" | "boolean" | "conditional" | "repeat" | "include" | "exception" => {
            Some(TokenKind::Keyword)
        }
        "string" | "character" | "escape" => Some(TokenKind::StringLit),
        "number" | "float" | "constant" => Some(TokenKind::Number),
        "comment" => Some(TokenKind::Comment),
        "type" | "constructor" | "tag" | "namespace" | "module" => Some(TokenKind::Type),
        "function" | "method" | "macro" | "attribute" => Some(TokenKind::Function),
        "operator" | "punctuation" => Some(TokenKind::Punct),
        "variable" | "parameter" | "property" | "field" | "label" => Some(TokenKind::Ident),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rust() -> TreeSitterHighlighter {
        TreeSitterHighlighter::new(
            tree_sitter_rust::LANGUAGE.into(),
            tree_sitter_rust::HIGHLIGHTS_QUERY,
        )
        .expect("query compiles against the grammar")
    }

    /// The kind of the token whose range slices `line`'s text to `word`.
    fn kind_of(hl: &TreeSitterHighlighter, source: &str, line: usize, word: &str) -> TokenKind {
        let text = source.split('\n').nth(line).unwrap();
        hl.tokens(line)
            .iter()
            .find(|(r, _)| &text[r.clone()] == word)
            .map(|(_, k)| *k)
            .unwrap_or_else(|| panic!("token {word:?} not found in line {line}: {text:?}"))
    }

    #[test]
    fn rust_source_gets_tree_sitter_kinds() {
        let mut hl = rust();
        let source = "fn main() {\n    let s = \"hi\";\n    println!(\"{s}\");\n}";
        hl.update(source);
        assert_eq!(kind_of(&hl, source, 0, "fn"), TokenKind::Keyword);
        assert_eq!(kind_of(&hl, source, 0, "main"), TokenKind::Function);
        assert_eq!(kind_of(&hl, source, 1, "let"), TokenKind::Keyword);
        assert_eq!(kind_of(&hl, source, 1, "\"hi\""), TokenKind::StringLit);
        // The grammar captures the macro name and its `!` as one family, so
        // coalescing yields a single `println!` token.
        assert_eq!(kind_of(&hl, source, 2, "println!"), TokenKind::Function);
    }

    #[test]
    fn multiline_comment_covers_every_line() {
        let mut hl = rust();
        let source = "/* one\ntwo\nthree */\nfn f() {}";
        hl.update(source);
        for line in 0..3 {
            let text = source.split('\n').nth(line).unwrap();
            assert_eq!(
                hl.tokens(line),
                &[(0..text.len(), TokenKind::Comment)],
                "line {line}"
            );
        }
        assert_eq!(kind_of(&hl, source, 3, "fn"), TokenKind::Keyword);
    }

    #[test]
    fn ranges_are_ascending_in_bounds_and_char_aligned() {
        let mut hl = rust();
        let source = "let s = \"héllo 日本語\"; // café";
        hl.update(source);
        let mut at = 0;
        for (range, _) in hl.tokens(0) {
            assert!(range.start >= at, "overlapping range");
            assert!(range.end <= source.len());
            assert!(source.is_char_boundary(range.start));
            assert!(source.is_char_boundary(range.end));
            at = range.end;
        }
        assert_eq!(kind_of(&hl, source, 0, "// café"), TokenKind::Comment);
    }

    #[test]
    fn update_replaces_the_previous_parse() {
        let mut hl = rust();
        hl.update("fn a() {}\nfn b() {}");
        assert!(!hl.tokens(1).is_empty());
        hl.update("fn a() {}");
        assert!(hl.tokens(1).is_empty());
        assert!(hl.tokens(99).is_empty());
    }

    #[test]
    fn empty_document_is_fine() {
        let mut hl = rust();
        hl.update("");
        assert!(hl.tokens(0).is_empty());
    }

    #[test]
    fn capture_names_map_by_root_segment() {
        assert_eq!(kind_for("keyword"), Some(TokenKind::Keyword));
        assert_eq!(kind_for("function.method"), Some(TokenKind::Function));
        assert_eq!(kind_for("function.macro"), Some(TokenKind::Function));
        assert_eq!(kind_for("punctuation.bracket"), Some(TokenKind::Punct));
        assert_eq!(kind_for("constant.builtin"), Some(TokenKind::Number));
        assert_eq!(kind_for("type.builtin"), Some(TokenKind::Type));
        assert_eq!(kind_for("string.special"), Some(TokenKind::StringLit));
        assert_eq!(kind_for("no.such.capture"), None);
    }
}
