//! Row planning: turn one classified source line into exactly what gets
//! drawn — the visible string, its style runs, and a source↔visible byte
//! mapping — for both preview (markers hidden) and revealed (markers dimmed)
//! modes.
//!
//! The visible string is always a concatenation of verbatim source slices
//! (never transformed text), so a [`Seg`] maps byte offsets between the two
//! by plain arithmetic. List markers are hidden in *both* modes: the editor
//! draws bullets, numbers, and checkboxes as decorations in the hanging
//! indent, which is what keeps wrapped items aligned.

use std::ops::Range;

use super::block::{indent_cols, Block};
use super::inline::{self, InlineStyle};

/// Block visuals the editor derives sizes and decorations from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowKind {
    Heading(u8),
    Paragraph,
    /// `cols` is the indent width in columns (tab = 4).
    Bullet { cols: usize },
    Ordered { cols: usize, number: u64 },
    Task { cols: usize, checked: bool },
    Quote { depth: u8 },
    Fence { open: bool },
    Code { lang: Option<String> },
    Rule,
    Table,
    FrontMatter,
    Blank,
}

/// One visible run: `len` bytes of the visible string sharing a style.
/// `marker` runs are markdown syntax (drawn dimmed); `dim` runs are
/// de-emphasized content (a checked task's text).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunAttrs {
    pub len: usize,
    pub marker: bool,
    pub dim: bool,
    pub style: InlineStyle,
}

/// A source-range → visible-range correspondence. Hidden source bytes map
/// to an empty visible range at the point they collapsed into.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seg {
    pub src: Range<usize>,
    pub vis: Range<usize>,
}

/// Everything needed to draw one source line.
#[derive(Debug, Clone, PartialEq)]
pub struct RowPlan {
    pub kind: RowKind,
    pub visible: String,
    /// Covers `visible` exactly, in order.
    pub runs: Vec<RunAttrs>,
    /// Covers the source line exactly, in order.
    pub segs: Vec<Seg>,
    /// Link targets by **source** byte range (text and url bytes alike).
    pub links: Vec<(Range<usize>, String)>,
    pub revealed: bool,
}

impl RowPlan {
    /// The link target covering source byte `src`, if any.
    pub fn link_at(&self, src: usize) -> Option<&str> {
        self.links
            .iter()
            .find(|(r, _)| r.contains(&src))
            .map(|(_, t)| t.as_str())
    }
}

/// Plan one line. `code_lang` is the enclosing fence's info string when
/// `block` is [`Block::CodeLine`]; `reveal` shows syntax instead of hiding it
/// (the cursor/selection lines of a focused editor).
pub fn plan(line: &str, block: &Block, code_lang: Option<&str>, reveal: bool) -> RowPlan {
    let mut b = Builder::new(line, reveal);
    let kind = match *block {
        Block::Blank => {
            b.hide(0..line.len());
            RowKind::Blank
        }
        Block::Rule => {
            b.syntax(0..line.len());
            RowKind::Rule
        }
        Block::FrontMatter => {
            b.mono_dim(0..line.len());
            RowKind::FrontMatter
        }
        Block::Table => {
            b.mono(0..line.len());
            RowKind::Table
        }
        Block::Fence { open, .. } => {
            b.mono_dim(0..line.len());
            RowKind::Fence { open }
        }
        Block::CodeLine => {
            b.mono(0..line.len());
            RowKind::Code {
                lang: code_lang.map(str::to_string),
            }
        }
        Block::Heading { level, content } => {
            b.syntax(0..content);
            b.inline(content, false);
            RowKind::Heading(level)
        }
        Block::Quote { depth, content } => {
            b.syntax(0..content);
            b.inline(content, false);
            RowKind::Quote { depth }
        }
        Block::Bullet { content, .. } => {
            b.hide(0..content);
            b.inline(content, false);
            RowKind::Bullet {
                cols: indent_cols(line),
            }
        }
        Block::Ordered { number, content, .. } => {
            b.hide(0..content);
            b.inline(content, false);
            RowKind::Ordered {
                cols: indent_cols(line),
                number,
            }
        }
        Block::Task {
            checked, content, ..
        } => {
            b.hide(0..content);
            b.inline(content, checked);
            RowKind::Task {
                cols: indent_cols(line),
                checked,
            }
        }
        Block::Paragraph => {
            b.inline(0, false);
            RowKind::Paragraph
        }
    };
    b.finish(kind)
}

/// Accumulates visible text, runs, and segments while walking a source line.
struct Builder<'a> {
    line: &'a str,
    reveal: bool,
    visible: String,
    runs: Vec<RunAttrs>,
    segs: Vec<Seg>,
    links: Vec<(Range<usize>, String)>,
}

impl<'a> Builder<'a> {
    fn new(line: &'a str, reveal: bool) -> Self {
        Builder {
            line,
            reveal,
            visible: String::new(),
            runs: Vec::new(),
            segs: Vec::new(),
            links: Vec::new(),
        }
    }

    /// Map `src` to nothing — hidden in both modes.
    fn hide(&mut self, src: Range<usize>) {
        if src.is_empty() {
            return;
        }
        let at = self.visible.len();
        self.segs.push(Seg { src, vis: at..at });
    }

    /// Emit `src` verbatim with the given attributes.
    fn emit(&mut self, src: Range<usize>, marker: bool, dim: bool, style: InlineStyle) {
        if src.is_empty() {
            return;
        }
        let at = self.visible.len();
        self.visible.push_str(&self.line[src.clone()]);
        let len = src.len();
        self.segs.push(Seg {
            src,
            vis: at..at + len,
        });
        // Coalesce with the previous run when nothing changed.
        if let Some(last) = self.runs.last_mut() {
            if last.marker == marker && last.dim == dim && last.style == style {
                last.len += len;
                return;
            }
        }
        self.runs.push(RunAttrs {
            len,
            marker,
            dim,
            style,
        });
    }

    /// Block syntax: hidden in preview, dimmed marker text when revealed.
    fn syntax(&mut self, src: Range<usize>) {
        if self.reveal {
            self.emit(src, true, false, InlineStyle::default());
        } else {
            self.hide(src);
        }
    }

    /// Verbatim monospace (code lines, tables).
    fn mono(&mut self, src: Range<usize>) {
        let style = InlineStyle {
            code: true,
            ..InlineStyle::default()
        };
        self.emit(src, false, false, style);
    }

    /// Verbatim monospace, dimmed (fence lines, frontmatter).
    fn mono_dim(&mut self, src: Range<usize>) {
        let style = InlineStyle {
            code: true,
            ..InlineStyle::default()
        };
        self.emit(src, true, false, style);
    }

    /// Parse and emit the line's content from `at`, with inline markup.
    fn inline(&mut self, at: usize, dim: bool) {
        let content = &self.line[at..];
        let inline = inline::parse(content);
        for span in &inline.spans {
            let src = at + span.range.start..at + span.range.end;
            if span.marker && !self.reveal {
                self.hide(src);
            } else {
                self.emit(src, span.marker, dim, span.style);
            }
        }
        // Collect link targets by construct extent, in source offsets.
        for (idx, target) in inline.links.iter().enumerate() {
            let mut range: Option<Range<usize>> = None;
            for span in inline.spans.iter().filter(|s| s.link == Some(idx)) {
                let r = at + span.range.start..at + span.range.end;
                range = Some(match range {
                    None => r,
                    Some(acc) => acc.start.min(r.start)..acc.end.max(r.end),
                });
            }
            if let Some(range) = range {
                self.links.push((range, target.clone()));
            }
        }
    }

    fn finish(self, kind: RowKind) -> RowPlan {
        RowPlan {
            kind,
            visible: self.visible,
            runs: self.runs,
            segs: self.segs,
            links: self.links,
            revealed: self.reveal,
        }
    }
}

// ---- source ↔ visible mapping ----------------------------------------------

/// Visible byte offset for source byte `src`. Bytes inside hidden segments
/// collapse to the point they disappeared into; offsets past the last
/// segment map to the end of the visible string.
pub fn vis_for_src(segs: &[Seg], src: usize) -> usize {
    for seg in segs {
        if src < seg.src.end {
            let off = src.saturating_sub(seg.src.start);
            return (seg.vis.start + off).min(seg.vis.end);
        }
    }
    segs.last().map_or(0, |s| s.vis.end)
}

/// Source byte offset for visible byte `vis`. Offsets at or past the end of
/// the visible string map to the source end (so clicking after the rendered
/// text puts the cursor at the end of the line).
pub fn src_for_vis(segs: &[Seg], vis: usize) -> usize {
    let end = segs.last().map_or(0, |s| s.src.end);
    for seg in segs {
        if seg.vis.is_empty() {
            continue;
        }
        if vis < seg.vis.end {
            return seg.src.start + vis.saturating_sub(seg.vis.start);
        }
    }
    end
}

/// Byte offset of char column `col` in `s`, clamped to the end.
pub fn byte_for_col(s: &str, col: usize) -> usize {
    s.char_indices().nth(col).map(|(i, _)| i).unwrap_or(s.len())
}

/// Char column of byte offset `byte` in `s` (interior offsets round down).
pub fn col_for_byte(s: &str, byte: usize) -> usize {
    s.char_indices().take_while(|&(i, _)| i < byte).count()
}

#[cfg(test)]
mod tests {
    use super::super::block::{classify, DocState};
    use super::*;

    fn plan_line(line: &str, reveal: bool) -> RowPlan {
        let mut st = DocState::default();
        // pretend mid-document so `---` is a rule
        classify("x", &mut st);
        let block = classify(line, &mut st);
        plan(line, &block, None, reveal)
    }

    fn covers(plan: &RowPlan, line: &str) {
        let mut src_at = 0;
        let mut vis_at = 0;
        for seg in &plan.segs {
            assert_eq!(seg.src.start, src_at, "segs must cover source in order");
            assert_eq!(seg.vis.start, vis_at);
            src_at = seg.src.end;
            vis_at = seg.vis.end;
        }
        assert_eq!(src_at, line.len());
        assert_eq!(vis_at, plan.visible.len());
        let run_total: usize = plan.runs.iter().map(|r| r.len).sum();
        assert_eq!(run_total, plan.visible.len(), "runs must cover visible");
    }

    #[test]
    fn heading_hides_marks_in_preview() {
        let line = "## Section **two**";
        let p = plan_line(line, false);
        assert_eq!(p.kind, RowKind::Heading(2));
        assert_eq!(p.visible, "Section two");
        covers(&p, line);
    }

    #[test]
    fn heading_reveals_marks_dimmed() {
        let line = "## Section";
        let p = plan_line(line, true);
        assert_eq!(p.visible, "## Section");
        assert!(p.runs[0].marker);
        assert!(!p.runs[1].marker);
        covers(&p, line);
    }

    #[test]
    fn list_marker_stays_hidden_even_revealed() {
        let line = "  - item with *em*";
        for reveal in [false, true] {
            let p = plan_line(line, reveal);
            assert_eq!(p.kind, RowKind::Bullet { cols: 2 });
            if reveal {
                assert_eq!(p.visible, "item with *em*");
            } else {
                assert_eq!(p.visible, "item with em");
            }
            covers(&p, line);
        }
    }

    #[test]
    fn checked_task_content_is_dim() {
        let line = "- [x] done deal";
        let p = plan_line(line, false);
        assert_eq!(p.kind, RowKind::Task { cols: 0, checked: true });
        assert_eq!(p.visible, "done deal");
        assert!(p.runs.iter().all(|r| r.dim));
        covers(&p, line);
    }

    #[test]
    fn rule_vanishes_in_preview() {
        let p = plan_line("---", false);
        assert_eq!(p.kind, RowKind::Rule);
        assert_eq!(p.visible, "");
        covers(&p, "---");
        let p = plan_line("---", true);
        assert_eq!(p.visible, "---");
    }

    #[test]
    fn code_lines_are_identity_mono() {
        let mut st = DocState::default();
        classify("```rust", &mut st);
        let lang = st.fence_lang().map(str::to_string);
        let block = classify("let x = 1;", &mut st);
        let p = plan("let x = 1;", &block, lang.as_deref(), false);
        assert_eq!(p.kind, RowKind::Code { lang: Some("rust".into()) });
        assert_eq!(p.visible, "let x = 1;");
        assert!(p.runs[0].style.code);
        covers(&p, "let x = 1;");
    }

    #[test]
    fn link_targets_use_source_ranges() {
        let line = "see [docs](https://x.dev)";
        let p = plan_line(line, false);
        assert_eq!(p.visible, "see docs");
        assert_eq!(p.link_at(line.find("docs").unwrap()), Some("https://x.dev"));
        assert_eq!(p.link_at(line.find("https").unwrap()), Some("https://x.dev"));
        assert_eq!(p.link_at(0), None);
        covers(&p, line);
    }

    #[test]
    fn mapping_roundtrips_through_hidden_prefix() {
        let line = "- [ ] task";
        let p = plan_line(line, false);
        // Source col inside the hidden marker collapses to visible start.
        assert_eq!(vis_for_src(&p.segs, 0), 0);
        assert_eq!(vis_for_src(&p.segs, 5), 0);
        // Content maps 1:1 after the marker.
        let t = line.find("task").unwrap();
        assert_eq!(vis_for_src(&p.segs, t), 0);
        assert_eq!(vis_for_src(&p.segs, t + 2), 2);
        assert_eq!(src_for_vis(&p.segs, 2), t + 2);
        // Past the end lands at the line end.
        assert_eq!(src_for_vis(&p.segs, 99), line.len());
    }

    #[test]
    fn mapping_collapses_hidden_url() {
        let line = "[a](url)";
        let p = plan_line(line, false);
        assert_eq!(p.visible, "a");
        // A cursor inside the hidden url sits at the end of the visible text.
        assert_eq!(vis_for_src(&p.segs, line.find("url").unwrap()), 1);
        // Clicking the single visible char maps back into the link text.
        assert_eq!(src_for_vis(&p.segs, 0), 1);
        covers(&p, line);
    }

    #[test]
    fn revealed_line_maps_identity() {
        let line = "a **b** c";
        let p = plan_line(line, true);
        assert_eq!(p.visible, line);
        for at in 0..=line.len() {
            assert_eq!(vis_for_src(&p.segs, at), at);
            assert_eq!(src_for_vis(&p.segs, at), at);
        }
    }

    #[test]
    fn blank_line_collapses() {
        let p = plan_line("   ", false);
        assert_eq!(p.kind, RowKind::Blank);
        assert_eq!(p.visible, "");
        assert_eq!(vis_for_src(&p.segs, 2), 0);
        assert_eq!(src_for_vis(&p.segs, 0), 3);
    }

    #[test]
    fn byte_col_helpers() {
        assert_eq!(byte_for_col("日本語", 1), 3);
        assert_eq!(byte_for_col("abc", 9), 3);
        assert_eq!(col_for_byte("日本語", 3), 1);
        assert_eq!(col_for_byte("abc", 9), 3);
    }
}
