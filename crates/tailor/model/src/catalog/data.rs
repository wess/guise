//! Lists, tables, and the containers that show one region at a time.
//!
//! `TableView` and `DataView` are deliberately absent: both are generic over a
//! row type the host owns, so there is nothing for a visual builder to fill in.
//! `Table` covers the static case, and a generated screen can hold either.

use crate::props::{boolean, color_name, float, int, items, size, text, variant, Emit, PropValue};
use crate::tokens::{ColorToken, SizeToken, VariantToken};

use super::spec::{ComponentSpec, Ctor, DynamicSlots, CHILDREN};

fn three() -> PropValue {
  PropValue::Items(vec!["First".into(), "Second".into(), "Third".into()])
}

pub static SPECS: &[ComponentSpec] = &[
  comp!(
      "avatar", "Avatar", "Avatar", Data, "circle-user",
      "Initials on a themed disc.",
      Ctor::Arg("initials"),
      props: &[
          text("initials", "Initials", Emit::None),
          color_name("color", "Color", Emit::Method("color"), ColorToken::Blue),
          variant("variant", "Variant", Emit::Method("variant"), VariantToken::Light),
          size("size", "Size", Emit::Method("size"), SizeToken::Md),
          size("radius", "Radius", Emit::Method("radius"), SizeToken::Xl),
      ],
  ),
  comp!(
      "avatargroup", "Avatar group", "AvatarGroup", Data, "users",
      "Overlapping avatars with an overflow count.",
      Ctor::Unit,
      props: &[
          items("avatars", "Initials", Emit::Method("avatars"), || {
              PropValue::Items(vec!["AB".into(), "CD".into(), "EF".into()])
          }),
          size("size", "Size", Emit::Method("size"), SizeToken::Md),
          int("limit", "Limit", Emit::Method("limit"), || PropValue::Int(3)),
      ],
  ),
  comp!(
      "list", "List", "List", Data, "list",
      "A bulleted or numbered list.",
      Ctor::Unit,
      props: &[
          items("items", "Items", Emit::Method("items"), three),
          boolean("ordered", "Numbered", Emit::Method("ordered"), false),
          size("size", "Size", Emit::Method("size"), SizeToken::Md),
          size("spacing", "Spacing", Emit::Method("spacing"), SizeToken::Xs),
          crate::props::icon("icon", "Bullet icon", Emit::Method("icon")),
      ],
  ),
  comp!(
      "table", "Table", "Table", Data, "table",
      "A static table. Rows are pipe-separated cells.",
      Ctor::Unit,
      props: &[
          items("head", "Header", Emit::None, || {
              PropValue::Items(vec!["Name".into(), "Role".into(), "Status".into()])
          }),
          crate::props::hinted(
              items("rows", "Rows", Emit::None, || {
                  PropValue::Items(vec![
                      "Ada | Engineer | Active".into(),
                      "Grace | Admiral | Active".into(),
                  ])
              }),
              "one row per line, cells separated by |",
          ),
          boolean("striped", "Striped", Emit::Method("striped"), false),
          boolean("highlight_on_hover", "Hover highlight", Emit::Method("highlight_on_hover"), false),
          boolean("with_border", "Border", Emit::Method("with_border"), true),
      ],
  ),
  comp!(
      "tabs", "Tabs", "Tabs", Data, "folder",
      "One panel at a time, with a tab per panel.",
      Ctor::Entity,
      props: &[
          items("tabs", "Tabs", Emit::Custom, three),
          int("active", "Active", Emit::Method("active"), || PropValue::Int(0)),
      ],
      dynamic: Some(DynamicSlots { from_prop: "tabs", prefix: "tab", method: "tab" }),
  ),
  comp!(
      "accordion", "Accordion", "Accordion", Data, "chevrons-up-down",
      "Collapsible sections.",
      Ctor::Entity,
      props: &[
          items("items", "Sections", Emit::Custom, three),
          boolean("multiple", "Allow multiple", Emit::Method("multiple"), false),
          int("default_open", "Open by default", Emit::Method("default_open"), || PropValue::Int(0)),
      ],
      dynamic: Some(DynamicSlots { from_prop: "items", prefix: "item", method: "item" }),
  ),
  comp!(
      "tabbar", "Tab bar", "TabBar", Data, "app-window",
      "An editor-style tab strip.",
      Ctor::Entity,
      props: &[
          items("tabs", "Tabs", Emit::Method("tabs"), three),
          int("active", "Active", Emit::Method("active"), || PropValue::Int(0)),
          boolean("with_add_button", "Add button", Emit::Method("with_add_button"), false),
      ],
  ),
  comp!(
      "timeline", "Timeline", "Timeline", Data, "git-commit-horizontal",
      "A vertical sequence of events.",
      Ctor::Unit,
      props: &[
          crate::props::hinted(
              items("items", "Items", Emit::Custom, three),
              "title, or title | description",
          ),
          int("active", "Active", Emit::Method("active"), || PropValue::Int(0)),
          color_name("color", "Color", Emit::Method("color"), ColorToken::Blue),
      ],
  ),
  comp!(
      "treeview", "Tree", "TreeView", Data, "folder-tree",
      "A collapsible tree. Indent with two spaces per level.",
      Ctor::Entity,
      props: &[
          crate::props::hinted(
              items("nodes", "Nodes", Emit::Custom, || {
                  PropValue::Items(vec![
                      "src".into(),
                      "  main.rs".into(),
                      "  lib.rs".into(),
                      "Cargo.toml".into(),
                  ])
              }),
              "two spaces of indent per level",
          ),
          float("height", "Height", Emit::Method("height"), || PropValue::Float(240.0)),
          boolean("default_expanded", "Expanded", Emit::Method("default_expanded"), true),
      ],
  ),
  comp!(
      "carousel", "Carousel", "Carousel", Data, "gallery-horizontal",
      "One slide at a time, with arrows and dots.",
      Ctor::Entity,
      props: &[
          float("height", "Height", Emit::None, || PropValue::Float(220.0)),
      ],
      slots: &[CHILDREN],
  ),
  comp!(
      "virtuallist", "Virtual list", "VirtualList", Data, "list-ordered",
      "Thousands of equal-height rows, only the visible ones built.",
      // Drawn: the row builder is a `'static` closure over an index. The
      // generator prints the real component over the rows you list here.
      Ctor::Special,
      props: &[
          items("rows", "Rows", Emit::Custom, || PropValue::Items(
              vec!["Row one".into(), "Row two".into(), "Row three".into()])),
          float("height", "Viewport height", Emit::Method("height"), || PropValue::Float(240.0)),
      ],
  ),
];
