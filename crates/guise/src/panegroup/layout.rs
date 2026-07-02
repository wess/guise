//! Pure rect math: turn a [`PaneTree`] into pane and divider rects.

use crate::SplitDirection;
use super::id::{PaneId, SplitId};
use super::tree::{Node, PaneTree};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn center(&self) -> (f32, f32) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }
}

/// Computed pane and divider rects, both in layout order.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    pub panes: Vec<(PaneId, Rect)>,
    pub dividers: Vec<(SplitId, Rect, SplitDirection)>,
}

impl Layout {
    pub fn pane_rect(&self, pane: PaneId) -> Option<Rect> {
        self.panes.iter().find(|(p, _)| *p == pane).map(|(_, r)| *r)
    }
}

/// Lay out `tree` inside `rect`, reserving `divider` thickness per split.
pub fn compute_layout(tree: &PaneTree, rect: Rect, divider: f32) -> Layout {
    let mut layout = Layout {
        panes: Vec::new(),
        dividers: Vec::new(),
    };
    walk(tree.root(), rect, divider, &mut layout);
    layout
}

fn walk(node: &Node, rect: Rect, divider: f32, out: &mut Layout) {
    match node {
        Node::Leaf(pane) => out.panes.push((*pane, rect)),
        Node::Split {
            id,
            axis,
            ratio,
            first,
            second,
        } => {
            let (frect, drect, srect) = match axis {
                SplitDirection::Horizontal => {
                    let avail = (rect.w - divider).max(0.0);
                    let fw = avail * ratio;
                    (
                        Rect::new(rect.x, rect.y, fw, rect.h),
                        Rect::new(rect.x + fw, rect.y, divider, rect.h),
                        Rect::new(rect.x + fw + divider, rect.y, avail - fw, rect.h),
                    )
                }
                SplitDirection::Vertical => {
                    let avail = (rect.h - divider).max(0.0);
                    let fh = avail * ratio;
                    (
                        Rect::new(rect.x, rect.y, rect.w, fh),
                        Rect::new(rect.x, rect.y + fh, rect.w, divider),
                        Rect::new(rect.x, rect.y + fh + divider, rect.w, avail - fh),
                    )
                }
            };
            out.dividers.push((*id, drect, *axis));
            walk(first, frect, divider, out);
            walk(second, srect, divider, out);
        }
    }
}
