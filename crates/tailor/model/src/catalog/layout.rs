//! Layout components: the boxes you build a screen out of.
//!
//! `frame` is the primitive — a plain `div` whose whole behaviour comes from
//! its style — and everything else is a themed container from `guise::layout`,
//! plus the pixel-based `guise::flex` trio for the cases where a flexbox by
//! numbers reads better than one by tokens.

use crate::node::{EventSpec, Node, TOGGLE};
use crate::props::{
  boolean, color, enums, float, int, items, size, text, Emit, PropSpec, PropValue,
};
use crate::style::{Dimension, Edges};
use crate::tokens::{ColorToken, SizeToken};

use super::spec::{slot, ComponentSpec, Ctor, SlotSpec, CHILDREN};

const ALIGN: &[&str] = &["start", "center", "end", "stretch"];
const JUSTIFY: &[&str] = &["start", "center", "end", "between", "around"];

const GAP: PropSpec = size("gap", "Gap", Emit::Method("gap"), SizeToken::Md);
const ALIGN_PROP: PropSpec = enums(
  "align",
  "Align",
  Emit::Method("align"),
  "Align",
  ALIGN,
  || PropValue::Choice("start".into()),
);
const JUSTIFY_PROP: PropSpec = enums(
  "justify",
  "Justify",
  Emit::Method("justify"),
  "Justify",
  JUSTIFY,
  || PropValue::Choice("start".into()),
);

const SURFACE_PROPS: &[PropSpec] = &[
  size("padding", "Padding", Emit::Method("padding"), SizeToken::Md),
  size("radius", "Radius", Emit::Method("radius"), SizeToken::Md),
  boolean("with_border", "Border", Emit::Method("with_border"), true),
  size("shadow", "Shadow", Emit::Method("shadow"), SizeToken::Xs),
];

const PANEL_PROPS: &[PropSpec] = &[
  text("title", "Title", Emit::Method("title")),
  text("description", "Description", Emit::Method("description")),
  size("padding", "Padding", Emit::Method("padding"), SizeToken::Md),
  size("radius", "Radius", Emit::Method("radius"), SizeToken::Md),
  boolean("with_border", "Border", Emit::Method("with_border"), true),
  size("shadow", "Shadow", Emit::Method("shadow"), SizeToken::Xs),
  boolean(
    "collapsible",
    "Collapsible",
    Emit::Flag("collapsible"),
    false,
  ),
  boolean("collapsed", "Collapsed", Emit::Method("collapsed"), false),
];

const PANEL_SLOTS: &[SlotSpec] = &[
  CHILDREN,
  slot("icon", "Icon", "icon"),
  slot("action", "Action", "action"),
  slot("footer", "Footer", "footer"),
];

const APPSHELL_SLOTS: &[SlotSpec] = &[
  CHILDREN,
  slot("header", "Header", "header"),
  slot("navbar", "Navbar", "navbar"),
  slot("aside", "Aside", "aside"),
  slot("footer", "Footer", "footer"),
];

const SPLIT_SLOTS: &[SlotSpec] = &[
  slot("first", "First", "first"),
  slot("second", "Second", "second"),
];

const MAIN_AXIS: &[&str] = &[
  "start",
  "center",
  "end",
  "space_between",
  "space_around",
  "space_evenly",
];
const CROSS_AXIS: &[&str] = &["start", "center", "end", "stretch"];

fn frame_defaults(node: &mut Node) {
  node.style.padding = Edges::all(16.0);
  node.style.gap = Some(12.0);
}

fn absolute_defaults(node: &mut Node) {
  node.style.layout = crate::style::LayoutMode::Absolute;
  node.style.width = Dimension::Px(320.0);
  node.style.height = Dimension::Px(240.0);
}

fn scroll_defaults(node: &mut Node) {
  node.style.height = Dimension::Px(240.0);
}

const CLOSE_EVENTS: &[EventSpec] = &[TOGGLE];

const SETTINGS_ROW_SLOTS: &[SlotSpec] = &[slot("control", "Control", "control")];

pub static SPECS: &[ComponentSpec] = &[
  comp!(
      "frame", "Frame", "div", Layout, "square-dashed",
      "A plain box. Everything about it comes from its style.",
      Ctor::Special,
      slots: &[CHILDREN],
      on_place: Some(frame_defaults),
  ),
  comp!(
      "canvas", "Absolute frame", "div", Layout, "move",
      "A frame whose children sit at explicit x/y offsets.",
      Ctor::Special,
      slots: &[CHILDREN],
      on_place: Some(absolute_defaults),
  ),
  comp!(
      "stack", "Stack", "Stack", Layout, "rows-3",
      "A vertical stack with a themed gap.",
      Ctor::Unit,
      props: &[GAP, ALIGN_PROP, JUSTIFY_PROP],
      slots: &[CHILDREN],
  ),
  comp!(
      "group", "Group", "Group", Layout, "columns-3",
      "A horizontal row with a themed gap.",
      Ctor::Unit,
      props: &[
          GAP,
          ALIGN_PROP,
          JUSTIFY_PROP,
          boolean("wrap", "Wrap", Emit::Method("wrap"), false),
          boolean("grow", "Grow children", Emit::Method("grow"), false),
      ],
      slots: &[CHILDREN],
  ),
  comp!(
      "center", "Center", "Center", Layout, "align-center-horizontal",
      "Centres its children on both axes.",
      Ctor::Unit,
      props: &[boolean("inline", "Inline", Emit::Method("inline"), false)],
      slots: &[CHILDREN],
  ),
  comp!(
      "grid", "Grid", "SimpleGrid", Layout, "grid-3x3",
      "An even column grid.",
      Ctor::Arg("cols"),
      props: &[
          int("cols", "Columns", Emit::None, || PropValue::Int(3)),
          size("spacing", "Spacing", Emit::Method("spacing"), SizeToken::Md),
      ],
      slots: &[CHILDREN],
  ),
  comp!(
      "container", "Container", "Container", Layout, "square",
      "A max-width, centred content column.",
      Ctor::Unit,
      props: &[
          size("size", "Max width", Emit::Method("size"), SizeToken::Lg),
          size("padding", "Padding", Emit::Method("padding"), SizeToken::Md),
      ],
      slots: &[CHILDREN],
  ),
  comp!(
      "space", "Space", "Space", Layout, "move-vertical",
      "A fixed gap on one axis.",
      Ctor::Special,
      props: &[
          enums("axis", "Axis", Emit::None, "", &["x", "y"], || PropValue::Choice("y".into())),
          size("size", "Size", Emit::None, SizeToken::Md),
      ],
  ),
  comp!(
    "spacer",
    "Spacer",
    "div",
    Layout,
    "unfold-horizontal",
    "Flexible space that pushes its siblings apart.",
    Ctor::Special,
  ),
  comp!(
      "divider", "Divider", "Divider", Layout, "minus",
      "A rule, optionally with a label.",
      Ctor::Special,
      props: &[
          enums("orientation", "Orientation", Emit::None, "", &["horizontal", "vertical"], || {
              PropValue::Choice("horizontal".into())
          }),
          text("label", "Label", Emit::Method("label")),
      ],
  ),
  comp!(
      "card", "Card", "Card", Layout, "square-round-corner",
      "A bordered surface with padding and a shadow.",
      Ctor::Unit,
      props: SURFACE_PROPS,
      slots: &[CHILDREN],
  ),
  comp!(
      "paper", "Paper", "Paper", Layout, "file",
      "Card's flatter sibling — a surface without the card affordances.",
      Ctor::Unit,
      props: SURFACE_PROPS,
      slots: &[CHILDREN],
  ),
  comp!(
      "panel", "Panel", "Panel", Layout, "panel-top",
      "A titled surface with an action row and a footer.",
      Ctor::Unit,
      props: PANEL_PROPS,
      slots: PANEL_SLOTS,
      events: CLOSE_EVENTS,
  ),
  comp!(
      "scrollarea", "Scroll area", "ScrollArea", Layout, "scroll-text",
      "Scrolls its content past a fixed height, or past the space it is given.",
      Ctor::Id,
      props: &[
          float("max_height", "Max height", Emit::Method("max_height"), || PropValue::Float(240.0)),
          boolean("fill", "Fill parent", Emit::Flag("fill"), false),
          boolean("horizontal", "Horizontal", Emit::Method("horizontal"), false),
      ],
      slots: &[CHILDREN],
      on_place: Some(scroll_defaults),
  ),
  comp!(
      "appshell", "App shell", "AppShell", Layout, "layout-panel-left",
      "Header, navbar, aside, footer, and the body between them.",
      Ctor::Special,
      props: &[
          float("header_height", "Header height", Emit::None, || PropValue::Float(56.0)),
          float("navbar_width", "Navbar width", Emit::None, || PropValue::Float(240.0)),
          float("aside_width", "Aside width", Emit::None, || PropValue::Float(280.0)),
          float("footer_height", "Footer height", Emit::None, || PropValue::Float(36.0)),
      ],
      slots: APPSHELL_SLOTS,
  ),
  comp!(
      "splitpanel", "Split panel", "SplitPanel", Layout, "columns-2",
      "Two resizable regions with a draggable divider.",
      Ctor::Entity,
      props: &[
          enums("direction", "Direction", Emit::Method("direction"), "SplitDirection",
              &["horizontal", "vertical"], || PropValue::Choice("horizontal".into())),
          float("ratio", "Ratio", Emit::Method("ratio"), || PropValue::Float(0.5)),
          float("min_first", "Min first", Emit::Method("min_first"), || PropValue::Float(120.0)),
          float("min_second", "Min second", Emit::Method("min_second"), || PropValue::Float(120.0)),
          float("handle_size", "Handle", Emit::Method("handle_size"), || PropValue::Float(6.0)),
      ],
      slots: SPLIT_SLOTS,
      on_place: Some(scroll_defaults),
  ),
  comp!(
      "flexrow", "Flex row", "Row", Layout, "align-horizontal-space-around",
      "Flutter-style row: pixel gaps and axis alignment.",
      Ctor::Unit,
      props: &[
          float("gap", "Gap", Emit::Method("gap"), || PropValue::Float(8.0)),
          enums("main", "Main axis", Emit::Method("main_axis_alignment"), "MainAxisAlignment",
              MAIN_AXIS, || PropValue::Choice("start".into())),
          enums("cross", "Cross axis", Emit::Method("cross_axis_alignment"), "CrossAxisAlignment",
              CROSS_AXIS, || PropValue::Choice("center".into())),
      ],
      slots: &[CHILDREN],
      imports: &["use guise::flex::{Row, MainAxisAlignment, CrossAxisAlignment};"],
  ),
  comp!(
      "flexcolumn", "Flex column", "Column", Layout, "align-vertical-space-around",
      "Flutter-style column: pixel gaps and axis alignment.",
      Ctor::Unit,
      props: &[
          float("gap", "Gap", Emit::Method("gap"), || PropValue::Float(8.0)),
          enums("main", "Main axis", Emit::Method("main_axis_alignment"), "MainAxisAlignment",
              MAIN_AXIS, || PropValue::Choice("start".into())),
          enums("cross", "Cross axis", Emit::Method("cross_axis_alignment"), "CrossAxisAlignment",
              CROSS_AXIS, || PropValue::Choice("start".into())),
      ],
      slots: &[CHILDREN],
      imports: &["use guise::flex::{Column, MainAxisAlignment, CrossAxisAlignment};"],
  ),
  comp!(
      "expanded", "Expanded", "Expanded", Layout, "unfold-vertical",
      "Takes the free space in a flex row or column.",
      Ctor::Special,
      props: &[float("flex", "Flex", Emit::Method("flex"), || PropValue::Float(1.0))],
      slots: &[CHILDREN],
      imports: &["use guise::flex::Expanded;"],
  ),
  comp!(
      "surface", "Surface", "div", Layout, "paintbrush",
      "A frame pre-filled with a themed colour — section backgrounds.",
      Ctor::Special,
      props: &[color("fill", "Fill", Emit::None, ColorToken::Dark)],
      slots: &[CHILDREN],
      on_place: Some(frame_defaults),
  ),
  comp!(
      "settingsview", "Settings screen", "SettingsView", Layout, "settings",
      "A page list beside a scrolling pane. One slot per page.",
      // Drawn, not instantiated: the pages are `'static` content closures,
      // which is exactly what a designer cannot drop into. The generated code
      // uses the real component, the way Tabs does.
      Ctor::Entity,
      props: &[
          items("pages", "Pages", Emit::Custom, || PropValue::Items(
              vec!["Appearance".into(), "Editor".into()])),
          boolean("searchable", "Searchable", Emit::Method("searchable"), true),
          float("sidebar_width", "Sidebar width", Emit::Method("sidebar_width"),
              || PropValue::Float(200.0)),
      ],
      dynamic: Some(super::spec::DynamicSlots { from_prop: "pages", prefix: "page", method: "" }),
  ),
  comp!(
      "settingssection", "Settings section", "SettingsSection", Layout, "rows-2",
      "A heading, a rule, and the rows under it.",
      Ctor::Arg("title"),
      props: &[
          text("title", "Title", Emit::None),
          text("description", "Description", Emit::Method("description")),
          boolean("rule", "Rule", Emit::Method("rule"), true),
      ],
      slots: &[CHILDREN],
  ),
  comp!(
      "settingsrow", "Settings row", "SettingsRow", Layout, "between-horizontal-start",
      "A name on the left, a control on the right.",
      Ctor::IdAnd("label"),
      props: &[
          text("label", "Label", Emit::None),
          text("description", "Description", Emit::Method("description")),
          boolean("modified", "Modified", Emit::Method("modified"), false),
          boolean("divider", "Divider", Emit::Method("divider"), true),
      ],
      slots: SETTINGS_ROW_SLOTS,
  ),
  comp!(
      "about", "About", "About", Layout, "info",
      "The name, version, and build of an app.",
      Ctor::Arg("name"),
      props: &[
          text("name", "Name", Emit::None),
          text("version", "Version", Emit::Method("version")),
          text("tagline", "Tagline", Emit::Method("tagline")),
          text("credits", "Credits", Emit::Method("credits")),
      ],
  ),
  comp!(
      "windowcontrols", "Window controls", "WindowControls", Layout, "app-window-mac",
      "Close, minimise, and zoom, for a titlebar you drew yourself.",
      Ctor::Unit,
      props: &[
          float("button_width", "Button width", Emit::Method("button_width"),
              || PropValue::Float(13.0)),
          float("height", "Height", Emit::Method("height"), || PropValue::Float(28.0)),
      ],
  ),
];
