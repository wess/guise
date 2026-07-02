//! Layout primitives: vertical [`Stack`], horizontal [`Group`], [`Center`],
//! plus app-structure helpers ([`AppShell`], [`Container`], [`Space`]).
//! These map Mantine's flex helpers onto gpui's flex container.

mod appshell;
mod center;
mod container;
mod grid;
mod group;
mod space;
mod stack;

pub use appshell::AppShell;
pub use center::Center;
pub use container::Container;
pub use grid::SimpleGrid;
pub use group::Group;
pub use space::Space;
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
