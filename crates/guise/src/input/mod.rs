//! Form inputs.
//!
//! Two flavors, matching how each control naturally behaves in gpui:
//!
//! - **Controlled** ([`Checkbox`], [`Switch`], [`Radio`]) are `RenderOnce`
//!   builders. The parent view owns the value and passes a change handler —
//!   wire it with `cx.listener(...)`.
//! - **Stateful** ([`TextInput`], [`Select`]) are gpui entities (`Render` +
//!   `EventEmitter`) that own their buffer/open-state. Create with
//!   `cx.new(|cx| TextInput::new(cx))` and subscribe for changes.

mod checkbox;
mod edit;
mod radio;
mod segmented;
mod select;
mod switch;
mod text;

pub use checkbox::Checkbox;
pub use edit::TextEdit;
pub use radio::Radio;
pub use segmented::{SegmentedControl, SegmentedControlEvent};
pub use select::{Select, SelectEvent};
pub use switch::Switch;
pub use text::{TextInput, TextInputEvent};

use gpui::{App, ClickEvent, Window};

use crate::theme::Size;

/// Boxed click handler shared by the controlled inputs.
pub(crate) type ClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// Box dimension (px) for square toggle controls (Checkbox, Radio).
pub(crate) fn control_box_size(size: Size) -> f32 {
    match size {
        Size::Xs => 16.0,
        Size::Sm => 18.0,
        Size::Md => 20.0,
        Size::Lg => 24.0,
        Size::Xl => 28.0,
    }
}

/// (height, horizontal padding, font size) for text-like controls.
pub(crate) fn control_metrics(size: Size) -> (f32, f32, f32) {
    match size {
        Size::Xs => (30.0, 10.0, 12.0),
        Size::Sm => (36.0, 12.0, 14.0),
        Size::Md => (42.0, 14.0, 16.0),
        Size::Lg => (50.0, 16.0, 18.0),
        Size::Xl => (60.0, 20.0, 20.0),
    }
}
