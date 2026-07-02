//! `Space` — a fixed spacing block on one axis, sized by the theme's
//! spacing scale.
//!
//! ```ignore
//! use guise::prelude::*;
//!
//! Stack::new()
//!     .child(Title::new("Heading").order(3))
//!     .child(Space::y(Size::Md))
//!     .child(Text::new("Body copy."))
//! ```

use gpui::prelude::*;
use gpui::{div, px, App, IntoElement, Window};

use crate::theme::{theme, Size};

/// The axis a [`Space`] occupies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpaceAxis {
    Horizontal,
    Vertical,
}

/// A fixed gap between siblings. The Mantine `Space`.
#[derive(IntoElement)]
pub struct Space {
    axis: SpaceAxis,
    size: Size,
}

impl Space {
    /// Horizontal space: a block `size` wide (for rows).
    pub fn x(size: Size) -> Self {
        Space {
            axis: SpaceAxis::Horizontal,
            size,
        }
    }

    /// Vertical space: a block `size` tall (for columns).
    pub fn y(size: Size) -> Self {
        Space {
            axis: SpaceAxis::Vertical,
            size,
        }
    }
}

impl RenderOnce for Space {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let gap = theme(cx).spacing(self.size);
        let el = div().flex_none();
        match self.axis {
            SpaceAxis::Horizontal => el.w(px(gap)),
            SpaceAxis::Vertical => el.h(px(gap)),
        }
    }
}
