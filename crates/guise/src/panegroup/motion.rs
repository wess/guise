//! Tab-strip motion: the cross-frame state behind tabs that grow in, collapse
//! out, and slide to a new order instead of snapping into it.
//!
//! gpui's `with_animation` drives one element's own properties from the moment
//! that element appears, which none of these are: a collapsing tab has already
//! left the model, and a reorder moves tabs that were on screen the whole time.
//! So the strip keeps its own clock — an effect is the instant its mutation
//! happened, and a frame is a pure function of how long ago that was. A
//! terminal repainting at 60fps therefore replays a finished animation as its
//! finished state rather than starting a new one.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use gpui::{ease_in_out, SharedString};

use super::{ItemId, PaneId};

/// How long a tab takes to grow in, collapse out, or slide home. Long enough
/// to read as motion, short enough that closing a run of tabs never feels like
/// waiting on the one before.
const SPAN: Duration = Duration::from_millis(140);

/// Sub-pixel widths and shifts are already where they belong; animating them
/// would only cost frames.
const EPSILON: f32 = 0.5;

/// A tab as it was last laid out. The width is read back from the strip's own
/// scroll handle (layout is the only place a real one exists) and the title
/// from render, so a tab can still be drawn after the host has dropped the item
/// behind it.
struct Seen {
  title: SharedString,
  width: f32,
}

/// A tab that has left its pane but is still on screen, collapsing.
struct Ghost {
  pane: PaneId,
  /// The slot it held, so the gap closes where the tab was rather than at the
  /// end of the strip.
  index: usize,
  title: SharedString,
  width: f32,
  at: Instant,
}

/// A collapsing tab as the strip has to draw it.
pub(super) struct Ghosting {
  /// The slot it held.
  pub(super) slot: usize,
  pub(super) title: SharedString,
  /// The width it had, which its label is still laid out against, and the
  /// width its slot is down to now.
  pub(super) was: f32,
  pub(super) now: f32,
}

/// One of a strip's children: a live tab, by its index in the pane, or a tab
/// collapsing out of it, by its index in the pane's ghosts.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Slot {
  Tab(usize),
  Ghost(usize),
}

/// The strip's children in order: every tab, with each collapsing tab back in
/// the slot it held. A ghost is a child of the scroller exactly as a tab is, so
/// this is the order every per-child measurement has to count in — ask the
/// scroller about tab `i` and you are off by every ghost ahead of it.
pub(super) fn slots(tabs: usize, ghosts: &[usize]) -> Vec<Slot> {
  let mut out = Vec::with_capacity(tabs + ghosts.len());
  let mut next = 0;
  for tab in 0..tabs {
    while ghosts.get(next).is_some_and(|&slot| slot <= tab) {
      out.push(Slot::Ghost(next));
      next += 1;
    }
    out.push(Slot::Tab(tab));
  }
  for ghost in next..ghosts.len() {
    out.push(Slot::Ghost(ghost));
  }
  out
}

/// Every tab effect in flight, keyed by item — the only identity that survives
/// a reorder, which is precisely what changes an index.
#[derive(Default)]
pub(super) struct TabMotion {
  seen: HashMap<ItemId, Seen>,
  opening: HashMap<ItemId, Instant>,
  closing: Vec<Ghost>,
  sliding: HashMap<ItemId, (f32, Instant)>,
}

impl TabMotion {
  /// Note the label a tab is drawing with, so the tab can outlive its item.
  pub(super) fn label(&mut self, item: ItemId, title: &SharedString) {
    match self.seen.get_mut(&item) {
      Some(seen) => seen.title = title.clone(),
      None => {
        self.seen.insert(
          item,
          Seen {
            title: title.clone(),
            width: 0.0,
          },
        );
      }
    }
  }

  /// Note the width a tab was just laid out at.
  pub(super) fn laid_out(&mut self, item: ItemId, width: f32) {
    if let Some(seen) = self.seen.get_mut(&item) {
      seen.width = width;
    }
  }

  pub(super) fn width(&self, item: ItemId) -> f32 {
    self.seen.get(&item).map_or(0.0, |seen| seen.width)
  }

  /// A tab was inserted: grow it in from nothing. Any shift it was owed is
  /// moot now that it is starting from zero width.
  pub(super) fn opened(&mut self, item: ItemId) {
    self.sliding.remove(&item);
    self.opening.insert(item, Instant::now());
  }

  /// A tab left `pane` from `index`: keep drawing it while it collapses. Does
  /// nothing for a tab that was never laid out, since there is no width to
  /// collapse from.
  pub(super) fn closed(&mut self, pane: PaneId, index: usize, item: ItemId) {
    let Some(seen) = self.seen.get(&item) else {
      return;
    };
    if seen.width < EPSILON {
      return;
    }
    self.closing.push(Ghost {
      pane,
      index,
      title: seen.title.clone(),
      width: seen.width,
      at: Instant::now(),
    });
  }

  /// The strip went from `before` to `after`. A reorder moves tabs without
  /// resizing any of them, so the same widths describe both strips and each
  /// tab's shift is exact — no measuring the new layout first, which would
  /// cost a frame with everything already snapped into place.
  pub(super) fn reordered(&mut self, before: &[ItemId], after: &[ItemId]) {
    let (was, now) = (self.offsets(before), self.offsets(after));
    let at = Instant::now();
    for item in after {
      let (Some(&from), Some(&to)) = (was.get(item), now.get(item)) else {
        continue;
      };
      // A tab reordered again mid-slide is still short of its old home;
      // carry that remainder so it never jumps back to start again.
      let shift = from - to + self.slide(*item).unwrap_or(0.0);
      if shift.abs() >= EPSILON {
        self.sliding.insert(*item, (shift, at));
      } else {
        self.sliding.remove(item);
      }
    }
  }

  /// Where each tab of `order` starts, laying the strip out from the widths
  /// already seen.
  fn offsets(&self, order: &[ItemId]) -> HashMap<ItemId, f32> {
    let mut x = 0.0;
    order
      .iter()
      .map(|&item| {
        let at = x;
        x += self.width(item);
        (item, at)
      })
      .collect()
  }

  /// How far a tab has grown in, `None` once it is an ordinary tab.
  pub(super) fn opening(&self, item: ItemId) -> Option<f32> {
    self.opening.get(&item).map(|at| eased(*at))
  }

  /// The pixels a tab is still short of its resting slot.
  pub(super) fn slide(&self, item: ItemId) -> Option<f32> {
    self
      .sliding
      .get(&item)
      .map(|(shift, at)| shift * (1.0 - eased(*at)))
  }

  /// The collapsing tabs of `pane`, in strip order.
  pub(super) fn ghosts(&self, pane: PaneId) -> Vec<Ghosting> {
    let mut out: Vec<_> = self
      .closing
      .iter()
      .filter(|g| g.pane == pane)
      .map(|g| Ghosting {
        slot: g.index,
        title: g.title.clone(),
        was: g.width,
        now: g.width * (1.0 - eased(g.at)),
      })
      .collect();
    out.sort_by_key(|g| g.slot);
    out
  }

  /// Retire finished effects and forget tabs that are gone. Reports whether
  /// anything is still moving, which is the only thing that should keep
  /// frames coming.
  pub(super) fn settle(&mut self, live: &[ItemId]) -> bool {
    self.seen.retain(|item, _| live.contains(item));
    self
      .opening
      .retain(|item, at| live.contains(item) && !spent(*at));
    self
      .sliding
      .retain(|item, (_, at)| live.contains(item) && !spent(*at));
    self.closing.retain(|g| !spent(g.at));
    !(self.opening.is_empty() && self.sliding.is_empty() && self.closing.is_empty())
  }
}

/// Progress of an effect started at `at`, eased.
fn eased(at: Instant) -> f32 {
  ease_in_out((at.elapsed().as_secs_f32() / SPAN.as_secs_f32()).clamp(0.0, 1.0))
}

fn spent(at: Instant) -> bool {
  at.elapsed() >= SPAN
}

#[cfg(test)]
mod tests {
  use super::super::id::{ItemIds, PaneIds};
  use super::*;

  #[test]
  fn ghosts_hold_the_slot_they_left() {
    use Slot::{Ghost, Tab};
    assert_eq!(slots(3, &[]), [Tab(0), Tab(1), Tab(2)]);
    assert_eq!(slots(3, &[1]), [Tab(0), Ghost(0), Tab(1), Tab(2)]);
    // Past the last tab, and two out of the same slot.
    assert_eq!(slots(2, &[2]), [Tab(0), Tab(1), Ghost(0)]);
    assert_eq!(slots(1, &[0, 0]), [Ghost(0), Ghost(1), Tab(0)]);
    assert_eq!(slots(0, &[0]), [Ghost(0)]);
  }

  #[test]
  fn a_reorder_shifts_by_the_widths_between() {
    let mut ids = ItemIds::new();
    let it: Vec<_> = (0..3).map(|_| ids.next()).collect();
    let mut m = TabMotion::default();
    for (i, &item) in it.iter().enumerate() {
      m.label(item, &SharedString::from(format!("{i}")));
      m.laid_out(item, 100.0);
    }

    m.reordered(&[it[0], it[1], it[2]], &[it[2], it[0], it[1]]);
    // The tab that jumped two slots left starts two widths right of home,
    // and the two it displaced start one width left of theirs.
    assert!((m.slide(it[2]).unwrap() - 200.0).abs() < 1.0);
    assert!((m.slide(it[0]).unwrap() + 100.0).abs() < 1.0);
    assert!((m.slide(it[1]).unwrap() + 100.0).abs() < 1.0);
  }

  #[test]
  fn a_tab_never_laid_out_leaves_no_ghost() {
    let mut ids = ItemIds::new();
    let mut panes = PaneIds::new();
    let (item, pane) = (ids.next(), panes.next());
    let mut m = TabMotion::default();

    m.closed(pane, 0, item); // never even labelled
    m.label(item, &SharedString::from("t"));
    m.closed(pane, 0, item); // labelled, but zero-width
    assert!(m.ghosts(pane).is_empty());

    m.laid_out(item, 120.0);
    m.closed(pane, 0, item);
    assert_eq!(m.ghosts(pane).len(), 1);
  }
}
