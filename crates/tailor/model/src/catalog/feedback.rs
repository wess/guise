//! Status, progress, and the overlays.
//!
//! Overlays are containers here rather than open/closed states: on the canvas a
//! `Modal` shows its content so you can lay it out, and generated code puts it
//! behind the `open` flag the component already takes.

use crate::node::{EventSpec, CLOSE};
use crate::props::{
  boolean, color, color_name, enums, float, icon, size, text, variant, Emit, PropValue,
};
use crate::tokens::{ColorToken, SizeToken, VariantToken};

use super::spec::{slot, ComponentSpec, Ctor, SlotSpec, CHILDREN};

const CLOSES: &[EventSpec] = &[CLOSE];

const TOOLTIP_SLOTS: &[SlotSpec] = &[slot("child", "Target", "child")];

pub static SPECS: &[ComponentSpec] = &[
  comp!(
      "alert", "Alert", "Alert", Feedback, "triangle-alert",
      "An inline message with a title and an icon.",
      Ctor::Arg("message"),
      props: &[
          crate::props::multiline("message", "Message", Emit::None),
          text("title", "Title", Emit::Method("title")),
          variant("variant", "Variant", Emit::Method("variant"), VariantToken::Light),
          color("color", "Color", Emit::Method("color"), ColorToken::Blue),
          icon("icon", "Icon", Emit::Method("icon")),
          boolean("closeable", "Closeable", Emit::None, false),
      ],
      events: CLOSES,
  ),
  comp!(
      "notification", "Notification", "Notification", Feedback, "bell",
      "A toast body.",
      Ctor::Arg("message"),
      props: &[
          text("message", "Message", Emit::None),
          text("title", "Title", Emit::Method("title")),
          color_name("color", "Color", Emit::Method("color"), ColorToken::Blue),
          icon("icon", "Icon", Emit::Method("icon")),
          boolean("closeable", "Closeable", Emit::None, false),
      ],
      events: CLOSES,
  ),
  comp!(
      "loader", "Loader", "Loader", Feedback, "loader",
      "A spinner, in three shapes.",
      Ctor::Unit,
      props: &[
          enums("variant", "Shape", Emit::Method("variant"), "LoaderVariant",
              &["dots", "bars"], || PropValue::Choice("dots".into())),
          size("size", "Size", Emit::Method("size"), SizeToken::Md),
          color("color", "Color", Emit::Method("color"), ColorToken::Blue),
      ],
  ),
  comp!(
      "progress", "Progress", "Progress", Feedback, "loader-pinwheel",
      "A horizontal progress bar, 0 to 1.",
      Ctor::Arg("value"),
      props: &[
          float("value", "Value", Emit::None, || PropValue::Float(0.5)),
          color("color", "Color", Emit::Method("color"), ColorToken::Blue),
          size("size", "Size", Emit::Method("size"), SizeToken::Md),
          size("radius", "Radius", Emit::Method("radius"), SizeToken::Xl),
      ],
  ),
  comp!(
      "ringprogress", "Ring progress", "RingProgress", Feedback, "circle-dashed",
      "A circular progress ring with a label.",
      Ctor::Arg("value"),
      props: &[
          float("value", "Value", Emit::None, || PropValue::Float(0.65)),
          float("size", "Diameter", Emit::Method("size"), || PropValue::Float(80.0)),
          color_name("color", "Color", Emit::Method("color"), ColorToken::Blue),
          text("label", "Label", Emit::Method("label")),
      ],
  ),
  comp!(
      "skeleton", "Skeleton", "Skeleton", Feedback, "rectangle-horizontal",
      "A loading placeholder block.",
      Ctor::Unit,
      props: &[
          float("width", "Width", Emit::Method("width"), || PropValue::Float(160.0)),
          float("height", "Height", Emit::Method("height"), || PropValue::Float(16.0)),
          size("radius", "Radius", Emit::Method("radius"), SizeToken::Sm),
      ],
  ),
  comp!(
      "modal", "Modal", "Modal", Feedback, "square-square",
      "A centred dialog over a scrim.",
      Ctor::Unit,
      props: &[
          text("title", "Title", Emit::Method("title")),
          float("width", "Width", Emit::Method("width"), || PropValue::Float(480.0)),
          size("padding", "Padding", Emit::Method("padding"), SizeToken::Md),
          size("radius", "Radius", Emit::Method("radius"), SizeToken::Md),
      ],
      slots: &[CHILDREN],
      events: CLOSES,
  ),
  comp!(
      "drawer", "Drawer", "Drawer", Feedback, "panel-right",
      "A panel that slides in from an edge.",
      Ctor::Unit,
      props: &[
          text("title", "Title", Emit::Method("title")),
          enums("side", "Side", Emit::Method("side"), "Side",
              &["left", "right", "top", "bottom"], || PropValue::Choice("right".into())),
          float("size", "Size", Emit::Method("size"), || PropValue::Float(360.0)),
          size("padding", "Padding", Emit::Method("padding"), SizeToken::Md),
      ],
      slots: &[CHILDREN],
      events: CLOSES,
  ),
  comp!(
      "tooltip", "Tooltip", "Tooltip", Feedback, "message-square",
      "A hover label on any child.",
      Ctor::Special,
      props: &[
          text("label", "Label", Emit::None),
          enums("placement", "Placement", Emit::None, "Placement",
              &["top", "bottom", "left", "right"], || PropValue::Choice("top".into())),
      ],
      slots: TOOLTIP_SLOTS,
  ),
  comp!(
      "loadingoverlay", "Loading overlay", "LoadingOverlay", Feedback, "loader-circle",
      "A scrim with a spinner. Place it inside the surface it covers.",
      Ctor::Unit,
      props: &[boolean("visible", "Visible", Emit::Method("visible"), true)],
  ),
  comp!(
      "confirmmodal", "Confirm", "ConfirmModal", Feedback, "circle-help",
      "The two-button question before something irreversible.",
      Ctor::Unit,
      props: &[
          text("title", "Title", Emit::Method("title")),
          text("message", "Message", Emit::Method("message")),
          text("confirm_label", "Confirm label", Emit::Method("confirm_label")),
          text("cancel_label", "Cancel label", Emit::Method("cancel_label")),
          boolean("danger", "Danger", Emit::Flag("danger"), false),
          float("width", "Width", Emit::Method("width"), || PropValue::Float(380.0)),
      ],
      slots: &[CHILDREN],
  ),
];
