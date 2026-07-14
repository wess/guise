//! Binary split tree of panes.

use crate::SplitDirection;
use super::id::{PaneId, SplitId};

pub const MIN_RATIO: f32 = 0.1;
pub const MAX_RATIO: f32 = 0.9;

/// Clamp a split ratio into the allowed `0.1..=0.9` range.
pub fn clamp_ratio(ratio: f32) -> f32 {
    ratio.clamp(MIN_RATIO, MAX_RATIO)
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Leaf(PaneId),
    Split {
        id: SplitId,
        axis: SplitDirection,
        /// Fraction of the available space given to `first`. Always clamped.
        ratio: f32,
        first: Box<Node>,
        second: Box<Node>,
    },
}

/// A tree of panes. Always contains at least one pane.
#[derive(Debug, Clone, PartialEq)]
pub struct PaneTree {
    root: Node,
    splits: u64,
}

impl PaneTree {
    pub fn new(root: PaneId) -> Self {
        Self {
            root: Node::Leaf(root),
            splits: 0,
        }
    }

    /// Assemble a tree from a prebuilt root (layout restore). `splits` must
    /// be at least the highest `SplitId` in `root` so future ids stay unique.
    pub(crate) fn from_parts(root: Node, splits: u64) -> Self {
        Self { root, splits }
    }

    pub fn root(&self) -> &Node {
        &self.root
    }

    /// Split the leaf holding `target`, placing `new_pane` beside it.
    /// `new_first` puts the new pane left/top. Ratio starts at 0.5.
    /// Returns the id of the created split, or `None` if `target` is absent
    /// or `new_pane` is already present.
    pub fn split(
        &mut self,
        target: PaneId,
        axis: SplitDirection,
        new_pane: PaneId,
        new_first: bool,
    ) -> Option<SplitId> {
        if !self.contains(target) || self.contains(new_pane) {
            return None;
        }
        self.splits += 1;
        let id = SplitId(self.splits);
        splitnode(&mut self.root, target, axis, new_pane, new_first, id);
        Some(id)
    }

    /// Split the leaf holding `target`, placing an entire `subtree` beside it
    /// (used when a whole tab — possibly itself split — is dropped onto a pane).
    /// `subtree`'s `SplitId`s are renumbered into this tree so they can't clash.
    /// `false` if `target` is absent or `subtree` shares a pane with this tree.
    pub fn split_subtree(
        &mut self,
        target: PaneId,
        axis: SplitDirection,
        mut subtree: Node,
        new_first: bool,
    ) -> bool {
        if !self.contains(target) {
            return false;
        }
        let mut sub_panes = Vec::new();
        leaves(&subtree, &mut sub_panes);
        if sub_panes.iter().any(|p| self.contains(*p)) {
            return false;
        }
        renumber(&mut subtree, &mut self.splits);
        self.splits += 1;
        let id = SplitId(self.splits);
        splitsubnode(&mut self.root, target, axis, subtree, new_first, id).is_ok()
    }

    /// Remove a pane, collapsing its parent split into the sibling subtree.
    /// Returns `false` if the pane is absent or is the last pane.
    pub fn remove(&mut self, pane: PaneId) -> bool {
        removenode(&mut self.root, pane)
    }

    /// Set a divider's ratio (clamped to `0.1..=0.9`). `false` if `split` is absent.
    pub fn set_ratio(&mut self, split: SplitId, ratio: f32) -> bool {
        setrationode(&mut self.root, split, clamp_ratio(ratio))
    }

    /// Current ratio of a split, if it exists.
    pub fn ratio(&self, split: SplitId) -> Option<f32> {
        rationode(&self.root, split)
    }

    /// The nearest ancestor split of `pane` whose divider runs along `axis`.
    /// Used to resize the split adjacent to the focused pane in a direction.
    pub fn nearest_split(&self, pane: PaneId, axis: SplitDirection) -> Option<SplitId> {
        nearestnode(&self.root, pane, axis)
    }

    /// All dividers in layout order (depth first, parent before children).
    pub fn list_dividers(&self) -> Vec<(SplitId, SplitDirection)> {
        let mut out = Vec::new();
        dividers(&self.root, &mut out);
        out
    }

    /// All panes in layout order (left/top before right/bottom).
    pub fn panes(&self) -> Vec<PaneId> {
        let mut out = Vec::new();
        leaves(&self.root, &mut out);
        out
    }

    pub fn contains(&self, pane: PaneId) -> bool {
        containsnode(&self.root, pane)
    }
}

/// Renumber every split in `node` using `counter` (incremented per split), so a
/// subtree lifted from another tree gets ids unique to its new home.
fn renumber(node: &mut Node, counter: &mut u64) {
    if let Node::Split { id, first, second, .. } = node {
        *counter += 1;
        *id = SplitId(*counter);
        renumber(first, counter);
        renumber(second, counter);
    }
}

/// Like [`splitnode`] but inserts a whole `subtree`. Threads the subtree back
/// via `Err` when a branch doesn't hold `target`, so it's only moved once.
fn splitsubnode(
    node: &mut Node,
    target: PaneId,
    axis: SplitDirection,
    subtree: Node,
    new_first: bool,
    id: SplitId,
) -> Result<(), Node> {
    match node {
        Node::Leaf(pane) if *pane == target => {
            let kept = Node::Leaf(target);
            let (first, second) = if new_first {
                (subtree, kept)
            } else {
                (kept, subtree)
            };
            *node = Node::Split {
                id,
                axis,
                ratio: 0.5,
                first: Box::new(first),
                second: Box::new(second),
            };
            Ok(())
        }
        Node::Leaf(_) => Err(subtree),
        Node::Split { first, second, .. } => {
            match splitsubnode(first, target, axis, subtree, new_first, id) {
                Ok(()) => Ok(()),
                Err(subtree) => splitsubnode(second, target, axis, subtree, new_first, id),
            }
        }
    }
}

fn splitnode(
    node: &mut Node,
    target: PaneId,
    axis: SplitDirection,
    new_pane: PaneId,
    new_first: bool,
    id: SplitId,
) -> bool {
    match node {
        Node::Leaf(pane) if *pane == target => {
            let (first, second) = if new_first {
                (Node::Leaf(new_pane), Node::Leaf(target))
            } else {
                (Node::Leaf(target), Node::Leaf(new_pane))
            };
            *node = Node::Split {
                id,
                axis,
                ratio: 0.5,
                first: Box::new(first),
                second: Box::new(second),
            };
            true
        }
        Node::Leaf(_) => false,
        Node::Split { first, second, .. } => {
            splitnode(first, target, axis, new_pane, new_first, id)
                || splitnode(second, target, axis, new_pane, new_first, id)
        }
    }
}

fn removenode(node: &mut Node, pane: PaneId) -> bool {
    let Node::Split { first, second, .. } = node else {
        return false;
    };
    let in_first = matches!(first.as_ref(), Node::Leaf(p) if *p == pane);
    let in_second = matches!(second.as_ref(), Node::Leaf(p) if *p == pane);
    if in_first || in_second {
        let keep = if in_first { second } else { first };
        *node = std::mem::replace(keep.as_mut(), Node::Leaf(pane));
        return true;
    }
    removenode(first, pane) || removenode(second, pane)
}

fn setrationode(node: &mut Node, split: SplitId, clamped: f32) -> bool {
    let Node::Split {
        id,
        ratio,
        first,
        second,
        ..
    } = node
    else {
        return false;
    };
    if *id == split {
        *ratio = clamped;
        return true;
    }
    setrationode(first, split, clamped) || setrationode(second, split, clamped)
}

fn nearestnode(node: &Node, pane: PaneId, axis: SplitDirection) -> Option<SplitId> {
    let Node::Split {
        id,
        axis: a,
        first,
        second,
        ..
    } = node
    else {
        return None;
    };
    let child: &Node = if containsnode(first, pane) {
        first
    } else if containsnode(second, pane) {
        second
    } else {
        return None;
    };
    nearestnode(child, pane, axis).or(if *a == axis { Some(*id) } else { None })
}

fn rationode(node: &Node, split: SplitId) -> Option<f32> {
    let Node::Split {
        id,
        ratio,
        first,
        second,
        ..
    } = node
    else {
        return None;
    };
    if *id == split {
        return Some(*ratio);
    }
    rationode(first, split).or_else(|| rationode(second, split))
}

fn dividers(node: &Node, out: &mut Vec<(SplitId, SplitDirection)>) {
    if let Node::Split {
        id,
        axis,
        first,
        second,
        ..
    } = node
    {
        out.push((*id, *axis));
        dividers(first, out);
        dividers(second, out);
    }
}

fn leaves(node: &Node, out: &mut Vec<PaneId>) {
    match node {
        Node::Leaf(pane) => out.push(*pane),
        Node::Split { first, second, .. } => {
            leaves(first, out);
            leaves(second, out);
        }
    }
}

fn containsnode(node: &Node, pane: PaneId) -> bool {
    match node {
        Node::Leaf(p) => *p == pane,
        Node::Split { first, second, .. } => {
            containsnode(first, pane) || containsnode(second, pane)
        }
    }
}
