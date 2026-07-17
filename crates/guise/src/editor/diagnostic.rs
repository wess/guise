//! Diagnostics for the editor: LSP-shaped severity + line/column ranges.
//!
//! Pure data — the host produces them (from a compiler, linter, or language
//! server) and hands them to [`Editor::set_diagnostics`](super::Editor::set_diagnostics).
//! The editor draws a gutter dot per affected line, underlines the range,
//! and shows the active line's first message in a strip under the buffer.

use std::ops::Range;

use gpui::{Hsla, SharedString};

use crate::theme::Theme;

/// Diagnostic severity, ordered so `max` picks the worst.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    Hint,
    Info,
    Warning,
    Error,
}

impl Severity {
    /// The theme accent for this severity.
    pub fn color(self, t: &Theme) -> Hsla {
        match self {
            Severity::Error => t.danger().hsla(),
            Severity::Warning => t.warning().hsla(),
            Severity::Info => t.info().hsla(),
            Severity::Hint => t.dimmed().hsla(),
        }
    }
}

/// One diagnostic, anchored to a line and a char-column range on it.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// 0-based line index.
    pub line: usize,
    /// Char columns on the line; an empty range means the whole line.
    pub cols: Range<usize>,
    pub severity: Severity,
    pub message: SharedString,
}

impl Diagnostic {
    pub fn new(
        line: usize,
        cols: Range<usize>,
        severity: Severity,
        message: impl Into<SharedString>,
    ) -> Self {
        Diagnostic {
            line,
            cols,
            severity,
            message: message.into(),
        }
    }

    /// An error/warning/… covering the whole line.
    pub fn line_wide(line: usize, severity: Severity, message: impl Into<SharedString>) -> Self {
        Diagnostic::new(line, 0..0, severity, message)
    }
}

/// The worst severity among `line`'s diagnostics, if any — drives the gutter
/// dot color.
pub(crate) fn line_severity(diagnostics: &[Diagnostic], line: usize) -> Option<Severity> {
    diagnostics
        .iter()
        .filter(|d| d.line == line)
        .map(|d| d.severity)
        .max()
}

/// The first (worst-first, then declaration order) message on `line`.
pub(crate) fn line_message(diagnostics: &[Diagnostic], line: usize) -> Option<&Diagnostic> {
    diagnostics
        .iter()
        .filter(|d| d.line == line)
        .max_by_key(|d| d.severity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_orders_worst_last() {
        assert!(Severity::Error > Severity::Warning);
        assert!(Severity::Warning > Severity::Info);
        assert!(Severity::Info > Severity::Hint);
    }

    #[test]
    fn line_lookups_pick_the_worst() {
        let diags = vec![
            Diagnostic::new(2, 0..4, Severity::Warning, "unused"),
            Diagnostic::new(2, 6..9, Severity::Error, "type mismatch"),
            Diagnostic::line_wide(5, Severity::Hint, "style"),
        ];
        assert_eq!(line_severity(&diags, 2), Some(Severity::Error));
        assert_eq!(line_severity(&diags, 5), Some(Severity::Hint));
        assert_eq!(line_severity(&diags, 0), None);
        assert_eq!(
            line_message(&diags, 2).unwrap().message.as_ref(),
            "type mismatch"
        );
    }

    #[test]
    fn line_wide_uses_an_empty_range() {
        let d = Diagnostic::line_wide(3, Severity::Error, "boom");
        assert!(d.cols.is_empty());
        assert_eq!(d.line, 3);
    }
}
