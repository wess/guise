//! `TreeView` — a hierarchical list with expandable branches (gpui entity).
//!
//! Nodes are plain [`TreeNode`] values; the view owns expansion state and a
//! single selection, and emits [`TreeViewEvent`] on select / toggle / activate.
//! Branches show a rotating chevron and rows indent per depth; arrow keys walk
//! the visible rows (right expands or steps into a branch, left collapses or
//! steps to the parent) and Enter activates the selection.
//!
//! ```ignore
//! let tree = cx.new(|cx| {
//!     TreeView::new(cx)
//!         .nodes(vec![
//!             TreeNode::new("src", "src")
//!                 .child(TreeNode::new("main", "main.rs"))
//!                 .child(TreeNode::new("lib", "lib.rs")),
//!             TreeNode::new("readme", "README.md"),
//!         ])
//!         .expand("src")
//! });
//! cx.subscribe(&tree, |_this, _tree, event: &TreeViewEvent, _cx| match event {
//!     TreeViewEvent::Selected(id) => println!("selected {id}"),
//!     TreeViewEvent::Toggled(id, open) => println!("{id} expanded: {open}"),
//!     TreeViewEvent::Activated(id) => println!("activated {id}"),
//! })
//! .detach();
//! ```

use std::collections::HashSet;

use gpui::prelude::*;
use gpui::{
    div, px, ClickEvent, Context, EventEmitter, FocusHandle, IntoElement, KeyDownEvent,
    MouseButton, SharedString, Window,
};

use crate::icon::{Icon, IconName};
use crate::reactive::Signal;
use crate::theme::{theme, Size};

/// One node in a [`TreeView`]. A node with children is a branch (chevron +
/// folder icon); a node without children is a leaf (file icon).
#[derive(Debug, Clone)]
pub struct TreeNode {
    /// Stable identifier — carried by every [`TreeViewEvent`].
    pub id: SharedString,
    /// The text shown on the row.
    pub label: SharedString,
    /// Optional glyph shown before the label. Defaults to a folder/file glyph
    /// picked by branch/leaf.
    pub icon: Option<IconName>,
    /// Child nodes; empty means leaf.
    pub children: Vec<TreeNode>,
}

impl TreeNode {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        TreeNode {
            id: id.into(),
            label: label.into(),
            icon: None,
            children: Vec::new(),
        }
    }

    /// Override the row glyph (defaults to folder/file by branch/leaf).
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Append one child node.
    pub fn child(mut self, child: TreeNode) -> Self {
        self.children.push(child);
        self
    }

    /// Append several child nodes.
    pub fn children(mut self, children: impl IntoIterator<Item = TreeNode>) -> Self {
        self.children.extend(children);
        self
    }

    /// A node with no children.
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }
}

/// Emitted by [`TreeView`]. All variants carry the node id.
#[derive(Debug, Clone)]
pub enum TreeViewEvent {
    /// The selection moved to this node.
    Selected(SharedString),
    /// A branch was expanded (`true`) or collapsed (`false`).
    Toggled(SharedString, bool),
    /// Enter or double-click on a node.
    Activated(SharedString),
}

/// One visible (not hidden by a collapsed ancestor) row, in paint order.
#[derive(Debug, Clone, PartialEq)]
struct VisibleRow {
    id: SharedString,
    label: SharedString,
    icon: Option<IconName>,
    depth: usize,
    is_branch: bool,
    expanded: bool,
}

/// Depth-first flatten of the nodes whose ancestors are all expanded.
fn flatten_visible(
    nodes: &[TreeNode],
    expanded: &HashSet<SharedString>,
    depth: usize,
    out: &mut Vec<VisibleRow>,
) {
    for node in nodes {
        let is_branch = !node.children.is_empty();
        let is_expanded = is_branch && expanded.contains(&node.id);
        out.push(VisibleRow {
            id: node.id.clone(),
            label: node.label.clone(),
            icon: node.icon,
            depth,
            is_branch,
            expanded: is_expanded,
        });
        if is_expanded {
            flatten_visible(&node.children, expanded, depth + 1, out);
        }
    }
}

/// The visible rows for a node list + expanded set.
fn visible(nodes: &[TreeNode], expanded: &HashSet<SharedString>) -> Vec<VisibleRow> {
    let mut out = Vec::new();
    flatten_visible(nodes, expanded, 0, &mut out);
    out
}

/// Every branch id in the tree (for `default_expanded`).
fn collect_branch_ids(nodes: &[TreeNode], out: &mut HashSet<SharedString>) {
    for node in nodes {
        if !node.children.is_empty() {
            out.insert(node.id.clone());
            collect_branch_ids(&node.children, out);
        }
    }
}

/// What a horizontal arrow key does, in visible-row terms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeyMove {
    /// Move the selection to this visible index.
    To(usize),
    /// Expand (`true`) or collapse (`false`) the branch at this index.
    Set(usize, bool),
    /// Nothing to do.
    None,
}

/// Down-arrow target: first row when nothing is selected, else clamp below.
fn step_down(len: usize, current: Option<usize>) -> Option<usize> {
    match (len, current) {
        (0, _) => None,
        (_, None) => Some(0),
        (len, Some(i)) => Some((i + 1).min(len - 1)),
    }
}

/// Up-arrow target: last row when nothing is selected, else clamp above.
fn step_up(len: usize, current: Option<usize>) -> Option<usize> {
    match (len, current) {
        (0, _) => None,
        (len, None) => Some(len - 1),
        (_, Some(i)) => Some(i.saturating_sub(1)),
    }
}

/// Right arrow: expand a collapsed branch, step into an expanded one.
fn step_right(rows: &[VisibleRow], current: usize) -> KeyMove {
    let Some(row) = rows.get(current) else {
        return KeyMove::None;
    };
    if !row.is_branch {
        return KeyMove::None;
    }
    if !row.expanded {
        return KeyMove::Set(current, true);
    }
    match rows.get(current + 1) {
        Some(next) if next.depth == row.depth + 1 => KeyMove::To(current + 1),
        _ => KeyMove::None,
    }
}

/// Left arrow: collapse an expanded branch, else step to the parent.
fn step_left(rows: &[VisibleRow], current: usize) -> KeyMove {
    let Some(row) = rows.get(current) else {
        return KeyMove::None;
    };
    if row.is_branch && row.expanded {
        return KeyMove::Set(current, false);
    }
    if row.depth == 0 {
        return KeyMove::None;
    }
    // The parent is the nearest preceding row one level up.
    (0..current)
        .rev()
        .find(|&i| rows[i].depth + 1 == row.depth)
        .map(KeyMove::To)
        .unwrap_or(KeyMove::None)
}

/// A hierarchical list. Create with `cx.new(|cx| TreeView::new(cx).nodes(...))`.
pub struct TreeView {
    nodes: Vec<TreeNode>,
    expanded: HashSet<SharedString>,
    selected: Option<SharedString>,
    expand_all: bool,
    focus: FocusHandle,
}

impl EventEmitter<TreeViewEvent> for TreeView {}

impl TreeView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        TreeView {
            nodes: Vec::new(),
            expanded: HashSet::new(),
            selected: None,
            expand_all: false,
            focus: cx.focus_handle(),
        }
    }

    /// Set the tree data.
    pub fn nodes(mut self, nodes: Vec<TreeNode>) -> Self {
        self.nodes = nodes;
        if self.expand_all {
            collect_branch_ids(&self.nodes, &mut self.expanded);
        }
        self
    }

    /// Drive the tree data from a `Signal<Vec<TreeNode>>`: the view adopts the
    /// signal's nodes now and re-reads them on every signal change. Expansion
    /// and selection survive data updates (they are keyed by node id).
    pub fn bind_nodes(mut self, signal: &Signal<Vec<TreeNode>>, cx: &mut Context<Self>) -> Self {
        self.nodes = signal.get(cx);
        if self.expand_all {
            collect_branch_ids(&self.nodes, &mut self.expanded);
        }
        cx.observe(signal.entity(), |this, observed, cx| {
            this.nodes = observed.read(cx).clone();
            // `default_expanded(true)` promises expand-all for nodes assigned
            // later too, so new branches join the expanded set here.
            if this.expand_all {
                collect_branch_ids(&this.nodes, &mut this.expanded);
            }
            cx.notify();
        })
        .detach();
        self
    }

    /// Expand the branch with this id (construction-time; users toggle live).
    pub fn expand(mut self, id: impl Into<SharedString>) -> Self {
        self.expanded.insert(id.into());
        self
    }

    /// Collapse the branch with this id.
    pub fn collapse(mut self, id: impl Into<SharedString>) -> Self {
        self.expanded.remove(&id.into());
        self
    }

    /// Start with every branch expanded. Applies to the current nodes and to
    /// nodes assigned later via [`nodes`](Self::nodes) / [`bind_nodes`](Self::bind_nodes).
    pub fn default_expanded(mut self, expanded: bool) -> Self {
        self.expand_all = expanded;
        if expanded {
            collect_branch_ids(&self.nodes, &mut self.expanded);
        }
        self
    }

    /// The ids of every expanded branch, sorted for determinism.
    pub fn expanded_ids(&self) -> Vec<SharedString> {
        let mut ids: Vec<SharedString> = self.expanded.iter().cloned().collect();
        ids.sort();
        ids
    }

    /// The id of the selected node, if any.
    pub fn selected_id(&self) -> Option<SharedString> {
        self.selected.clone()
    }

    /// Move the selection, emit, repaint. No-op when already selected.
    fn select(&mut self, id: SharedString, cx: &mut Context<Self>) {
        if self.selected.as_ref() == Some(&id) {
            return;
        }
        self.selected = Some(id.clone());
        cx.emit(TreeViewEvent::Selected(id));
        cx.notify();
    }

    /// Flip a branch open/closed.
    fn toggle(&mut self, id: SharedString, cx: &mut Context<Self>) {
        let open = !self.expanded.contains(&id);
        self.set_expanded(id, open, cx);
    }

    /// Set a branch's expansion, emit, repaint. No-op when unchanged.
    fn set_expanded(&mut self, id: SharedString, open: bool, cx: &mut Context<Self>) {
        let changed = if open {
            self.expanded.insert(id.clone())
        } else {
            self.expanded.remove(&id)
        };
        if changed {
            cx.emit(TreeViewEvent::Toggled(id, open));
            cx.notify();
        }
    }

    /// Carry out a [`KeyMove`] against the current visible rows.
    fn apply(&mut self, mv: KeyMove, rows: &[VisibleRow], cx: &mut Context<Self>) {
        match mv {
            KeyMove::To(i) => self.select(rows[i].id.clone(), cx),
            KeyMove::Set(i, open) => self.set_expanded(rows[i].id.clone(), open, cx),
            KeyMove::None => {}
        }
    }

    fn on_key(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let rows = visible(&self.nodes, &self.expanded);
        if rows.is_empty() {
            return;
        }
        let current = self
            .selected
            .as_ref()
            .and_then(|id| rows.iter().position(|row| &row.id == id));

        let handled = match event.keystroke.key.as_str() {
            "down" => {
                if let Some(i) = step_down(rows.len(), current) {
                    self.select(rows[i].id.clone(), cx);
                }
                true
            }
            "up" => {
                if let Some(i) = step_up(rows.len(), current) {
                    self.select(rows[i].id.clone(), cx);
                }
                true
            }
            "right" => match current {
                Some(i) => {
                    self.apply(step_right(&rows, i), &rows, cx);
                    true
                }
                None => false,
            },
            "left" => match current {
                Some(i) => {
                    self.apply(step_left(&rows, i), &rows, cx);
                    true
                }
                None => false,
            },
            "enter" => match self.selected.clone() {
                Some(id) => {
                    cx.emit(TreeViewEvent::Activated(id));
                    true
                }
                None => false,
            },
            _ => false,
        };
        if handled {
            cx.stop_propagation();
        }
    }
}

impl Render for TreeView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        let text = t.text().hsla();
        let dimmed = t.dimmed().hsla();
        let accent = t.primary().hsla();
        let surface_hover = t.surface_hover().hsla();
        let selected_bg = t.primary().alpha(0.12);
        let indent = t.spacing(Size::Md);
        let radius = t.radius(Size::Sm);
        let font = t.font_size(Size::Sm);

        let rows = visible(&self.nodes, &self.expanded);
        let selected = self.selected.clone();

        let mut root = div()
            .id("guise-treeview")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _ev, window, cx| {
                    window.focus(&this.focus, cx);
                    cx.notify();
                }),
            )
            .flex()
            .flex_col()
            .gap(px(2.0));

        for (i, row) in rows.into_iter().enumerate() {
            let is_selected = selected.as_ref() == Some(&row.id);
            let is_branch = row.is_branch;
            let id = row.id.clone();
            let hover_bg = if is_selected {
                selected_bg
            } else {
                surface_hover
            };

            // Fixed-width chevron cell so branch and leaf labels align.
            let mut chevron = div()
                .w(px(16.0))
                .flex()
                .items_center()
                .justify_center()
                .text_color(dimmed);
            if is_branch {
                chevron = chevron.child(SharedString::new_static(if row.expanded {
                    IconName::ChevronDown.glyph()
                } else {
                    IconName::ChevronRight.glyph()
                }));
            }

            let fallback = if is_branch {
                IconName::Menu
            } else {
                IconName::Dot
            };
            let glyph = row.icon.unwrap_or(fallback);
            let icon = div()
                .text_color(if is_selected { accent } else { dimmed })
                .child(Icon::new(glyph).size(Size::Xs));

            let mut el = div()
                .id(("guise-tree-row", i))
                .flex()
                .items_center()
                .gap(px(6.0))
                .pl(px(6.0 + indent * row.depth as f32))
                .pr(px(8.0))
                .py(px(4.0))
                .rounded(px(radius))
                .text_size(px(font))
                .text_color(text)
                .hover(move |s| s.bg(hover_bg))
                .child(chevron)
                .child(icon)
                .child(row.label.clone())
                .on_click(cx.listener(move |this, ev: &ClickEvent, _window, cx| {
                    this.select(id.clone(), cx);
                    if ev.click_count() > 1 {
                        cx.emit(TreeViewEvent::Activated(id.clone()));
                    } else if is_branch {
                        this.toggle(id.clone(), cx);
                    }
                }));
            if is_selected {
                el = el.bg(selected_bg);
            }
            root = root.child(el);
        }

        root
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<TreeNode> {
        vec![
            TreeNode::new("src", "src")
                .child(TreeNode::new("main", "main.rs"))
                .child(TreeNode::new("data", "data").child(TreeNode::new("tree", "tree.rs"))),
            TreeNode::new("readme", "README.md"),
        ]
    }

    fn expanded(ids: &[&'static str]) -> HashSet<SharedString> {
        ids.iter().map(|id| SharedString::from(*id)).collect()
    }

    fn ids(rows: &[VisibleRow]) -> Vec<&str> {
        rows.iter().map(|row| row.id.as_ref()).collect()
    }

    #[test]
    fn collapsed_tree_shows_only_roots() {
        let rows = visible(&sample(), &expanded(&[]));
        assert_eq!(ids(&rows), ["src", "readme"]);
        assert!(rows[0].is_branch && !rows[0].expanded);
        assert!(!rows[1].is_branch);
    }

    #[test]
    fn expanded_branches_flatten_depth_first() {
        let rows = visible(&sample(), &expanded(&["src", "data"]));
        assert_eq!(ids(&rows), ["src", "main", "data", "tree", "readme"]);
        let depths: Vec<usize> = rows.iter().map(|row| row.depth).collect();
        assert_eq!(depths, [0, 1, 1, 2, 0]);
        assert!(rows[2].expanded);
    }

    #[test]
    fn collapsed_parent_hides_expanded_descendants() {
        // "data" is expanded but its parent "src" is not, so it stays hidden.
        let rows = visible(&sample(), &expanded(&["data"]));
        assert_eq!(ids(&rows), ["src", "readme"]);
    }

    #[test]
    fn up_and_down_clamp_at_the_edges() {
        assert_eq!(step_down(3, None), Some(0));
        assert_eq!(step_down(3, Some(1)), Some(2));
        assert_eq!(step_down(3, Some(2)), Some(2));
        assert_eq!(step_up(3, None), Some(2));
        assert_eq!(step_up(3, Some(1)), Some(0));
        assert_eq!(step_up(3, Some(0)), Some(0));
        assert_eq!(step_down(0, None), None);
        assert_eq!(step_up(0, Some(1)), None);
    }

    #[test]
    fn right_expands_then_steps_into_the_branch() {
        let closed = visible(&sample(), &expanded(&[]));
        assert_eq!(step_right(&closed, 0), KeyMove::Set(0, true));

        let open = visible(&sample(), &expanded(&["src"]));
        assert_eq!(step_right(&open, 0), KeyMove::To(1));
        // Leaf rows don't react to right.
        assert_eq!(step_right(&open, 1), KeyMove::None);
    }

    #[test]
    fn left_collapses_then_walks_to_the_parent() {
        let rows = visible(&sample(), &expanded(&["src", "data"]));
        // Expanded branch collapses in place.
        assert_eq!(step_left(&rows, 0), KeyMove::Set(0, false));
        // A child moves to its parent, skipping same-depth siblings.
        assert_eq!(step_left(&rows, 1), KeyMove::To(0));
        assert_eq!(step_left(&rows, 3), KeyMove::To(2));
        // A collapsed root has nowhere to go.
        assert_eq!(step_left(&rows, 4), KeyMove::None);
    }

    #[test]
    fn branch_ids_cover_nested_branches_only() {
        let mut out = HashSet::new();
        collect_branch_ids(&sample(), &mut out);
        assert_eq!(out, expanded(&["src", "data"]));
    }
}
