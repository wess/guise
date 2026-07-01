//! `SplitPanel` — two live panes with a draggable divider (gpui entity).
//!
//! Pane content is a builder closure re-invoked every render (like `Tabs`), so
//! panes show live data — including another `SplitPanel`'s element, which is
//! how nested layouts are built.
//!
//! ```ignore
//! let split = cx.new(|cx| {
//!     SplitPanel::new(cx)
//!         .direction(SplitDirection::Horizontal)
//!         .ratio(0.3)
//!         .min_first(120.0)
//!         .first(|_, _| Text::new("Sidebar"))
//!         .second(|_, _| Text::new("Main content"))
//! });
//! cx.subscribe(&split, |_, _, SplitPanelEvent::Resized(ratio), _| { /* … */ })
//!     .detach();
//! ```

use gpui::prelude::*;
use gpui::{
    div, px, App, Context, DragMoveEvent, Empty, EntityId, EventEmitter, IntoElement, Window,
};

use crate::data::Content;
use crate::style::FlexExt;
use crate::theme::theme;

/// Which way the panes are laid out. `Horizontal` places them side by side
/// (a vertical divider, column-resize cursor); `Vertical` stacks them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SplitDirection {
    #[default]
    Horizontal,
    Vertical,
}

/// Emitted while the divider is dragged. Carries the new first-pane ratio
/// in `0.0..=1.0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SplitPanelEvent {
    Resized(f32),
}

/// Drag payload for the divider. Carries the owning panel's id so nested
/// `SplitPanel`s ignore each other's drags (`on_drag_move` fires for every
/// active drag of this type, anywhere in the window).
struct DividerDrag {
    panel: EntityId,
}

/// A resizable two-pane layout. Create with
/// `cx.new(|cx| SplitPanel::new(cx).first(..).second(..))` and give the
/// element a sized parent — the panel fills it.
pub struct SplitPanel {
    direction: SplitDirection,
    first: Option<Content>,
    second: Option<Content>,
    ratio: f32,
    min_first: f32,
    min_second: f32,
    handle_size: f32,
}

impl EventEmitter<SplitPanelEvent> for SplitPanel {}

impl SplitPanel {
    pub fn new(_cx: &mut Context<Self>) -> Self {
        SplitPanel {
            direction: SplitDirection::Horizontal,
            first: None,
            second: None,
            ratio: 0.5,
            min_first: 40.0,
            min_second: 40.0,
            handle_size: 6.0,
        }
    }

    pub fn direction(mut self, direction: SplitDirection) -> Self {
        self.direction = direction;
        self
    }

    /// The first pane (left / top). Rebuilt each render so it can show live
    /// data — including another `SplitPanel`'s element for nesting.
    pub fn first<E>(mut self, content: impl Fn(&mut Window, &mut App) -> E + 'static) -> Self
    where
        E: IntoElement,
    {
        self.first = Some(Box::new(move |window, cx| {
            content(window, cx).into_any_element()
        }));
        self
    }

    /// The second pane (right / bottom). Rebuilt each render.
    pub fn second<E>(mut self, content: impl Fn(&mut Window, &mut App) -> E + 'static) -> Self
    where
        E: IntoElement,
    {
        self.second = Some(Box::new(move |window, cx| {
            content(window, cx).into_any_element()
        }));
        self
    }

    /// Initial share of the axis given to the first pane (clamped to `0..=1`).
    pub fn ratio(mut self, ratio: f32) -> Self {
        self.ratio = ratio.clamp(0.0, 1.0);
        self
    }

    /// Minimum pixel size of the first pane while dragging.
    pub fn min_first(mut self, min: f32) -> Self {
        self.min_first = min.max(0.0);
        self
    }

    /// Minimum pixel size of the second pane while dragging.
    pub fn min_second(mut self, min: f32) -> Self {
        self.min_second = min.max(0.0);
        self
    }

    /// Thickness of the divider's grab area in pixels.
    pub fn handle_size(mut self, size: f32) -> Self {
        self.handle_size = size.max(1.0);
        self
    }

    /// The current first-pane ratio.
    pub fn current_ratio(&self) -> f32 {
        self.ratio
    }
}

/// Resolve a divider drag into the next first-pane ratio. `pos` is the pointer
/// offset from the container's leading edge along the split axis, `extent` the
/// container's size on that axis. The divider centers under the pointer, and
/// both panes keep their minimum sizes.
fn drag_ratio(pos: f32, extent: f32, handle: f32, min_first: f32, min_second: f32) -> f32 {
    let avail = (extent - handle).max(1.0);
    let lo = min_first.min(avail);
    let hi = (avail - min_second).max(lo);
    (pos - handle * 0.5).clamp(lo, hi) / avail
}

impl Render for SplitPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        let line = t.border().hsla();
        let grip = t.primary().alpha(0.35);

        let horizontal = matches!(self.direction, SplitDirection::Horizontal);
        let handle = self.handle_size;
        let ratio = self.ratio.clamp(0.0, 1.0);
        let min_first = self.min_first;
        let min_second = self.min_second;
        let panel = cx.entity().entity_id();

        let first = self.first.as_ref().map(|build| build(window, cx));
        let second = self.second.as_ref().map(|build| build(window, cx));

        let mut first_pane = div().flex_basis(px(0.0)).grow(ratio).overflow_hidden();
        first_pane = if horizontal {
            first_pane.min_w(px(min_first))
        } else {
            first_pane.min_h(px(min_first))
        };
        if let Some(el) = first {
            first_pane = first_pane.child(el);
        }

        let mut second_pane = div()
            .flex_basis(px(0.0))
            .grow(1.0 - ratio)
            .overflow_hidden();
        second_pane = if horizontal {
            second_pane.min_w(px(min_second))
        } else {
            second_pane.min_h(px(min_second))
        };
        if let Some(el) = second {
            second_pane = second_pane.child(el);
        }

        let mut divider = div()
            .id("guise-splitpanel-divider")
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .hover(move |s| s.bg(grip))
            .on_drag(DividerDrag { panel }, |_, _offset, _window, cx| {
                cx.new(|_| Empty)
            });
        divider = if horizontal {
            divider
                .w(px(handle))
                .h_full()
                .cursor_col_resize()
                .child(div().w(px(1.0)).h_full().bg(line))
        } else {
            divider
                .h(px(handle))
                .w_full()
                .cursor_row_resize()
                .child(div().h(px(1.0)).w_full().bg(line))
        };

        let mut root = div()
            .id("guise-splitpanel")
            .size_full()
            .flex()
            .on_drag_move(cx.listener(
                move |this, ev: &DragMoveEvent<DividerDrag>, _window, cx| {
                    let source = ev.drag(cx).panel;
                    if source != panel {
                        return;
                    }
                    let bounds = ev.bounds;
                    let (pos, extent) = if matches!(this.direction, SplitDirection::Horizontal) {
                        (
                            f32::from(ev.event.position.x - bounds.left()),
                            f32::from(bounds.size.width),
                        )
                    } else {
                        (
                            f32::from(ev.event.position.y - bounds.top()),
                            f32::from(bounds.size.height),
                        )
                    };
                    let next = drag_ratio(
                        pos,
                        extent,
                        this.handle_size,
                        this.min_first,
                        this.min_second,
                    );
                    if (next - this.ratio).abs() > f32::EPSILON {
                        this.ratio = next;
                        cx.emit(SplitPanelEvent::Resized(next));
                        cx.notify();
                    }
                },
            ));
        root = if horizontal {
            root.flex_row()
        } else {
            root.flex_col()
        };

        root.child(first_pane).child(divider).child(second_pane)
    }
}

#[cfg(test)]
mod tests {
    use super::drag_ratio;

    #[test]
    fn centered_pointer_is_half() {
        // 206px container, 6px handle: pointer at 103 puts 100px of the
        // 200px of pane space on each side.
        assert_eq!(drag_ratio(103.0, 206.0, 6.0, 0.0, 0.0), 0.5);
    }

    #[test]
    fn clamps_to_min_first() {
        let ratio = drag_ratio(10.0, 406.0, 6.0, 80.0, 0.0);
        assert_eq!(ratio, 80.0 / 400.0);
    }

    #[test]
    fn clamps_to_min_second() {
        let ratio = drag_ratio(400.0, 406.0, 6.0, 0.0, 120.0);
        assert_eq!(ratio, (400.0 - 120.0) / 400.0);
    }

    #[test]
    fn overshoot_stays_in_range() {
        assert_eq!(drag_ratio(-500.0, 206.0, 6.0, 0.0, 0.0), 0.0);
        assert_eq!(drag_ratio(900.0, 206.0, 6.0, 0.0, 0.0), 1.0);
    }

    #[test]
    fn degenerate_extent_prefers_min_first() {
        // Container smaller than the minimums: the first pane's floor wins,
        // and the result never divides by zero.
        let ratio = drag_ratio(30.0, 60.0, 6.0, 100.0, 100.0);
        assert_eq!(ratio, 1.0);
    }
}
