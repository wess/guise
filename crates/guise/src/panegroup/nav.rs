//! Focus navigation between panes: directional (via layout rects) and ordinal.

use super::id::PaneId;
use super::layout::{Layout, Rect};
use super::tree::PaneTree;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

const EPS: f32 = 0.001;

/// The pane reached by moving `direction` from `from`: the pane whose facing
/// edge is closest (adjoining wins) and whose center is nearest on the
/// perpendicular axis. `None` at the workspace edge or if `from` is unknown.
pub fn neighbor(layout: &Layout, from: PaneId, direction: Direction) -> Option<PaneId> {
    let origin = layout.pane_rect(from)?;
    let mut best: Option<(f32, f32, PaneId)> = None;
    for (pane, rect) in &layout.panes {
        if *pane == from {
            continue;
        }
        let Some(gap) = edgegap(origin, *rect, direction) else {
            continue;
        };
        let dist = centerdistance(origin, *rect, direction);
        let closer = match best {
            None => true,
            Some((bgap, bdist, _)) => {
                gap < bgap - EPS || ((gap - bgap).abs() <= EPS && dist < bdist)
            }
        };
        if closer {
            best = Some((gap, dist, *pane));
        }
    }
    best.map(|(_, _, pane)| pane)
}

/// Next pane after `from` in layout order, wrapping.
pub fn next(tree: &PaneTree, from: PaneId) -> Option<PaneId> {
    let panes = tree.panes();
    let i = panes.iter().position(|p| *p == from)?;
    Some(panes[(i + 1) % panes.len()])
}

/// Previous pane before `from` in layout order, wrapping.
pub fn prev(tree: &PaneTree, from: PaneId) -> Option<PaneId> {
    let panes = tree.panes();
    let i = panes.iter().position(|p| *p == from)?;
    Some(panes[(i + panes.len() - 1) % panes.len()])
}

/// Distance from `from`'s facing edge to `to`'s opposing edge, or `None`
/// if `to` is not on the `dir` side of `from`.
fn edgegap(from: Rect, to: Rect, dir: Direction) -> Option<f32> {
    let gap = match dir {
        Direction::Left => from.x - (to.x + to.w),
        Direction::Right => to.x - (from.x + from.w),
        Direction::Up => from.y - (to.y + to.h),
        Direction::Down => to.y - (from.y + from.h),
    };
    (gap >= -EPS).then(|| gap.max(0.0))
}

fn centerdistance(from: Rect, to: Rect, dir: Direction) -> f32 {
    let (fx, fy) = from.center();
    let (tx, ty) = to.center();
    match dir {
        Direction::Left | Direction::Right => (fy - ty).abs(),
        Direction::Up | Direction::Down => (fx - tx).abs(),
    }
}
