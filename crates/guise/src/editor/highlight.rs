//! Pluggable, line-based syntax highlighting for the editor.
//!
//! A [`Highlighter`] tokenizes one line at a time into byte-range
//! [`TokenKind`] spans, threading a [`LineState`] through consecutive lines
//! so block comments carry across them. [`Language`] ships small
//! keyword/scanner tokenizers for Rust, SQL, and JSON, and [`token_color`]
//! maps kinds onto the active theme. Ranges are **byte** offsets into the
//! line (aligned to char boundaries), ready for gpui `TextRun` lengths.
//!
//! ```ignore
//! use guise::editor::{Highlighter, Language, LineState};
//!
//! let mut state = LineState::default();
//! for line in source.lines() {
//!     let tokens = Language::Rust.line(line, &mut state);
//!     // tokens: Vec<(Range<usize>, TokenKind)>
//! }
//! ```

use std::ops::Range;

use gpui::Hsla;

use crate::theme::{ColorName, Theme};

/// What a token is, for coloring. See [`token_color`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    Keyword,
    Ident,
    Number,
    StringLit,
    Comment,
    Punct,
    Type,
    Function,
}

impl TokenKind {
    /// Every kind, in [`index`](Self::index) order — lets a renderer resolve
    /// the theme palette once into an array.
    pub const ALL: [TokenKind; 8] = [
        TokenKind::Keyword,
        TokenKind::Ident,
        TokenKind::Number,
        TokenKind::StringLit,
        TokenKind::Comment,
        TokenKind::Punct,
        TokenKind::Type,
        TokenKind::Function,
    ];

    /// Stable index into [`ALL`](Self::ALL)-sized lookup tables.
    pub fn index(self) -> usize {
        self as usize
    }
}

/// Tokenizer state carried from one line to the next — currently the open
/// block-comment depth. Start each document with `LineState::default()`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LineState {
    block_depth: u32,
}

/// Tokenizes one line at a time. `state` carries block-comment continuation
/// across lines; feed lines in document order.
pub trait Highlighter {
    /// Tokenize `text` (a single line, no `\n`). Returned ranges are byte
    /// offsets into `text`, ascending and non-overlapping; uncovered gaps
    /// are unstyled.
    fn line(&self, text: &str, state: &mut LineState) -> Vec<(Range<usize>, TokenKind)>;
}

/// Whole-document highlighter — the seam for parse-tree backends (the
/// `treesitter` feature's `TreeSitterHighlighter`). Where [`Highlighter`]
/// re-scans line by line, an implementation parses the full document once
/// per edit and serves per-line tokens from that parse. The editor calls
/// [`update`](Self::update) only when the text changed, never per frame.
pub trait DocumentHighlighter {
    /// The document changed; reparse. `text` is the full buffer.
    fn update(&mut self, text: &str);

    /// Tokens for line `i` of the text last passed to
    /// [`update`](Self::update): ascending, non-overlapping byte ranges into
    /// that line, shaped like [`Highlighter::line`] output. Empty for
    /// out-of-range lines.
    fn tokens(&self, line: usize) -> &[(Range<usize>, TokenKind)];
}

/// Built-in languages with keyword/scanner tokenizers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Language {
    /// No highlighting — every line is plain text.
    #[default]
    None,
    Rust,
    Sql,
    Json,
    Toml,
    Python,
    JavaScript,
    TypeScript,
    Go,
    C,
    /// Line-structure highlighting: headings, quotes, fences, `code` spans.
    Markdown,
}

impl Highlighter for Language {
    fn line(&self, text: &str, state: &mut LineState) -> Vec<(Range<usize>, TokenKind)> {
        match self {
            Language::None => Vec::new(),
            Language::Rust => tokenize(&RUST, text, state),
            Language::Sql => tokenize(&SQL, text, state),
            Language::Json => tokenize(&JSON, text, state),
            Language::Toml => tokenize(&TOML, text, state),
            Language::Python => tokenize(&PYTHON, text, state),
            Language::JavaScript => tokenize(&JAVASCRIPT, text, state),
            Language::TypeScript => tokenize(&TYPESCRIPT, text, state),
            Language::Go => tokenize(&GO, text, state),
            Language::C => tokenize(&C, text, state),
            Language::Markdown => markdown_line(text, state),
        }
    }
}

/// The theme color for a token kind (light/dark aware). Comments use the
/// dimmed text color; identifiers and punctuation the normal text color.
pub fn token_color(kind: TokenKind, t: &Theme) -> Hsla {
    let shade = if t.scheme.is_dark() { 4 } else { 7 };
    match kind {
        TokenKind::Keyword => t.color(ColorName::Violet, shade).hsla(),
        TokenKind::StringLit => t.color(ColorName::Green, shade).hsla(),
        TokenKind::Number => t.color(ColorName::Orange, shade).hsla(),
        TokenKind::Type => t.color(ColorName::Teal, shade).hsla(),
        TokenKind::Function => t.color(ColorName::Blue, shade).hsla(),
        TokenKind::Comment => t.dimmed().hsla(),
        TokenKind::Ident | TokenKind::Punct => t.text().hsla(),
    }
}

// ---- scanner ---------------------------------------------------------------

/// How a quoted string escapes its own quote char.
#[derive(Clone, Copy)]
enum Escape {
    /// `\"` (Rust, JSON).
    Backslash,
    /// `''` (SQL).
    Doubled,
}

/// A language description the shared scanner runs over.
struct Syntax {
    line_comment: Option<&'static str>,
    block_comment: Option<(&'static str, &'static str)>,
    /// Whether block comments nest (Rust) or not (SQL).
    nested_blocks: bool,
    strings: &'static [(char, Escape)],
    keywords: &'static [&'static str],
    types: &'static [&'static str],
    /// Match keywords/types case-insensitively (SQL).
    case_insensitive: bool,
    /// Idents starting uppercase are types (Rust).
    uppercase_types: bool,
}

const RUST: Syntax = Syntax {
    line_comment: Some("//"),
    block_comment: Some(("/*", "*/")),
    nested_blocks: true,
    strings: &[('"', Escape::Backslash)],
    keywords: &[
        "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum",
        "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move",
        "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait",
        "true", "type", "union", "unsafe", "use", "where", "while",
    ],
    types: &[],
    case_insensitive: false,
    uppercase_types: true,
};

const SQL: Syntax = Syntax {
    line_comment: Some("--"),
    block_comment: Some(("/*", "*/")),
    nested_blocks: false,
    strings: &[('\'', Escape::Doubled)],
    keywords: &[
        "add",
        "all",
        "alter",
        "and",
        "as",
        "asc",
        "begin",
        "between",
        "by",
        "case",
        "cast",
        "check",
        "column",
        "commit",
        "constraint",
        "create",
        "cross",
        "default",
        "delete",
        "desc",
        "distinct",
        "drop",
        "else",
        "end",
        "exists",
        "false",
        "foreign",
        "from",
        "full",
        "group",
        "having",
        "if",
        "in",
        "index",
        "inner",
        "insert",
        "into",
        "is",
        "join",
        "key",
        "left",
        "like",
        "limit",
        "not",
        "null",
        "offset",
        "on",
        "or",
        "order",
        "outer",
        "primary",
        "references",
        "replace",
        "returning",
        "right",
        "rollback",
        "select",
        "set",
        "table",
        "then",
        "transaction",
        "true",
        "union",
        "unique",
        "update",
        "values",
        "view",
        "when",
        "where",
        "with",
    ],
    types: &[
        "bigint",
        "blob",
        "bool",
        "boolean",
        "bytea",
        "char",
        "date",
        "decimal",
        "double",
        "float",
        "int",
        "integer",
        "interval",
        "json",
        "jsonb",
        "numeric",
        "real",
        "serial",
        "smallint",
        "text",
        "time",
        "timestamp",
        "timestamptz",
        "uuid",
        "varchar",
    ],
    case_insensitive: true,
    uppercase_types: false,
};

const JSON: Syntax = Syntax {
    line_comment: None,
    block_comment: None,
    nested_blocks: false,
    strings: &[('"', Escape::Backslash)],
    keywords: &["false", "null", "true"],
    types: &[],
    case_insensitive: false,
    uppercase_types: false,
};

const TOML: Syntax = Syntax {
    line_comment: Some("#"),
    block_comment: None,
    nested_blocks: false,
    strings: &[('"', Escape::Backslash), ('\'', Escape::Backslash)],
    keywords: &["false", "true"],
    types: &[],
    case_insensitive: false,
    uppercase_types: false,
};

const PYTHON: Syntax = Syntax {
    line_comment: Some("#"),
    block_comment: None,
    nested_blocks: false,
    strings: &[('"', Escape::Backslash), ('\'', Escape::Backslash)],
    keywords: &[
        "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class",
        "continue", "def", "del", "elif", "else", "except", "finally", "for", "from", "global",
        "if", "import", "in", "is", "lambda", "match", "nonlocal", "not", "or", "pass", "raise",
        "return", "try", "while", "with", "yield",
    ],
    types: &["bool", "bytes", "dict", "float", "int", "list", "set", "str", "tuple"],
    case_insensitive: false,
    uppercase_types: true,
};

const JAVASCRIPT: Syntax = Syntax {
    line_comment: Some("//"),
    block_comment: Some(("/*", "*/")),
    nested_blocks: false,
    strings: &[
        ('"', Escape::Backslash),
        ('\'', Escape::Backslash),
        ('`', Escape::Backslash),
    ],
    keywords: &[
        "async", "await", "break", "case", "catch", "class", "const", "continue", "debugger",
        "default", "delete", "do", "else", "export", "extends", "false", "finally", "for",
        "function", "if", "import", "in", "instanceof", "let", "new", "null", "of", "return",
        "static", "super", "switch", "this", "throw", "true", "try", "typeof", "undefined",
        "var", "void", "while", "with", "yield",
    ],
    types: &[],
    case_insensitive: false,
    uppercase_types: true,
};

const TYPESCRIPT: Syntax = Syntax {
    line_comment: Some("//"),
    block_comment: Some(("/*", "*/")),
    nested_blocks: false,
    strings: &[
        ('"', Escape::Backslash),
        ('\'', Escape::Backslash),
        ('`', Escape::Backslash),
    ],
    keywords: &[
        "abstract", "as", "async", "await", "break", "case", "catch", "class", "const",
        "continue", "debugger", "declare", "default", "delete", "do", "else", "enum", "export",
        "extends", "false", "finally", "for", "function", "if", "implements", "import", "in",
        "infer", "instanceof", "interface", "is", "keyof", "let", "namespace", "new", "null",
        "of", "readonly", "return", "satisfies", "static", "super", "switch", "this", "throw",
        "true", "try", "type", "typeof", "undefined", "var", "void", "while", "with", "yield",
    ],
    types: &["any", "bigint", "boolean", "never", "number", "object", "string", "symbol", "unknown", "void"],
    case_insensitive: false,
    uppercase_types: true,
};

const GO: Syntax = Syntax {
    line_comment: Some("//"),
    block_comment: Some(("/*", "*/")),
    nested_blocks: false,
    strings: &[('"', Escape::Backslash), ('`', Escape::Backslash)],
    keywords: &[
        "break", "case", "chan", "const", "continue", "default", "defer", "else", "fallthrough",
        "false", "for", "func", "go", "goto", "if", "import", "interface", "iota", "map", "nil",
        "package", "range", "return", "select", "struct", "switch", "true", "type", "var",
    ],
    types: &[
        "any", "bool", "byte", "complex128", "complex64", "error", "float32", "float64", "int",
        "int16", "int32", "int64", "int8", "rune", "string", "uint", "uint16", "uint32",
        "uint64", "uint8", "uintptr",
    ],
    case_insensitive: false,
    uppercase_types: true,
};

const C: Syntax = Syntax {
    line_comment: Some("//"),
    block_comment: Some(("/*", "*/")),
    nested_blocks: false,
    strings: &[('"', Escape::Backslash), ('\'', Escape::Backslash)],
    keywords: &[
        "break", "case", "const", "continue", "default", "do", "else", "enum", "extern", "for",
        "goto", "if", "inline", "register", "restrict", "return", "sizeof", "static", "struct",
        "switch", "typedef", "union", "volatile", "while",
    ],
    types: &[
        "bool", "char", "double", "float", "int", "long", "short", "signed", "size_t",
        "unsigned", "void",
    ],
    case_insensitive: false,
    uppercase_types: false,
};

/// Markdown is line-structural, not keyword-based, so it gets its own
/// tokenizer. `LineState::block_depth` doubles as the "inside a code fence"
/// flag (1 = fenced).
fn markdown_line(text: &str, state: &mut LineState) -> Vec<(Range<usize>, TokenKind)> {
    let trimmed = text.trim_start();
    let indent = text.len() - trimmed.len();

    if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
        state.block_depth = if state.block_depth > 0 { 0 } else { 1 };
        return vec![(indent..text.len(), TokenKind::Punct)];
    }
    if state.block_depth > 0 {
        if text.is_empty() {
            return Vec::new();
        }
        return vec![(0..text.len(), TokenKind::StringLit)];
    }
    if trimmed.starts_with('#') {
        return vec![(indent..text.len(), TokenKind::Keyword)];
    }
    if trimmed.starts_with('>') {
        return vec![(indent..text.len(), TokenKind::Comment)];
    }

    let mut out = Vec::new();
    // List bullet: "- ", "* ", "+ ", or "1. " — mark just the marker.
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ") {
        out.push((indent..indent + 1, TokenKind::Punct));
    } else {
        let digits = trimmed.chars().take_while(char::is_ascii_digit).count();
        if digits > 0 && trimmed[digits..].starts_with(". ") {
            out.push((indent..indent + digits + 1, TokenKind::Punct));
        }
    }
    // Inline `code` spans (ticks included). Unmatched ticks stay plain.
    let mut open: Option<usize> = None;
    for (b, c) in text.char_indices() {
        if c == '`' {
            match open.take() {
                Some(start) => out.push((start..b + 1, TokenKind::StringLit)),
                None => open = Some(b),
            }
        }
    }
    out.sort_by_key(|(range, _)| range.start);
    out
}

/// Run `syntax` over one line. Works on `char_indices` so every emitted
/// range is char-boundary aligned (multibyte-safe).
fn tokenize(syntax: &Syntax, text: &str, state: &mut LineState) -> Vec<(Range<usize>, TokenKind)> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let n = chars.len();
    let byte_at = |i: usize| chars.get(i).map(|&(b, _)| b).unwrap_or(text.len());
    let mut out: Vec<(Range<usize>, TokenKind)> = Vec::new();
    let mut i = 0;

    // A block comment left open by a previous line swallows the line start.
    if state.block_depth > 0 {
        match syntax.block_comment {
            Some((open, close)) => {
                let (end, depth) = scan_block(
                    &chars,
                    0,
                    open,
                    close,
                    syntax.nested_blocks,
                    state.block_depth,
                );
                state.block_depth = depth;
                if byte_at(end) > 0 {
                    out.push((0..byte_at(end), TokenKind::Comment));
                }
                i = end;
            }
            // Stale state from another language: ignore it.
            None => state.block_depth = 0,
        }
    }

    while i < n {
        let (b, c) = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if let Some(lc) = syntax.line_comment {
            if starts_with_at(&chars, i, lc) {
                out.push((b..text.len(), TokenKind::Comment));
                break;
            }
        }
        if let Some((open, close)) = syntax.block_comment {
            if starts_with_at(&chars, i, open) {
                let after_open = i + open.chars().count();
                let (end, depth) =
                    scan_block(&chars, after_open, open, close, syntax.nested_blocks, 1);
                state.block_depth = depth;
                out.push((b..byte_at(end), TokenKind::Comment));
                i = end;
                continue;
            }
        }
        if let Some(&(_, esc)) = syntax.strings.iter().find(|&&(q, _)| q == c) {
            let end = scan_string(&chars, i + 1, c, esc);
            out.push((b..byte_at(end), TokenKind::StringLit));
            i = end;
            continue;
        }
        if c.is_ascii_digit() {
            let end = scan_number(&chars, i);
            out.push((b..byte_at(end), TokenKind::Number));
            i = end;
            continue;
        }
        if c.is_alphabetic() || c == '_' {
            let end = scan_ident(&chars, i);
            let word = &text[b..byte_at(end)];
            out.push((b..byte_at(end), classify_word(syntax, word, &chars, end)));
            i = end;
            continue;
        }
        out.push((b..byte_at(i + 1), TokenKind::Punct));
        i += 1;
    }

    coalesce(out)
}

/// Does the char sequence at `i` spell out `pat`?
fn starts_with_at(chars: &[(usize, char)], i: usize, pat: &str) -> bool {
    let mut j = i;
    for p in pat.chars() {
        match chars.get(j) {
            Some(&(_, c)) if c == p => j += 1,
            _ => return false,
        }
    }
    true
}

/// Scan a block-comment body from `i` at `depth` (>= 1 means inside).
/// Returns the char index just past the final close, and the depth still
/// open at the line end (0 = closed).
fn scan_block(
    chars: &[(usize, char)],
    mut i: usize,
    open: &str,
    close: &str,
    nested: bool,
    mut depth: u32,
) -> (usize, u32) {
    let n = chars.len();
    while i < n {
        if nested && starts_with_at(chars, i, open) {
            depth += 1;
            i += open.chars().count();
        } else if starts_with_at(chars, i, close) {
            depth -= 1;
            i += close.chars().count();
            if depth == 0 {
                return (i, 0);
            }
        } else {
            i += 1;
        }
    }
    (n, depth)
}

/// Scan a string body from `i` (just past the opening quote). Returns the
/// char index just past the closing quote, or the line end if unterminated
/// (strings do not continue across lines).
fn scan_string(chars: &[(usize, char)], mut i: usize, quote: char, esc: Escape) -> usize {
    let n = chars.len();
    while i < n {
        let c = chars[i].1;
        match esc {
            Escape::Backslash if c == '\\' => {
                i += 2;
                continue;
            }
            Escape::Doubled if c == quote => {
                if i + 1 < n && chars[i + 1].1 == quote {
                    i += 2;
                    continue;
                }
                return i + 1;
            }
            _ if c == quote => return i + 1,
            _ => i += 1,
        }
    }
    n
}

/// Scan a number from `i` (a digit): integers, `0x`/`0b`/`0o` prefixes,
/// decimals, exponents, and trailing type suffixes (`1u8`, `2.5f64`).
fn scan_number(chars: &[(usize, char)], mut i: usize) -> usize {
    let n = chars.len();
    if chars[i].1 == '0' && i + 1 < n && matches!(chars[i + 1].1, 'x' | 'X' | 'b' | 'B' | 'o' | 'O')
    {
        i += 2;
        while i < n && (chars[i].1.is_ascii_alphanumeric() || chars[i].1 == '_') {
            i += 1;
        }
        return i;
    }
    while i < n && (chars[i].1.is_ascii_digit() || chars[i].1 == '_') {
        i += 1;
    }
    if i + 1 < n && chars[i].1 == '.' && chars[i + 1].1.is_ascii_digit() {
        i += 1;
        while i < n && (chars[i].1.is_ascii_digit() || chars[i].1 == '_') {
            i += 1;
        }
    }
    if i < n && matches!(chars[i].1, 'e' | 'E') {
        let mut j = i + 1;
        if j < n && matches!(chars[j].1, '+' | '-') {
            j += 1;
        }
        if j < n && chars[j].1.is_ascii_digit() {
            i = j;
            while i < n && chars[i].1.is_ascii_digit() {
                i += 1;
            }
        }
    }
    while i < n && (chars[i].1.is_ascii_alphanumeric() || chars[i].1 == '_') {
        i += 1;
    }
    i
}

/// Scan an identifier from `i` (a letter or `_`).
fn scan_ident(chars: &[(usize, char)], mut i: usize) -> usize {
    let n = chars.len();
    while i < n && (chars[i].1.is_alphanumeric() || chars[i].1 == '_') {
        i += 1;
    }
    i
}

/// Keyword / type / function-call / plain ident, in that priority. `end` is
/// the char index just past the word, for call-site lookahead.
fn classify_word(syntax: &Syntax, word: &str, chars: &[(usize, char)], end: usize) -> TokenKind {
    let in_set = |set: &[&str]| {
        if syntax.case_insensitive {
            set.iter().any(|k| k.eq_ignore_ascii_case(word))
        } else {
            set.contains(&word)
        }
    };
    if in_set(syntax.keywords) {
        return TokenKind::Keyword;
    }
    if in_set(syntax.types) {
        return TokenKind::Type;
    }
    if syntax.uppercase_types && word.chars().next().is_some_and(char::is_uppercase) {
        return TokenKind::Type;
    }
    // `name(` is a call; `name!(` a macro invocation.
    match chars.get(end).map(|&(_, c)| c) {
        Some('(') => TokenKind::Function,
        Some('!') if matches!(chars.get(end + 1), Some(&(_, '('))) => TokenKind::Function,
        _ => TokenKind::Ident,
    }
}

/// Merge adjacent tokens of the same kind with contiguous ranges, so a run
/// of punctuation becomes one span.
pub(crate) fn coalesce(tokens: Vec<(Range<usize>, TokenKind)>) -> Vec<(Range<usize>, TokenKind)> {
    let mut out: Vec<(Range<usize>, TokenKind)> = Vec::new();
    for (range, kind) in tokens {
        if let Some((last, last_kind)) = out.last_mut() {
            if *last_kind == kind && last.end == range.start {
                last.end = range.end;
                continue;
            }
        }
        out.push((range, kind));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(lang: Language, line: &str) -> Vec<(String, TokenKind)> {
        let mut state = LineState::default();
        lang.line(line, &mut state)
            .into_iter()
            .map(|(r, k)| (line[r].to_string(), k))
            .collect()
    }

    fn kind_of(lang: Language, line: &str, word: &str) -> TokenKind {
        kinds(lang, line)
            .into_iter()
            .find(|(w, _)| w == word)
            .map(|(_, k)| k)
            .unwrap_or_else(|| panic!("token {word:?} not found in {line:?}"))
    }

    #[test]
    fn none_language_emits_nothing() {
        assert!(kinds(Language::None, "let x = 1;").is_empty());
    }

    #[test]
    fn new_languages_classify_keywords_strings_comments() {
        assert_eq!(kind_of(Language::Toml, "name = \"guise\" # crate", "\"guise\""), TokenKind::StringLit);
        assert_eq!(kind_of(Language::Toml, "flag = true", "true"), TokenKind::Keyword);
        assert_eq!(kind_of(Language::Python, "def run(): pass  # go", "def"), TokenKind::Keyword);
        assert_eq!(kind_of(Language::Python, "def run(): pass  # go", "# go"), TokenKind::Comment);
        assert_eq!(kind_of(Language::JavaScript, "const x = `hi`;", "const"), TokenKind::Keyword);
        assert_eq!(kind_of(Language::JavaScript, "const x = `hi`;", "`hi`"), TokenKind::StringLit);
        assert_eq!(kind_of(Language::TypeScript, "let n: number = 5;", "number"), TokenKind::Type);
        assert_eq!(kind_of(Language::TypeScript, "interface A {}", "interface"), TokenKind::Keyword);
        assert_eq!(kind_of(Language::Go, "func main() {}", "func"), TokenKind::Keyword);
        assert_eq!(kind_of(Language::Go, "var n int64", "int64"), TokenKind::Type);
        assert_eq!(kind_of(Language::C, "static int n = 0; // c", "static"), TokenKind::Keyword);
        assert_eq!(kind_of(Language::C, "static int n = 0; // c", "int"), TokenKind::Type);
    }

    #[test]
    fn markdown_structures_lines() {
        assert_eq!(kinds(Language::Markdown, "# Title"), vec![("# Title".into(), TokenKind::Keyword)]);
        assert_eq!(kinds(Language::Markdown, "> quoted"), vec![("> quoted".into(), TokenKind::Comment)]);
        let bullets = kinds(Language::Markdown, "- item with `code` span");
        assert_eq!(bullets[0], ("-".into(), TokenKind::Punct));
        assert_eq!(bullets[1], ("`code`".into(), TokenKind::StringLit));
        let ordered = kinds(Language::Markdown, "12. step");
        assert_eq!(ordered[0], ("12.".into(), TokenKind::Punct));
        // Unmatched ticks stay plain.
        assert!(kinds(Language::Markdown, "just a ` tick").is_empty());
    }

    #[test]
    fn markdown_fences_carry_state() {
        let mut state = LineState::default();
        let fence = Language::Markdown.line("```rust", &mut state);
        assert_eq!(fence[0].1, TokenKind::Punct);
        let inside = Language::Markdown.line("# not a heading", &mut state);
        assert_eq!(inside, vec![(0.."# not a heading".len(), TokenKind::StringLit)]);
        Language::Markdown.line("```", &mut state);
        let after = Language::Markdown.line("# heading again", &mut state);
        assert_eq!(after[0].1, TokenKind::Keyword);
    }

    #[test]
    fn ranges_are_ascending_and_in_bounds() {
        let line = "let s = \"héllo\"; // café";
        let mut state = LineState::default();
        let tokens = Language::Rust.line(line, &mut state);
        let mut at = 0;
        for (range, _) in &tokens {
            assert!(range.start >= at, "overlapping range");
            assert!(range.end <= line.len());
            assert!(line.is_char_boundary(range.start));
            assert!(line.is_char_boundary(range.end));
            at = range.end;
        }
    }

    #[test]
    fn rust_basics() {
        assert_eq!(
            kind_of(Language::Rust, "let x = 1;", "let"),
            TokenKind::Keyword
        );
        assert_eq!(kind_of(Language::Rust, "let x = 1;", "x"), TokenKind::Ident);
        assert_eq!(
            kind_of(Language::Rust, "let x = 10.5e3;", "10.5e3"),
            TokenKind::Number
        );
        assert_eq!(
            kind_of(Language::Rust, "let n = 0xff_u8;", "0xff_u8"),
            TokenKind::Number
        );
        assert_eq!(
            kind_of(
                Language::Rust,
                r#"let s = "hi \" there";"#,
                r#""hi \" there""#
            ),
            TokenKind::StringLit
        );
        assert_eq!(
            kind_of(Language::Rust, "let v: Vec<u8>;", "Vec"),
            TokenKind::Type
        );
        assert_eq!(
            kind_of(Language::Rust, "foo(1)", "foo"),
            TokenKind::Function
        );
        assert_eq!(
            kind_of(Language::Rust, "println!(\"x\")", "println"),
            TokenKind::Function
        );
        assert_eq!(
            kind_of(Language::Rust, "a + b // sum", "// sum"),
            TokenKind::Comment
        );
    }

    #[test]
    fn rust_block_comment_carries_state() {
        let mut state = LineState::default();
        let t1 = Language::Rust.line("start /* open", &mut state);
        assert_eq!(
            t1.last()
                .map(|(r, k)| ("start /* open"[r.clone()].to_string(), *k)),
            Some(("/* open".to_string(), TokenKind::Comment))
        );
        let t2 = Language::Rust.line("all comment", &mut state);
        assert_eq!(t2, vec![(0..11, TokenKind::Comment)]);
        let t3 = Language::Rust.line("done */ let x", &mut state);
        assert_eq!(t3[0], (0..7, TokenKind::Comment));
        assert_eq!("done */ let x"[t3[1].clone().0].to_string(), "let");
        assert_eq!(state, LineState::default());
    }

    #[test]
    fn rust_block_comments_nest() {
        let mut state = LineState::default();
        Language::Rust.line("/* a /* b */ still", &mut state);
        assert_ne!(state, LineState::default());
        let t = Language::Rust.line("c */ code", &mut state);
        assert_eq!(t[0], (0..4, TokenKind::Comment));
        assert_eq!(state, LineState::default());
    }

    #[test]
    fn sql_keywords_are_case_insensitive() {
        assert_eq!(
            kind_of(Language::Sql, "SELECT * FROM users;", "SELECT"),
            TokenKind::Keyword
        );
        assert_eq!(
            kind_of(Language::Sql, "select * from users;", "select"),
            TokenKind::Keyword
        );
        assert_eq!(
            kind_of(Language::Sql, "id INT PRIMARY KEY", "INT"),
            TokenKind::Type
        );
        assert_eq!(
            kind_of(Language::Sql, "count(*)", "count"),
            TokenKind::Function
        );
    }

    #[test]
    fn sql_comments_and_strings() {
        assert_eq!(
            kind_of(Language::Sql, "x -- note", "-- note"),
            TokenKind::Comment
        );
        assert_eq!(
            kind_of(Language::Sql, "name = 'it''s'", "'it''s'"),
            TokenKind::StringLit
        );
        let mut state = LineState::default();
        Language::Sql.line("/* multi", &mut state);
        let t = Language::Sql.line("line */ SELECT", &mut state);
        assert_eq!(t[0], (0..7, TokenKind::Comment));
        assert_eq!(state, LineState::default());
    }

    #[test]
    fn sql_blocks_do_not_nest() {
        let mut state = LineState::default();
        Language::Sql.line("/* a /* b */ tail", &mut state);
        // The first `*/` closes the whole comment.
        assert_eq!(state, LineState::default());
    }

    #[test]
    fn json_tokens() {
        let line = r#"{"key": [1.5, true, null, "value"]}"#;
        assert_eq!(
            kind_of(Language::Json, line, r#""key""#),
            TokenKind::StringLit
        );
        assert_eq!(kind_of(Language::Json, line, "1.5"), TokenKind::Number);
        assert_eq!(kind_of(Language::Json, line, "true"), TokenKind::Keyword);
        assert_eq!(kind_of(Language::Json, line, "null"), TokenKind::Keyword);
    }

    #[test]
    fn multibyte_idents_and_strings() {
        let line = "let café = \"日本語\"; // été";
        assert_eq!(kind_of(Language::Rust, line, "café"), TokenKind::Ident);
        assert_eq!(
            kind_of(Language::Rust, line, "\"日本語\""),
            TokenKind::StringLit
        );
        assert_eq!(kind_of(Language::Rust, line, "// été"), TokenKind::Comment);
    }

    #[test]
    fn unterminated_string_stops_at_line_end() {
        let line = "let s = \"open";
        assert_eq!(
            kind_of(Language::Rust, line, "\"open"),
            TokenKind::StringLit
        );
        // ...and does not leak into the next line.
        let mut state = LineState::default();
        Language::Rust.line(line, &mut state);
        assert_eq!(state, LineState::default());
    }

    #[test]
    fn punctuation_coalesces() {
        let tokens = kinds(Language::Rust, "a->b");
        assert_eq!(
            tokens,
            vec![
                ("a".to_string(), TokenKind::Ident),
                ("->".to_string(), TokenKind::Punct),
                ("b".to_string(), TokenKind::Ident),
            ]
        );
    }

    #[test]
    fn empty_line_inside_block_comment() {
        let mut state = LineState::default();
        Language::Rust.line("/* open", &mut state);
        let t = Language::Rust.line("", &mut state);
        assert!(t.is_empty());
        assert_ne!(state, LineState::default());
    }
}
