//! `Table` — a simple data table of string cells.

use gpui::prelude::*;
use gpui::{div, px, App, Div, FontWeight, Hsla, IntoElement, SharedString, Window};

use crate::theme::{theme, ColorName, Size};

/// One table cell. A free fn (not a closure) so it can be reused by both the
/// header and body iterators without being moved twice.
fn cell(value: SharedString, color: Hsla, weight: FontWeight, font: f32) -> Div {
    div()
        .flex_1()
        .px(px(12.0))
        .py(px(8.0))
        .text_size(px(font))
        .text_color(color)
        .font_weight(weight)
        .child(value)
}

/// A data table. The Mantine `Table`. Cells are plain strings; columns size
/// equally.
#[derive(IntoElement)]
pub struct Table {
    head: Vec<SharedString>,
    rows: Vec<Vec<SharedString>>,
    striped: bool,
    highlight_on_hover: bool,
    with_border: bool,
}

impl Table {
    pub fn new() -> Self {
        Table {
            head: Vec::new(),
            rows: Vec::new(),
            striped: false,
            highlight_on_hover: false,
            with_border: false,
        }
    }

    pub fn head<I, S>(mut self, head: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.head = head.into_iter().map(Into::into).collect();
        self
    }

    pub fn row<I, S>(mut self, row: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.rows.push(row.into_iter().map(Into::into).collect());
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
}

impl Default for Table {
    fn default() -> Self {
        Table::new()
    }
}

impl RenderOnce for Table {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let font = t.font_size(Size::Sm);
        let text = t.text().hsla();
        let dimmed = t.dimmed().hsla();
        let line = t.border().hsla();
        let stripe = t.surface_hover().hsla();
        let hover = t.color(ColorName::Gray, if t.scheme.is_dark() { 6 } else { 1 }).hsla();
        let radius = t.radius(Size::Sm);
        let striped = self.striped;
        let highlight = self.highlight_on_hover;

        let header = div()
            .flex()
            .border_b_1()
            .border_color(line)
            .children(
                self.head
                    .into_iter()
                    .map(move |h| cell(h, dimmed, FontWeight::SEMIBOLD, font)),
            );

        let body = self.rows.into_iter().enumerate().map(move |(i, row)| {
            let mut tr = div().flex().border_b_1().border_color(line).children(
                row.into_iter()
                    .map(move |c| cell(c, text, FontWeight::NORMAL, font)),
            );
            if striped && i % 2 == 1 {
                tr = tr.bg(stripe);
            }
            if highlight {
                tr.id(("guise-table-row", i))
                    .hover(move |s| s.bg(hover))
                    .into_any_element()
            } else {
                tr.into_any_element()
            }
        });

        let mut table = div()
            .flex()
            .flex_col()
            .rounded(px(radius))
            .child(header)
            .children(body);
        if self.with_border {
            table = table.border_1().border_color(line);
        }
        table
    }
}
