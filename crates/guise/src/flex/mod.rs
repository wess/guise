//! Flexbox layout primitives, for people who think in `Row`/`Column`/
//! `Container`/`Expanded` (à la Flutter).
//!
//! These map a Flutter-like box model onto gpui's flexbox. Several names
//! (`Row`, `Column`, `Stack`, `Center`, ...) overlap with guise's own `layout`
//! module (`Stack`/`Center`), so this module is **not** glob-exported at the
//! crate root. Import it explicitly:
//!
//! ```ignore
//! use guise::flex::*;
//!
//! Column::new()
//!     .cross_axis_alignment(CrossAxisAlignment::Stretch)
//!     .child(Row::new().child(Expanded::new(header)).child(actions))
//!     .child(SizedBox::height(12.0))
//!     .child(Expanded::new(body))
//! ```

mod container;
mod flexible;
mod rowcolumn;
mod stack;
mod wrap;

pub use container::{Align, Center, Container, Padding};
pub use flexible::{Expanded, Flexible, SizedBox, Spacer};
pub use rowcolumn::{Column, Row};
pub use stack::{Positioned, Stack};
pub use wrap::Wrap;

use gpui::prelude::*;
use gpui::{px, Div};


/// Distribution of children along the main axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainAxisAlignment {
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

/// Alignment of children across the cross axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossAxisAlignment {
    Start,
    End,
    Center,
    Stretch,
    Baseline,
}

/// Whether a Row/Column shrinks to its children or fills the main axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainAxisSize {
    Min,
    Max,
}

/// A 2-D alignment within a box (`Container`/`Align`/`Center`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

/// Padding/margin offsets, like Flutter's `EdgeInsets`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EdgeInsets {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl EdgeInsets {
    pub const fn all(value: f32) -> Self {
        EdgeInsets {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    pub const fn symmetric(horizontal: f32, vertical: f32) -> Self {
        EdgeInsets {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }

    pub const fn only(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        EdgeInsets {
            top,
            right,
            bottom,
            left,
        }
    }

    pub const fn horizontal(value: f32) -> Self {
        EdgeInsets::symmetric(value, 0.0)
    }

    pub const fn vertical(value: f32) -> Self {
        EdgeInsets::symmetric(0.0, value)
    }
}

pub(crate) fn apply_main(div: Div, main: MainAxisAlignment) -> Div {
    match main {
        MainAxisAlignment::Start => div.justify_start(),
        MainAxisAlignment::End => div.justify_end(),
        MainAxisAlignment::Center => div.justify_center(),
        MainAxisAlignment::SpaceBetween => div.justify_between(),
        MainAxisAlignment::SpaceAround => div.justify_around(),
        MainAxisAlignment::SpaceEvenly => div.justify_evenly(),
    }
}

pub(crate) fn apply_cross(div: Div, cross: CrossAxisAlignment) -> Div {
    match cross {
        CrossAxisAlignment::Start => div.items_start(),
        CrossAxisAlignment::End => div.items_end(),
        CrossAxisAlignment::Center => div.items_center(),
        CrossAxisAlignment::Stretch => div.items_stretch(),
        CrossAxisAlignment::Baseline => div.items_baseline(),
    }
}

pub(crate) fn apply_alignment(div: Div, alignment: Alignment) -> Div {
    use Alignment::*;
    let div = match alignment {
        TopLeft | CenterLeft | BottomLeft => div.justify_start(),
        TopCenter | Center | BottomCenter => div.justify_center(),
        TopRight | CenterRight | BottomRight => div.justify_end(),
    };
    match alignment {
        TopLeft | TopCenter | TopRight => div.items_start(),
        CenterLeft | Center | CenterRight => div.items_center(),
        BottomLeft | BottomCenter | BottomRight => div.items_end(),
    }
}

pub(crate) fn apply_padding(div: Div, e: EdgeInsets) -> Div {
    div.pt(px(e.top))
        .pr(px(e.right))
        .pb(px(e.bottom))
        .pl(px(e.left))
}

pub(crate) fn apply_margin(div: Div, e: EdgeInsets) -> Div {
    div.mt(px(e.top))
        .mr(px(e.right))
        .mb(px(e.bottom))
        .ml(px(e.left))
}

#[cfg(test)]
mod tests {
    use super::EdgeInsets;

    #[test]
    fn edge_insets_constructors() {
        assert_eq!(EdgeInsets::all(8.0), EdgeInsets::only(8.0, 8.0, 8.0, 8.0));
        assert_eq!(
            EdgeInsets::symmetric(10.0, 4.0),
            EdgeInsets::only(4.0, 10.0, 4.0, 10.0)
        );
        assert_eq!(
            EdgeInsets::horizontal(6.0),
            EdgeInsets::only(0.0, 6.0, 0.0, 6.0)
        );
        assert_eq!(
            EdgeInsets::vertical(6.0),
            EdgeInsets::only(6.0, 0.0, 6.0, 0.0)
        );
    }
}
