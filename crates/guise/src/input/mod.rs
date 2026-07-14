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

mod accept;
mod calendar;
mod checkbox;
mod checkboxgroup;
mod colorinput;
mod combobox;
mod date;
mod datepicker;
mod dropzone;
mod edit;
mod fileinput;
mod field;
mod keys;
mod number;
mod password;
mod pin;
mod radio;
mod radiogroup;
mod rangeslider;
mod rating;
mod segmented;
mod select;
mod slider;
mod switch;
mod tags;
mod text;
mod textarea;
mod time;
mod timepicker;

pub use calendar::Calendar;
pub use checkbox::Checkbox;
pub use checkboxgroup::CheckboxGroup;
pub use colorinput::{ColorInput, ColorInputEvent};
pub use combobox::{Combobox, ComboboxEvent};
pub use date::{days_in_month, is_leap_year, month_grid, Date, Weekday, MONTH_NAMES};
pub use datepicker::{DatePicker, DatePickerEvent};
pub use dropzone::Dropzone;
pub use edit::TextEdit;
pub use fileinput::{FileInput, FileInputEvent};
pub use field::Field;
pub use keys::{apply_key, KeyOutcome};
pub use number::{NumberInput, NumberInputEvent};
pub use password::{PasswordInput, PasswordInputEvent};
pub use pin::{PinInput, PinInputEvent};
pub use radio::Radio;
pub use radiogroup::RadioGroup;
pub use rangeslider::{RangeSlider, RangeSliderEvent};
pub use rating::Rating;
pub use segmented::{SegmentedControl, SegmentedControlEvent};
pub use select::{Select, SelectEvent};
pub use slider::{Slider, SliderEvent};
pub use switch::Switch;
pub use tags::{TagsInput, TagsInputEvent};
pub use text::{TextInput, TextInputEvent};
pub use textarea::{TextArea, TextAreaEvent};
pub use time::Time;
pub use timepicker::{TimePicker, TimePickerEvent};

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
