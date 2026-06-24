//! Layout primitives: vertical [`Stack`], horizontal [`Group`], and [`Center`].
//! These map Mantine's flex helpers onto gpui's flex container.

mod center;
mod group;
mod stack;

pub use center::Center;
pub use group::Group;
pub use stack::Stack;

use gpui::prelude::*;
use gpui::Div;

/// Cross-axis alignment of flex children.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Start,
    Center,
    End,
    Stretch,
}

/// Main-axis distribution of flex children.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Justify {
    Start,
    Center,
    End,
    Between,
    Around,
}

pub(crate) fn apply_align(div: Div, align: Align) -> Div {
    match align {
        Align::Start => div.items_start(),
        Align::Center => div.items_center(),
        Align::End => div.items_end(),
        Align::Stretch => div.items_stretch(),
    }
}

pub(crate) fn apply_justify(div: Div, justify: Justify) -> Div {
    match justify {
        Justify::Start => div.justify_start(),
        Justify::Center => div.justify_center(),
        Justify::End => div.justify_end(),
        Justify::Between => div.justify_between(),
        Justify::Around => div.justify_around(),
    }
}
