//! One node as a Rust expression.
//!
//! The generic path is the catalog: a constructor, a chained call per set prop,
//! a `.child(..)` per slot child, a handler per bound event. The `special`
//! function below is the escape hatch for the components whose shape is not one
//! chained call — a `div` with no type of its own, a chart that pairs its
//! numbers, a container whose regions take closures.

use std::collections::{BTreeMap, BTreeSet};

use tailor_model::catalog::{self, Ctor};
use tailor_model::motion::Resolved as ResolvedMotion;
use tailor_model::props::{Emit, PropValue};
use tailor_model::style::{LayoutMode, StyleProps};
use tailor_model::tokens::{EnterToken, LoopToken};
use tailor_model::{Document, Flavor, Node, NodeId, Project};

use crate::expr::{self, Hoist};
use crate::rust::{float, indent, string};
use crate::style::{self, Placement};

/// Whether the generated file has a `self` to hang handlers off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Owner {
  /// A `Render` entity: events go through `cx.listener` into a method.
  Entity,
  /// A `RenderOnce` builder: no `self` at render time, so handlers are stubs.
  Plain,
}

/// The entity components guise gives a two-way `X::bind(&entity, &signal, cx)`,
/// and the prop that binding drives. Binding any *other* prop on these is a
/// one-shot read of the signal at construction, which is what the fallback in
/// `prop_calls` emits.
///
/// The controlled builders (`Checkbox`, `Switch`, `Radio`, `Chip`, `Rating`)
/// take `.bind(signal.binding())` in the builder chain instead, and are handled
/// where their props are emitted — they have no `new` to bind after.
const ENTITY_BINDS: &[(&str, &str)] = &[
  ("textinput", "value"),
  ("textarea", "value"),
  ("passwordinput", "value"),
  ("numberinput", "value"),
  ("pininput", "value"),
  ("colorinput", "value"),
  ("tagsinput", "tags"),
  ("autocomplete", "value"),
  ("markdowneditor", "value"),
  ("select", "selected"),
  ("segmented", "selected"),
  ("slider", "value"),
  ("rangeslider", "value"),
];

/// The `.animate(..)` call that plays a node's entrance.
///
/// It lands on the node's own box — `Motioned::animate` puts the sampled
/// values straight onto the element's style, so the generated tree has
/// exactly the same shape whether a node animates or not.
fn animate_calls(
  element_id: &str,
  motion: ResolvedMotion,
  pinned: bool,
  flavor: Flavor,
) -> Vec<String> {
  let clip = match flavor {
    Flavor::Plain => motion_builder(motion, pinned),
    Flavor::Macros => motion_block(motion, pinned),
  };

  let mut out = vec![".animate(".to_string()];
  out.push(format!("    {},", string(element_id)));
  out.extend(indent(&clip).iter().enumerate().map(|(i, line)| {
    if i + 1 == clip.len() {
      format!("{line},")
    } else {
      line.clone()
    }
  }));
  out.push(")".into());
  out
}

/// The motion as a chained builder.
fn motion_builder(motion: ResolvedMotion, pinned: bool) -> Vec<String> {
  let mut clip = if motion.enter == EnterToken::Fade {
    // Distance means nothing to a fade, and printing it invites the
    // reader to wonder what it does.
    vec![format!("Motion::enter({})", motion.enter.path())]
  } else {
    vec![format!(
      "Motion::enter_from({}, {})",
      motion.enter.path(),
      float(motion.distance)
    )]
  };
  clip.push(format!("    .duration({})", float(motion.duration)));
  if motion.delay > 0.0 {
    clip.push(format!("    .delay({})", float(motion.delay)));
  }
  clip.push(format!("    .ease({})", motion.ease.path()));
  if motion.repeat == LoopToken::Forever {
    clip.push("    .repeat_forever()".into());
  }
  if motion.alternate {
    clip.push("    .alternate(true)".into());
  }
  if pinned {
    // A pinned node *is* its inset, so the offsets move to margins or the
    // animation would drag it off its pin.
    clip.push("    .as_margins()".into());
  }
  clip
}

/// The same motion as a `motion!` block — the macro flavour's half, matching
/// what `style!` does for the box.
fn motion_block(motion: ResolvedMotion, pinned: bool) -> Vec<String> {
  let mut clip = vec!["motion! {".to_string()];
  if motion.enter == EnterToken::Fade {
    clip.push(format!("    enter: {};", motion.enter.word()));
  } else {
    clip.push(format!(
      "    enter: {} {};",
      motion.enter.word(),
      float(motion.distance)
    ));
  }
  clip.push(format!("    duration: {};", float(motion.duration)));
  if motion.delay > 0.0 {
    clip.push(format!("    delay: {};", float(motion.delay)));
  }
  clip.push(format!("    ease: {};", motion.ease.words()));
  if motion.repeat == LoopToken::Forever {
    clip.push("    repeat: forever;".into());
  }
  if motion.alternate {
    clip.push("    alternate;".into());
  }
  if pinned {
    clip.push("    margins;".into());
  }
  clip.push("}".into());
  clip
}

/// The line a node's expression is tagged with while the file is being built,
/// so the finished text can be mapped back to the design. Stripped before
/// anything is written — see [`crate::file::document`].
pub const MARK: &str = "//__tailor:";

/// What a region closure calls its weak handle back to the view. `cx.listener`
/// borrows the context, and a region closure is `'static`, so a handler inside
/// one goes through this instead.
const VIEW: &str = "view";

/// The prop `X::bind` drives for this kind, if guise has one.
fn entity_bind_prop(kind: &str) -> Option<&'static str> {
  ENTITY_BINDS
    .iter()
    .find(|(k, _)| *k == kind)
    .map(|(_, prop)| *prop)
}

/// The controlled builders, which take `.bind(signal.binding())` in the chain.
/// They hold no state of their own, so there is no entity to bind afterwards —
/// the binding is a setter like any other, and it is what makes them two-way.
/// Without it a bound checkbox reads its signal and never writes back, which
/// looks like a binding right up until you click it.
const CONTROLLED_BINDS: &[(&str, &str)] = &[
  ("checkbox", "checked"),
  ("switch", "checked"),
  ("chip", "checked"),
  ("rating", "value"),
];

fn controlled_bind_prop(kind: &str) -> Option<&'static str> {
  CONTROLLED_BINDS
    .iter()
    .find(|(k, _)| *k == kind)
    .map(|(_, prop)| *prop)
}

/// Where the expression being built will sit. It decides how an entity-backed
/// node refers to itself: a field in `render`, a local in `new`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
  Render,
  Init,
}

pub struct Emitter<'a> {
  pub project: &'a Project,
  pub doc: &'a Document,
  pub flavor: Flavor,
  /// Entity-backed nodes and the struct field each one lives in.
  pub fields: &'a BTreeMap<NodeId, String>,
  pub hoist: &'a mut Hoist,
  pub owner: Owner,
  pub imports: BTreeSet<String>,
  /// Things worth telling the user about the file that came out.
  pub notes: Vec<String>,
  /// `X::bind(&field, &signal, cx);` lines, emitted after the entity and the
  /// signal are both locals in `new`.
  pub binds: Vec<String>,
  phase: Phase,
  /// One frame per open closure. A closure is `'static`, so anything it
  /// touches has to be cloned in ahead of it; this records what to clone.
  captures: Vec<BTreeSet<String>>,
}

impl<'a> Emitter<'a> {
  pub fn new(
    project: &'a Project,
    doc: &'a Document,
    fields: &'a BTreeMap<NodeId, String>,
    hoist: &'a mut Hoist,
    owner: Owner,
  ) -> Self {
    Emitter {
      project,
      doc,
      flavor: project.gen.flavor,
      fields,
      hoist,
      owner,
      imports: BTreeSet::new(),
      notes: Vec::new(),
      binds: Vec::new(),
      phase: Phase::Render,
      captures: Vec::new(),
    }
  }

  /// Build the `let <field> = cx.new(..)` line for an entity-backed node.
  /// Post-order over the document means the children it captures are already
  /// locals by the time this runs.
  pub fn emit_init(&mut self, id: NodeId) -> Vec<String> {
    let Some(node) = self.doc.node(id) else {
      return Vec::new();
    };
    let Some(field) = self.fields.get(&id).cloned() else {
      return Vec::new();
    };
    let Some(spec) = catalog::get(&node.kind) else {
      return Vec::new();
    };

    let previous = self.phase;
    self.phase = Phase::Init;
    self.captures.push(BTreeSet::new());

    let (ctor, takes_cx) = match spec.ctor {
      Ctor::EntityArg(key) => {
        let value = self.prop_value(node, key);
        let prop = spec
          .prop(key)
          .expect("catalog checks constructor props exist");
        let arg = expr::value(self.hoist, prop, &value, self.doc);
        (format!("{}::new(cx, {arg})", spec.rust), true)
      }
      Ctor::EntityValue(key) => {
        let value = self.prop_value(node, key);
        let prop = spec
          .prop(key)
          .expect("catalog checks constructor props exist");
        let arg = expr::value(self.hoist, prop, &value, self.doc);
        (format!("{}::new({arg})", spec.rust), false)
      }
      _ => (format!("{}::new(cx)", spec.rust), true),
    };
    let param = if takes_cx { "cx" } else { "_cx" };
    let mut body = vec![ctor];
    body.extend(indent(&self.prop_calls(node)));
    body.extend(indent(&self.slot_calls(node)));

    // Two-way binding is a call after construction, not a setter: the
    // entity and the signal both have to exist first.
    if let Some(key) = entity_bind_prop(&node.kind) {
      if let Some(var) = node.prop(key).and_then(|value| value.as_binding()) {
        self.binds.push(format!(
          "{}::bind(&{field}, &{}, cx);",
          spec.rust,
          tailor_model::snake_case(var)
        ));
      }
    }

    let captured = self.captures.pop().unwrap_or_default();
    self.phase = previous;

    let mut out = Vec::new();
    if captured.is_empty() {
      out.push(format!("let {field} = cx.new(|{param}| {{"));
      out.extend(indent(&body));
      out.push("});".into());
    } else {
      out.push(format!("let {field} = {{"));
      for name in &captured {
        out.push(format!("    let {name} = {name}.clone();"));
      }
      out.push(format!("    cx.new(move |{param}| {{"));
      out.extend(indent(&indent(&body)));
      out.push("    })".into());
      out.push("};".into());
    }
    out
  }

  /// Subscriptions for entity-backed nodes with a bound event. guise entities
  /// emit rather than take a handler, so this is where their events land.
  pub fn emit_subscriptions(&self) -> Vec<String> {
    let mut out = Vec::new();
    for (id, field) in self.fields {
      let Some(node) = self.doc.node(*id) else {
        continue;
      };
      for action in node.events.values() {
        if action.is_empty() {
          continue;
        }
        let method = tailor_model::snake_case(action);
        out.push(format!(
          "cx.subscribe(&{field}, |this, _entity, _event, cx| this.{method}(cx)).detach();"
        ));
      }
    }
    out
  }

  /// The expression for a node, as lines. Line 0 is the constructor; the rest
  /// are chained calls, already indented one level relative to it.
  pub fn emit(&mut self, id: NodeId, placement: Placement) -> Vec<String> {
    let mut lines = self.emit_inner(id, placement);
    // Tag the expression with the node it came from. The tag is a comment
    // on its own line, so removing it later cannot disturb anything around
    // it — and removing it is how the line number is learned.
    lines.insert(0, format!("{MARK}{}", id.0));
    lines
  }

  fn emit_inner(&mut self, id: NodeId, placement: Placement) -> Vec<String> {
    let Some(node) = self.doc.node(id) else {
      return vec!["div()".into()];
    };
    if node.hidden {
      // Hidden is a canvas affordance, not a runtime condition — but a
      // node you have hidden is one you are not ready to ship, so it
      // leaves a marker rather than silently appearing in the output.
      let mut lines = vec!["div()".into()];
      lines.push(format!(
        "    // hidden in the designer: {}",
        self.label(node)
      ));
      return lines;
    }

    let mut lines = self.base(node);
    let container = matches!(
      node.kind.as_str(),
      "frame" | "canvas" | "surface" | "spacer"
    );
    // The animation goes on the box the style system already emits, not
    // on a wrapper of its own: a wrapper is a new flex item, and a child
    // that was `w_full` would start measuring against it instead of the
    // row it was in. So a motion is one more reason to have a box.
    let motion = self.doc.motion_of(node.id);
    // A pinned child needs a box to pin, even if nothing else styles it.
    let styled = container || node.style.needs_wrapper() || placement.absolute || motion.is_some();
    // An entity was configured when it was built; here it is only a handle.
    let configured_elsewhere = self.fields.contains_key(&node.id);

    if container {
      lines.extend(indent(&style::calls(
        &node.style,
        placement,
        true,
        self.flavor,
        self.hoist,
      )));
    }
    if !configured_elsewhere {
      lines.extend(indent(&self.prop_calls(node)));
      lines.extend(indent(&self.slot_calls(node)));
      lines.extend(indent(&self.event_calls(node)));
    }

    let mut lines = if styled && !container {
      self.wrap(lines, &node.style, placement)
    } else {
      lines
    };
    if let Some(motion) = motion {
      lines.extend(indent(&animate_calls(
        &node.id.element_id(),
        motion,
        placement.absolute,
        self.flavor,
      )));
    }
    lines
  }

  /// Put a styled `div` around a component that has box styling of its own.
  fn wrap(&mut self, inner: Vec<String>, style: &StyleProps, placement: Placement) -> Vec<String> {
    let mut lines = vec!["div()".into()];
    lines.extend(indent(&style::calls(
      style,
      placement,
      false,
      self.flavor,
      self.hoist,
    )));
    lines.extend(indent(&child_call(inner)));
    lines
  }

  fn label(&self, node: &Node) -> String {
    node
      .name
      .clone()
      .or_else(|| catalog::get(&node.kind).map(|spec| spec.title.to_string()))
      .unwrap_or_else(|| node.kind.clone())
  }

  /// The constructor line(s).
  fn base(&mut self, node: &Node) -> Vec<String> {
    if let Some(name) = node.component_ref() {
      let ty = tailor_model::pascal_case(name);
      // Generated documents are siblings in one module, so the placed
      // component is one `use super::` away.
      self.imports.insert(format!("use super::{ty};"));
      return vec![format!("{ty}::new()")];
    }
    // An entity lives in a field. How it names itself depends on where the
    // expression sits: a field in `render`, a local in `new`, and a cloned
    // local anywhere inside a closure.
    if let Some(field) = self.fields.get(&node.id).cloned() {
      if let Some(scope) = self.captures.last_mut() {
        scope.insert(field.clone());
        return vec![format!("{field}.clone()")];
      }
      return match self.phase {
        Phase::Render => vec![format!("self.{field}.clone()")],
        Phase::Init => vec![format!("{field}.clone()")],
      };
    }
    let Some(spec) = catalog::get(&node.kind) else {
      return vec!["div()".into()];
    };
    for import in spec.imports {
      self.imports.insert((*import).to_string());
    }
    if let Some(lines) = self.special(node) {
      return lines;
    }
    let arg = |emitter: &mut Self, key: &str| -> String {
      let value = emitter.prop_value(node, key);
      let spec = spec
        .prop(key)
        .expect("catalog checks constructor props exist");
      expr::value(emitter.hoist, spec, &value, emitter.doc)
    };
    match spec.ctor {
      Ctor::Unit => vec![format!("{}::new()", spec.rust)],
      Ctor::Id => vec![format!(
        "{}::new({})",
        spec.rust,
        string(&node.id.element_id())
      )],
      Ctor::IdAnd(key) => {
        let value = arg(self, key);
        vec![format!(
          "{}::new({}, {value})",
          spec.rust,
          string(&node.id.element_id())
        )]
      }
      Ctor::Arg(key) => {
        let value = arg(self, key);
        vec![format!("{}::new({value})", spec.rust)]
      }
      Ctor::Args(keys) => {
        let values: Vec<String> = keys.iter().map(|key| arg(self, key)).collect();
        vec![format!("{}::new({})", spec.rust, values.join(", "))]
      }
      // Entities are constructed in `new()`, never here.
      Ctor::Entity | Ctor::EntityArg(_) | Ctor::EntityValue(_) => {
        vec![format!("{}::new(cx)", spec.rust)]
      }
      Ctor::Special => vec!["div()".into()],
    }
  }

  /// The value of a prop, falling back to the catalog default.
  fn prop_value(&self, node: &Node, key: &str) -> PropValue {
    node
      .prop(key)
      .cloned()
      .or_else(|| catalog::get(&node.kind).and_then(|spec| spec.default_prop(key)))
      .unwrap_or(PropValue::Text(String::new()))
  }

  fn prop_calls(&mut self, node: &Node) -> Vec<String> {
    let Some(spec) = catalog::get(&node.kind) else {
      return Vec::new();
    };
    let mut out = Vec::new();
    for prop in spec.props {
      let Some(value) = node.prop(prop.key) else {
        continue;
      };
      // A binding is never "the default" — it is a live read.
      let bound = value.as_binding().is_some();
      // The prop a two-way `X::bind` drives is set by that call, not by a
      // setter here; emitting both would fight over the same value.
      if bound && entity_bind_prop(&node.kind) == Some(prop.key) {
        continue;
      }
      // In `new` there is no `self` yet — a state variable is still a
      // local.
      let prefix = match self.phase {
        Phase::Render => "self.",
        Phase::Init => "",
      };
      // A controlled builder binds in the chain, and that binding replaces
      // the setter it drives.
      if bound && controlled_bind_prop(&node.kind) == Some(prop.key) {
        if let Some(var) = value.as_binding() {
          out.push(format!(
            ".bind({prefix}{}.binding())",
            tailor_model::snake_case(var)
          ));
          continue;
        }
      }
      match prop.emit {
        Emit::Method(method) => {
          if bound || (!expr::is_default(prop, value) && !value.is_empty()) {
            let arg = expr::value_with(self.hoist, prop, value, self.doc, prefix);
            out.push(format!(".{method}({arg})"));
          }
        }
        Emit::Flag(method) => {
          if value.as_bool() == Some(true) {
            out.push(format!(".{method}()"));
          }
        }
        Emit::Custom | Emit::None => {}
      }
    }
    out.extend(self.custom_prop_calls(node));
    out
  }

  /// The props the catalog marks `Custom` — lists that become one call per
  /// item rather than one call with a list.
  fn custom_prop_calls(&mut self, node: &Node) -> Vec<String> {
    let mut out = Vec::new();
    match node.kind.as_str() {
      "table" => {
        if let Some(head) = self.items(node, "head") {
          if !head.is_empty() {
            out.push(format!(".head({})", expr::items(&head)));
          }
        }
        for row in self.items(node, "rows").unwrap_or_default() {
          let cells: Vec<String> = row.split('|').map(|cell| cell.trim().to_string()).collect();
          out.push(format!(".row({})", expr::items(&cells)));
        }
      }
      "timeline" => {
        for item in self.items(node, "items").unwrap_or_default() {
          match item.split_once('|') {
            Some((title, description)) => out.push(format!(
              ".item_desc({}, {})",
              string(title.trim()),
              string(description.trim())
            )),
            None => out.push(format!(".item({})", string(item.trim()))),
          }
        }
      }
      "stepper" => {
        for item in self.items(node, "steps").unwrap_or_default() {
          match item.split_once('|') {
            Some((label, description)) => out.push(format!(
              ".step_desc({}, {})",
              string(label.trim()),
              string(description.trim())
            )),
            None => out.push(format!(".step({})", string(item.trim()))),
          }
        }
      }
      "navigationmenu" => {
        for item in self.items(node, "items").unwrap_or_default() {
          let (id, label) = item.split_once(':').unwrap_or((&item, &item));
          out.push(format!(
            ".item({}, {})",
            string(id.trim()),
            string(label.trim())
          ));
        }
      }
      "menu" | "contextmenu" => {
        for line in self.items(node, "items").unwrap_or_default() {
          let line = line.trim();
          if line == "-" {
            out.push(".divider()".into());
          } else if let Some(label) = line.strip_prefix('#') {
            out.push(format!(".section({})", string(label.trim())));
          } else {
            // The handler is the caller's to fill in; an empty one keeps the
            // export compiling and says plainly where the work goes.
            out.push(format!(".item({}, |_window, _cx| {{}})", string(line)));
          }
        }
      }
      "aichatview" => {
        // One turn per line, alternating the way a transcript reads.
        let turns: Vec<String> = self
          .items(node, "turns")
          .unwrap_or_default()
          .iter()
          .enumerate()
          .map(|(index, body)| {
            let role = if index % 2 == 0 { "User" } else { "Assistant" };
            format!("AITurn::new(AIRole::{role}, {})", string(body))
          })
          .collect();
        if !turns.is_empty() {
          out.push(format!(".turns([{}])", turns.join(", ")));
        }
      }
      "aimodelpicker" => {
        let models: Vec<String> = self
          .items(node, "models")
          .unwrap_or_default()
          .iter()
          .map(|label| {
            format!(
              "AIModel::new({}, {})",
              string(&tailor_model::snake_case(label)),
              string(label)
            )
          })
          .collect();
        if !models.is_empty() {
          out.push(format!(".models([{}])", models.join(", ")));
        }
      }
      "treeview" => {
        let nodes = tree_nodes(&self.items(node, "nodes").unwrap_or_default());
        if !nodes.is_empty() {
          out.push(format!(".nodes(vec![{}])", nodes.join(", ")));
        }
      }
      _ => {}
    }
    out
  }

  fn items(&self, node: &Node, key: &str) -> Option<Vec<String>> {
    match self.prop_value(node, key) {
      PropValue::Items(values) => Some(values),
      _ => None,
    }
  }

  fn numbers(&self, node: &Node, key: &str) -> Vec<f64> {
    match self.prop_value(node, key) {
      PropValue::Numbers(values) => values,
      _ => Vec::new(),
    }
  }

  fn slot_calls(&mut self, node: &Node) -> Vec<String> {
    let Some(spec) = catalog::get(&node.kind) else {
      return Vec::new();
    };
    let mut out = Vec::new();
    let absolute = node.style.layout == LayoutMode::Absolute;
    let placement = Placement { absolute };

    // The regions that take a closure rather than an element.
    let deferred: &[&str] = match node.kind.as_str() {
      "appshell" => &["header", "navbar", "aside", "footer"],
      "splitpanel" => &["first", "second"],
      _ => &[],
    };

    for slot in spec.slots {
      let children = node.slot(slot.key);
      if children.is_empty() {
        continue;
      }
      if deferred.contains(&slot.key) {
        let head = match self.region_size(node, slot.key) {
          Some(value) => format!(".{}({}, ", slot.method, float(value)),
          None => format!(".{}(", slot.method),
        };
        out.extend(self.closure_region(head, children.first().copied()));
        continue;
      }
      for child in children {
        let inner = self.emit(*child, placement);
        out.extend(prefixed_call(slot.method, inner));
        if slot.single {
          break;
        }
      }
    }

    if node.kind == "settingsview" {
      out.extend(self.settings_pages(node, placement));
      return out;
    }

    if let Some(dynamic) = spec.dynamic {
      let labels = self.items(node, dynamic.from_prop).unwrap_or_default();
      let method = dynamic.method;
      for (index, label) in labels.iter().enumerate() {
        let key = format!("{}:{index}", dynamic.prefix);
        let children = node.slot(&key).to_vec();
        let head = format!(".{method}({}, ", string(label));
        if children.len() == 1 {
          out.extend(self.closure_region(head, children.first().copied()));
        } else {
          self.captures.push(BTreeSet::new());
          let mut wrapper = vec!["div().flex().flex_col().gap(px(8.))".into()];
          for child in &children {
            let child_lines = self.emit(*child, placement);
            wrapper.extend(indent(&child_call(child_lines)));
          }
          let captured = self.captures.pop().unwrap_or_default();
          out.extend(self.wrap_closure(head, captured, wrapper));
        }
      }
    }
    out
  }

  /// `SettingsView` declares its pages and then fills them from one closure
  /// keyed by page id — not one closure per region like `Tabs`. So the pages
  /// are printed first and the slots become the arms of a `match`.
  fn settings_pages(&mut self, node: &Node, placement: Placement) -> Vec<String> {
    let labels = self.items(node, "pages").unwrap_or_default();
    if labels.is_empty() {
      return Vec::new();
    }
    let ids: Vec<String> = labels
      .iter()
      .map(|label| tailor_model::snake_case(label))
      .collect();

    let mut out = Vec::new();
    for (id, label) in ids.iter().zip(&labels) {
      out.push(format!(".page({}, {})", string(id), string(label)));
    }

    self.captures.push(BTreeSet::new());
    let mut body = vec!["match page {".into()];
    for (index, id) in ids.iter().enumerate() {
      let children = node.slot(&format!("page:{index}")).to_vec();
      let mut region = vec!["div().flex().flex_col().gap(px(12.))".into()];
      for child in &children {
        let child_lines = self.emit(*child, placement);
        region.extend(indent(&child_call(child_lines)));
      }
      // The last arm is the catch-all: `match` on a `&str` needs one, and an
      // unreachable arm would be worse than reusing the final page.
      let head = if index + 1 == ids.len() {
        "    _ => ".to_string()
      } else {
        format!("    {} => ", string(id))
      };
      let mut arm = indent(&indent(&region));
      arm[0] = format!("{head}{}", arm[0].trim_start());
      let last = arm.len() - 1;
      arm[last] = format!("{}.into_any_element(),", arm[last]);
      body.extend(arm);
    }
    body.push("}".into());
    let captured = self.captures.pop().unwrap_or_default();
    out.extend(self.wrap_closure_with(
      ".content(".into(),
      "|page, _query, _window, _cx|",
      captured,
      body,
    ));
    out
  }

  /// AppShell's regions take their size before their content.
  fn region_size(&self, node: &Node, slot: &str) -> Option<f32> {
    let key = match slot {
      "header" => "header_height",
      "navbar" => "navbar_width",
      "aside" => "aside_width",
      "footer" => "footer_height",
      _ => return None,
    };
    self.prop_value(node, key).as_f64().map(|v| v as f32)
  }

  /// Emit a region whose content is a `'static` closure, cloning in whatever
  /// the closure reaches for. `head` is everything before the closure —
  /// `.header(56., ` or `.tab("Overview", `.
  fn closure_region(&mut self, head: String, id: Option<NodeId>) -> Vec<String> {
    self.captures.push(BTreeSet::new());
    let inner = match id {
      Some(id) => self.emit(id, Placement { absolute: false }),
      None => vec!["div()".into()],
    };
    let captured = self.captures.pop().unwrap_or_default();
    self.wrap_closure(head, captured, inner)
  }

  fn wrap_closure(
    &mut self,
    head: String,
    captured: BTreeSet<String>,
    inner: Vec<String>,
  ) -> Vec<String> {
    self.wrap_closure_with(head, "|_window, _cx|", captured, inner)
  }

  /// The same, for a region whose closure takes more than a window and a
  /// context — `SettingsView::content` is handed the active page and the
  /// search query first.
  fn wrap_closure_with(
    &mut self,
    head: String,
    params: &str,
    captured: BTreeSet<String>,
    inner: Vec<String>,
  ) -> Vec<String> {
    let mut out = Vec::new();
    if captured.is_empty() {
      out.push(format!("{head}{params} {{"));
      out.extend(indent(&inner));
      out.push("})".into());
      return out;
    }
    out.push(format!("{head}{{"));
    for name in &captured {
      let source = if name == VIEW {
        // Weak, not strong: a closure held by a live component tree
        // must not own the view that renders it.
        "cx.entity().downgrade()".to_string()
      } else {
        match self.phase {
          Phase::Render => format!("self.{name}.clone()"),
          Phase::Init => format!("{name}.clone()"),
        }
      };
      out.push(format!("    let {name} = {source};"));
    }
    out.push(format!("    move {params} {{"));
    out.extend(indent(&indent(&inner)));
    out.push("    }".into());
    out.push("})".into());
    out
  }

  fn event_calls(&mut self, node: &Node) -> Vec<String> {
    let Some(spec) = catalog::get(&node.kind) else {
      return Vec::new();
    };
    // Entity components emit rather than take a handler; `emit_subscriptions`
    // wires those in `new`.
    if spec.ctor.is_entity() {
      return Vec::new();
    }
    let mut out = Vec::new();
    for event in spec.events {
      let Some(action) = node.events.get(event.key) else {
        continue;
      };
      if action.is_empty() {
        continue;
      }
      let method = tailor_model::snake_case(action);
      let args: Vec<String> = event
        .args
        .iter()
        .map(|arg| format!("_{}", arg.trim_start_matches('_')))
        .collect();
      let params = if args.is_empty() {
        String::new()
      } else {
        format!("{}, ", args.join(", "))
      };
      match self.owner {
        // Inside a region closure there is no `cx` to listen with: the
        // closure is `'static` and outlives the borrow. A weak handle
        // is cloned in ahead of it and upgraded when the event fires —
        // the same shape a hand-written host uses.
        Owner::Entity if !self.captures.is_empty() => {
          if let Some(scope) = self.captures.last_mut() {
            scope.insert(VIEW.to_string());
          }
          // Cloned again per handler: the region closure is `Fn`, so
          // a `move` handler inside it would move the shared handle
          // out of the closure that owns it.
          out.push(format!(".{}({{", event.method));
          out.push(format!("    let {VIEW} = {VIEW}.clone();"));
          out.push(format!("    move |{params}_window, cx| {{"));
          out.push(format!(
            "        {VIEW}.update(cx, |this, cx| this.{method}(cx)).ok();"
          ));
          out.push("    }".into());
          out.push("})".into());
        }
        Owner::Entity => out.push(format!(
          ".{}(cx.listener(|this, {params}_window, cx| this.{method}(cx)))",
          event.method
        )),
        Owner::Plain => out.push(format!(
          ".{}(|{params}_window, _cx| {{ /* {action} */ }})",
          event.method
        )),
      }
    }
    out
  }

  /// The components whose expression is not `Type::new(..)`.
  fn special(&mut self, node: &Node) -> Option<Vec<String>> {
    let lines = match node.kind.as_str() {
      "frame" | "canvas" => vec!["div()".into()],
      "surface" => {
        let mut lines = vec!["div()".into()];
        if let PropValue::Color(color) = self.prop_value(node, "fill") {
          let local = expr::hsla(self.hoist, &color);
          lines.push(format!("    .bg({local})"));
        }
        lines
      }
      "spacer" => vec!["div()".into(), "    .flex_grow()".into()],
      "space" => {
        let axis = self.prop_value(node, "axis");
        let size = self
          .prop_value(node, "size")
          .as_size()
          .unwrap_or(tailor_model::SizeToken::Md);
        let method = if axis.as_str() == Some("x") { "x" } else { "y" };
        vec![format!("Space::{method}({})", size.path())]
      }
      "divider" => {
        let vertical = self.prop_value(node, "orientation").as_str() == Some("vertical");
        if vertical {
          vec!["Divider::vertical()".into()]
        } else {
          vec!["Divider::new()".into()]
        }
      }
      "indicator" => {
        let child = node.slot("child").first().copied();
        let inner = match child {
          Some(id) => self.emit(id, Placement { absolute: false }),
          None => vec!["div()".into()],
        };
        let mut lines = vec!["Indicator::new(".into()];
        lines.extend(indent(&inner));
        lines.push(")".into());
        lines
      }
      "expanded" => {
        let child = node.children().first().copied();
        let inner = match child {
          Some(id) => self.emit(id, Placement { absolute: false }),
          None => vec!["div()".into()],
        };
        let mut lines = vec!["Expanded::new(".into()];
        lines.extend(indent(&inner));
        lines.push(")".into());
        lines
      }
      "tooltip" => {
        let label = self.prop_value(node, "label");
        let label = label.as_str().unwrap_or("");
        let child = node.slot("child").first().copied();
        let inner = match child {
          Some(id) => self.emit(id, Placement { absolute: false }),
          None => vec!["div()".into()],
        };
        let mut lines = vec!["div()".into()];
        lines.extend(indent(&child_call(inner)));
        lines.push(format!("    .tooltip(tooltip({}))", string(label)));
        lines
      }
      "appshell" => vec!["AppShell::new()".into()],
      "virtuallist" => {
        // The rows a designer typed *are* the row builder: the real component
        // takes a closure over an index, so the list becomes a const it reads.
        let rows = self.items(node, "rows").unwrap_or_default();
        let literals: Vec<String> = rows.iter().map(|row| string(row)).collect();
        vec![
          format!(
            "VirtualList::new({}, {}, |index, _window, _cx| {{",
            string(&node.id.element_id()),
            rows.len()
          ),
          format!(
            "    const ROWS: [&str; {}] = [{}];",
            rows.len(),
            literals.join(", ")
          ),
          "    div().px(px(10.)).py(px(6.)).child(ROWS[index])".into(),
          "})".into(),
        ]
      }
      "aicost" => {
        let usage = format!(
          "AIUsage::new({}, {})",
          self.prop_value(node, "input").as_i64().unwrap_or(0),
          self.prop_value(node, "output").as_i64().unwrap_or(0)
        );
        let price = |key: &str| -> String {
          let value = self.prop_value(node, key);
          let text = value.as_str().unwrap_or("").trim().trim_start_matches('$');
          format!("{:?}", text.parse::<f64>().unwrap_or(0.0))
        };
        let pricing = format!(
          "AIPricing::new({}, {})",
          price("input_price"),
          price("output_price")
        );
        vec![format!("AICost::new({usage}, {pricing})")]
      }
      "aisources" => {
        let sources: Vec<String> = self
          .items(node, "sources")
          .unwrap_or_default()
          .iter()
          .map(|line| {
            let (title, location) = split_source(line);
            format!("AISource::new({}, {})", string(title), string(location))
          })
          .collect();
        vec![format!("AISources::new([{}])", sources.join(", "))]
      }
      "kbdgroup" => {
        let keys = self.items(node, "keys").unwrap_or_default();
        let mut lines = vec![
          "div()".into(),
          "    .flex()".into(),
          "    .gap(px(4.))".into(),
        ];
        for key in keys {
          lines.push(format!("    .child(Kbd::new({}))", string(&key)));
        }
        lines
      }
      "barchart" => {
        // `entries` is a second constructor, not a builder call: it
        // takes the labels and the values together.
        let values = self.numbers(node, "values");
        let labels = self.items(node, "labels").unwrap_or_default();
        if !labels.is_empty() && labels.len() == values.len() {
          let pairs: Vec<String> = labels
            .iter()
            .zip(values.iter())
            .map(|(label, value)| format!("({}, {})", string(label), float(*value as f32)))
            .collect();
          vec![format!("BarChart::entries([{}])", pairs.join(", "))]
        } else {
          vec![format!("BarChart::new({})", expr::numbers(&values))]
        }
      }
      "scatterchart" => {
        let values = self.numbers(node, "values");
        let pairs: Vec<String> = values
          .chunks(2)
          .filter(|pair| pair.len() == 2)
          .map(|pair| format!("({}, {})", float(pair[0] as f32), float(pair[1] as f32)))
          .collect();
        vec![format!("ScatterChart::new([{}])", pairs.join(", "))]
      }
      _ => return None,
    };
    Some(lines)
  }
}

/// `.child(<expr>)`, inlined when the expression is short enough to read on
/// one line.
pub fn child_call(inner: Vec<String>) -> Vec<String> {
  prefixed_call("child", inner)
}

fn prefixed_call(method: &str, inner: Vec<String>) -> Vec<String> {
  // A node's tag rides in front of its expression. Take it off before
  // deciding whether that expression fits on one line, then put it back in
  // front of the call — the tag has to name the line the expression lands on,
  // and after collapsing that line is the `.child(..)` itself.
  let (mark, inner) = split_mark(inner);
  if inner.len() == 1 && inner[0].len() + method.len() + 8 <= 96 {
    // Collapsed: the call and the expression are the same line, so the tag
    // goes in front of it.
    let mut out = vec![format!(".{method}({})", inner[0])];
    if let Some(mark) = mark {
      out.insert(0, mark);
    }
    return out;
  }
  // Expanded: the tag goes *inside*, so it names the constructor rather than
  // the `.child(` that wraps it. Landing on `.child(` is landing next to the
  // component instead of on it.
  let mut out = vec![format!(".{method}(")];
  if let Some(mark) = mark {
    out.push(mark);
  }
  out.extend(indent(&inner));
  out.push(")".into());
  out
}

/// Split a leading node tag off a fragment, if it has one.
fn split_mark(mut lines: Vec<String>) -> (Option<String>, Vec<String>) {
  if lines
    .first()
    .map(|line| line.trim_start().starts_with(MARK))
    .unwrap_or(false)
  {
    let mark = lines.remove(0);
    return (Some(mark), lines);
  }
  (None, lines)
}

/// Turn indented lines into nested `TreeNode` constructors.
fn tree_nodes(lines: &[String]) -> Vec<String> {
  fn depth(line: &str) -> usize {
    (line.len() - line.trim_start().len()) / 2
  }
  fn build(lines: &[String], index: &mut usize, level: usize) -> Vec<String> {
    let mut out = Vec::new();
    while *index < lines.len() {
      let line = &lines[*index];
      if line.trim().is_empty() {
        *index += 1;
        continue;
      }
      let this = depth(line);
      if this < level {
        break;
      }
      let label = line.trim().to_string();
      *index += 1;
      let children = build(lines, index, level + 1);
      let id = tailor_model::snake_case(&label);
      let mut node = format!("TreeNode::new({}, {})", string(&id), string(&label));
      if !children.is_empty() {
        node = format!("{node}.children([{}])", children.join(", "));
      }
      out.push(node);
    }
    out
  }
  let mut index = 0;
  build(lines, &mut index, 0)
}

/// Every entity-backed node in the document, **in the order they must be
/// built**: children before the parents that capture them. A `Tabs` whose panel
/// holds a `Slider` clones that slider into its content closure, so the slider
/// has to be a local by the time the tabs are constructed.
///
/// A `Vec`, not a map: the order is the whole point, and a map keyed by id
/// would silently sort it back into creation order.
pub fn entity_fields(doc: &Document) -> Vec<(NodeId, String)> {
  let mut ordered = Vec::new();
  collect(doc, doc.root, &mut ordered);
  // State variables get their names first: the user typed those, and a node
  // called "Email" beside a variable called `email` is a real thing to do.
  let mut used: BTreeSet<String> = doc
    .state
    .iter()
    .map(|var| tailor_model::snake_case(&var.name))
    .collect();
  let mut fields = Vec::new();
  for id in ordered {
    let Some(node) = doc.node(id) else { continue };
    let base = node
      .name
      .clone()
      .or_else(|| catalog::get(&node.kind).map(|spec| spec.title.to_string()))
      .unwrap_or_else(|| node.kind.clone());
    let mut name = tailor_model::snake_case(&base);
    if used.contains(&name) {
      name = format!("{name}_field");
    }
    if used.contains(&name) {
      name = format!("{name}_{}", id.0);
    }
    used.insert(name.clone());
    fields.push((id, name));
  }
  fields
}

/// Post-order so a parent entity is built after the children it may capture.
fn collect(doc: &Document, id: NodeId, out: &mut Vec<NodeId>) {
  let Some(node) = doc.node(id) else { return };
  for child in node.all_children() {
    collect(doc, child, out);
  }
  if node.component_ref().is_none() {
    if let Some(spec) = catalog::get(&node.kind) {
      if spec.ctor.is_entity() {
        out.push(id);
      }
    }
  }
}

/// `title — location`, the way the sources prop is typed. An em dash because
/// that is what reads on the canvas; a plain hyphen works too.
fn split_source(line: &str) -> (&str, &str) {
  match line
    .split_once('\u{2014}')
    .or_else(|| line.split_once(" - "))
  {
    Some((title, location)) => (title.trim(), location.trim()),
    None => (line.trim(), ""),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn short_children_inline_and_long_ones_nest() {
    assert_eq!(
      child_call(vec!["Text::new(\"hi\")".into()]),
      [".child(Text::new(\"hi\"))"]
    );
    let nested = child_call(vec!["div()".into(), "    .flex()".into()]);
    assert_eq!(nested, [".child(", "    div()", "        .flex()", ")"]);
  }

  #[test]
  fn indented_lines_become_nested_tree_nodes() {
    let lines: Vec<String> = ["src", "  main.rs", "  lib.rs", "Cargo.toml"]
      .iter()
      .map(|s| s.to_string())
      .collect();
    let nodes = tree_nodes(&lines);
    assert_eq!(nodes.len(), 2);
    assert!(nodes[0].starts_with("TreeNode::new(\"src\", \"src\").children(["));
    assert!(nodes[0].contains("TreeNode::new(\"main_rs\", \"main.rs\")"));
    assert_eq!(nodes[1], "TreeNode::new(\"cargo_toml\", \"Cargo.toml\")");
  }
}
