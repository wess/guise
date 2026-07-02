//! # guise
//!
//! A Mantine-inspired component library for [gpui](https://github.com/zed-industries/zed).
//!
//! `guise` provides a themed palette, sizing tokens, and a growing set of
//! ready-to-use components built on gpui's `RenderOnce` builder pattern.
//!
//! ```ignore
//! use guise::prelude::*;
//!
//! Stack::new().gap(Size::Md).child(
//!     Button::new("save", "Save").variant(Variant::Filled),
//! )
//! ```
//!
//! Install a [`Theme`] once at startup, then read it from any component:
//!
//! ```ignore
//! guise::theme::Theme::dark().init(cx);
//! ```

#[macro_use]
mod macros;

pub mod style;
pub mod theme;

/// Re-exported so the layout macros can bring `.child()` into scope without
/// requiring callers to import gpui's `ParentElement`.
#[doc(hidden)]
pub use gpui::ParentElement as __ParentElement;

/// Re-exported so the `style!` macro can reference gpui (`px`, `FontWeight`, …)
/// via `$crate::gpui::…` without the caller importing gpui.
#[doc(hidden)]
pub use ::gpui;

pub mod chart;
pub mod data;
pub mod editor;
pub mod feedback;
pub mod flex;
pub mod input;
pub mod layout;
pub mod nav;
pub mod overlay;
pub mod reactive;

mod actionicon;
mod anchor;
mod badge;
mod blockquote;
mod button;
mod card;
mod chip;
mod closebutton;
mod code;
mod copybutton;
mod divider;
mod icon;
mod image;
mod indicator;
mod kbd;
mod mark;
mod panel;
mod paper;
mod scrollarea;
mod skeleton;
pub mod panegroup;
mod splitpanel;
mod spoiler;
mod text;
mod themeicon;
mod title;
mod transition;
mod webview;

pub use actionicon::ActionIcon;
pub use anchor::Anchor;
pub use badge::Badge;
pub use blockquote::Blockquote;
pub use button::Button;
pub use card::Card;
pub use chip::Chip;
pub use closebutton::CloseButton;
pub use code::Code;
pub use copybutton::CopyButton;
pub use divider::{Divider, Orientation};
pub use icon::{Icon, IconName};
pub use image::{Image, ObjectFit};
pub use indicator::Indicator;
pub use kbd::Kbd;
pub use mark::Mark;
pub use panel::Panel;
pub use paper::Paper;
pub use scrollarea::ScrollArea;
pub use skeleton::Skeleton;
pub use panegroup::{PaneGroup, PaneGroupEvent};
pub use splitpanel::{SplitDirection, SplitPanel, SplitPanelEvent};
pub use spoiler::Spoiler;
pub use text::Text;
pub use themeicon::ThemeIcon;
pub use title::Title;
pub use transition::{Collapse, Transition, TransitionKind};
pub use webview::{WebView, WebViewEvent};

pub use chart::{BarChart, LineChart, PieChart, Sparkline};
pub use data::{
    Accordion, Avatar, AvatarGroup, Column, DataView, DataViewEvent, DataViewLayout, List,
    SelectionMode, SortDir, TabBar, TabBarEvent, Table, TableView, TableViewEvent, Tabs, Timeline,
    TreeNode, TreeView, TreeViewEvent,
};
pub use editor::{Editor, EditorEvent, EditorModel, EditorStyle, Highlighter, Language, Pos, TokenKind};
pub use feedback::{
    Alert, Loader, LoaderVariant, Notification, Progress, RingProgress, ToastStack,
};
pub use input::{
    apply_key, Checkbox, CheckboxGroup, ColorInput, ColorInputEvent, Combobox, ComboboxEvent,
    Field, KeyOutcome, NumberInput, NumberInputEvent, PasswordInput, PasswordInputEvent, PinInput,
    PinInputEvent, Radio, RadioGroup, RangeSlider, RangeSliderEvent, Rating, SegmentedControl,
    SegmentedControlEvent, Select, SelectEvent, Slider, SliderEvent, Switch, TagsInput,
    TagsInputEvent, TextArea, TextAreaEvent, TextEdit, TextInput, TextInputEvent,
};
pub use layout::{Align, AppShell, Center, Container, Group, Justify, SimpleGrid, Space, Stack};
pub use nav::{Breadcrumbs, NavLink, Pagination, PaginationEvent, StatusBar, Stepper};
pub use overlay::{
    tooltip, ConfirmModal, ContextMenu, Drawer, HoverCard, LoadingOverlay, Menu, MenuBar,
    MenuColumn, Modal, Placement, Popover, Side, Spotlight, Tooltip,
};
pub use reactive::{
    provide, use_context, use_effect, use_form, use_memo, use_state, watch, Binding, FormState,
    Signal,
};
pub use style::{surface, ColorValue, StyleExt, Surface, Variant};
pub use theme::{
    css, hsl, hsla, rgb, rgba, theme, Color, ColorName, ColorScheme, CssColorError, Palette, Scale,
    Shades, Size, Theme,
};

pub mod prelude {
    //! Common imports for building with `guise`.
    pub use crate::chart::{BarChart, LineChart, PieChart, Sparkline};
    pub use crate::data::{
        Accordion, Avatar, AvatarGroup, Column, DataView, DataViewEvent, DataViewLayout, List,
        SelectionMode, SortDir, TabBar, TabBarEvent, Table, TableView, TableViewEvent, Tabs,
        Timeline, TreeNode, TreeView, TreeViewEvent,
    };
    pub use crate::editor::{Editor, EditorEvent, EditorModel, EditorStyle, Language, Pos};
    pub use crate::feedback::{
        Alert, Loader, LoaderVariant, Notification, Progress, RingProgress, ToastStack,
    };
    pub use crate::input::{
        apply_key, Checkbox, CheckboxGroup, ColorInput, ColorInputEvent, Combobox, ComboboxEvent,
        Field, KeyOutcome, NumberInput, NumberInputEvent, PasswordInput, PasswordInputEvent,
        PinInput, PinInputEvent, Radio, RadioGroup, RangeSlider, RangeSliderEvent, Rating, Select,
        SelectEvent, Slider, SliderEvent, Switch, TagsInput, TagsInputEvent, TextArea,
        TextAreaEvent, TextEdit, TextInput, TextInputEvent,
    };
    pub use crate::input::{SegmentedControl, SegmentedControlEvent};
    pub use crate::layout::SimpleGrid;
    pub use crate::layout::{Align, AppShell, Center, Container, Group, Justify, Space, Stack};
    pub use crate::nav::{Breadcrumbs, NavLink, Pagination, PaginationEvent, StatusBar, Stepper};
    pub use crate::overlay::{
        tooltip, ConfirmModal, ContextMenu, Drawer, HoverCard, LoadingOverlay, Menu, MenuBar,
        MenuColumn, Modal, Placement, Popover, Side, Spotlight, Tooltip,
    };
    pub use crate::reactive::{
        provide, use_context, use_effect, use_form, use_memo, use_state, watch, Binding, FormState,
        Signal,
    };
    pub use crate::style::{ColorValue, StyleExt, Variant};
    pub use crate::theme::{
        css, hsl, hsla, rgb, rgba, theme, Color, ColorName, ColorScheme, Size, Theme,
    };
    pub use crate::{badge, button, code, kbd, text, title};
    pub use crate::{card, center, col, hstack, modal, paper, row, vstack, wrap, zstack};
    pub use crate::{color, style};
    pub use crate::{
        ActionIcon, Anchor, Chip, CloseButton, Code, CopyButton, Icon, IconName, Indicator, Kbd,
        ScrollArea, Skeleton, ThemeIcon,
    };
    pub use crate::{Badge, Button, Card, Divider, Panel, Paper, Text, Title};
    pub use crate::{Blockquote, Image, Mark, ObjectFit, Spoiler};
    pub use crate::{Collapse, Transition, TransitionKind};
    pub use crate::{SplitDirection, SplitPanel, SplitPanelEvent};
    pub use crate::{WebView, WebViewEvent};
}
