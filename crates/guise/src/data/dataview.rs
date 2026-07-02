//! `DataView` — a collection-bound list/grid (gpui entity).
//!
//! The collection-binding counterpart to [`crate::reactive::Binding`]: the view
//! observes a `Signal<Vec<T>>` and repaints whenever the collection changes —
//! no manual wiring. Filtering and sorting are *projections* applied at render
//! time over the borrowed data (NSArrayController-style): the source vector is
//! never copied or reordered, the view just renders a filtered + sorted list of
//! indices into it.
//!
//! ```ignore
//! let todos = use_state(cx, vec!["Write docs".to_string(), "Ship".to_string()]);
//! let view = cx.new(|cx| {
//!     DataView::new(cx, &todos)
//!         .item(|todo, _ix, _window, _cx| {
//!             Text::new(todo.clone()).into_any_element()
//!         })
//!         .sort_by(|a, b| a.cmp(b))
//!         .selectable()
//! });
//! cx.subscribe(&view, |_, _, DataViewEvent::Selected(ix), _| {
//!     println!("picked source row {ix}");
//! })
//! .detach();
//!
//! // Anywhere, later: the view repaints by itself.
//! todos.update(cx, |list| list.push("Celebrate".into()));
//! ```

use std::cmp::Ordering;

use gpui::prelude::*;
use gpui::{div, px, AnyElement, App, Context, EventEmitter, IntoElement, SharedString, Window};

use super::Content;
use crate::reactive::Signal;
use crate::style::{surface, Variant};
use crate::theme::{theme, Size};

/// Emitted when a selectable item is clicked. Carries the item's index into
/// the **source** vector (not its display position), so it stays valid under
/// any filter/sort projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataViewEvent {
    Selected(usize),
}

/// How the items flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DataViewLayout {
    /// A vertical list (the default).
    #[default]
    List,
    /// Rows of `n` equal-width cells.
    Grid(usize),
}

type ItemBuilder<T> = Box<dyn Fn(&T, usize, &mut Window, &mut App) -> AnyElement + 'static>;
type FilterFn<T> = Box<dyn Fn(&T) -> bool + 'static>;
type SortFn<T> = Box<dyn Fn(&T, &T) -> Ordering + 'static>;
type FilterRef<'a, T> = Option<&'a dyn Fn(&T) -> bool>;
type SortRef<'a, T> = Option<&'a dyn Fn(&T, &T) -> Ordering>;

/// A signal-bound collection view. Create with
/// `cx.new(|cx| DataView::new(cx, &signal).item(...))`.
pub struct DataView<T: 'static> {
    source: Signal<Vec<T>>,
    item: Option<ItemBuilder<T>>,
    filter: Option<FilterFn<T>>,
    sort: Option<SortFn<T>>,
    layout: DataViewLayout,
    gap: Size,
    empty: Option<Content>,
    selectable: bool,
    selected: Option<usize>,
}

impl<T: 'static> EventEmitter<DataViewEvent> for DataView<T> {}

impl<T: 'static> DataView<T> {
    /// Bind the view to a collection signal. Every `set`/`update` on the
    /// signal repaints the view.
    pub fn new(cx: &mut Context<Self>, source: &Signal<Vec<T>>) -> Self {
        cx.observe(source.entity(), |this, source, cx| {
            // Drop a selection whose item fell off the end: keeping the stale
            // index would hand callers an out-of-range value and silently
            // re-select whatever item lands there after the source regrows.
            let len = source.read(cx).len();
            if this.selected.is_some_and(|i| i >= len) {
                this.selected = None;
            }
            cx.notify();
        })
        .detach();
        DataView {
            source: source.clone(),
            item: None,
            filter: None,
            sort: None,
            layout: DataViewLayout::List,
            gap: Size::Sm,
            empty: None,
            selectable: false,
            selected: None,
        }
    }

    /// The item template, re-invoked every frame with the borrowed item and
    /// its source index — items always show live data.
    pub fn item<E>(
        mut self,
        template: impl Fn(&T, usize, &mut Window, &mut App) -> E + 'static,
    ) -> Self
    where
        E: IntoElement,
    {
        self.item = Some(Box::new(move |item, ix, window, cx| {
            template(item, ix, window, cx).into_any_element()
        }));
        self
    }

    /// Show only the items matching `pred`. A projection: the source vector
    /// is untouched.
    pub fn filter(mut self, pred: impl Fn(&T) -> bool + 'static) -> Self {
        self.filter = Some(Box::new(pred));
        self
    }

    /// Display order (stable sort). A projection: the source vector is
    /// untouched.
    pub fn sort_by(mut self, cmp: impl Fn(&T, &T) -> Ordering + 'static) -> Self {
        self.sort = Some(Box::new(cmp));
        self
    }

    pub fn layout(mut self, layout: DataViewLayout) -> Self {
        self.layout = layout;
        self
    }

    /// Spacing between items (default `Sm`).
    pub fn gap(mut self, gap: Size) -> Self {
        self.gap = gap;
        self
    }

    /// Shown when the projection yields nothing (empty source or everything
    /// filtered out). Rebuilt each render.
    pub fn empty<E>(mut self, content: impl Fn(&mut Window, &mut App) -> E + 'static) -> Self
    where
        E: IntoElement,
    {
        self.empty = Some(Box::new(move |window, cx| {
            content(window, cx).into_any_element()
        }));
        self
    }

    /// Enable single selection: items get hover/selected styling and clicks
    /// emit [`DataViewEvent::Selected`].
    pub fn selectable(mut self) -> Self {
        self.selectable = true;
        self
    }

    /// The selected **source** index, if any.
    pub fn selected_index(&self) -> Option<usize> {
        self.selected
    }
}

/// The display order: indices into `items`, filtered then stably sorted.
fn projection<T>(items: &[T], filter: FilterRef<'_, T>, sort: SortRef<'_, T>) -> Vec<usize> {
    let mut order: Vec<usize> = (0..items.len())
        .filter(|&i| filter.is_none_or(|keep| keep(&items[i])))
        .collect();
    if let Some(cmp) = sort {
        order.sort_by(|&a, &b| cmp(&items[a], &items[b]));
    }
    order
}

impl<T: 'static> Render for DataView<T> {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        let gap = t.spacing(self.gap);
        let radius = t.radius(t.default_radius);
        let hover_bg = t.surface_hover().hsla();
        let dimmed = t.dimmed().hsla();
        let font_sm = t.font_size(Size::Sm);
        // Same treatment as an active NavLink: the primary color's Light
        // (tinted) surface.
        let sel = surface(t, t.primary_color, Variant::Light);
        let (selected_bg, selected_fg) = (sel.bg, sel.fg);

        // Build the projected items while the source entity is leased; the
        // template borrows each item in place — no clone of the collection.
        let template = self.item.as_ref();
        let filter = self.filter.as_deref();
        let sort = self.sort.as_deref();
        let entity = self.source.entity().clone();
        let built: Vec<(usize, AnyElement)> = entity.update(cx, |items, cx| {
            let order = projection(items, filter, sort);
            match template {
                Some(build) => order
                    .into_iter()
                    .map(|i| (i, build(&items[i], i, window, cx)))
                    .collect(),
                None => Vec::new(),
            }
        });

        let selectable = self.selectable;
        // The source observer prunes out-of-range selections, so this index
        // is always valid for the current collection.
        let selected = self.selected;

        if built.is_empty() {
            let content = match &self.empty {
                Some(build) => build(window, cx),
                None => div()
                    .text_size(px(font_sm))
                    .text_color(dimmed)
                    .child(SharedString::new_static("Nothing to show"))
                    .into_any_element(),
            };
            return div()
                .w_full()
                .flex()
                .justify_center()
                .py(px(16.0))
                .child(content);
        }

        let cells: Vec<AnyElement> = built
            .into_iter()
            .map(|(source_ix, element)| {
                if !selectable {
                    return element;
                }
                let is_selected = selected == Some(source_ix);
                let mut cell = div()
                    .id(("guise-dataview-item", source_ix))
                    .px(px(10.0))
                    .py(px(8.0))
                    .rounded(px(radius))
                    .cursor_pointer()
                    .child(element)
                    .on_click(cx.listener(move |this, _ev, _window, cx| {
                        this.selected = Some(source_ix);
                        cx.emit(DataViewEvent::Selected(source_ix));
                        cx.notify();
                    }));
                cell = if is_selected {
                    cell.bg(selected_bg).text_color(selected_fg)
                } else {
                    cell.hover(move |s| s.bg(hover_bg))
                };
                cell.into_any_element()
            })
            .collect();

        let root = div().w_full().flex().flex_col().gap(px(gap));
        match self.layout {
            DataViewLayout::List => root.children(cells),
            DataViewLayout::Grid(cols) => {
                let cols = cols.max(1);
                let count = cells.len();
                let mut rows = Vec::new();
                let mut row = Vec::new();
                for (i, cell) in cells.into_iter().enumerate() {
                    row.push(div().flex_1().min_w(px(0.0)).child(cell));
                    if row.len() == cols || i + 1 == count {
                        // Pad the last row so cells keep equal widths.
                        while row.len() < cols {
                            row.push(div().flex_1().min_w(px(0.0)));
                        }
                        rows.push(div().flex().gap(px(gap)).children(std::mem::take(&mut row)));
                    }
                }
                root.children(rows)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_without_projections() {
        assert_eq!(projection(&[10, 20, 30], None, None), vec![0, 1, 2]);
        assert_eq!(projection::<i32>(&[], None, None), Vec::<usize>::new());
    }

    #[test]
    fn filter_keeps_source_indices() {
        let even = |n: &i32| n % 2 == 0;
        let order = projection(&[1, 2, 3, 4, 5, 6], Some(&even), None);
        assert_eq!(order, vec![1, 3, 5]);
    }

    #[test]
    fn sort_orders_indices_without_moving_items() {
        let cmp = |a: &i32, b: &i32| a.cmp(b);
        let items = [30, 10, 20];
        let order = projection(&items, None, Some(&cmp));
        assert_eq!(order, vec![1, 2, 0]);
        // The source is untouched; the order just points into it.
        assert_eq!(items, [30, 10, 20]);
    }

    #[test]
    fn filter_then_sort_compose() {
        let over_two = |n: &i32| *n > 2;
        let desc = |a: &i32, b: &i32| b.cmp(a);
        let order = projection(&[1, 4, 3, 2, 5], Some(&over_two), Some(&desc));
        assert_eq!(order, vec![4, 1, 2]); // values 5, 4, 3
    }

    #[test]
    fn sort_is_stable_for_equal_keys() {
        let by_len = |a: &&str, b: &&str| a.len().cmp(&b.len());
        let items = ["bb", "aa", "c", "dd"];
        let order = projection(&items, None, Some(&by_len));
        // "c" first, then the three two-char items in source order.
        assert_eq!(order, vec![2, 0, 1, 3]);
    }
}
