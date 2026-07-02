//! `Mark` — an inline highlighted span of text.
//!
//! ```ignore
//! Group::new()
//!     .gap(Size::Xs)
//!     .child(Text::new("Highlight the"))
//!     .child(Mark::new("important part"))
//!     .child(Text::new("of a sentence."))
//! ```

use gpui::prelude::*;
use gpui::{div, px, App, IntoElement, SharedString, Window};

use crate::theme::{theme, ColorName, Size};

/// A highlighter-pen span. The Mantine `Mark`.
///
/// Inherits the surrounding font size unless [`Mark::size`] is set, so it
/// drops into a `Group` next to `Text` runs.
#[derive(IntoElement)]
pub struct Mark {
    content: SharedString,
    color: ColorName,
    size: Option<Size>,
}

impl Mark {
    pub fn new(content: impl Into<SharedString>) -> Self {
        Mark {
            content: content.into(),
            color: ColorName::Yellow,
            size: None,
        }
    }

    /// The highlight tint (default `Yellow`).
    pub fn color(mut self, color: ColorName) -> Self {
        self.color = color;
        self
    }

    /// Explicit font size; unset inherits from the parent.
    pub fn size(mut self, size: Size) -> Self {
        self.size = Some(size);
        self
    }
}

impl RenderOnce for Mark {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        // Mantine's mark: a light shade-2 wash in light mode, a translucent
        // shade-5 tint in dark mode; text stays the theme text color.
        let bg = if t.scheme.is_dark() {
            t.color(self.color, 5).alpha(0.35)
        } else {
            t.color(self.color, 2).hsla()
        };
        let fg = t.text().hsla();

        let mut el = div()
            .px(px(4.0))
            .rounded(px(t.radius(Size::Xs)))
            .bg(bg)
            .text_color(fg)
            .child(self.content);
        if let Some(size) = self.size {
            el = el.text_size(px(t.font_size(size)));
        }
        el
    }
}
