//! What the catalog knows about one component.
//!
//! Three consumers read this table and nothing else: the palette and inspector
//! (what you can place and what you can set), the renderer (what to build), and
//! the generator (what to print). Keeping the description declarative is what
//! stops those three from drifting — adding a prop is one line here, not three
//! edits in three crates.

use crate::node::{EventSpec, Node, DEFAULT_SLOT};
use crate::props::{PropSpec, PropValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
  Layout,
  Typography,
  Controls,
  Inputs,
  Data,
  Feedback,
  Navigation,
  Charts,
  Media,
  /// The `ai/` module: a transcript and the parts of one.
  Ai,
  /// Components you built in this project.
  Project,
}

impl Category {
  pub const ALL: &'static [Category] = &[
    Category::Layout,
    Category::Typography,
    Category::Controls,
    Category::Inputs,
    Category::Data,
    Category::Feedback,
    Category::Navigation,
    Category::Charts,
    Category::Media,
    Category::Ai,
    Category::Project,
  ];

  pub fn label(self) -> &'static str {
    match self {
      Category::Layout => "Layout",
      Category::Typography => "Typography",
      Category::Controls => "Controls",
      Category::Inputs => "Inputs",
      Category::Data => "Data",
      Category::Feedback => "Feedback",
      Category::Navigation => "Navigation",
      Category::Charts => "Charts",
      Category::Media => "Media",
      Category::Ai => "AI",
      Category::Project => "Project",
    }
  }

  /// A Lucide name for the section header in the palette.
  pub fn icon(self) -> &'static str {
    match self {
      Category::Layout => "layout-dashboard",
      Category::Typography => "type",
      Category::Controls => "mouse-pointer-click",
      Category::Inputs => "text-cursor-input",
      Category::Data => "table",
      Category::Feedback => "bell",
      Category::Navigation => "compass",
      Category::Charts => "chart-line",
      Category::Media => "image",
      Category::Ai => "sparkles",
      Category::Project => "package",
    }
  }
}

/// How the component's constructor is called.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Ctor {
  /// `Type::new()`
  Unit,
  /// `Type::new("node-3")` — components that need a stable element id.
  Id,
  /// `Type::new("node-3", <prop>)`
  IdAnd(&'static str),
  /// `Type::new(<prop>)`
  Arg(&'static str),
  /// `Type::new(<prop>, <prop>, ..)` — components whose constructor takes the
  /// two or three facts that make them mean anything (`AIMessage::new(role,
  /// body)`), rather than one plus a chain of setters.
  Args(&'static [&'static str]),
  /// A gpui entity: `cx.new(Type::new)`. The host gets a field for it, which
  /// is exactly how these components are used in a hand-written app.
  Entity,
  /// `cx.new(|cx| Type::new(cx, <prop>))`
  EntityArg(&'static str),
  /// An entity whose constructor does not take a context:
  /// `cx.new(|_| Type::new(<prop>))`.
  EntityValue(&'static str),
  /// Not one call — the renderer and the generator special-case it by kind.
  Special,
}

impl Ctor {
  pub fn is_entity(self) -> bool {
    matches!(
      self,
      Ctor::Entity | Ctor::EntityArg(_) | Ctor::EntityValue(_)
    )
  }
}

/// A region that holds children.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotSpec {
  pub key: &'static str,
  pub label: &'static str,
  /// Holds at most one child — a `Panel`'s icon, an `Indicator`'s target.
  pub single: bool,
  /// Printed as `.method(child)`. The default slot uses `.child(..)`.
  pub method: &'static str,
}

pub const CHILDREN: SlotSpec = SlotSpec {
  key: DEFAULT_SLOT,
  label: "Children",
  single: false,
  method: "child",
};

pub const fn slot(key: &'static str, label: &'static str, method: &'static str) -> SlotSpec {
  SlotSpec {
    key,
    label,
    single: true,
    method,
  }
}

/// Slots that come from a prop's item list — a tab per tab title.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DynamicSlots {
  /// The `Items` prop whose length decides how many slots there are.
  pub from_prop: &'static str,
  /// Slot keys are `<prefix>:<index>`.
  pub prefix: &'static str,
  /// The builder call each slot becomes — `.tab(label, ..)`, `.item(..)`.
  /// Empty when the component takes its regions some other way and the
  /// generator special-cases it (`SettingsView`, whose pages are declared
  /// apart from the one closure that fills them).
  pub method: &'static str,
}

/// A slot resolved against a specific node — dynamic slots have real labels
/// only once you know the node's items.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotRef {
  pub key: String,
  pub label: String,
  pub single: bool,
}

pub struct ComponentSpec {
  /// The catalog key, as it appears in the file format.
  pub kind: &'static str,
  /// The name shown in the palette and the layers tree.
  pub title: &'static str,
  /// The guise type the generator prints. Empty for `Ctor::Special` kinds
  /// that are not a single type (a frame is a `div`).
  pub rust: &'static str,
  pub category: Category,
  /// A Lucide name for the palette row.
  pub icon: &'static str,
  /// One line, shown under the name in the palette.
  pub blurb: &'static str,
  pub ctor: Ctor,
  pub props: &'static [PropSpec],
  pub slots: &'static [SlotSpec],
  pub dynamic: Option<DynamicSlots>,
  pub events: &'static [EventSpec],
  /// Applied when the component is first placed — the sensible starting size
  /// or padding that makes a fresh drop look like something.
  pub on_place: Option<fn(&mut Node)>,
  /// Extra `use` lines the generated file needs. The prelude covers almost
  /// everything; `flex/` is the exception, because its names deliberately
  /// overlap `layout/` and it is not glob-exported.
  pub imports: &'static [&'static str],
}

impl ComponentSpec {
  pub fn prop(&self, key: &str) -> Option<&PropSpec> {
    self.props.iter().find(|p| p.key == key)
  }

  /// The default for a prop, whether or not the node has set it.
  pub fn default_prop(&self, key: &str) -> Option<PropValue> {
    self.prop(key).map(|p| p.default_value())
  }

  pub fn is_container(&self) -> bool {
    !self.slots.is_empty() || self.dynamic.is_some()
  }

  /// Whether the default `children` slot exists — what a canvas drop asks.
  pub fn takes_children(&self) -> bool {
    self.slots.iter().any(|s| s.key == DEFAULT_SLOT)
  }

  pub fn slot_spec(&self, key: &str) -> Option<&SlotSpec> {
    self.slots.iter().find(|s| s.key == key)
  }

  /// Every slot this node has right now, static and dynamic together.
  pub fn slots_of(&self, node: &Node) -> Vec<SlotRef> {
    let mut out: Vec<SlotRef> = self
      .slots
      .iter()
      .map(|s| SlotRef {
        key: s.key.into(),
        label: s.label.into(),
        single: s.single,
      })
      .collect();
    if let Some(dynamic) = self.dynamic {
      let labels = node
        .prop(dynamic.from_prop)
        .and_then(|v| v.as_items().map(|i| i.to_vec()))
        .or_else(|| {
          self
            .default_prop(dynamic.from_prop)
            .and_then(|v| v.as_items().map(|i| i.to_vec()))
        })
        .unwrap_or_default();
      for (index, label) in labels.iter().enumerate() {
        out.push(SlotRef {
          key: format!("{}:{index}", dynamic.prefix),
          label: label.clone(),
          single: false,
        });
      }
    }
    out
  }

  /// A node of this kind, with the catalog's placement defaults applied.
  pub fn build(&self, id: crate::id::NodeId) -> Node {
    let mut node = Node::new(id, self.kind);
    if let Some(on_place) = self.on_place {
      on_place(&mut node);
    }
    node
  }
}

impl std::fmt::Debug for ComponentSpec {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("ComponentSpec")
      .field("kind", &self.kind)
      .finish()
  }
}
