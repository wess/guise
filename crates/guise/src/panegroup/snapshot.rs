//! Layout persistence: capture a group's split/tab arrangement as a
//! [`LayoutSnapshot`], encode it to a compact string, decode and
//! [`restore`](super::PaneGroup::restore) it later.
//!
//! Item ids serialize as raw numbers — the host owns what they mean, so it
//! should persist its own id → content mapping alongside the layout string
//! and re-register content builders before restoring.
//!
//! The format is deliberately tiny (no serde):
//! `h0.35(p0@1,2|v0.5(p0@3|p1@4,5))` — `h`/`v` splits with a ratio, `p`
//! panes as `p<active>@<item>,<item>,…`.

use std::fmt;

use crate::SplitDirection;

/// A pure description of a pane-group layout.
#[derive(Debug, Clone, PartialEq)]
pub enum LayoutSnapshot {
    /// A tabbed pane: item ids (in tab order) and the active tab's index.
    Pane { items: Vec<u64>, active: usize },
    /// A divider: `ratio` is `first`'s share.
    Split {
        axis: SplitDirection,
        ratio: f32,
        first: Box<LayoutSnapshot>,
        second: Box<LayoutSnapshot>,
    },
}

/// Why a layout string failed to decode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotError {
    pub at: usize,
    pub reason: &'static str,
}

impl fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "layout snapshot: {} at byte {}", self.reason, self.at)
    }
}

impl std::error::Error for SnapshotError {}

impl LayoutSnapshot {
    /// Every item id in the snapshot, in traversal (visual) order.
    pub fn item_ids(&self) -> Vec<u64> {
        let mut out = Vec::new();
        self.collect_items(&mut out);
        out
    }

    fn collect_items(&self, out: &mut Vec<u64>) {
        match self {
            LayoutSnapshot::Pane { items, .. } => out.extend(items),
            LayoutSnapshot::Split { first, second, .. } => {
                first.collect_items(out);
                second.collect_items(out);
            }
        }
    }

    /// Encode to the compact layout string.
    pub fn encode(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
    }

    fn write(&self, out: &mut String) {
        match self {
            LayoutSnapshot::Pane { items, active } => {
                out.push('p');
                out.push_str(&active.to_string());
                out.push('@');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    out.push_str(&item.to_string());
                }
            }
            LayoutSnapshot::Split {
                axis,
                ratio,
                first,
                second,
            } => {
                out.push(match axis {
                    SplitDirection::Horizontal => 'h',
                    SplitDirection::Vertical => 'v',
                });
                let ratio = format!("{ratio:.4}");
                out.push_str(ratio.trim_end_matches('0').trim_end_matches('.'));
                out.push('(');
                first.write(out);
                out.push('|');
                second.write(out);
                out.push(')');
            }
        }
    }

    /// Decode a layout string. Validates shape (panes non-empty, active in
    /// range, ratios finite) but not item-id meaning — that's the host's.
    pub fn decode(source: &str) -> Result<LayoutSnapshot, SnapshotError> {
        let bytes = source.trim().as_bytes();
        let mut pos = 0;
        let node = parse_node(bytes, &mut pos)?;
        if pos != bytes.len() {
            return Err(SnapshotError {
                at: pos,
                reason: "trailing content",
            });
        }
        Ok(node)
    }
}

fn parse_number(bytes: &[u8], pos: &mut usize, reason: &'static str) -> Result<u64, SnapshotError> {
    let start = *pos;
    while *pos < bytes.len() && bytes[*pos].is_ascii_digit() {
        *pos += 1;
    }
    if start == *pos {
        return Err(SnapshotError { at: start, reason });
    }
    std::str::from_utf8(&bytes[start..*pos])
        .ok()
        .and_then(|s| s.parse().ok())
        .ok_or(SnapshotError { at: start, reason })
}

fn parse_ratio(bytes: &[u8], pos: &mut usize) -> Result<f32, SnapshotError> {
    let start = *pos;
    while *pos < bytes.len() && (bytes[*pos].is_ascii_digit() || bytes[*pos] == b'.') {
        *pos += 1;
    }
    std::str::from_utf8(&bytes[start..*pos])
        .ok()
        .and_then(|s| s.parse::<f32>().ok())
        .filter(|r| r.is_finite() && *r > 0.0 && *r < 1.0)
        .ok_or(SnapshotError {
            at: start,
            reason: "expected a ratio in (0, 1)",
        })
}

fn parse_node(bytes: &[u8], pos: &mut usize) -> Result<LayoutSnapshot, SnapshotError> {
    match bytes.get(*pos) {
        Some(b'p') => {
            *pos += 1;
            let active = parse_number(bytes, pos, "expected the active index")? as usize;
            if bytes.get(*pos) != Some(&b'@') {
                return Err(SnapshotError {
                    at: *pos,
                    reason: "expected '@'",
                });
            }
            *pos += 1;
            let mut items = vec![parse_number(bytes, pos, "expected an item id")?];
            while bytes.get(*pos) == Some(&b',') {
                *pos += 1;
                items.push(parse_number(bytes, pos, "expected an item id")?);
            }
            if active >= items.len() {
                return Err(SnapshotError {
                    at: *pos,
                    reason: "active index out of range",
                });
            }
            Ok(LayoutSnapshot::Pane { items, active })
        }
        Some(b'h') | Some(b'v') => {
            let axis = if bytes[*pos] == b'h' {
                SplitDirection::Horizontal
            } else {
                SplitDirection::Vertical
            };
            *pos += 1;
            let ratio = parse_ratio(bytes, pos)?;
            if bytes.get(*pos) != Some(&b'(') {
                return Err(SnapshotError {
                    at: *pos,
                    reason: "expected '('",
                });
            }
            *pos += 1;
            let first = parse_node(bytes, pos)?;
            if bytes.get(*pos) != Some(&b'|') {
                return Err(SnapshotError {
                    at: *pos,
                    reason: "expected '|'",
                });
            }
            *pos += 1;
            let second = parse_node(bytes, pos)?;
            if bytes.get(*pos) != Some(&b')') {
                return Err(SnapshotError {
                    at: *pos,
                    reason: "expected ')'",
                });
            }
            *pos += 1;
            Ok(LayoutSnapshot::Split {
                axis,
                ratio,
                first: Box::new(first),
                second: Box::new(second),
            })
        }
        _ => Err(SnapshotError {
            at: *pos,
            reason: "expected 'p', 'h', or 'v'",
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pane(items: &[u64], active: usize) -> LayoutSnapshot {
        LayoutSnapshot::Pane {
            items: items.to_vec(),
            active,
        }
    }

    #[test]
    fn round_trips_nested_layouts() {
        let layout = LayoutSnapshot::Split {
            axis: SplitDirection::Horizontal,
            ratio: 0.35,
            first: Box::new(pane(&[1, 2], 1)),
            second: Box::new(LayoutSnapshot::Split {
                axis: SplitDirection::Vertical,
                ratio: 0.5,
                first: Box::new(pane(&[3], 0)),
                second: Box::new(pane(&[4, 5, 6], 2)),
            }),
        };
        let encoded = layout.encode();
        assert_eq!(encoded, "h0.35(p1@1,2|v0.5(p0@3|p2@4,5,6))");
        assert_eq!(LayoutSnapshot::decode(&encoded).unwrap(), layout);
    }

    #[test]
    fn single_pane_round_trips() {
        let layout = pane(&[42], 0);
        assert_eq!(layout.encode(), "p0@42");
        assert_eq!(LayoutSnapshot::decode("p0@42").unwrap(), layout);
        assert_eq!(LayoutSnapshot::decode("  p0@42  ").unwrap(), layout);
    }

    #[test]
    fn item_ids_walk_in_visual_order() {
        let layout = LayoutSnapshot::decode("h0.5(p0@9|v0.5(p0@3|p0@7,1))").unwrap();
        assert_eq!(layout.item_ids(), vec![9, 3, 7, 1]);
    }

    #[test]
    fn decode_rejects_malformed_input() {
        for (src, reason) in [
            ("", "expected 'p', 'h', or 'v'"),
            ("x", "expected 'p', 'h', or 'v'"),
            ("p@1", "expected the active index"),
            ("p0-1", "expected '@'"),
            ("p1@5", "active index out of range"),
            ("h1.5(p0@1|p0@2)", "expected a ratio in (0, 1)"),
            ("h0.5(p0@1p0@2)", "expected '|'"),
            ("h0.5(p0@1|p0@2", "expected ')'"),
            ("p0@1extra", "trailing content"),
        ] {
            let err = LayoutSnapshot::decode(src).unwrap_err();
            assert_eq!(err.reason, reason, "for {src:?}");
        }
    }
}
