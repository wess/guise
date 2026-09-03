//! Buttons, badges, and the small interactive bits.

use crate::node::{EventSpec, CLICK};
use crate::props::{boolean, color, color_name, enums, icon, size, text, variant, Emit, PropValue};
use crate::tokens::{ColorToken, SizeToken, VariantToken};

use super::spec::{slot, ComponentSpec, Ctor, SlotSpec};

const CLICKS: &[EventSpec] = &[CLICK];

const BUTTON_SLOTS: &[SlotSpec] = &[
  slot("left", "Left section", "left_section"),
  slot("right", "Right section", "right_section"),
];

pub static SPECS: &[ComponentSpec] = &[
  comp!(
      "button", "Button", "Button", Controls, "square-mouse-pointer",
      "A labelled action.",
      Ctor::IdAnd("label"),
      props: &[
          text("label", "Label", Emit::None),
          variant("variant", "Variant", Emit::Method("variant"), VariantToken::Filled),
          color("color", "Color", Emit::Method("color"), ColorToken::Blue),
          size("size", "Size", Emit::Method("size"), SizeToken::Md),
          size("radius", "Radius", Emit::Method("radius"), SizeToken::Md),
          boolean("full_width", "Full width", Emit::Method("full_width"), false),
          boolean("disabled", "Disabled", Emit::Method("disabled"), false),
      ],
      slots: BUTTON_SLOTS,
      events: CLICKS,
  ),
  comp!(
      "actionicon", "Icon button", "ActionIcon", Controls, "mouse-pointer-click",
      "An icon-only button.",
      Ctor::IdAnd("icon"),
      props: &[
          icon("icon", "Icon", Emit::None),
          text("label", "Label", Emit::Method("label")),
          variant("variant", "Variant", Emit::Method("variant"), VariantToken::Subtle),
          color("color", "Color", Emit::Method("color"), ColorToken::Gray),
          size("size", "Size", Emit::Method("size"), SizeToken::Md),
          size("radius", "Radius", Emit::Method("radius"), SizeToken::Md),
          boolean("disabled", "Disabled", Emit::Method("disabled"), false),
      ],
      events: CLICKS,
  ),
  comp!(
      "closebutton", "Close button", "CloseButton", Controls, "x",
      "The dismiss affordance overlays use.",
      Ctor::Id,
      props: &[size("size", "Size", Emit::Method("size"), SizeToken::Md)],
      events: CLICKS,
  ),
  comp!(
      "copybutton", "Copy button", "CopyButton", Controls, "clipboard-copy",
      "Copies a string and confirms it did.",
      Ctor::EntityValue("value"),
      props: &[
          text("value", "Value", Emit::None),
          text("label", "Label", Emit::Method("label")),
      ],
  ),
  comp!(
      "badge", "Badge", "Badge", Controls, "badge",
      "A small status label.",
      Ctor::Arg("label"),
      props: &[
          text("label", "Label", Emit::None),
          variant("variant", "Variant", Emit::Method("variant"), VariantToken::Light),
          color("color", "Color", Emit::Method("color"), ColorToken::Blue),
          size("size", "Size", Emit::Method("size"), SizeToken::Md),
      ],
  ),
  comp!(
      "chip", "Chip", "Chip", Controls, "tag",
      "A toggleable pill.",
      Ctor::IdAnd("label"),
      props: &[
          text("label", "Label", Emit::None),
          boolean("checked", "Checked", Emit::Method("checked"), false),
          color("color", "Color", Emit::Method("color"), ColorToken::Blue),
          size("size", "Size", Emit::Method("size"), SizeToken::Md),
      ],
      events: &[crate::node::CHANGE_BOOL],
  ),
  comp!(
      "icon", "Icon", "Icon", Controls, "shapes",
      "A Lucide glyph.",
      Ctor::Arg("icon"),
      props: &[
          icon("icon", "Icon", Emit::None),
          size("size", "Size", Emit::Method("size"), SizeToken::Md),
          color_name("color", "Color", Emit::Method("color"), ColorToken::Gray),
      ],
  ),
  comp!(
      "themeicon", "Theme icon", "ThemeIcon", Controls, "square-asterisk",
      "An icon on a themed tile.",
      Ctor::Arg("icon"),
      props: &[
          icon("icon", "Icon", Emit::None),
          variant("variant", "Variant", Emit::Method("variant"), VariantToken::Light),
          color_name("color", "Color", Emit::Method("color"), ColorToken::Blue),
          size("size", "Size", Emit::Method("size"), SizeToken::Md),
          size("radius", "Radius", Emit::Method("radius"), SizeToken::Md),
      ],
  ),
  comp!(
      "indicator", "Indicator", "Indicator", Controls, "circle-dot",
      "A dot or count pinned to the corner of its child.",
      Ctor::Special,
      props: &[
          text("label", "Label", Emit::Method("label")),
          color_name("color", "Color", Emit::Method("color"), ColorToken::Red),
          boolean("disabled", "Hidden", Emit::Method("disabled"), false),
      ],
      slots: &[slot("child", "Target", "child")],
  ),
  comp!(
      "rating", "Rating", "Rating", Controls, "star",
      "A star rating.",
      Ctor::Id,
      props: &[
          crate::props::int("count", "Stars", Emit::Method("count"), || PropValue::Int(5)),
          crate::props::float("value", "Value", Emit::None, || PropValue::Float(3.0)),
          color("color", "Color", Emit::Method("color"), ColorToken::Yellow),
          size("size", "Size", Emit::Method("size"), SizeToken::Md),
          boolean("readonly", "Read only", Emit::Method("readonly"), false),
      ],
      events: &[crate::node::CHANGE_VALUE],
  ),
  comp!(
      "kbdgroup", "Shortcut", "Kbd", Controls, "command",
      "A row of key caps, split on +.",
      Ctor::Special,
      props: &[
          crate::props::items("keys", "Keys", Emit::None, || {
              PropValue::Items(vec!["cmd".into(), "K".into()])
          }),
      ],
  ),
  comp!(
      "webview", "Web view", "WebView", Controls, "globe",
      "A native embedded browser surface.",
      Ctor::Entity,
      props: &[
          text("url", "URL", Emit::None),
          enums("mode", "Mode", Emit::None, "", &["url", "html"], || PropValue::Choice("url".into())),
      ],
  ),
  comp!(
      "themepicker", "Theme picker", "ThemePicker", Controls, "palette",
      "The installed themes, and the click that wears one.",
      Ctor::Unit,
      props: &[
          enums("layout", "Layout", Emit::Method("layout"), "ThemePickerLayout",
              &["list", "grid"], || PropValue::Choice("list".into())),
          boolean("system_option", "Offer system", Emit::Method("system_option"), true),
      ],
  ),
];
