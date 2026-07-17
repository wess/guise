//! Block-level markdown line classification.
//!
//! [`classify`] maps one source line to a [`Block`], threading a [`DocState`]
//! through consecutive lines so fenced code blocks and YAML frontmatter carry
//! across them (feed lines in document order, one state per pass).
//!
//! All offsets returned in a [`Block`] are **byte** offsets into the line.
//! Every prefix this module recognizes is pure ASCII (whitespace, `#`, `>`,
//! digits, `-*+`, `[x]`), so those offsets are also char columns.

/// What a source line is, at the block level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    /// `#`–`######` heading; `content` is where the text starts.
    Heading { level: u8, content: usize },
    /// `- ` / `* ` / `+ ` list item.
    Bullet { indent: usize, content: usize },
    /// `1. ` / `1) ` list item.
    Ordered {
        indent: usize,
        number: u64,
        content: usize,
    },
    /// `- [ ] ` / `- [x] ` list item; `state` is the byte of the char
    /// inside the brackets (the one a toggle rewrites).
    Task {
        indent: usize,
        checked: bool,
        state: usize,
        content: usize,
    },
    /// `> ` quote line; `depth` counts nested `>` markers.
    Quote { depth: u8, content: usize },
    /// A ``` or ~~~ fence line. `open` is whether it starts a block;
    /// `lang` is the info string on openers (lowercased).
    Fence { open: bool, lang: Option<String> },
    /// A line inside a fenced code block.
    CodeLine,
    /// `---` / `***` / `___` thematic break.
    Rule,
    /// A `|`-leading table line.
    Table,
    /// A line of the YAML frontmatter block (including its `---` delimiters).
    FrontMatter,
    /// Only whitespace.
    Blank,
    /// Anything else.
    Paragraph,
}

/// Classifier state carried from one line to the next. Start each document
/// pass with `DocState::default()`.
#[derive(Debug, Clone, Default)]
pub struct DocState {
    line_no: usize,
    frontmatter: bool,
    /// Open fence: (fence char, marker run length, info string).
    fence: Option<(u8, usize, Option<String>)>,
}

impl DocState {
    /// The language of the fenced code block the *next* line would belong to.
    pub fn fence_lang(&self) -> Option<&str> {
        self.fence.as_ref().and_then(|(_, _, lang)| lang.as_deref())
    }
}

/// Classify one line, advancing `state`.
pub fn classify(line: &str, state: &mut DocState) -> Block {
    let first = state.line_no == 0;
    state.line_no += 1;

    if state.frontmatter {
        if line.trim() == "---" {
            state.frontmatter = false;
        }
        return Block::FrontMatter;
    }
    if first && line.trim_end() == "---" {
        state.frontmatter = true;
        return Block::FrontMatter;
    }

    if let Some((ch, len, _)) = state.fence {
        let t = line.trim_start();
        let run = t.bytes().take_while(|&b| b == ch).count();
        if run >= len && t[run..].trim().is_empty() {
            state.fence = None;
            return Block::Fence {
                open: false,
                lang: None,
            };
        }
        return Block::CodeLine;
    }
    if let Some((ch, len, lang)) = fence_marker(line) {
        state.fence = Some((ch, len, lang.clone()));
        return Block::Fence { open: true, lang };
    }

    if line.trim().is_empty() {
        return Block::Blank;
    }
    if is_rule(line) {
        return Block::Rule;
    }

    let indent = line.len() - line.trim_start().len();
    let rest = &line[indent..];

    if let Some((level, content)) = heading_marker(rest) {
        return Block::Heading {
            level,
            content: indent + content,
        };
    }
    if rest.starts_with('>') {
        let (depth, content) = quote_marker(rest);
        return Block::Quote {
            depth,
            content: indent + content,
        };
    }
    if let Some((checked, state, content)) = task_marker(rest) {
        return Block::Task {
            indent,
            checked,
            state: indent + state,
            content: indent + content,
        };
    }
    if let Some(content) = bullet_marker(rest) {
        return Block::Bullet {
            indent,
            content: indent + content,
        };
    }
    if let Some((number, content)) = ordered_marker(rest) {
        return Block::Ordered {
            indent,
            number,
            content: indent + content,
        };
    }
    if rest.starts_with('|') {
        return Block::Table;
    }
    Block::Paragraph
}

/// Leading-whitespace width in columns, with tabs as 4.
pub fn indent_cols(line: &str) -> usize {
    line.chars()
        .take_while(|c| c.is_whitespace())
        .map(|c| if c == '\t' { 4 } else { 1 })
        .sum()
}

/// A ``` / ~~~ opener (≤3 spaces of indent): (fence char, run length, info).
fn fence_marker(line: &str) -> Option<(u8, usize, Option<String>)> {
    let indent = line.len() - line.trim_start().len();
    if indent > 3 {
        return None;
    }
    let t = &line[indent..];
    let ch = *t.as_bytes().first()?;
    if ch != b'`' && ch != b'~' {
        return None;
    }
    let len = t.bytes().take_while(|&b| b == ch).count();
    if len < 3 {
        return None;
    }
    let info = t[len..].trim();
    // An info string with a backtick is not a fence (CommonMark).
    if ch == b'`' && info.contains('`') {
        return None;
    }
    let lang = info
        .split_whitespace()
        .next()
        .map(|s| s.to_ascii_lowercase());
    Some((ch, len, lang))
}

/// `---` / `***` / `___` with optional interior spaces, three or more.
fn is_rule(line: &str) -> bool {
    let t: String = line.chars().filter(|c| !c.is_whitespace()).collect();
    if t.len() < 3 {
        return false;
    }
    let ch = t.chars().next().unwrap();
    matches!(ch, '-' | '*' | '_') && t.chars().all(|c| c == ch)
}

/// `#{1,6}` followed by a space (or end of line): (level, content offset).
fn heading_marker(rest: &str) -> Option<(u8, usize)> {
    let level = rest.bytes().take_while(|&b| b == b'#').count();
    if level == 0 || level > 6 {
        return None;
    }
    match rest.as_bytes().get(level) {
        None => Some((level as u8, level)),
        Some(b' ') => Some((level as u8, level + 1)),
        _ => None,
    }
}

/// One or more `>` markers (optionally space-separated): (depth, content).
fn quote_marker(rest: &str) -> (u8, usize) {
    let bytes = rest.as_bytes();
    let mut depth = 0u8;
    let mut at = 0;
    while at < bytes.len() && bytes[at] == b'>' {
        depth = depth.saturating_add(1);
        at += 1;
        if bytes.get(at) == Some(&b' ') {
            at += 1;
        }
    }
    (depth, at)
}

/// `- ` / `* ` / `+ ` (or the bare marker at end of line): content offset.
fn bullet_marker(rest: &str) -> Option<usize> {
    let bytes = rest.as_bytes();
    if !matches!(bytes.first(), Some(b'-' | b'*' | b'+')) {
        return None;
    }
    match bytes.get(1) {
        None => Some(1),
        Some(b' ') => Some(2),
        _ => None,
    }
}

/// A bullet followed by `[ ]` / `[x]` and a space or end of line:
/// (checked, state-char offset, content offset).
fn task_marker(rest: &str) -> Option<(bool, usize, usize)> {
    let open = bullet_marker(rest)?;
    let bytes = rest.as_bytes();
    if bytes.get(open) != Some(&b'[') {
        return None;
    }
    let checked = match bytes.get(open + 1) {
        Some(b' ') => false,
        Some(b'x' | b'X') => true,
        _ => return None,
    };
    if bytes.get(open + 2) != Some(&b']') {
        return None;
    }
    match bytes.get(open + 3) {
        None => Some((checked, open + 1, open + 3)),
        Some(b' ') => Some((checked, open + 1, open + 4)),
        _ => None,
    }
}

/// `1. ` / `1) ` (≤9 digits): (number, content offset).
fn ordered_marker(rest: &str) -> Option<(u64, usize)> {
    let digits = rest.bytes().take_while(|b| b.is_ascii_digit()).count();
    if digits == 0 || digits > 9 {
        return None;
    }
    if !matches!(rest.as_bytes().get(digits), Some(b'.' | b')')) {
        return None;
    }
    let number: u64 = rest[..digits].parse().ok()?;
    match rest.as_bytes().get(digits + 1) {
        None => Some((number, digits + 1)),
        Some(b' ') => Some((number, digits + 2)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(line: &str) -> Block {
        let mut state = DocState::default();
        state.line_no = 1; // pretend mid-document so `---` is a rule
        classify(line, &mut state)
    }

    #[test]
    fn headings() {
        assert_eq!(
            one("# Title"),
            Block::Heading {
                level: 1,
                content: 2
            }
        );
        assert_eq!(
            one("###### deep"),
            Block::Heading {
                level: 6,
                content: 7
            }
        );
        assert_eq!(
            one("  ## indented"),
            Block::Heading {
                level: 2,
                content: 5
            }
        );
        assert_eq!(
            one("##"),
            Block::Heading {
                level: 2,
                content: 2
            }
        );
        assert_eq!(one("#######"), Block::Paragraph); // 7 deep is not a heading
        assert_eq!(one("#tag"), Block::Paragraph); // no space: a tag, not a heading
    }

    #[test]
    fn lists() {
        assert_eq!(
            one("- item"),
            Block::Bullet {
                indent: 0,
                content: 2
            }
        );
        assert_eq!(
            one("* item"),
            Block::Bullet {
                indent: 0,
                content: 2
            }
        );
        assert_eq!(
            one("  + item"),
            Block::Bullet {
                indent: 2,
                content: 4
            }
        );
        assert_eq!(
            one("-"),
            Block::Bullet {
                indent: 0,
                content: 1
            }
        );
        assert_eq!(one("-nope"), Block::Paragraph);
        assert_eq!(
            one("3. third"),
            Block::Ordered {
                indent: 0,
                number: 3,
                content: 3
            }
        );
        assert_eq!(
            one("12) x"),
            Block::Ordered {
                indent: 0,
                number: 12,
                content: 4
            }
        );
        assert_eq!(one("3.x"), Block::Paragraph);
    }

    #[test]
    fn tasks() {
        assert_eq!(
            one("- [ ] todo"),
            Block::Task {
                indent: 0,
                checked: false,
                state: 3,
                content: 6
            }
        );
        assert_eq!(
            one("- [x] done"),
            Block::Task {
                indent: 0,
                checked: true,
                state: 3,
                content: 6
            }
        );
        assert_eq!(
            one("- [X] done"),
            Block::Task {
                indent: 0,
                checked: true,
                state: 3,
                content: 6
            }
        );
        assert_eq!(
            one("- [ ]"),
            Block::Task {
                indent: 0,
                checked: false,
                state: 3,
                content: 5
            }
        );
        assert_eq!(
            one("  - [ ] in"),
            Block::Task {
                indent: 2,
                checked: false,
                state: 5,
                content: 8
            }
        );
        assert_eq!(
            one("- [y] no"),
            Block::Bullet {
                indent: 0,
                content: 2
            }
        );
    }

    #[test]
    fn quotes() {
        assert_eq!(
            one("> hi"),
            Block::Quote {
                depth: 1,
                content: 2
            }
        );
        assert_eq!(
            one(">hi"),
            Block::Quote {
                depth: 1,
                content: 1
            }
        );
        assert_eq!(
            one("> > deep"),
            Block::Quote {
                depth: 2,
                content: 4
            }
        );
    }

    #[test]
    fn rules_and_tables() {
        assert_eq!(one("---"), Block::Rule);
        assert_eq!(one("***"), Block::Rule);
        assert_eq!(one("___"), Block::Rule);
        assert_eq!(one("- - -"), Block::Rule);
        assert_eq!(one("--"), Block::Paragraph);
        assert_eq!(one("| a | b |"), Block::Table);
    }

    #[test]
    fn blank_and_paragraph() {
        assert_eq!(one(""), Block::Blank);
        assert_eq!(one("   "), Block::Blank);
        assert_eq!(one("plain text"), Block::Paragraph);
    }

    #[test]
    fn fenced_code_threads_state() {
        let mut state = DocState::default();
        let doc = ["```rust", "fn main() {}", "```", "after"];
        let blocks: Vec<Block> = doc.iter().map(|l| classify(l, &mut state)).collect();
        assert_eq!(
            blocks[0],
            Block::Fence {
                open: true,
                lang: Some("rust".into())
            }
        );
        assert_eq!(blocks[1], Block::CodeLine);
        assert_eq!(
            blocks[2],
            Block::Fence {
                open: false,
                lang: None
            }
        );
        assert_eq!(blocks[3], Block::Paragraph);
    }

    #[test]
    fn fence_lang_is_readable_mid_block() {
        let mut state = DocState::default();
        classify("```json", &mut state);
        assert_eq!(state.fence_lang(), Some("json"));
        classify("{}", &mut state);
        assert_eq!(state.fence_lang(), Some("json"));
        classify("```", &mut state);
        assert_eq!(state.fence_lang(), None);
    }

    #[test]
    fn unterminated_fence_swallows_the_rest() {
        let mut state = DocState::default();
        classify("```", &mut state);
        assert_eq!(classify("# not a heading", &mut state), Block::CodeLine);
        assert_eq!(classify("- not a list", &mut state), Block::CodeLine);
    }

    #[test]
    fn markdown_inside_code_is_literal() {
        let mut state = DocState::default();
        classify("~~~", &mut state);
        assert_eq!(classify("---", &mut state), Block::CodeLine);
        // ``` does not close a ~~~ fence
        assert_eq!(classify("```", &mut state), Block::CodeLine);
        assert_eq!(
            classify("~~~", &mut state),
            Block::Fence {
                open: false,
                lang: None
            }
        );
    }

    #[test]
    fn frontmatter_only_at_document_start() {
        let mut state = DocState::default();
        let doc = ["---", "title: x", "---", "body"];
        let blocks: Vec<Block> = doc.iter().map(|l| classify(l, &mut state)).collect();
        assert_eq!(blocks[0], Block::FrontMatter);
        assert_eq!(blocks[1], Block::FrontMatter);
        assert_eq!(blocks[2], Block::FrontMatter);
        assert_eq!(blocks[3], Block::Paragraph);
        // Mid-document `---` is a rule, not frontmatter.
        let mut state = DocState::default();
        classify("intro", &mut state);
        assert_eq!(classify("---", &mut state), Block::Rule);
    }

    #[test]
    fn indent_cols_counts_tabs_as_four() {
        assert_eq!(indent_cols("    x"), 4);
        assert_eq!(indent_cols("\tx"), 4);
        assert_eq!(indent_cols(" \t x"), 6);
        assert_eq!(indent_cols("x"), 0);
    }
}
