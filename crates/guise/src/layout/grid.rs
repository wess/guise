//! `SimpleGrid` — equal-width columns that wrap into rows.
//!
//! gpui's flexbox has no CSS-grid track system, so this lays children out as a
//! column of flex rows, each holding up to `cols` equal-weight cells. The final
//! row is padded with empty cells so columns stay aligned.

use gpui::prelude::*;
use gpui::{div, px, relative, AnyElement, App, IntoElement, Window};

use crate::theme::{theme, Size};

/// A responsive-feeling fixed-column grid. `SimpleGrid::new(3).spacing(Size::Md)`.
#[derive(IntoElement)]
pub struct SimpleGrid {
    children: Vec<AnyElement>,
    cols: usize,
    spacing: Size,
}

impl SimpleGrid {
    pub fn new(cols: usize) -> Self {
        SimpleGrid {
            children: Vec::new(),
            cols: cols.max(1),
            spacing: Size::Md,
        }
    }

    pub fn spacing(mut self, spacing: Size) -> Self {
        self.spacing = spacing;
        self
    }
}

impl ParentElement for SimpleGrid {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

/// One equal-weight grid cell.
fn cell() -> gpui::Div {
    div().flex_grow(1.0).flex_shrink(1.0).flex_basis(relative(0.0))
}

impl RenderOnce for SimpleGrid {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let gap = theme(cx).spacing(self.spacing);
        let cols = self.cols;

        let mut column = div().flex().flex_col().gap(px(gap));
        let mut iter = self.children.into_iter();
        loop {
            let mut row_cells: Vec<AnyElement> = Vec::with_capacity(cols);
            for _ in 0..cols {
                match iter.next() {
                    Some(child) => row_cells.push(cell().child(child).into_any_element()),
                    None => break,
                }
            }
            if row_cells.is_empty() {
                break;
            }
            // Pad the final short row so columns line up.
            while row_cells.len() < cols {
                row_cells.push(cell().into_any_element());
            }
            column = column.child(
                div()
                    .flex()
                    .flex_row()
                    .gap(px(gap))
                    .children(row_cells),
            );
        }
        column
    }
}
