//! `TableView` — a rich, generic data table (gpui entity).
//!
//! Renders typed rows through per-column cell closures, with sortable
//! headers, click/cmd/shift row selection, a sticky header, drag-resizable
//! columns and an optionally virtualized body. The simple string [`Table`]
//! (`data/table.rs`) remains for simple cases.
//!
//! ```ignore
//! struct User { name: String, age: u32 }
//!
//! let table = cx.new(|cx| {
//!     TableView::new(cx)
//!         .columns(vec![
//!             Column::new("Name")
//!                 .text(|u: &User| u.name.clone().into())
//!                 .sortable_by(|a, b| a.name.cmp(&b.name)),
//!             Column::new("Age")
//!                 .width(80.0)
//!                 .align(Align::End)
//!                 .text(|u: &User| u.age.to_string().into())
//!                 .sortable_by(|a, b| a.age.cmp(&b.age)),
//!         ])
//!         .rows(users)
//!         .selection_mode(SelectionMode::Multi)
//!         .striped(true)
//!         .with_border(true)
//!         .height(320.0) // fixed height => virtualized, scrollable body
//! });
//! cx.subscribe(&table, |_, _, event: &TableViewEvent, _| match event {
//!     TableViewEvent::SelectionChanged(rows) => println!("selected {rows:?}"),
//!     TableViewEvent::Activated(row) => println!("open {row}"),
//!     TableViewEvent::Sorted(sort) => println!("sort {sort:?}"),
//! })
//! .detach();
//! ```

mod state;

pub use state::{SelectionMode, SortDir};

use std::cmp::Ordering;
use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    div, px, uniform_list, AnyElement, App, Bounds, Context, Div, DragMoveEvent, Empty, EntityId,
    EventEmitter, FocusHandle, FontWeight, KeyDownEvent, MouseButton, MouseDownEvent, Pixels,
    ScrollStrategy, SharedString, Subscription, UniformListScrollHandle, WeakEntity, Window,
};

use self::state::{cycle_sort, identity_order, sorted_order, SelectionState};
use super::Content;
use crate::layout::Align;
use crate::reactive::Signal;
use crate::style::FlexExt;
use crate::theme::{theme, ColorName, Size};

/// Events emitted by [`TableView`]. All row indices refer to the **source**
/// rows, not the current display order.
#[derive(Debug, Clone)]
pub enum TableViewEvent {
    /// The set of selected source rows changed (ascending indices).
    SelectionChanged(Vec<usize>),
    /// A row was activated by double-click or Enter.
    Activated(usize),
    /// The sort changed: `Some((column, dir))`, or `None` when cleared.
    Sorted(Option<(usize, SortDir)>),
}

type Comparator<T> = Rc<dyn Fn(&T, &T) -> Ordering>;
type CellBuilder<T> = Rc<dyn Fn(&T, &mut Window, &mut App) -> AnyElement>;

enum CellContent<T> {
    Text(Rc<dyn Fn(&T) -> SharedString>),
    Element(CellBuilder<T>),
}

/// One column of a [`TableView`]: header title, width policy, alignment,
/// optional sort comparator, and a cell renderer.
pub struct Column<T> {
    title: SharedString,
    width: Option<f32>,
    flex: f32,
    min_width: f32,
    align: Align,
    sort: Option<Comparator<T>>,
    content: Option<CellContent<T>>,
}

impl<T> Column<T> {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Column {
            title: title.into(),
            width: None,
            flex: 1.0,
            min_width: 60.0,
            align: Align::Start,
            sort: None,
            content: None,
        }
    }

    /// Fixed pixel width. Without it the column flexes (see [`Column::flex`]).
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Grow factor for flexing columns (default `1.0`).
    pub fn flex(mut self, flex: f32) -> Self {
        self.flex = flex;
        self
    }

    /// Lower width bound, honored by both flex sizing and drag-resizing
    /// (default `60.0`).
    pub fn min_width(mut self, min_width: f32) -> Self {
        self.min_width = min_width;
        self
    }

    /// Horizontal alignment of the header and cells (default `Align::Start`).
    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    /// Make the column sortable. A header click cycles ascending →
    /// descending → unsorted; the sort is a stable reorder of display
    /// indices and never mutates the rows.
    pub fn sortable_by(mut self, cmp: impl Fn(&T, &T) -> Ordering + 'static) -> Self {
        self.sort = Some(Rc::new(cmp));
        self
    }

    /// Custom cell renderer, re-invoked every frame so cells show live data.
    pub fn cell<E>(mut self, cell: impl Fn(&T, &mut Window, &mut App) -> E + 'static) -> Self
    where
        E: IntoElement,
    {
        self.content = Some(CellContent::Element(Rc::new(move |row, window, cx| {
            cell(row, window, cx).into_any_element()
        })));
        self
    }

    /// Text-cell convenience: the string truncates with an ellipsis when the
    /// column is too narrow.
    pub fn text(mut self, text: impl Fn(&T) -> SharedString + 'static) -> Self {
        self.content = Some(CellContent::Text(Rc::new(text)));
        self
    }
}

/// Row storage: an owned snapshot, or a live binding to a `Signal`.
enum Rows<T> {
    Owned(Rc<Vec<T>>),
    Bound(Signal<Vec<T>>),
}

impl<T> Clone for Rows<T> {
    fn clone(&self) -> Self {
        match self {
            Rows::Owned(rows) => Rows::Owned(rows.clone()),
            Rows::Bound(signal) => Rows::Bound(signal.clone()),
        }
    }
}

/// Drag payload for the header resize grips. `owner` scopes `on_drag_move` to
/// the table that started the drag — the listener fires for every active drag
/// of this type in the window, including other tables'.
struct ResizeDrag {
    owner: EntityId,
    column: usize,
}

/// Resolved width policy for one column.
#[derive(Clone, Copy)]
enum ColWidth {
    Fixed(f32),
    Flex(f32, f32), // (grow factor, min width)
}

/// A rich data table. Create with
/// `cx.new(|cx| TableView::new(cx).columns(...).rows(...))`.
pub struct TableView<T: 'static> {
    columns: Vec<Column<T>>,
    rows: Rows<T>,
    focus: FocusHandle,
    mode: SelectionMode,
    selection: SelectionState,
    sort: Option<(usize, SortDir)>,
    /// Source index of each visible row, in display order. Recomputed at the
    /// top of every render; listeners map display → source through it.
    display_order: Vec<usize>,
    /// Columns converted to fixed widths by drag-resizing.
    resized: HashMap<usize, f32>,
    /// Header-cell bounds captured after prepaint, for resize math.
    header_bounds: Vec<Bounds<Pixels>>,
    /// The `bind_rows` observer; dropped (cancelled) by `set_rows`/rebinding.
    rows_sub: Option<Subscription>,
    striped: bool,
    highlight_on_hover: bool,
    with_border: bool,
    height: Option<f32>,
    empty: Option<Content>,
    scroll: UniformListScrollHandle,
}

impl<T: 'static> EventEmitter<TableViewEvent> for TableView<T> {}

impl<T: 'static> TableView<T> {
    pub fn new(cx: &mut Context<Self>) -> Self {
        TableView {
            columns: Vec::new(),
            rows: Rows::Owned(Rc::new(Vec::new())),
            focus: cx.focus_handle(),
            mode: SelectionMode::None,
            selection: SelectionState::default(),
            sort: None,
            display_order: Vec::new(),
            resized: HashMap::new(),
            header_bounds: Vec::new(),
            rows_sub: None,
            striped: false,
            highlight_on_hover: false,
            with_border: false,
            height: None,
            empty: None,
            scroll: UniformListScrollHandle::new(),
        }
    }

    pub fn columns(mut self, columns: Vec<Column<T>>) -> Self {
        self.columns = columns;
        self
    }

    /// Provide the rows as an owned snapshot. Replace later with
    /// [`TableView::set_rows`].
    pub fn rows(mut self, rows: Vec<T>) -> Self {
        self.rows = Rows::Owned(Rc::new(rows));
        self
    }

    /// Bind the rows to a `Signal<Vec<T>>`: the table observes the signal
    /// (signal writes repaint it) and reads the rows at render, so it always
    /// shows the live value. Selection is pruned when rows disappear.
    pub fn bind_rows(mut self, signal: &Signal<Vec<T>>, cx: &mut Context<Self>) -> Self {
        self.rows = Rows::Bound(signal.clone());
        // Held, not detached: `set_rows` (or a rebind) drops the subscription,
        // so a stale observer never prunes against the old signal's length.
        self.rows_sub = Some(cx.observe(signal.entity(), |this, rows, cx| {
            let len = rows.read(cx).len();
            this.prune_selection(len, cx);
            cx.notify();
        }));
        self
    }

    pub fn selection_mode(mut self, mode: SelectionMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn striped(mut self, striped: bool) -> Self {
        self.striped = striped;
        self
    }

    pub fn highlight_on_hover(mut self, highlight: bool) -> Self {
        self.highlight_on_hover = highlight;
        self
    }

    pub fn with_border(mut self, with_border: bool) -> Self {
        self.with_border = with_border;
        self
    }

    /// Fix the body height (px). The body becomes a virtualized
    /// `uniform_list` scroll region — rows must share one height — and the
    /// header stays outside it, so it is sticky for free.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// Rendered instead of the body when there are no rows.
    pub fn empty<E>(mut self, builder: impl Fn(&mut Window, &mut App) -> E + 'static) -> Self
    where
        E: IntoElement,
    {
        self.empty = Some(Box::new(move |window, cx| {
            builder(window, cx).into_any_element()
        }));
        self
    }

    // --- Entity methods ------------------------------------------------------

    /// Replace the rows with a new owned snapshot (drops any signal binding).
    pub fn set_rows(&mut self, rows: Vec<T>, cx: &mut Context<Self>) {
        let len = rows.len();
        self.rows = Rows::Owned(Rc::new(rows));
        self.rows_sub = None;
        self.prune_selection(len, cx);
        cx.notify();
    }

    /// The selected source-row indices, ascending.
    pub fn selected(&self) -> Vec<usize> {
        self.selection.selected()
    }

    /// The active sort, if any.
    pub fn sort_state(&self) -> Option<(usize, SortDir)> {
        self.sort
    }

    pub fn focus_handle(&self) -> FocusHandle {
        self.focus.clone()
    }

    // --- Internals -----------------------------------------------------------

    fn prune_selection(&mut self, len: usize, cx: &mut Context<Self>) {
        if self.selection.retain_below(len) {
            cx.emit(TableViewEvent::SelectionChanged(self.selection.selected()));
        }
    }

    /// The display order for this frame: a stable index sort when a sorted
    /// column is active, identity otherwise. Never touches the source rows.
    fn compute_order(&self, cx: &App) -> Vec<usize> {
        let sort = self.sort.and_then(|(col, dir)| {
            let cmp = self.columns.get(col)?.sort.clone()?;
            Some((dir, cmp))
        });
        match &self.rows {
            Rows::Owned(rows) => order_of(rows, sort),
            Rows::Bound(signal) => order_of(signal.read(cx), sort),
        }
    }

    fn col_width(&self, ix: usize) -> ColWidth {
        let col = &self.columns[ix];
        if let Some(&w) = self.resized.get(&ix) {
            ColWidth::Fixed(w.max(col.min_width))
        } else if let Some(w) = col.width {
            ColWidth::Fixed(w.max(col.min_width))
        } else {
            ColWidth::Flex(col.flex, col.min_width)
        }
    }

    fn toggle_sort(&mut self, column: usize, cx: &mut Context<Self>) {
        self.sort = cycle_sort(self.sort, column);
        cx.emit(TableViewEvent::Sorted(self.sort));
        cx.notify();
    }

    /// Header-grip drags: the grip carries its column index; the mouse's
    /// window x minus the header cell's left edge is the new fixed width.
    fn on_resize_drag(
        &mut self,
        ev: &DragMoveEvent<ResizeDrag>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (owner, column) = {
            let drag = ev.drag(cx);
            (drag.owner, drag.column)
        };
        if owner != cx.entity_id() {
            return;
        }
        let Some(bounds) = self.header_bounds.get(column) else {
            return;
        };
        let min = self.columns.get(column).map(|c| c.min_width).unwrap_or(0.0);
        let width = f32::from(ev.event.position.x - bounds.left()).max(min);
        self.resized.insert(column, width);
        cx.notify();
    }

    fn row_mouse_down(
        &mut self,
        display: usize,
        toggle: bool,
        range: bool,
        click_count: usize,
        cx: &mut Context<Self>,
    ) {
        if click_count == 2 {
            if let Some(&source) = self.display_order.get(display) {
                cx.emit(TableViewEvent::Activated(source));
            }
            return;
        }
        if matches!(self.mode, SelectionMode::None) {
            return;
        }
        let before = self.selection.selected();
        self.selection
            .click(self.mode, &self.display_order, display, toggle, range);
        let after = self.selection.selected();
        if before != after {
            cx.emit(TableViewEvent::SelectionChanged(after));
        }
        cx.notify();
    }

    /// Arrow keys: only consume the key when the cursor actually moves —
    /// `SelectionMode::None` (the default) and empty tables are no-ops, and
    /// the host should keep receiving those arrows.
    fn step(&mut self, delta: isize, extend: bool, cx: &mut Context<Self>) {
        let before = self.selection.selected();
        let Some(display) = self
            .selection
            .step(self.mode, &self.display_order, delta, extend)
        else {
            return;
        };
        if self.height.is_some() {
            self.scroll.scroll_to_item(display, ScrollStrategy::Center);
        }
        let after = self.selection.selected();
        if before != after {
            cx.emit(TableViewEvent::SelectionChanged(after));
        }
        cx.notify();
        cx.stop_propagation();
    }

    fn on_key(&mut self, ev: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let shift = ev.keystroke.modifiers.shift;
        match ev.keystroke.key.as_str() {
            "up" => self.step(-1, shift, cx),
            "down" => self.step(1, shift, cx),
            "enter" => {
                let target = self.selection.cursor().or_else(|| {
                    let selected = self.selection.selected();
                    (selected.len() == 1).then(|| selected[0])
                });
                if let Some(source) = target {
                    cx.emit(TableViewEvent::Activated(source));
                    cx.stop_propagation();
                }
            }
            "escape" => self.clear_selection(cx),
            _ => {}
        }
    }

    /// Escape: only consume the key when it actually clears something, so
    /// hosts (dialogs, ...) still see it otherwise.
    fn clear_selection(&mut self, cx: &mut Context<Self>) {
        if self.selection.clear() {
            cx.emit(TableViewEvent::SelectionChanged(Vec::new()));
            cx.notify();
            cx.stop_propagation();
        }
    }

    // --- Rendering -----------------------------------------------------------

    fn render_header(&self, cx: &mut Context<Self>) -> Div {
        let t = theme(cx);
        let font = t.font_size(Size::Sm);
        let dimmed = t.dimmed().hsla();
        let text = t.text().hsla();
        let accent = t.primary().hsla();
        let grip_hover = t.primary().alpha(0.6);
        let line = t.border().hsla();

        let owner = cx.entity_id();
        let view = cx.weak_entity();
        let mut row = div()
            .flex()
            .w_full()
            .border_b_1()
            .border_color(line)
            // The header cells' painted bounds, for resize math: children map
            // 1:1 to columns (grips are nested inside the cells).
            .on_children_prepainted(move |bounds, _window, app| {
                view.update(app, |this, _| this.header_bounds = bounds).ok();
            });

        for ix in 0..self.columns.len() {
            let col = &self.columns[ix];
            let sortable = col.sort.is_some();
            let sort_dir = self.sort.filter(|&(c, _)| c == ix).map(|(_, d)| d);

            let grip = div()
                .id(("guise-tableview-grip", ix))
                .absolute()
                .top(px(0.0))
                .bottom(px(0.0))
                .right(px(-3.0))
                .w(px(6.0))
                .cursor_col_resize()
                .hover(move |s| s.bg(grip_hover))
                .on_drag(ResizeDrag { owner, column: ix }, |_, _, _, cx| {
                    cx.new(|_| Empty)
                })
                // Don't let a stray click on the grip toggle the sort.
                .on_click(|_ev, _window, cx| cx.stop_propagation());

            let mut cell = div()
                .relative()
                .flex()
                .items_center()
                .gap(px(6.0))
                .px(px(12.0))
                .py(px(8.0))
                .text_size(px(font))
                .text_color(dimmed)
                .font_weight(FontWeight::SEMIBOLD);
            cell = sized(cell, self.col_width(ix));
            cell = aligned(cell, col.align);
            cell = cell.child(div().min_w(px(0.0)).truncate().child(col.title.clone()));
            if let Some(dir) = sort_dir {
                cell = cell.child(div().text_size(px(font * 0.65)).text_color(accent).child(
                    SharedString::new_static(match dir {
                        SortDir::Asc => "\u{25b2}",
                        SortDir::Desc => "\u{25bc}",
                    }),
                ));
            }
            cell = cell.child(grip);

            let cell: AnyElement = if sortable {
                cell.id(("guise-tableview-head", ix))
                    .cursor_pointer()
                    .hover(move |s| s.text_color(text))
                    .on_click(cx.listener(move |this, _ev, _window, cx| {
                        this.toggle_sort(ix, cx);
                    }))
                    .into_any_element()
            } else {
                cell.into_any_element()
            };
            row = row.child(cell);
        }
        row
    }

    /// Rows for the display range. For signal-bound rows the backing entity is
    /// leased with `Entity::update`, which yields `&Vec<T>` *and* a usable
    /// `&mut App` at once — cell closures need both.
    fn render_rows(
        &self,
        range: Range<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let view = cx.weak_entity();
        match self.rows.clone() {
            Rows::Owned(rows) => range
                .filter_map(|display| {
                    let source = *self.display_order.get(display)?;
                    let row = rows.get(source)?;
                    Some(self.render_row(&view, display, source, row, window, cx))
                })
                .collect(),
            Rows::Bound(signal) => signal.entity().update(cx, |rows, cx| {
                range
                    .filter_map(|display| {
                        let source = *self.display_order.get(display)?;
                        let row = rows.get(source)?;
                        Some(self.render_row(&view, display, source, row, window, cx))
                    })
                    .collect()
            }),
        }
    }

    fn render_row(
        &self,
        view: &WeakEntity<Self>,
        display: usize,
        source: usize,
        row: &T,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let t = theme(cx);
        let font = t.font_size(Size::Sm);
        let text = t.text().hsla();
        let line = t.border().hsla();
        let stripe = t.surface_hover().hsla();
        let hover = t
            .color(ColorName::Gray, if t.scheme.is_dark() { 6 } else { 1 })
            .hsla();
        let selected_bg = t.primary().alpha(0.12);

        let is_selected = self.selection.is_selected(source);

        let mut tr = div()
            .id(("guise-tableview-row", display))
            .flex()
            .w_full()
            .border_b_1()
            .border_color(line)
            .text_size(px(font))
            .text_color(text);

        if is_selected {
            tr = tr.bg(selected_bg);
        } else if self.striped && display % 2 == 1 {
            tr = tr.bg(stripe);
        }
        if self.highlight_on_hover && !is_selected {
            tr = tr.hover(move |s| s.bg(hover));
        }

        for (ix, col) in self.columns.iter().enumerate() {
            let mut cell = div()
                .flex()
                .items_center()
                .px(px(12.0))
                .py(px(8.0))
                .overflow_hidden();
            cell = sized(cell, self.col_width(ix));
            cell = aligned(cell, col.align);
            cell = match &col.content {
                Some(CellContent::Text(to_text)) => {
                    cell.child(div().min_w(px(0.0)).truncate().child(to_text(row)))
                }
                Some(CellContent::Element(build)) => cell.child(build(row, window, cx)),
                None => cell,
            };
            tr = tr.child(cell);
        }

        let view = view.clone();
        tr = tr.on_mouse_down(
            MouseButton::Left,
            move |ev: &MouseDownEvent, window, app| {
                let toggle = ev.modifiers.platform;
                let range = ev.modifiers.shift;
                let count = ev.click_count;
                view.update(app, |this, cx| {
                    window.focus(&this.focus, cx);
                    this.row_mouse_down(display, toggle, range, count, cx);
                })
                .ok();
            },
        );

        tr.into_any_element()
    }
}

impl<T: 'static> Render for TableView<T> {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.display_order = self.compute_order(cx);
        let count = self.display_order.len();

        let t = theme(cx);
        let line = t.border().hsla();
        let dimmed = t.dimmed().hsla();
        let font = t.font_size(Size::Sm);
        let radius = t.radius(t.default_radius);

        let header = self.render_header(cx);

        let body: AnyElement = if count == 0 {
            match &self.empty {
                Some(builder) => builder(window, cx),
                None => div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .py(px(24.0))
                    .text_size(px(font))
                    .text_color(dimmed)
                    .child(SharedString::new_static("No data"))
                    .into_any_element(),
            }
        } else if let Some(height) = self.height {
            uniform_list(
                "guise-tableview-body",
                count,
                cx.processor(|this, range: Range<usize>, window, cx| {
                    this.render_rows(range, window, cx)
                }),
            )
            .h(px(height))
            .w_full()
            .track_scroll(&self.scroll)
            .into_any_element()
        } else {
            div()
                .flex()
                .flex_col()
                .w_full()
                .children(self.render_rows(0..count, window, cx))
                .into_any_element()
        };

        let mut table = div()
            .id("guise-tableview")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key))
            .on_drag_move(cx.listener(Self::on_resize_drag))
            .flex()
            .flex_col()
            .w_full()
            .child(header)
            .child(body);
        if self.with_border {
            table = table
                .border_1()
                .border_color(line)
                .rounded(px(radius))
                .overflow_hidden();
        }
        table
    }
}

/// The display order given optional sorting: pure index math from `state`.
fn order_of<T>(rows: &[T], sort: Option<(SortDir, Comparator<T>)>) -> Vec<usize> {
    match sort {
        Some((dir, cmp)) => sorted_order(rows, dir, &*cmp),
        None => identity_order(rows.len()),
    }
}

/// Apply a column's width policy. Fixed columns never flex; flexing columns
/// share leftover space by grow factor from a zero basis.
fn sized(cell: Div, width: ColWidth) -> Div {
    match width {
        ColWidth::Fixed(w) => cell.w(px(w)).flex_none(),
        ColWidth::Flex(factor, min) => cell
            .grow(factor)
            .shrink(1.0)
            .flex_basis(px(0.0))
            .min_w(px(min)),
    }
}

/// Horizontal alignment of a cell's content.
fn aligned(cell: Div, align: Align) -> Div {
    match align {
        Align::Start | Align::Stretch => cell.justify_start(),
        Align::Center => cell.justify_center(),
        Align::End => cell.justify_end(),
    }
}
