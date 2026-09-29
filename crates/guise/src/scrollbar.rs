//! `Scrollbar` — a draggable thumb over anything that tracks a `ScrollHandle`.
//!
//! gpui scrolls a container but draws nothing for it, so a long list gives no
//! hint of how much there is or that the end has been reached. `ScrollArea`
//! carries one of these by default; use `Scrollbar` directly for a container
//! you scroll yourself: give the container `.track_scroll(&handle)`, put it in
//! a `.relative()` parent, and add `Scrollbar::new(id, &handle)` beside it. A
//! virtualized `list(state, ..)` has no `ScrollHandle`; `Scrollbar::for_list`
//! reads its `ListState` instead. The bar lays itself over the whole parent,
//! draws nothing while the content fits, and follows the source — including
//! when the wheel or a program moves it.
//!
//! The geometry is pure functions over three lengths (viewport, scrollable
//! extent, track) so it can be tested without a window.

use gpui::prelude::*;
use gpui::{
  div, point, px, Axis, Bounds, ElementId, Entity, IntoElement, ListState, MouseButton,
  MouseDownEvent, Pixels, Render, ScrollHandle, Window,
};

use crate::devtools::Probed;
use crate::theme::{theme, Size};

/// Shortest thumb, so a very long document leaves something to grab.
const MIN_THUMB: f32 = 24.0;

/// Where the thumb sits along a track: `(start, len)` in px, or `None` when
/// nothing overflows and there is nothing to draw. `offset` is how far the
/// content has scrolled (positive), `max` how far it can.
pub(crate) fn thumb(viewport: f32, max: f32, offset: f32, track: f32) -> Option<(f32, f32)> {
  if max <= 0.5 || viewport <= 0.0 || track <= 0.0 {
    return None;
  }
  let len = (track * viewport / (viewport + max))
    .max(MIN_THUMB)
    .min(track);
  let travel = track - len;
  let start = travel * (offset / max).clamp(0.0, 1.0);
  Some((start, len))
}

/// The scroll offset (positive, px) that puts the thumb's start at `pos` along
/// the track — the inverse of [`thumb`], used while dragging.
pub(crate) fn offset_for(pos: f32, max: f32, track: f32, len: f32) -> f32 {
  let travel = track - len;
  if travel <= 0.0 {
    return 0.0;
  }
  (pos / travel).clamp(0.0, 1.0) * max
}

/// The drag payload. `on_drag_move::<ScrollDrag>` is what keeps the events
/// coming when the pointer leaves the thumb, but it fires for every bar on the
/// page — so the payload names the bar being dragged and the others ignore it.
struct ScrollDrag(ElementId);

/// gpui shows the entity a drag constructs under the pointer; an empty one.
struct Ghost;

impl Render for Ghost {
  fn render(&mut self, _window: &mut Window, _cx: &mut gpui::Context<Self>) -> impl IntoElement {
    div()
  }
}

/// What the bar reads and moves: a plain container's handle, or a virtualized
/// list's state (which measures its items lazily and so needs the drag hints).
#[derive(Clone)]
enum Source {
  Handle(ScrollHandle),
  List(ListState),
}

impl Source {
  fn bounds(&self) -> Bounds<Pixels> {
    match self {
      Source::Handle(h) => h.bounds(),
      Source::List(l) => l.viewport_bounds(),
    }
  }

  fn max_offset(&self) -> gpui::Size<Pixels> {
    match self {
      Source::Handle(h) => h.max_offset(),
      Source::List(l) => l.max_offset_for_scrollbar(),
    }
  }

  fn offset(&self) -> gpui::Point<Pixels> {
    match self {
      Source::Handle(h) => h.offset(),
      Source::List(l) => l.scroll_px_offset_for_scrollbar(),
    }
  }

  fn set_offset(&self, to: gpui::Point<Pixels>) {
    match self {
      Source::Handle(h) => h.set_offset(to),
      Source::List(l) => l.set_offset_from_scrollbar(to),
    }
  }

  fn drag(&self, started: bool) {
    if let Source::List(l) = self {
      if started {
        l.scrollbar_drag_started();
      } else {
        l.scrollbar_drag_ended();
      }
    }
  }
}

/// A scrollbar for a container tracked by a `ScrollHandle`, or for a `list`.
/// Vertical unless `.horizontal(true)`.
#[derive(IntoElement)]
pub struct Scrollbar {
  id: ElementId,
  source: Source,
  axis: Axis,
}

impl Scrollbar {
  pub fn new(id: impl Into<ElementId>, handle: &ScrollHandle) -> Self {
    Scrollbar {
      id: id.into(),
      source: Source::Handle(handle.clone()),
      axis: Axis::Vertical,
    }
  }

  /// A bar for a virtualized `list(state, ..)`, which scrolls by its
  /// `ListState` rather than a `ScrollHandle`.
  pub fn for_list(id: impl Into<ElementId>, state: &ListState) -> Self {
    Scrollbar {
      id: id.into(),
      source: Source::List(state.clone()),
      axis: Axis::Vertical,
    }
  }

  /// Run along the bottom edge for a horizontally scrolling container.
  pub fn horizontal(mut self, horizontal: bool) -> Self {
    self.axis = if horizontal {
      Axis::Horizontal
    } else {
      Axis::Vertical
    };
    self
  }
}

/// `(viewport, extent, offset, track origin)` along `axis`, from the handle.
fn measure(handle: &Source, axis: Axis) -> (f32, f32, f32, f32) {
  let bounds: Bounds<Pixels> = handle.bounds();
  let max = handle.max_offset();
  let offset = handle.offset();
  match axis {
    Axis::Vertical => (
      bounds.size.height.into(),
      max.height.into(),
      -f32::from(offset.y),
      bounds.origin.y.into(),
    ),
    Axis::Horizontal => (
      bounds.size.width.into(),
      max.width.into(),
      -f32::from(offset.x),
      bounds.origin.x.into(),
    ),
  }
}

impl RenderOnce for Scrollbar {
  fn render(self, window: &mut Window, cx: &mut gpui::App) -> impl IntoElement {
    let axis = self.axis;
    let vertical = axis == Axis::Vertical;
    let (viewport, max, offset, origin) = measure(&self.source, axis);
    // The handle learns its size when the container is first laid out, which
    // is after this renders; ask for the frame that has it.
    if viewport <= 0.0 {
      window.request_animation_frame();
    }
    let Some((start, len)) = thumb(viewport, max, offset, viewport) else {
      return div()
        .probe("Scrollbar")
        .attr("visible", "no")
        .into_any_element();
    };

    let t = theme(cx);
    let girth = t.spacing(Size::Sm);
    let inset = t.spacing(Size::Xs) / 4.0;
    let radius = t.radius(Size::Xl);
    let rest = t.dimmed().hsla().opacity(0.45);
    let active = t.dimmed().hsla().opacity(0.8);

    // Where in the thumb the pointer took hold, so it doesn't jump to centre
    // under the cursor when the drag starts.
    let grab: Entity<f32> = window.use_keyed_state((self.id.clone(), "grab"), cx, |_, _| 0.0);

    let pos = move |p: gpui::Point<Pixels>| -> f32 {
      if vertical {
        f32::from(p.y) - origin
      } else {
        f32::from(p.x) - origin
      }
    };

    let handle = self.source.clone();
    let on_track = {
      let handle = handle.clone();
      move |ev: &MouseDownEvent, window: &mut Window, _cx: &mut gpui::App| {
        // A click on the bare track centres the thumb there.
        let to = offset_for(pos(ev.position) - len / 2.0, max, viewport, len);
        set(&handle, axis, to);
        window.refresh();
      }
    };
    let on_thumb = {
      let grab = grab.clone();
      let source = handle.clone();
      move |ev: &MouseDownEvent, _window: &mut Window, cx: &mut gpui::App| {
        let inside = pos(ev.position) - start;
        source.drag(true);
        grab.update(cx, |g, _| *g = inside);
        cx.stop_propagation();
      }
    };
    let on_move = {
      let handle = handle.clone();
      let grab = grab.clone();
      let id = self.id.clone();
      move |ev: &gpui::DragMoveEvent<ScrollDrag>, window: &mut Window, cx: &mut gpui::App| {
        if ev.drag(cx).0 != id {
          return;
        }
        let held = *grab.read(cx);
        let to = offset_for(pos(ev.event.position) - held, max, viewport, len);
        set(&handle, axis, to);
        window.refresh();
      }
    };

    let mut thumb = div()
      .id((self.id.clone(), "thumb"))
      .absolute()
      .rounded(px(radius))
      .bg(rest)
      .hover(move |s| s.bg(active))
      .on_mouse_down(MouseButton::Left, on_thumb)
      .on_mouse_up(MouseButton::Left, {
        let source = handle.clone();
        move |_, _, _| source.drag(false)
      })
      .on_mouse_up_out(MouseButton::Left, {
        let source = handle.clone();
        move |_, _, _| source.drag(false)
      })
      .on_drag(ScrollDrag(self.id.clone()), |_, _, _, cx| cx.new(|_| Ghost));
    thumb = if vertical {
      thumb
        .left(px(inset))
        .right(px(inset))
        .top(px(start))
        .h(px(len))
    } else {
      thumb
        .top(px(inset))
        .bottom(px(inset))
        .left(px(start))
        .w(px(len))
    };

    let mut track = div()
      .id(self.id)
      .absolute()
      .on_mouse_down(MouseButton::Left, on_track)
      .on_drag_move::<ScrollDrag>(on_move)
      .child(thumb);
    track = if vertical {
      track.top_0().bottom_0().right_0().w(px(girth))
    } else {
      track.left_0().right_0().bottom_0().h(px(girth))
    };
    track
      .probe("Scrollbar")
      .attr("visible", "yes")
      .into_any_element()
  }
}

/// Scroll to `offset` px (positive) along `axis`, leaving the other axis alone.
fn set(handle: &Source, axis: Axis, offset: f32) {
  let now = handle.offset();
  handle.set_offset(match axis {
    Axis::Vertical => point(now.x, px(-offset)),
    Axis::Horizontal => point(px(-offset), now.y),
  });
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn nothing_to_draw_when_the_content_fits() {
    assert_eq!(thumb(400.0, 0.0, 0.0, 400.0), None);
    assert_eq!(thumb(0.0, 100.0, 0.0, 0.0), None);
  }

  #[test]
  fn thumb_is_the_visible_fraction_of_the_track() {
    // 400 visible of 1600 total: a quarter of the track.
    let (start, len) = thumb(400.0, 1200.0, 0.0, 400.0).unwrap();
    assert_eq!(start, 0.0);
    assert_eq!(len, 100.0);
  }

  #[test]
  fn thumb_reaches_the_far_end_at_full_scroll() {
    let (start, len) = thumb(400.0, 1200.0, 1200.0, 400.0).unwrap();
    assert_eq!(start + len, 400.0);
  }

  #[test]
  fn thumb_never_shrinks_below_a_grabbable_size() {
    let (_, len) = thumb(400.0, 400_000.0, 0.0, 400.0).unwrap();
    assert_eq!(len, MIN_THUMB);
  }

  #[test]
  fn overscroll_is_clamped_onto_the_track() {
    let (start, len) = thumb(400.0, 1200.0, 5000.0, 400.0).unwrap();
    assert_eq!(start + len, 400.0);
    let (start, _) = thumb(400.0, 1200.0, -50.0, 400.0).unwrap();
    assert_eq!(start, 0.0);
  }

  #[test]
  fn dragging_inverts_the_geometry() {
    let (start, len) = thumb(400.0, 1200.0, 600.0, 400.0).unwrap();
    let back = offset_for(start, 1200.0, 400.0, len);
    assert!((back - 600.0).abs() < 0.01, "got {back}");
    assert_eq!(offset_for(-30.0, 1200.0, 400.0, len), 0.0);
    assert_eq!(offset_for(9000.0, 1200.0, 400.0, len), 1200.0);
  }

  #[test]
  fn a_track_with_no_travel_stays_at_the_top() {
    assert_eq!(offset_for(10.0, 100.0, 24.0, 24.0), 0.0);
  }
}
