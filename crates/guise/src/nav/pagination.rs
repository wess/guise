//! `Pagination` — a stateful page selector (gpui entity).

use gpui::prelude::*;
use gpui::{div, px, Context, EventEmitter, IntoElement, SharedString, Window};

use crate::theme::{theme, ColorName, Size};

/// Emitted when the active page changes. Carries the 1-based page number.
#[derive(Debug, Clone)]
pub struct PaginationEvent(pub usize);

/// A page selector. Create with `cx.new(|cx| Pagination::new(cx, 10))`.
pub struct Pagination {
    total: usize,
    active: usize,
    color: ColorName,
}

impl EventEmitter<PaginationEvent> for Pagination {}

impl Pagination {
    pub fn new(_cx: &mut Context<Self>, total: usize) -> Self {
        Pagination {
            total: total.max(1),
            active: 1,
            color: ColorName::Blue,
        }
    }

    pub fn active(mut self, page: usize) -> Self {
        self.active = page.clamp(1, self.total);
        self
    }

    pub fn color(mut self, color: ColorName) -> Self {
        self.color = color;
        self
    }

    /// The current 1-based page.
    pub fn active_page(&self) -> usize {
        self.active
    }

    /// Move to `page`, emit, and re-render.
    fn goto(&mut self, page: usize, cx: &mut Context<Self>) {
        self.active = page;
        cx.emit(PaginationEvent(page));
        cx.notify();
    }
}

/// The pages to render, 1-based. `None` marks an ellipsis gap.
fn page_range(total: usize, active: usize) -> Vec<Option<usize>> {
    if total <= 7 {
        return (1..=total).map(Some).collect();
    }
    let siblings = 1usize;
    let left = active.saturating_sub(siblings).max(2);
    let right = (active + siblings).min(total - 1);
    let mut pages = vec![Some(1)];
    if left > 2 {
        pages.push(None);
    }
    for p in left..=right {
        pages.push(Some(p));
    }
    if right < total - 1 {
        pages.push(None);
    }
    pages.push(Some(total));
    pages
}

#[cfg(test)]
mod tests {
    use super::page_range;

    #[test]
    fn small_totals_show_every_page() {
        assert_eq!(
            page_range(5, 3),
            vec![Some(1), Some(2), Some(3), Some(4), Some(5)]
        );
    }

    #[test]
    fn large_totals_window_with_ellipses() {
        // active in the middle: 1 … 4 5 6 … 10
        assert_eq!(
            page_range(10, 5),
            vec![Some(1), None, Some(4), Some(5), Some(6), None, Some(10)]
        );
        // active near the start: no leading ellipsis
        assert_eq!(
            page_range(10, 2),
            vec![Some(1), Some(2), Some(3), None, Some(10)]
        );
    }
}

impl Render for Pagination {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        let radius = t.radius(Size::Sm);
        let font = t.font_size(Size::Sm);
        let accent = t.color(self.color, t.primary_shade());
        let accent_hsla = accent.hsla();
        let on_accent = accent.contrasting().hsla();
        let text = t.text().hsla();
        let dimmed = t.dimmed().hsla();
        let border = t.border().hsla();
        let surface_hover = t.surface_hover().hsla();

        let active = self.active;
        let total = self.total;

        // A single square control cell.
        let arrow = |id: &'static str,
                     glyph: &'static str,
                     target: Option<usize>,
                     cx: &mut Context<Self>| {
            let mut el = div()
                .id(id)
                .w(px(32.0))
                .h(px(32.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(radius))
                .border_1()
                .border_color(border)
                .text_size(px(font))
                .text_color(if target.is_some() { text } else { dimmed })
                .child(SharedString::new_static(glyph));
            if let Some(page) = target {
                el = el
                    .hover(move |s| s.bg(surface_hover))
                    .on_click(cx.listener(move |this, _ev, _window, cx| this.goto(page, cx)));
            } else {
                el = el.opacity(0.4);
            }
            el
        };

        let prev = arrow(
            "guise-page-prev",
            "\u{2039}",
            (active > 1).then(|| active - 1),
            cx,
        );
        let next = arrow(
            "guise-page-next",
            "\u{203a}",
            (active < total).then(|| active + 1),
            cx,
        );

        let mut row = div().flex().items_center().gap(px(6.0)).child(prev);

        for entry in page_range(total, active) {
            match entry {
                Some(page) => {
                    let is_active = page == active;
                    let mut cell = div()
                        .id(("guise-page", page))
                        .w(px(32.0))
                        .h(px(32.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(radius))
                        .text_size(px(font))
                        .child(SharedString::from(page.to_string()));
                    if is_active {
                        cell = cell.bg(accent_hsla).text_color(on_accent);
                    } else {
                        cell = cell
                            .border_1()
                            .border_color(border)
                            .text_color(text)
                            .hover(move |s| s.bg(surface_hover))
                            .on_click(
                                cx.listener(move |this, _ev, _window, cx| this.goto(page, cx)),
                            );
                    }
                    row = row.child(cell);
                }
                None => {
                    row = row.child(
                        div()
                            .w(px(32.0))
                            .h(px(32.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(dimmed)
                            .child(SharedString::new_static("\u{2026}")),
                    );
                }
            }
        }

        row.child(next)
    }
}
