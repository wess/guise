//! `Transfer` — a dual-list membership editor (gpui entity).
//!
//! One item pool, two panes: items are either left (available) or right
//! (chosen). Click rows to check them, move checked items with the middle
//! buttons. Emits [`TransferEvent`] with the right side's indices after
//! every move.

use std::collections::BTreeSet;

use gpui::prelude::*;
use gpui::{div, px, Context, EventEmitter, FocusHandle, IntoElement, SharedString, Window};

use crate::icon::{Icon, IconName};
use crate::theme::{theme, Size};

/// Emitted after a move. Carries the right pane's item indices, ascending.
#[derive(Debug, Clone)]
pub struct TransferEvent(pub Vec<usize>);

/// Move `checked ∩ side` to the other side; returns whether anything moved.
fn move_checked(right: &mut BTreeSet<usize>, checked: &mut BTreeSet<usize>, to_right: bool) -> bool {
    let movers: Vec<usize> = checked
        .iter()
        .copied()
        .filter(|i| right.contains(i) != to_right)
        .collect();
    for i in &movers {
        if to_right {
            right.insert(*i);
        } else {
            right.remove(i);
        }
        checked.remove(i);
    }
    !movers.is_empty()
}

/// A dual-list picker. Create with `cx.new(|cx| Transfer::new(cx).data([..]))`.
pub struct Transfer {
    items: Vec<SharedString>,
    right: BTreeSet<usize>,
    checked: BTreeSet<usize>,
    titles: (SharedString, SharedString),
    height: f32,
    focus: FocusHandle,
    disabled: bool,
}

impl EventEmitter<TransferEvent> for Transfer {}

impl Transfer {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Transfer {
            items: Vec::new(),
            right: BTreeSet::new(),
            checked: BTreeSet::new(),
            titles: (
                SharedString::new_static("Available"),
                SharedString::new_static("Chosen"),
            ),
            height: 200.0,
            focus: cx.focus_handle(),
            disabled: false,
        }
    }

    pub fn data<I, S>(mut self, items: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.items = items.into_iter().map(Into::into).collect();
        self
    }

    /// Start with these item indices on the right side.
    pub fn chosen(mut self, indices: impl IntoIterator<Item = usize>) -> Self {
        self.right = indices.into_iter().collect();
        self
    }

    pub fn titles(
        mut self,
        left: impl Into<SharedString>,
        right: impl Into<SharedString>,
    ) -> Self {
        self.titles = (left.into(), right.into());
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height.max(80.0);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// The right pane's item indices, ascending.
    pub fn chosen_indices(&self) -> Vec<usize> {
        self.right.iter().copied().collect()
    }

    fn toggle_checked(&mut self, index: usize, cx: &mut Context<Self>) {
        if !self.checked.remove(&index) {
            self.checked.insert(index);
        }
        cx.notify();
    }

    fn transfer(&mut self, to_right: bool, cx: &mut Context<Self>) {
        if move_checked(&mut self.right, &mut self.checked, to_right) {
            cx.emit(TransferEvent(self.chosen_indices()));
            cx.notify();
        }
    }

    fn pane(
        &self,
        title: &SharedString,
        indices: Vec<usize>,
        list_id: &'static str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let t = theme(cx);
        let radius = t.radius(t.default_radius);
        let surface = t.surface().hsla();
        let surface_hover = t.surface_hover().hsla();
        let border = t.border().hsla();
        let text_color = t.text().hsla();
        let dimmed = t.dimmed().hsla();
        let checked_bg = t.primary().alpha(0.12);
        let font = t.font_size(Size::Sm);

        let mut rows = div()
            .id(list_id)
            .flex()
            .flex_col()
            .gap(px(2.0))
            .p(px(4.0))
            .flex_1()
            .overflow_y_scroll();
        for i in indices {
            let is_checked = self.checked.contains(&i);
            let mut row = div()
                .id((list_id, i))
                .px(px(8.0))
                .py(px(5.0))
                .rounded(px(4.0))
                .text_size(px(font))
                .text_color(text_color)
                .child(self.items[i].clone())
                .on_click(cx.listener(move |this, _ev, _window, cx| {
                    if !this.disabled {
                        this.toggle_checked(i, cx);
                    }
                }));
            if is_checked {
                row = row.bg(checked_bg);
            } else {
                row = row.hover(move |s| s.bg(surface_hover));
            }
            rows = rows.child(row);
        }

        div()
            .flex()
            .flex_col()
            .flex_1()
            .h(px(self.height))
            .rounded(px(radius))
            .border_1()
            .border_color(border)
            .bg(surface)
            .child(
                div()
                    .px(px(8.0))
                    .py(px(5.0))
                    .border_b_1()
                    .border_color(border)
                    .text_size(px(font - 1.0))
                    .text_color(dimmed)
                    .child(title.clone()),
            )
            .child(rows)
    }
}

impl Render for Transfer {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        let surface = t.surface().hsla();
        let surface_hover = t.surface_hover().hsla();
        let border = t.border().hsla();
        let dimmed = t.dimmed().hsla();

        let left: Vec<usize> = (0..self.items.len())
            .filter(|i| !self.right.contains(i))
            .collect();
        let right: Vec<usize> = self.chosen_indices();

        let (left_title, right_title) = self.titles.clone();
        let left_pane = self.pane(&left_title, left, "guise-transfer-left", cx);
        let right_pane = self.pane(&right_title, right, "guise-transfer-right", cx);

        let mut buttons = div().flex().flex_col().justify_center().gap(px(6.0));
        for (key, icon, to_right) in [
            ("guise-transfer-toright", IconName::ChevronRight, true),
            ("guise-transfer-toleft", IconName::ChevronLeft, false),
        ] {
            buttons = buttons.child(
                div()
                    .id(key)
                    .flex()
                    .items_center()
                    .justify_center()
                    .w(px(26.0))
                    .h(px(26.0))
                    .rounded(px(6.0))
                    .bg(surface)
                    .border_1()
                    .border_color(border)
                    .text_color(dimmed)
                    .hover(move |s| s.bg(surface_hover))
                    .child(Icon::new(icon).size(Size::Xs))
                    .on_click(cx.listener(move |this, _ev, _window, cx| {
                        if !this.disabled {
                            this.transfer(to_right, cx);
                        }
                    })),
            );
        }

        let row = div()
            .track_focus(&self.focus)
            .flex()
            .items_center()
            .gap(px(10.0))
            .w_full()
            .child(left_pane)
            .child(buttons)
            .child(right_pane);

        if self.disabled {
            row.opacity(0.6)
        } else {
            row
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moves_only_checked_items_on_the_source_side() {
        let mut right: BTreeSet<usize> = [3].into_iter().collect();
        let mut checked: BTreeSet<usize> = [0, 1, 3].into_iter().collect();
        // Moving right: 0 and 1 cross; 3 is already right so it stays checked-cleared? No —
        // it's skipped (not a mover) and stays checked.
        assert!(move_checked(&mut right, &mut checked, true));
        assert_eq!(right.iter().copied().collect::<Vec<_>>(), vec![0, 1, 3]);
        assert!(checked.contains(&3) && !checked.contains(&0));

        // Move 3 back left.
        assert!(move_checked(&mut right, &mut checked, false));
        assert_eq!(right.iter().copied().collect::<Vec<_>>(), vec![0, 1]);
        assert!(checked.is_empty());
    }

    #[test]
    fn no_movers_reports_false() {
        let mut right = BTreeSet::new();
        let mut checked = BTreeSet::new();
        assert!(!move_checked(&mut right, &mut checked, true));
        checked.insert(2);
        // 2 is on the left; moving left is a no-op.
        assert!(!move_checked(&mut right, &mut checked, false));
        assert!(checked.contains(&2));
    }
}
