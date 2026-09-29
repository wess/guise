//! `ScrollArea` — a bounded, scrollable container.
//!
//! Desktop UIs scroll; most builders assume their content fits. Wrap an
//! overflowing column (or row) in a `ScrollArea` and give it a bound. There are
//! two, and which one is right is a layout question, not a preference:
//! `max_height` for a list that occupies a fixed slice of a larger layout, and
//! `fill` for a pane that should be as tall as whatever the window gives it.
//! Each instance needs a unique id so gpui can track its scroll offset.
//!
//! It draws a [`Scrollbar`] over its edge whenever the content overflows, so a
//! long list says how long it is; `.scrollbar(false)` leaves it bare.

use crate::devtools::Probed;
use crate::scrollbar::Scrollbar;
use gpui::prelude::*;
use gpui::{
  div, px, AnyElement, App, ElementId, Entity, IntoElement, ScrollHandle, SharedString, Window,
};

/// A scrollable region. `ScrollArea::new("id").max_height(240.0)`, or
/// `ScrollArea::new("id").fill()` to take the space the parent has left.
#[derive(IntoElement)]
pub struct ScrollArea {
  id: ElementId,
  children: Vec<AnyElement>,
  max_height: Option<f32>,
  fill: bool,
  horizontal: bool,
  scrollbar: bool,
}

impl ScrollArea {
  pub fn new(id: impl Into<ElementId>) -> Self {
    ScrollArea {
      id: id.into(),
      children: Vec::new(),
      max_height: None,
      fill: false,
      horizontal: false,
      scrollbar: true,
    }
  }

  /// Clip to this height (px) and scroll past it.
  pub fn max_height(mut self, height: f32) -> Self {
    self.max_height = Some(height);
    self
  }

  /// Take the space the parent has left over, and scroll past it — the mode
  /// for a full-height pane, where any fixed number is wrong at every window
  /// size but one.
  ///
  /// The parent still has to be bounded itself; filling an unbounded parent
  /// sizes to the content and there is nothing to scroll.
  pub fn fill(mut self) -> Self {
    self.fill = true;
    self
  }

  /// Draw a scrollbar while the content overflows (on by default). It floats
  /// over the edge rather than taking layout space, so turning it off changes
  /// nothing about where the content sits.
  pub fn scrollbar(mut self, show: bool) -> Self {
    self.scrollbar = show;
    self
  }

  /// Scroll horizontally instead of vertically.
  pub fn horizontal(mut self, horizontal: bool) -> Self {
    self.horizontal = horizontal;
    self
  }
}

impl ParentElement for ScrollArea {
  fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
    self.children.extend(elements);
  }
}

impl RenderOnce for ScrollArea {
  fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
    let bound: SharedString = match (self.fill, self.max_height) {
      (true, _) => "fill".into(),
      (false, Some(height)) => format!("{height}px").into(),
      (false, None) => "none".into(),
    };
    // The scrollbar needs the offset, so the scroller tracks a handle we own.
    // It lives in element state under this area's id, across frames.
    let state: Entity<ScrollHandle> =
      window.use_keyed_state((self.id.clone(), "handle"), cx, |_, _| ScrollHandle::new());
    let handle = state.read(cx).clone();

    // Two boxes: the outer one is what the parent lays out (so it takes the
    // fill/cap rules the scroller used to), the inner one scrolls and the bar
    // floats over the outer one — an absolute child of the scroller itself
    // would scroll away with the content.
    let mut outer = div().relative().flex();
    let mut inner = div().id(self.id.clone()).flex().track_scroll(&handle);
    if self.horizontal {
      inner = inner.flex_row().overflow_x_scroll().min_w_0();
      if self.fill {
        // Three settings for three parents: `flex_1` claims the leftover
        // main axis under a flex parent, the relative size does the same
        // under a plain block one (where grow means nothing, and where a
        // flex basis would win anyway if both applied), and the zero
        // minimum is what lets the box shrink under its content instead
        // of pushing the parent open.
        outer = outer.flex_1().w_full().min_w_0();
      }
      inner = inner.w_full();
    } else {
      inner = inner.flex_col().overflow_y_scroll().min_h_0();
      if self.fill {
        outer = outer.flex_1().h_full().min_h_0();
      }
      inner = inner.h_full().w_full();
    }
    // A cap still applies while filling: grow into the window, but never
    // past this. Both boxes carry it — the outer so it stops growing, the
    // inner so it is the one that scrolls.
    if let Some(height) = self.max_height {
      outer = outer.max_h(px(height));
      inner = inner.max_h(px(height));
    }
    let bar = self
      .scrollbar
      .then(|| Scrollbar::new((self.id.clone(), "bar"), &handle).horizontal(self.horizontal));
    outer
      .child(inner.children(self.children))
      .children(bar)
      .probe("ScrollArea")
      .attr("axis", if self.horizontal { "x" } else { "y" })
      .attr("bound", bound)
      .attr("scrollbar", if self.scrollbar { "on" } else { "off" })
  }
}
