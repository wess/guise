//! # guise
//!
//! A component library for [gpui](https://github.com/zed-industries/zed),
//! Zed's GPU-accelerated UI framework.
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

#[cfg(test)]
mod apptests;

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

pub mod ai;
pub mod anim;
pub mod chart;
pub mod data;
pub mod devtools;
pub mod dnd;
pub mod editor;
pub mod feedback;
pub mod flex;
pub mod gpuview;
pub mod input;
pub mod layout;
pub mod markdown;
pub mod nav;
pub mod overlay;
pub mod reactive;
pub mod settings;
pub mod update;

mod about;
mod actionicon;
mod anchor;
mod badge;
mod blockquote;
mod button;
mod card;
mod carousel;
mod chip;
mod closebutton;
mod code;
mod copybutton;
mod divider;
mod frameclock;
mod icon;
mod image;
mod indicator;
mod kbd;
mod mark;
pub mod panegroup;
mod panel;
mod paper;
mod scrollarea;
mod skeleton;
mod splitpanel;
mod spoiler;
mod text;
mod themeicon;
mod themepicker;
mod title;
mod transition;
mod webview;
mod windowcontrols;

pub use about::{About, BuildKind};
pub use actionicon::ActionIcon;
pub use ai::{
  format_cost, AIChatView, AIChatViewEvent, AICitation, AIComposer, AIComposerEvent, AICost,
  AIMessage, AIModel, AIModelPicker, AIModelPickerEvent, AIPricing, AIReasoning, AIRole,
  AISettings, AISettingsEvent, AISource, AISources, AIStreamingText, AIThinking, AITokenMeter,
  AIToolCall, AIToolStatus, AITurn, AITurnTool, AIUsage,
};
pub use anchor::Anchor;
pub use anim::{
  AnimValue, Animated, Animator, AnimatorEvent, At, Clip, Curve, Easing, Frame, IntoKeyframe,
  Keyframe, Loop, Motion, Motioned, Presence, PresenceEvent, Prop, Sequence, Spring, Stagger,
  StaggerAxis, StaggerFrom, Track,
};
pub use badge::Badge;
pub use blockquote::Blockquote;
pub use button::Button;
pub use card::Card;
pub use carousel::{Carousel, CarouselEvent};
pub use chip::Chip;
pub use closebutton::CloseButton;
pub use code::Code;
pub use copybutton::CopyButton;
pub use divider::{Divider, Orientation};
pub use dnd::{apply_reorder, Draggable, DropTarget, SortableList};
pub use icon::{Glyph, Icon, IconName, LUCIDE_VERSION};
pub use image::{Image, ObjectFit};
pub use indicator::Indicator;
pub use kbd::Kbd;
pub use mark::Mark;
pub use panegroup::{PaneGroup, PaneGroupEvent};
pub use panel::Panel;
pub use paper::Paper;
pub use scrollarea::ScrollArea;
pub use skeleton::Skeleton;
pub use splitpanel::{SplitDirection, SplitPanel, SplitPanelEvent};
pub use spoiler::Spoiler;
pub use text::Text;
pub use themeicon::ThemeIcon;
pub use themepicker::{ThemePicker, ThemePickerLayout};
pub use title::Title;
pub use transition::{Collapse, Transition, TransitionKind};
pub use webview::{WebView, WebViewEvent};
pub use windowcontrols::{ResizeHandles, WindowControls, TRAFFIC_LIGHT_INSET};

pub use chart::{AreaChart, BarChart, LineChart, PieChart, ScatterChart, Sparkline};
pub use data::{
  Accordion, Avatar, AvatarGroup, Column, DataView, DataViewEvent, DataViewLayout, List,
  SelectionMode, SortDir, TabBar, TabBarEvent, Table, TableView, TableViewEvent, Tabs, Timeline,
  TreeNode, TreeView, TreeViewEvent, VirtualList,
};
#[cfg(feature = "treesitter")]
pub use editor::TreeSitterHighlighter;
pub use editor::{
  Diagnostic, DocumentHighlighter, Editor, EditorEvent, EditorModel, EditorStyle, Highlighter,
  Language, Pos, Severity, TokenKind,
};
pub use feedback::{
  Alert, Loader, LoaderVariant, Notification, Progress, RingProgress, ToastStack,
};
pub use gpuview::{GpuFit, GpuRect, GpuScene, GpuTexture, GpuView};
pub use input::{
  apply_key, days_in_month, is_leap_year, month_grid, Autocomplete, AutocompleteEvent, Calendar,
  Checkbox, CheckboxGroup, ColorInput, ColorInputEvent, Combobox, ComboboxEvent, Date, DatePicker,
  DatePickerEvent, Dropzone, Field, FileInput, FileInputEvent, KeyOutcome, NumberInput,
  NumberInputEvent, PasswordInput, PasswordInputEvent, PinInput, PinInputEvent, Radio, RadioGroup,
  RangeSlider, RangeSliderEvent, Rating, SegmentedControl, SegmentedControlEvent, Select,
  SelectEvent, Slider, SliderEvent, Switch, TagsInput, TagsInputEvent, TextArea, TextAreaEvent,
  TextAreaSubmit, TextEdit, TextInput, TextInputEvent, Time, TimePicker, TimePickerEvent, Transfer,
  TransferEvent, Weekday, MONTH_NAMES,
};
pub use layout::{
  Align, AppShell, Breakpoint, Center, Container, Group, Justify, Responsive, SimpleGrid, Space,
  Stack,
};
pub use markdown::{Markdown, MarkdownEditor, MarkdownEditorEvent, MarkdownStyle};
pub use nav::{
  Breadcrumbs, NavLink, NavigationMenu, NavigationMenuEvent, Pagination, PaginationEvent,
  StatusBar, Stepper,
};
pub use overlay::{
  tooltip, ConfirmModal, ContextMenu, Drawer, HoverCard, LoadingOverlay, Menu, MenuBar, MenuColumn,
  Modal, ModalCloser, OverlayHost, Placement, Popover, Side, Spotlight, Tooltip, Tour, TourEvent,
};
pub use reactive::{
  provide, use_context, use_effect, use_form, use_memo, use_state, validators, watch, Binding,
  Form, FormState, FormValues, Rule, Signal, Validator,
};
pub use style::{surface, ColorValue, StyleExt, Surface, Variant};
pub use theme::{
  css, hsl, hsla, manager, rgb, rgba, scheme_of, theme, Color, ColorName, ColorScheme,
  CssColorError, Palette, Scale, Shades, Size, Theme, ThemeChoice, ThemeEntry, ThemeJsonError,
  ThemeLoadError, ThemeManager, ThemeSource, PRESET_NAMES,
};
// The free functions (`start`, `check_now`, `detect`, `is_newer`, …) stay behind
// `update::` — their names only read clearly next to the module.
pub use update::{
  InstallKind, Relaunch, Release, ReleaseAsset, UpdateCheck, UpdateConfig, UpdateNotice,
  UpdateNoticeEvent, UpdateOutcome, UpdatePrompt, UpdatePromptEvent, UpdateSource, UpdateStage,
  Updater,
};

pub mod prelude {
  //! Common imports for building with `guise`.
  pub use crate::ai::{
    AIChatView, AIChatViewEvent, AICitation, AIComposer, AIComposerEvent, AICost, AIMessage,
    AIModel, AIModelPicker, AIModelPickerEvent, AIPricing, AIReasoning, AIRole, AISettings,
    AISettingsEvent, AISource, AISources, AIStreamingText, AIThinking, AITokenMeter, AIToolCall,
    AIToolStatus, AITurn, AITurnTool, AIUsage,
  };
  pub use crate::chart::{AreaChart, BarChart, LineChart, PieChart, ScatterChart, Sparkline};
  pub use crate::data::{
    Accordion, Avatar, AvatarGroup, Column, DataView, DataViewEvent, DataViewLayout, List,
    SelectionMode, SortDir, TabBar, TabBarEvent, Table, TableView, TableViewEvent, Tabs, Timeline,
    TreeNode, TreeView, TreeViewEvent, VirtualList,
  };
  pub use crate::devtools::{
    DevTools, DevToolsEvent, DevToolsState, DevToolsTab, Dock, LogLevel, LogRecord, NetworkRecord,
    Probed, ProbedAny, RequestState, ResourceKind, SourceRef, StorageDomain, StorageEntry,
    StorageKind, TimelineEvent, TimelineKind, Timings,
  };
  pub use crate::dnd::{apply_reorder, Draggable, DropTarget, SortableList};
  pub use crate::editor::{
    Diagnostic, Editor, EditorEvent, EditorModel, EditorStyle, Language, Pos, Severity,
  };
  pub use crate::feedback::{
    Alert, Loader, LoaderVariant, Notification, Progress, RingProgress, ToastStack,
  };
  pub use crate::gpuview::{GpuFit, GpuRect, GpuScene, GpuTexture, GpuView};
  pub use crate::input::{
    apply_key, apply_nav, Autocomplete, AutocompleteEvent, Calendar, Checkbox, CheckboxGroup,
    ColorInput, ColorInputEvent, Combobox, ComboboxEvent, Date, DatePicker, DatePickerEvent,
    Dropzone, Field, FileInput, FileInputEvent, KeyOutcome, NumberInput, NumberInputEvent,
    PasswordInput, PasswordInputEvent, PinInput, PinInputEvent, Radio, RadioGroup, RangeSlider,
    RangeSliderEvent, Rating, Select, SelectEvent, Slider, SliderEvent, Switch, TagsInput,
    TagsInputEvent, TextArea, TextAreaEvent, TextAreaSubmit, TextEdit, TextInput, TextInputEvent,
    Time, TimePicker, TimePickerEvent, Transfer, TransferEvent, Weekday,
  };
  pub use crate::input::{SegmentedControl, SegmentedControlEvent};
  pub use crate::layout::SimpleGrid;
  pub use crate::layout::{
    Align, AppShell, Breakpoint, Center, Container, Group, Justify, Responsive, Space, Stack,
  };
  pub use crate::markdown::{Markdown, MarkdownEditor, MarkdownEditorEvent, MarkdownStyle};
  pub use crate::nav::{
    Breadcrumbs, NavLink, NavigationMenu, NavigationMenuEvent, Pagination, PaginationEvent,
    StatusBar, Stepper,
  };
  pub use crate::overlay::{
    tooltip, ConfirmModal, ContextMenu, Drawer, HoverCard, LoadingOverlay, Menu, MenuBar,
    MenuColumn, Modal, ModalCloser, OverlayHost, Placement, Popover, Side, Spotlight, Tooltip,
    Tour, TourEvent,
  };
  pub use crate::reactive::{
    provide, use_context, use_effect, use_form, use_memo, use_state, validators, watch, Binding,
    Form, FormState, FormValues, Rule, Signal, Validator,
  };
  pub use crate::settings::{
    SettingsPage, SettingsRow, SettingsSection, SettingsView, SettingsViewEvent,
  };
  pub use crate::style::{ColorValue, StyleExt, Variant};
  pub use crate::theme::{
    css, hsl, hsla, rgb, rgba, theme, Color, ColorName, ColorScheme, Size, Theme, ThemeChoice,
    ThemeEntry, ThemeManager, ThemeSource,
  };
  pub use crate::update::{
    InstallKind, Relaunch, Release, ReleaseAsset, UpdateCheck, UpdateConfig, UpdateNotice,
    UpdateNoticeEvent, UpdateOutcome, UpdatePrompt, UpdatePromptEvent, UpdateSource, UpdateStage,
    Updater,
  };
  pub use crate::{badge, button, code, kbd, text, title};
  pub use crate::{card, center, col, hstack, modal, paper, row, vstack, wrap, zstack};
  pub use crate::{color, motion, sequence, style};
  pub use crate::{About, BuildKind, ResizeHandles, WindowControls};
  pub use crate::{
    ActionIcon, Anchor, Chip, CloseButton, Code, CopyButton, Glyph, Icon, IconName, Indicator, Kbd,
    ScrollArea, Skeleton, ThemeIcon, ThemePicker, ThemePickerLayout,
  };
  pub use crate::{
    AnimValue, Animated, Animator, AnimatorEvent, At, Clip, Curve, Easing, Frame, IntoKeyframe,
    Keyframe, Loop, Motion, Motioned, Presence, PresenceEvent, Prop, Sequence, Spring, Stagger,
    StaggerAxis, StaggerFrom, Track,
  };
  pub use crate::{
    Badge, Button, Card, Carousel, CarouselEvent, Divider, Panel, Paper, Text, Title,
  };
  pub use crate::{Blockquote, Image, Mark, ObjectFit, Spoiler};
  pub use crate::{Collapse, Transition, TransitionKind};
  pub use crate::{SplitDirection, SplitPanel, SplitPanelEvent};
  pub use crate::{WebView, WebViewEvent};
}
