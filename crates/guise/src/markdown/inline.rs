//! Inline markdown span parsing: emphasis, code, strikethrough, highlight,
//! links, and wikilinks over one line (or the content part of one).
//!
//! [`parse`] returns contiguous [`InlineSpan`]s covering the input exactly.
//! Marker bytes (the `**`, backticks, `[[`, url parts, …) are flagged so a
//! renderer can hide them in preview and dim them when the line is revealed.
//! All delimiters are ASCII, so span boundaries always land on char
//! boundaries.
//!
//! The grammar is deliberately small: code spans protect their contents,
//! emphasis uses simplified flanking rules (`_` never matches inside words),
//! and unmatched delimiters stay literal text.

use std::ops::Range;

/// Style flags accumulated on a span. `link` marks clickable link text.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InlineStyle {
    pub bold: bool,
    pub italic: bool,
    pub code: bool,
    pub strike: bool,
    pub highlight: bool,
    pub link: bool,
}

/// One run of bytes sharing a style. `marker` bytes are markdown syntax;
/// `link` indexes into [`Inline::links`] for the whole construct (text and
/// url alike), so any click inside it can resolve the target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineSpan {
    pub range: Range<usize>,
    pub marker: bool,
    pub style: InlineStyle,
    pub link: Option<usize>,
}

/// The parse result: spans covering the input exactly, plus link targets.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Inline {
    pub spans: Vec<InlineSpan>,
    pub links: Vec<String>,
}

impl Inline {
    /// The link target covering byte `at`, if any.
    pub fn link_at(&self, at: usize) -> Option<&str> {
        let span = self
            .spans
            .iter()
            .find(|s| s.range.contains(&at) && s.link.is_some())?;
        Some(&self.links[span.link?])
    }
}

const NO_LINK: usize = usize::MAX;

/// Byte-attribute working set for one parse.
struct Attrs {
    marker: Vec<bool>,
    code: Vec<bool>,
    bold: Vec<bool>,
    italic: Vec<bool>,
    strike: Vec<bool>,
    highlight: Vec<bool>,
    /// Link *style* (underlined text), content bytes only.
    link_text: Vec<bool>,
    /// Link target index for the whole construct.
    link_idx: Vec<usize>,
}

/// Parse one line's inline markup.
pub fn parse(text: &str) -> Inline {
    let n = text.len();
    let mut a = Attrs {
        marker: vec![false; n],
        code: vec![false; n],
        bold: vec![false; n],
        italic: vec![false; n],
        strike: vec![false; n],
        highlight: vec![false; n],
        link_text: vec![false; n],
        link_idx: vec![NO_LINK; n],
    };
    let mut links = Vec::new();

    mark_code_spans(text, &mut a);
    mark_links(text, &mut a, &mut links);
    mark_emphasis(text, &mut a);

    // Coalesce per-byte attributes into spans.
    let mut spans: Vec<InlineSpan> = Vec::new();
    for i in 0..n {
        let style = InlineStyle {
            bold: a.bold[i],
            italic: a.italic[i],
            code: a.code[i],
            strike: a.strike[i],
            highlight: a.highlight[i],
            link: a.link_text[i],
        };
        let link = (a.link_idx[i] != NO_LINK).then_some(a.link_idx[i]);
        match spans.last_mut() {
            Some(last)
                if last.marker == a.marker[i] && last.style == style && last.link == link =>
            {
                last.range.end = i + 1;
            }
            _ => spans.push(InlineSpan {
                range: i..i + 1,
                marker: a.marker[i],
                style,
                link,
            }),
        }
    }
    Inline { spans, links }
}

fn set(flags: &mut [bool], range: Range<usize>) {
    flags[range].iter_mut().for_each(|f| *f = true);
}

/// Backtick code spans: a run of N backticks closes on the next run of
/// exactly N. Contents are protected from every later pass.
fn mark_code_spans(text: &str, a: &mut Attrs) {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'`' {
            i += 1;
            continue;
        }
        let open = run_len(bytes, i, b'`');
        let mut j = i + open;
        let close = loop {
            match bytes[j..].iter().position(|&b| b == b'`') {
                None => break None,
                Some(off) => {
                    let at = j + off;
                    let len = run_len(bytes, at, b'`');
                    if len == open {
                        break Some(at);
                    }
                    j = at + len;
                }
            }
        };
        match close {
            Some(at) => {
                set(&mut a.code, i..at + open);
                set(&mut a.marker, i..i + open);
                set(&mut a.marker, at..at + open);
                i = at + open;
            }
            None => i += open,
        }
    }
}

/// `[[target]]`, `[[target|alias]]`, `[text](url)`, and `![alt](url)`.
fn mark_links(text: &str, a: &mut Attrs, links: &mut Vec<String>) {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if a.code[i] || bytes[i] != b'[' && !(bytes[i] == b'!' && bytes.get(i + 1) == Some(&b'[')) {
            i += 1;
            continue;
        }
        let start = i;
        let bracket = if bytes[i] == b'!' { i + 1 } else { i };

        // Wikilink.
        if bytes[i] != b'!' && bytes.get(i + 1) == Some(&b'[') {
            if let Some(end) = find_outside_code(text, a, i + 2, "]]") {
                let inner = i + 2..end;
                let pipe = text[inner.clone()].find('|').map(|p| i + 2 + p);
                let (target, vis) = match pipe {
                    Some(p) => (&text[i + 2..p], p + 1..end),
                    None => (&text[inner.clone()], inner.clone()),
                };
                let idx = links.len();
                links.push(target.to_string());
                set(&mut a.marker, i..vis.start);
                set(&mut a.marker, end..end + 2);
                set(&mut a.link_text, vis);
                a.link_idx[i..end + 2].iter_mut().for_each(|l| *l = idx);
                i = end + 2;
                continue;
            }
            i += 2;
            continue;
        }

        // `[text](url)` / `![alt](url)`.
        let Some(text_end) = find_outside_code(text, a, bracket + 1, "]") else {
            i = bracket + 1;
            continue;
        };
        if bytes.get(text_end + 1) != Some(&b'(') {
            i = bracket + 1;
            continue;
        }
        let Some(url_end) = find_outside_code(text, a, text_end + 2, ")") else {
            i = bracket + 1;
            continue;
        };
        let idx = links.len();
        links.push(text[text_end + 2..url_end].to_string());
        set(&mut a.marker, start..bracket + 1);
        set(&mut a.marker, text_end..url_end + 1);
        set(&mut a.link_text, bracket + 1..text_end);
        a.link_idx[start..url_end + 1]
            .iter_mut()
            .for_each(|l| *l = idx);
        i = url_end + 1;
    }
}

/// `**bold**`, `*italic*`, `***both***` (and the `_` forms), `~~strike~~`,
/// `==highlight==`. Unmatched delimiters stay literal.
fn mark_emphasis(text: &str, a: &mut Attrs) {
    // Open delimiter marker ranges, per construct.
    let mut bold: Option<Range<usize>> = None;
    let mut italic: Option<Range<usize>> = None;
    let mut strike: Option<Range<usize>> = None;
    let mut highlight: Option<Range<usize>> = None;

    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if a.code[i] || a.marker[i] || !matches!(b, b'*' | b'_' | b'~' | b'=') {
            i += 1;
            continue;
        }
        let len = run_len(bytes, i, b);
        let prev = text[..i].chars().next_back();
        let next = text[i + len..].chars().next();
        let mut can_open = next.is_some_and(|c| !c.is_whitespace());
        let mut can_close = prev.is_some_and(|c| !c.is_whitespace());
        if b == b'_' {
            // No intraword emphasis for underscores (snake_case stays plain).
            can_open &= prev.is_none_or(|c| !c.is_alphanumeric());
            can_close &= next.is_none_or(|c| !c.is_alphanumeric());
        }

        match b {
            b'~' | b'=' if len == 2 => {
                let (open, flags) = if b == b'~' {
                    (&mut strike, &mut a.strike)
                } else {
                    (&mut highlight, &mut a.highlight)
                };
                if let Some(o) = open.take_if(|_| can_close) {
                    set(flags, o.start..i + 2);
                    set(&mut a.marker, o);
                    set(&mut a.marker, i..i + 2);
                } else if can_open {
                    *open = Some(i..i + 2);
                }
            }
            b'*' | b'_' => {
                // A run of 3+ acts as bold (outer two) plus italic (inner one).
                let (close_i, close_b) = match len {
                    1 => (can_close && italic.is_some(), false),
                    2 => (false, can_close && bold.is_some()),
                    _ => (
                        can_close && italic.is_some(),
                        can_close && bold.is_some(),
                    ),
                };
                let mut at = i;
                if close_i {
                    let o = italic.take().unwrap();
                    set(&mut a.italic, o.start..at + 1);
                    set(&mut a.marker, o);
                    set(&mut a.marker, at..at + 1);
                    at += 1;
                }
                if close_b && at + 2 <= i + len {
                    let o = bold.take().unwrap();
                    set(&mut a.bold, o.start..at + 2);
                    set(&mut a.marker, o);
                    set(&mut a.marker, at..at + 2);
                    at += 2;
                }
                if at == i && can_open {
                    match len {
                        1 => italic = Some(i..i + 1),
                        2 => bold = Some(i..i + 2),
                        _ => {
                            bold = Some(i..i + 2);
                            italic = Some(i + 2..i + 3);
                        }
                    }
                }
            }
            _ => {}
        }
        i += len;
    }
}

/// Length of the run of `b` starting at `i`.
fn run_len(bytes: &[u8], i: usize, b: u8) -> usize {
    bytes[i..].iter().take_while(|&&x| x == b).count()
}

/// The byte offset of `needle` at or after `from`, skipping code spans.
fn find_outside_code(text: &str, a: &Attrs, from: usize, needle: &str) -> Option<usize> {
    let mut at = from;
    while at < text.len() {
        let off = text[at..].find(needle)?;
        let pos = at + off;
        if !a.code[pos] {
            return Some(pos);
        }
        at = pos + needle.len();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// (text, marker) fragments, in order — the readable spine of a parse.
    fn frags(text: &str) -> Vec<(String, bool)> {
        parse(text)
            .spans
            .iter()
            .map(|s| (text[s.range.clone()].to_string(), s.marker))
            .collect()
    }

    fn style_of(text: &str, frag: &str) -> InlineStyle {
        let at = text.find(frag).unwrap();
        let inline = parse(text);
        let span = inline
            .spans
            .iter()
            .find(|s| s.range.contains(&at))
            .unwrap();
        span.style
    }

    #[test]
    fn spans_cover_input_exactly() {
        for text in ["", "plain", "**b** *i* `c` ~~s~~ ==h== [t](u) [[w]]"] {
            let inline = parse(text);
            let mut at = 0;
            for s in &inline.spans {
                assert_eq!(s.range.start, at);
                at = s.range.end;
            }
            assert_eq!(at, text.len());
        }
    }

    #[test]
    fn bold_italic_and_both() {
        assert_eq!(
            frags("**bold**"),
            vec![("**".into(), true), ("bold".into(), false), ("**".into(), true)]
        );
        assert!(style_of("**bold**", "bold").bold);
        assert!(style_of("*it*", "it").italic);
        assert!(style_of("_it_", "it").italic);
        assert!(style_of("__b__", "b").bold);
        let s = style_of("***both***", "both");
        assert!(s.bold && s.italic);
    }

    #[test]
    fn unmatched_delimiters_stay_literal() {
        assert_eq!(frags("a ** b"), vec![("a ** b".into(), false)]);
        assert_eq!(frags("*x"), vec![("*x".into(), false)]);
        assert_eq!(frags("2 * 3 * 4"), vec![("2 * 3 * 4".into(), false)]);
    }

    #[test]
    fn underscores_never_match_inside_words() {
        assert_eq!(frags("snake_case_name"), vec![("snake_case_name".into(), false)]);
        assert!(style_of("_word_", "word").italic);
    }

    #[test]
    fn code_spans_protect_contents() {
        let text = "`**not bold**`";
        assert!(!style_of(text, "not bold").bold);
        assert!(style_of(text, "not bold").code);
        // Double-backtick span containing a single backtick.
        let text = "``a ` b``";
        assert!(style_of(text, "a ` b").code);
    }

    #[test]
    fn strike_and_highlight() {
        assert!(style_of("~~gone~~", "gone").strike);
        assert!(style_of("==note==", "note").highlight);
        assert_eq!(frags("a ~ b"), vec![("a ~ b".into(), false)]);
        assert_eq!(frags("a == b"), vec![("a == b".into(), false)]);
    }

    #[test]
    fn nesting_composes() {
        let s = style_of("**bold *and italic***", "and italic");
        assert!(s.bold && s.italic);
        let s = style_of("*it with `code`*", "code");
        assert!(s.italic && s.code);
    }

    #[test]
    fn markdown_link() {
        let text = "see [docs](https://x.dev) now";
        let inline = parse(text);
        assert_eq!(inline.links, vec!["https://x.dev".to_string()]);
        assert!(style_of(text, "docs").link);
        // Url bytes are markers, and clicking anywhere in the construct
        // resolves the target.
        assert_eq!(
            frags(text),
            vec![
                ("see ".into(), false),
                ("[".into(), true),
                ("docs".into(), false),
                ("](https://x.dev)".into(), true),
                (" now".into(), false),
            ]
        );
        assert_eq!(inline.link_at(text.find("https").unwrap()), Some("https://x.dev"));
        assert_eq!(inline.link_at(0), None);
    }

    #[test]
    fn image_parses_like_a_link() {
        let text = "![alt](img.png)";
        let inline = parse(text);
        assert_eq!(inline.links, vec!["img.png".to_string()]);
        assert_eq!(frags(text)[0], ("![".into(), true));
    }

    #[test]
    fn wikilinks() {
        let text = "go [[Home]] or [[Page|the page]]";
        let inline = parse(text);
        assert_eq!(inline.links, vec!["Home".to_string(), "Page".to_string()]);
        assert!(style_of(text, "Home").link);
        assert!(style_of(text, "the page").link);
        // The target of an aliased wikilink is hidden marker text.
        let at = text.find("Page|").unwrap();
        let span = inline.spans.iter().find(|s| s.range.contains(&at)).unwrap();
        assert!(span.marker);
    }

    #[test]
    fn bare_brackets_stay_literal() {
        assert_eq!(frags("a [b] c"), vec![("a [b] c".into(), false)]);
        assert_eq!(frags("f(x)[0]"), vec![("f(x)[0]".into(), false)]);
    }

    #[test]
    fn emphasis_inside_link_text() {
        let text = "[**bold** link](u)";
        let s = style_of(text, "bold");
        assert!(s.bold && s.link);
    }

    #[test]
    fn utf8_content_is_safe() {
        let text = "**héllo — 日本語** *ok*";
        assert!(style_of(text, "héllo").bold);
        assert!(style_of(text, "ok").italic);
        let inline = parse(text);
        let mut at = 0;
        for s in &inline.spans {
            assert!(text.is_char_boundary(s.range.start));
            assert_eq!(s.range.start, at);
            at = s.range.end;
        }
    }
}
