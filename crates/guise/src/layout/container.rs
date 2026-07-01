//! `Container` — a max-width centered column, matching Mantine's container
//! size scale.
//!
//! Not to be confused with [`crate::flex::Container`], the Flutter-style
//! pixel box (which is why `flex` is not glob-exported).
//!
//! ```ignore
//! use guise::prelude::*;
//!
//! Container::new()
//!     .size(Size::Sm)
//!     .padding(Size::Md)
//!     .child(Title::new("Article").order(2))
//!     .child(Text::new("Readable line lengths on any window width."))
//! ```

use gpui::prelude::*;
use gpui::{div, px, AnyElement, App, IntoElement, Window};

use crate::theme::{theme, Size};

/// Max content width (px) for each [`Size`] — Mantine's container scale.
fn max_width(size: Size) -> f32 {
    match size {
        Size::Xs => 540.0,
        Size::Sm => 720.0,
        Size::Md => 960.0,
        Size::Lg => 1140.0,
        Size::Xl => 1320.0,
    }
}

/// A centered column with a capped width. The Mantine `Container`.
#[derive(IntoElement)]
pub struct Container {
    size: Size,
    padding: Size,
    children: Vec<AnyElement>,
}

impl Container {
    pub fn new() -> Self {
        Container {
            size: Size::Md,
            padding: Size::Md,
            children: Vec::new(),
        }
    }

    /// Max content width: `Xs..Xl` map to 540 / 720 / 960 / 1140 / 1320 px.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Horizontal padding inside the capped column (theme spacing scale).
    pub fn padding(mut self, padding: Size) -> Self {
        self.padding = padding;
        self
    }
}

impl Default for Container {
    fn default() -> Self {
        Container::new()
    }
}

impl ParentElement for Container {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Container {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let pad = theme(cx).spacing(self.padding);
        div().w_full().flex().flex_col().items_center().child(
            div()
                .w_full()
                .max_w(px(max_width(self.size)))
                .px(px(pad))
                .flex()
                .flex_col()
                .children(self.children),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widths_match_mantine_scale() {
        assert_eq!(max_width(Size::Xs), 540.0);
        assert_eq!(max_width(Size::Sm), 720.0);
        assert_eq!(max_width(Size::Md), 960.0);
        assert_eq!(max_width(Size::Lg), 1140.0);
        assert_eq!(max_width(Size::Xl), 1320.0);
    }
}
