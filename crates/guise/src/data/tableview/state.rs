//! Pure sort and selection logic for [`TableView`](super::TableView).
//!
//! Sorting produces a *display order* — a stable permutation of source-row
//! indices — so the source data is never reordered. Selection is a set of
//! source indices, which is why it survives resorting unchanged.
//!
//! ```ignore
//! let order = sorted_order(&rows, SortDir::Asc, &|a, b| a.name.cmp(&b.name));
//! let mut sel = SelectionState::default();
//! sel.click(SelectionMode::Multi, &order, 0, false, false);
//! sel.click(SelectionMode::Multi, &order, 3, false, true); // shift: rows 0..=3
//! ```

use std::cmp::Ordering;
use std::collections::BTreeSet;

/// Sort direction of a table column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDir {
    Asc,
    Desc,
}

/// How rows respond to clicks and arrow keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectionMode {
    /// Rows are not selectable.
    #[default]
    None,
    /// At most one row selected at a time.
    Single,
    /// Many rows: cmd-click toggles, shift-click selects a range.
    Multi,
}

/// Header-click cycling: none → asc → desc → none on the same column; a click
/// on a different column starts fresh at ascending.
pub fn cycle_sort(current: Option<(usize, SortDir)>, column: usize) -> Option<(usize, SortDir)> {
    match current {
        Some((col, SortDir::Asc)) if col == column => Some((column, SortDir::Desc)),
        Some((col, SortDir::Desc)) if col == column => None,
        _ => Some((column, SortDir::Asc)),
    }
}

/// The unsorted display order: `0..len`.
pub fn identity_order(len: usize) -> Vec<usize> {
    (0..len).collect()
}

/// A stable sort of row indices by `cmp`; the source slice is never mutated.
/// `Desc` flips the comparator arguments (rather than reversing the result),
/// so equal rows keep their source order in both directions.
pub fn sorted_order<T>(rows: &[T], dir: SortDir, cmp: &dyn Fn(&T, &T) -> Ordering) -> Vec<usize> {
    let mut order = identity_order(rows.len());
    order.sort_by(|&a, &b| match dir {
        SortDir::Asc => cmp(&rows[a], &rows[b]),
        SortDir::Desc => cmp(&rows[b], &rows[a]),
    });
    order
}

/// Where `source` currently sits in the display order.
fn position_of(order: &[usize], source: usize) -> Option<usize> {
    order.iter().position(|&s| s == source)
}

/// Row selection over **source** indices. The shift anchor and keyboard cursor
/// are also source indices, mapped through the display order at use, so all
/// three survive resorting.
#[derive(Debug, Clone, Default)]
pub struct SelectionState {
    selected: BTreeSet<usize>,
    anchor: Option<usize>,
    cursor: Option<usize>,
}

impl SelectionState {
    /// The selected source indices, ascending.
    pub fn selected(&self) -> Vec<usize> {
        self.selected.iter().copied().collect()
    }

    pub fn is_selected(&self, source: usize) -> bool {
        self.selected.contains(&source)
    }

    /// The keyboard cursor (a source index), if any.
    pub fn cursor(&self) -> Option<usize> {
        self.cursor
    }

    /// Drop everything. Returns whether the selected set changed.
    pub fn clear(&mut self) -> bool {
        self.anchor = None;
        self.cursor = None;
        if self.selected.is_empty() {
            return false;
        }
        self.selected.clear();
        true
    }

    /// Prune indices that fell off the end after a row-count change. Returns
    /// whether the selected set changed.
    pub fn retain_below(&mut self, len: usize) -> bool {
        let before = self.selected.len();
        self.selected.retain(|&s| s < len);
        self.anchor = self.anchor.filter(|&s| s < len);
        self.cursor = self.cursor.filter(|&s| s < len);
        self.selected.len() != before
    }

    /// A mouse click on the row at `display` position. `toggle` is cmd-click,
    /// `range` is shift-click (range wins when both are held).
    pub fn click(
        &mut self,
        mode: SelectionMode,
        order: &[usize],
        display: usize,
        toggle: bool,
        range: bool,
    ) {
        let Some(&source) = order.get(display) else {
            return;
        };
        match mode {
            SelectionMode::None => {}
            SelectionMode::Single => {
                if toggle && self.selected.contains(&source) {
                    self.selected.clear();
                } else {
                    self.selected.clear();
                    self.selected.insert(source);
                }
                self.anchor = Some(source);
                self.cursor = Some(source);
            }
            SelectionMode::Multi => {
                if range {
                    let from = match self.anchor.and_then(|a| position_of(order, a)) {
                        Some(pos) => pos,
                        // No live anchor (first interaction, or its row is
                        // gone): this click starts and anchors the range.
                        None => {
                            self.anchor = Some(source);
                            display
                        }
                    };
                    self.select_span(order, from, display);
                } else if toggle {
                    if !self.selected.remove(&source) {
                        self.selected.insert(source);
                    }
                    self.anchor = Some(source);
                } else {
                    self.selected.clear();
                    self.selected.insert(source);
                    self.anchor = Some(source);
                }
                self.cursor = Some(source);
            }
        }
    }

    /// Move the cursor by `delta` display positions (arrow keys), selecting
    /// the row it lands on. `extend` (shift) grows the range from the anchor
    /// in `Multi` mode. Returns the new cursor's display position so the
    /// caller can scroll it into view.
    pub fn step(
        &mut self,
        mode: SelectionMode,
        order: &[usize],
        delta: isize,
        extend: bool,
    ) -> Option<usize> {
        if matches!(mode, SelectionMode::None) || order.is_empty() {
            return None;
        }
        let last = order.len() - 1;
        let display = match self.cursor.and_then(|c| position_of(order, c)) {
            Some(pos) => (pos as isize + delta).clamp(0, last as isize) as usize,
            // Nothing focused yet: Down enters at the top, Up at the bottom.
            None if delta < 0 => last,
            None => 0,
        };
        let source = order[display];
        if extend && matches!(mode, SelectionMode::Multi) {
            let from = match self.anchor.and_then(|a| position_of(order, a)) {
                Some(pos) => pos,
                // No live anchor yet: the row this step lands on anchors the
                // range, so further shift-steps grow from it.
                None => {
                    self.anchor = Some(source);
                    display
                }
            };
            self.select_span(order, from, display);
        } else {
            self.selected.clear();
            self.selected.insert(source);
            self.anchor = Some(source);
        }
        self.cursor = Some(source);
        Some(display)
    }

    /// Select exactly the display range `a..=b` (either direction).
    fn select_span(&mut self, order: &[usize], a: usize, b: usize) {
        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
        self.selected.clear();
        self.selected.extend(order[lo..=hi].iter().copied());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_cycles_asc_desc_none() {
        let s1 = cycle_sort(None, 2);
        assert_eq!(s1, Some((2, SortDir::Asc)));
        let s2 = cycle_sort(s1, 2);
        assert_eq!(s2, Some((2, SortDir::Desc)));
        assert_eq!(cycle_sort(s2, 2), None);
    }

    #[test]
    fn sort_switching_column_restarts_ascending() {
        let current = Some((0, SortDir::Desc));
        assert_eq!(cycle_sort(current, 1), Some((1, SortDir::Asc)));
    }

    #[test]
    fn sorted_order_never_touches_the_source() {
        let rows = vec![3, 1, 2];
        let order = sorted_order(&rows, SortDir::Asc, &|a, b| a.cmp(b));
        assert_eq!(order, vec![1, 2, 0]);
        assert_eq!(rows, vec![3, 1, 2]);
    }

    #[test]
    fn sorted_order_is_stable_in_both_directions() {
        // Equal keys (by first tuple field) must keep source order.
        let rows = vec![(1, "a"), (0, "b"), (1, "c"), (0, "d")];
        let cmp = |a: &(i32, &str), b: &(i32, &str)| a.0.cmp(&b.0);
        assert_eq!(sorted_order(&rows, SortDir::Asc, &cmp), vec![1, 3, 0, 2]);
        assert_eq!(sorted_order(&rows, SortDir::Desc, &cmp), vec![0, 2, 1, 3]);
    }

    #[test]
    fn single_mode_holds_one_row_and_cmd_click_deselects() {
        let order = identity_order(4);
        let mut sel = SelectionState::default();
        sel.click(SelectionMode::Single, &order, 1, false, false);
        assert_eq!(sel.selected(), vec![1]);
        sel.click(SelectionMode::Single, &order, 3, false, false);
        assert_eq!(sel.selected(), vec![3]);
        sel.click(SelectionMode::Single, &order, 3, true, false);
        assert_eq!(sel.selected(), Vec::<usize>::new());
    }

    #[test]
    fn none_mode_ignores_clicks_and_steps() {
        let order = identity_order(3);
        let mut sel = SelectionState::default();
        sel.click(SelectionMode::None, &order, 0, false, false);
        assert!(sel.selected().is_empty());
        assert_eq!(sel.step(SelectionMode::None, &order, 1, false), None);
    }

    #[test]
    fn multi_mode_cmd_click_toggles() {
        let order = identity_order(4);
        let mut sel = SelectionState::default();
        sel.click(SelectionMode::Multi, &order, 0, false, false);
        sel.click(SelectionMode::Multi, &order, 2, true, false);
        assert_eq!(sel.selected(), vec![0, 2]);
        sel.click(SelectionMode::Multi, &order, 0, true, false);
        assert_eq!(sel.selected(), vec![2]);
    }

    #[test]
    fn multi_mode_shift_click_selects_a_range_from_the_anchor() {
        let order = identity_order(6);
        let mut sel = SelectionState::default();
        sel.click(SelectionMode::Multi, &order, 1, false, false);
        sel.click(SelectionMode::Multi, &order, 4, false, true);
        assert_eq!(sel.selected(), vec![1, 2, 3, 4]);
        // Shift again re-ranges from the same anchor, upward this time.
        sel.click(SelectionMode::Multi, &order, 0, false, true);
        assert_eq!(sel.selected(), vec![0, 1]);
    }

    #[test]
    fn shift_click_first_establishes_the_anchor() {
        let order = identity_order(6);
        let mut sel = SelectionState::default();
        // Fresh table: a shift-click anchors at the clicked row...
        sel.click(SelectionMode::Multi, &order, 2, false, true);
        assert_eq!(sel.selected(), vec![2]);
        // ...so the next shift-click grows a range from it.
        sel.click(SelectionMode::Multi, &order, 5, false, true);
        assert_eq!(sel.selected(), vec![2, 3, 4, 5]);
    }

    #[test]
    fn shift_step_first_extends_from_the_entry_row() {
        let order = identity_order(4);
        let mut sel = SelectionState::default();
        // Fresh table: shift+down enters at the top and anchors there...
        assert_eq!(sel.step(SelectionMode::Multi, &order, 1, true), Some(0));
        assert_eq!(sel.selected(), vec![0]);
        // ...so the next shift+down extends instead of moving the selection.
        assert_eq!(sel.step(SelectionMode::Multi, &order, 1, true), Some(1));
        assert_eq!(sel.selected(), vec![0, 1]);
    }

    #[test]
    fn shift_range_follows_display_order_but_stores_source_indices() {
        // Display order reversed: display 0 shows source 3, etc.
        let order = vec![3, 2, 1, 0];
        let mut sel = SelectionState::default();
        sel.click(SelectionMode::Multi, &order, 0, false, false);
        assert_eq!(sel.selected(), vec![3]);
        sel.click(SelectionMode::Multi, &order, 2, false, true);
        assert_eq!(sel.selected(), vec![1, 2, 3]);
    }

    #[test]
    fn selection_survives_resorting() {
        let mut sel = SelectionState::default();
        sel.click(SelectionMode::Single, &identity_order(4), 2, false, false);
        assert_eq!(sel.selected(), vec![2]);
        // Order flips; source 2 is still the selected row.
        assert!(sel.is_selected(2));
        // Stepping down from it moves through the *new* display order.
        let order = vec![3, 2, 1, 0]; // source 2 is at display 1
        let display = sel.step(SelectionMode::Single, &order, 1, false);
        assert_eq!(display, Some(2));
        assert_eq!(sel.selected(), vec![1]);
    }

    #[test]
    fn step_enters_at_the_edges_and_clamps() {
        let order = identity_order(3);
        let mut sel = SelectionState::default();
        assert_eq!(sel.step(SelectionMode::Single, &order, 1, false), Some(0));
        assert_eq!(sel.step(SelectionMode::Single, &order, -1, false), Some(0));
        assert_eq!(sel.selected(), vec![0]);

        let mut sel = SelectionState::default();
        assert_eq!(sel.step(SelectionMode::Single, &order, -1, false), Some(2));
        assert_eq!(sel.selected(), vec![2]);
    }

    #[test]
    fn shift_step_extends_from_the_anchor() {
        let order = identity_order(5);
        let mut sel = SelectionState::default();
        sel.click(SelectionMode::Multi, &order, 2, false, false);
        sel.step(SelectionMode::Multi, &order, 1, true);
        sel.step(SelectionMode::Multi, &order, 1, true);
        assert_eq!(sel.selected(), vec![2, 3, 4]);
        // Un-extending walks the range back.
        sel.step(SelectionMode::Multi, &order, -1, true);
        assert_eq!(sel.selected(), vec![2, 3]);
    }

    #[test]
    fn clear_and_retain_report_changes() {
        let order = identity_order(4);
        let mut sel = SelectionState::default();
        assert!(!sel.clear());
        sel.click(SelectionMode::Multi, &order, 1, false, false);
        sel.click(SelectionMode::Multi, &order, 3, true, false);
        assert!(sel.retain_below(2)); // drops source 3
        assert_eq!(sel.selected(), vec![1]);
        assert!(!sel.retain_below(2));
        assert!(sel.clear());
        assert_eq!(sel.cursor(), None);
    }
}
