//! `AvatarGroup` — a row of overlapping avatars with an optional overflow chip.

use gpui::prelude::*;
use gpui::{div, px, App, FontWeight, IntoElement, SharedString, Window};

use crate::theme::{theme, ColorName, Size};

const PALETTE: [ColorName; 6] = [
    ColorName::Blue,
    ColorName::Teal,
    ColorName::Grape,
    ColorName::Orange,
    ColorName::Pink,
    ColorName::Lime,
];

/// A stack of overlapping avatars. The Mantine `Avatar.Group`.
#[derive(IntoElement)]
pub struct AvatarGroup {
    items: Vec<SharedString>,
    size: Size,
    limit: Option<usize>,
}

impl AvatarGroup {
    pub fn new() -> Self {
        AvatarGroup {
            items: Vec::new(),
            size: Size::Md,
            limit: None,
        }
    }

    pub fn avatar(mut self, initials: impl Into<SharedString>) -> Self {
        self.items.push(initials.into());
        self
    }

    pub fn avatars<I, S>(mut self, items: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        self.items.extend(items.into_iter().map(Into::into));
        self
    }

    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Show at most `limit` avatars; the rest collapse into a `+N` chip.
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

impl Default for AvatarGroup {
    fn default() -> Self {
        AvatarGroup::new()
    }
}

impl RenderOnce for AvatarGroup {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let dim = super::avatar::avatar_size(self.size);
        let ring = t.body().hsla();
        let overflow_bg = t
            .color(ColorName::Gray, if t.scheme.is_dark() { 6 } else { 3 })
            .hsla();
        let overflow_fg = t.text().hsla();
        let dark = t.scheme.is_dark();

        let total = self.items.len();
        let shown = self.limit.unwrap_or(total).min(total);
        let overflow = total - shown;

        // A circle with a ring border so overlaps read cleanly.
        let bubble = |bg, fg, content: SharedString, first: bool| {
            let mut b = div()
                .w(px(dim))
                .h(px(dim))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(dim))
                .border_2()
                .border_color(ring)
                .bg(bg)
                .text_color(fg)
                .text_size(px(dim * 0.38))
                .font_weight(FontWeight::SEMIBOLD)
                .child(content);
            if !first {
                b = b.ml(px(-(dim * 0.3)));
            }
            b
        };

        let mut row = div().flex().items_center();
        for (i, initials) in self.items.into_iter().take(shown).enumerate() {
            let name = PALETTE[i % PALETTE.len()];
            let bg = t.color(name, if dark { 8 } else { 1 }).hsla();
            let fg = t.color(name, if dark { 2 } else { 8 }).hsla();
            row = row.child(bubble(bg, fg, initials, i == 0));
        }
        if overflow > 0 {
            row = row.child(bubble(
                overflow_bg,
                overflow_fg,
                SharedString::from(format!("+{overflow}")),
                shown == 0,
            ));
        }
        row
    }
}
