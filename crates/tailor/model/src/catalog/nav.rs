//! Navigation chrome.

use crate::node::CLICK;
use crate::props::{boolean, color_name, float, icon, int, items, size, text, Emit, PropValue};
use crate::tokens::{ColorToken, SizeToken};

use super::spec::{slot, ComponentSpec, Ctor, SlotSpec};

const STATUSBAR_SLOTS: &[SlotSpec] = &[
  slot("left", "Left", "left"),
  slot("center", "Center", "center"),
  slot("right", "Right", "right"),
];

fn crumbs() -> PropValue {
  PropValue::Items(vec!["Home".into(), "Projects".into(), "Tailor".into()])
}

pub static SPECS: &[ComponentSpec] = &[
  comp!(
      "breadcrumbs", "Breadcrumbs", "Breadcrumbs", Navigation, "chevrons-right",
      "A path of links with separators.",
      Ctor::Unit,
      props: &[
          items("items", "Items", Emit::Method("items"), crumbs),
          text("separator", "Separator", Emit::Method("separator")),
      ],
  ),
  comp!(
      "navlink", "Nav link", "NavLink", Navigation, "square-arrow-right",
      "A sidebar row with an icon and a description.",
      Ctor::IdAnd("label"),
      props: &[
          text("label", "Label", Emit::None),
          text("description", "Description", Emit::Method("description")),
          icon("icon", "Icon", Emit::Method("icon")),
          color_name("color", "Color", Emit::Method("color"), ColorToken::Blue),
          boolean("active", "Active", Emit::Method("active"), false),
      ],
      events: &[CLICK],
  ),
  comp!(
      "stepper", "Stepper", "Stepper", Navigation, "footprints",
      "Numbered steps with a current position.",
      Ctor::Unit,
      props: &[
          crate::props::hinted(
              items("steps", "Steps", Emit::Custom, || {
                  PropValue::Items(vec!["Account".into(), "Details".into(), "Review".into()])
              }),
              "label, or label | description",
          ),
          int("active", "Active", Emit::Method("active"), || PropValue::Int(1)),
          color_name("color", "Color", Emit::Method("color"), ColorToken::Blue),
      ],
  ),
  comp!(
      "pagination", "Pagination", "Pagination", Navigation, "ellipsis",
      "Page numbers with previous and next.",
      Ctor::EntityArg("total"),
      props: &[
          int("total", "Total pages", Emit::None, || PropValue::Int(10)),
          int("active", "Active page", Emit::Method("active"), || PropValue::Int(1)),
          color_name("color", "Color", Emit::Method("color"), ColorToken::Blue),
      ],
  ),
  comp!(
      "statusbar", "Status bar", "StatusBar", Navigation, "panel-bottom",
      "A three-region footer strip.",
      Ctor::Unit,
      props: &[crate::props::float("height", "Height", Emit::Method("height"), || {
          PropValue::Float(28.0)
      })],
      slots: STATUSBAR_SLOTS,
  ),
  comp!(
      "navigationmenu", "Navigation menu", "NavigationMenu", Navigation, "menu",
      "A horizontal menu bar with dropdowns.",
      Ctor::Entity,
      props: &[
          crate::props::hinted(
              items("items", "Items", Emit::Custom, || {
                  PropValue::Items(vec!["file:File".into(), "edit:Edit".into(), "view:View".into()])
              }),
              "id:Label per line",
          ),
          text("active", "Active id", Emit::Method("active")),
      ],
  ),
  comp!(
      "menu", "Menu", "Menu", Navigation, "chevron-down",
      "A labelled trigger with a dropdown of actions.",
      Ctor::EntityArg("trigger"),
      props: &[
          text("trigger", "Trigger", Emit::None),
          // One `.item(label, handler)` per line; a line starting with `-` is a
          // divider and one starting with `#` a section heading.
          items("items", "Items", Emit::Custom, || PropValue::Items(
              vec!["Rename".into(), "Duplicate".into(), "-".into(), "Delete".into()])),
          size("size", "Size", Emit::Method("size"), SizeToken::Sm),
      ],
  ),
  comp!(
      "contextmenu", "Context menu", "ContextMenu", Navigation, "mouse-pointer-2",
      "The same list, on a right-click anywhere in its target.",
      Ctor::Entity,
      props: &[
          items("items", "Items", Emit::Custom, || PropValue::Items(
              vec!["Cut".into(), "Copy".into(), "-".into(), "Paste".into()])),
          float("width", "Width", Emit::Method("width"), || PropValue::Float(200.0)),
          size("size", "Size", Emit::Method("size"), SizeToken::Sm),
      ],
  ),
];
