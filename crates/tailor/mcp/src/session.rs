//! The project the server has open, and every operation over it.
//!
//! Mutations save to disk immediately. That is what makes the pairing with the
//! app work: Tailor watches the file it has open, so an agent building a screen
//! here shows up on the canvas a moment later without anything being wired
//! between the two processes.

use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};
use tailor_model::catalog::{self, Ctor};
use tailor_model::motion::MotionProps;
use tailor_model::node::DEFAULT_SLOT;
use tailor_model::props::{PropType, PropValue, Props};
use tailor_model::style::{Dimension, Direction, Edges, LayoutMode, ShadowToken, StyleProps};
use tailor_model::tokens::{
  AlignToken, ColorSpec, ColorToken, EaseToken, EnterToken, JustifyToken, LoopToken, SizeToken,
  VariantToken,
};
use tailor_model::{
  ActionDef, DocKind, Document, Node, NodeId, Project, Scheme, StateVar, VarType,
};

#[derive(Default)]
pub struct Session {
  pub path: Option<PathBuf>,
  pub project: Option<Project>,
  /// The document new nodes go into when a call does not name one.
  pub current: String,
}

type Answer = Result<Value, String>;

/// Everything `add_node` needs beyond the document. One struct rather than
/// eight positional arguments, which is one transposition away from a bug.
#[derive(Debug, Default, Clone, Copy)]
pub struct Placement<'a> {
  pub parent: Option<u32>,
  pub kind: &'a str,
  pub slot: Option<&'a str>,
  pub index: Option<usize>,
  pub name: Option<&'a str>,
  pub props: Option<&'a Map<String, Value>>,
  pub style: Option<&'a Map<String, Value>>,
  pub motion: Option<&'a Map<String, Value>>,
}

/// The same for `set_node`. Every field is optional: absent means "leave it".
#[derive(Debug, Default, Clone, Copy)]
pub struct Edit<'a> {
  pub name: Option<&'a str>,
  pub props: Option<&'a Map<String, Value>>,
  pub style: Option<&'a Map<String, Value>>,
  pub motion: Option<&'a Map<String, Value>>,
  pub hidden: Option<bool>,
  pub locked: Option<bool>,
}

impl Session {
  pub fn open(&mut self, path: &Path) -> Answer {
    let project = tailor_store::open(path).map_err(|err| err.to_string())?;
    self.current = project
      .docs
      .first()
      .map(|doc| doc.id.clone())
      .unwrap_or_default();
    self.path = Some(path.to_path_buf());
    self.project = Some(project);
    self.overview()
  }

  pub fn create(&mut self, path: &Path, name: &str) -> Answer {
    let mut project = Project::new(name);
    project.name = name.to_string();
    let path = tailor_store::with_extension(path.to_path_buf());
    // Creating over somebody's project would lose it with no undo anywhere.
    if path.exists() {
      return Err(format!(
        "{} already exists; open it instead, or choose another path",
        path.display()
      ));
    }
    tailor_store::save(&path, &project).map_err(|err| err.to_string())?;
    self.current = project
      .docs
      .first()
      .map(|doc| doc.id.clone())
      .unwrap_or_default();
    self.path = Some(path);
    self.project = Some(project);
    self.overview()
  }

  fn project(&self) -> Result<&Project, String> {
    self
      .project
      .as_ref()
      .ok_or_else(|| "no project is open; call `open` first".to_string())
  }

  fn project_mut(&mut self) -> Result<&mut Project, String> {
    self
      .project
      .as_mut()
      .ok_or_else(|| "no project is open; call `open` first".to_string())
  }

  /// Which document a call is about.
  fn doc_id(&self, requested: Option<&str>) -> Result<String, String> {
    let project = self.project()?;
    let wanted = requested.unwrap_or(&self.current);
    project
      .docs
      .iter()
      .find(|doc| doc.id == wanted || doc.name == wanted)
      .map(|doc| doc.id.clone())
      .ok_or_else(|| {
        let names: Vec<&str> = project.docs.iter().map(|doc| doc.id.as_str()).collect();
        format!(
          "no document called {wanted}; this project has {}",
          names.join(", ")
        )
      })
  }

  fn doc(&self, requested: Option<&str>) -> Result<&Document, String> {
    let id = self.doc_id(requested)?;
    self
      .project()?
      .doc(&id)
      .ok_or_else(|| "document vanished".to_string())
  }

  fn doc_mut(&mut self, requested: Option<&str>) -> Result<&mut Document, String> {
    let id = self.doc_id(requested)?;
    self
      .project_mut()?
      .doc_mut(&id)
      .ok_or_else(|| "document vanished".to_string())
  }

  /// Write the project back. Every mutation ends here.
  pub fn save(&self) -> Result<(), String> {
    let (Some(path), Some(project)) = (&self.path, &self.project) else {
      return Err("no project is open".into());
    };
    tailor_store::save(path, project).map_err(|err| err.to_string())
  }

  // --- reading -----------------------------------------------------------

  pub fn overview(&self) -> Answer {
    let project = self.project()?;
    let problems = tailor_model::lint::check(project);
    let (errors, warnings, _) = tailor_model::lint::counts(&problems);
    Ok(json!({
        "name": project.name,
        "path": self.path.as_ref().map(|p| p.display().to_string()),
        "current": self.current,
        "theme": {
            "scheme": project.theme.scheme.label(),
            "primary": project.theme.primary.label(),
            "radius": project.theme.radius.label(),
        },
        "generator": { "flavor": project.gen.flavor.label(), "module": project.gen.module },
        "documents": project.docs.iter().map(|doc| json!({
            "id": doc.id,
            "name": doc.name,
            "kind": doc.kind.label(),
            "generates_as": tailor_model::pascal_case(&doc.name),
            "nodes": doc.nodes.len(),
            "state": doc.state.iter().map(|var| json!({
                "name": var.name, "type": var.ty.label(), "initial": var.initial,
            })).collect::<Vec<_>>(),
            "actions": doc.actions.iter().map(|a| a.name.clone()).collect::<Vec<_>>(),
            "canvas": { "width": doc.canvas.width, "height": doc.canvas.height },
        })).collect::<Vec<_>>(),
        "problems": { "errors": errors, "warnings": warnings },
    }))
  }

  pub fn outline(&self, doc: Option<&str>) -> Answer {
    let doc = self.doc(doc)?;
    let mut lines = Vec::new();
    write_outline(doc, doc.root, 0, &mut lines);
    Ok(json!({ "document": doc.name, "root": doc.root.0, "tree": lines.join("\n") }))
  }

  pub fn catalog(&self, query: Option<&str>, category: Option<&str>) -> Answer {
    let specs: Vec<_> = match query {
      Some(query) if !query.trim().is_empty() => catalog::search(query),
      _ => catalog::all().to_vec(),
    };
    let rows: Vec<Value> = specs
      .into_iter()
      .filter(|spec| {
        category
          .map(|wanted| spec.category.label().eq_ignore_ascii_case(wanted))
          .unwrap_or(true)
      })
      .map(|spec| {
        json!({
            "kind": spec.kind,
            "title": spec.title,
            "category": spec.category.label(),
            "blurb": spec.blurb,
            "takes_children": spec.takes_children(),
            "slots": spec.slots.iter().map(|slot| slot.key).collect::<Vec<_>>(),
            "owns_state": spec.ctor.is_entity(),
        })
      })
      .collect();
    Ok(json!({ "count": rows.len(), "components": rows }))
  }

  pub fn component(&self, kind: &str) -> Answer {
    let spec = catalog::get(kind).ok_or_else(|| format!("no component called {kind}"))?;
    let props: Vec<Value> = spec
      .props
      .iter()
      .map(|prop| {
        json!({
            "key": prop.key,
            "label": prop.label,
            "type": type_name(prop.ty),
            "choices": prop.choices,
            "default": prop_to_json(&prop.default_value()),
            "hint": prop.hint,
        })
      })
      .collect();
    Ok(json!({
        "kind": spec.kind,
        "title": spec.title,
        "rust": spec.rust,
        "category": spec.category.label(),
        "blurb": spec.blurb,
        "owns_state": spec.ctor.is_entity(),
        "constructor": match spec.ctor {
            Ctor::Unit => "new()",
            Ctor::Id => "new(id)",
            Ctor::IdAnd(_) => "new(id, value)",
            Ctor::Arg(_) => "new(value)",
            Ctor::Args(_) => "new(value, ..)",
            Ctor::Entity | Ctor::EntityArg(_) | Ctor::EntityValue(_) => "cx.new(..)",
            Ctor::Special => "special",
        },
        "props": props,
        "slots": spec.slots.iter().map(|slot| json!({
            "key": slot.key, "label": slot.label, "single": slot.single,
        })).collect::<Vec<_>>(),
        "events": spec.events.iter().map(|event| event.key).collect::<Vec<_>>(),
    }))
  }

  pub fn problems(&self) -> Answer {
    let project = self.project()?;
    let problems = tailor_model::lint::check(project);
    Ok(json!({
        "count": problems.len(),
        "problems": problems.iter().map(|problem| json!({
            "severity": problem.severity.label(),
            "document": problem.doc_id,
            "node": problem.node.map(|id| id.0),
            "message": problem.message,
            "fix": problem.fix,
        })).collect::<Vec<_>>(),
    }))
  }

  pub fn code(&self, doc: Option<&str>) -> Answer {
    let project = self.project()?;
    let doc = self.doc(doc)?;
    let file = tailor_codegen::preview(project, doc);
    Ok(json!({ "path": file.path, "notes": file.notes, "source": file.source }))
  }

  pub fn export(&mut self, dir: &Path) -> Answer {
    let project = self.project()?;
    let report = tailor_store::export(dir, project);
    if !report.ok() {
      return Err(format!(
        "export failed: {}",
        report
          .failed
          .iter()
          .map(|(path, err)| format!("{}: {err}", path.display()))
          .collect::<Vec<_>>()
          .join("; ")
      ));
    }
    // Remember where it went, the same as the app does — it is what
    // *Open in Editor* reads to find the file a node is in, and what
    // `--reveal` reads to go the other way.
    if let Some(path) = self.path.clone() {
      tailor_store::ExportIndex::record(dir, &path);
    }
    let directory = dir.to_string_lossy().to_string();
    if self.project_mut()?.gen.export_dir.as_deref() != Some(directory.as_str()) {
      self.project_mut()?.gen.export_dir = Some(directory);
      self.save()?;
    }
    Ok(json!({
        "summary": report.summary(),
        "written": report.written.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
        "notes": report.notes,
    }))
  }

  // --- writing ------------------------------------------------------------

  pub fn add_document(&mut self, name: &str, kind: &str) -> Answer {
    let kind = match kind {
      "component" => DocKind::Component,
      _ => DocKind::Screen,
    };
    let project = self.project_mut()?;
    let name = project.unique_doc_name(name);
    let id = project.unique_doc_id(&tailor_model::snake_case(&name));
    project
      .docs
      .push(Document::new(id.clone(), name.clone(), kind));
    self.current = id.clone();
    self.save()?;
    Ok(json!({ "id": id, "name": name, "kind": kind.label() }))
  }

  pub fn add_node(&mut self, doc: Option<&str>, at: Placement<'_>) -> Answer {
    let Placement {
      parent,
      kind,
      slot,
      index,
      name,
      props,
      style,
      motion,
    } = at;
    // A component reference has to be checked before it is placed, or the
    // canvas will recurse the first time it draws.
    if let Some(component) = kind.strip_prefix('@') {
      let host = self.doc(doc)?.name.clone();
      if self.project()?.would_recurse(&host, component) {
        return Err(format!("{component} would contain {host}"));
      }
    }
    let parsed_props = match props {
      Some(props) => props_from_json(kind, props)?,
      None => Props::new(),
    };
    let parsed_style = match style {
      Some(style) => Some(style_from_json(style)?),
      None => None,
    };
    let parsed_motion = match motion {
      Some(motion) => Some(motion_from_json_onto(MotionProps::default(), motion)?),
      None => None,
    };

    let document = self.doc_mut(doc)?;
    let parent = parent.map(NodeId).unwrap_or(document.root);
    if document.node(parent).is_none() {
      return Err(format!("no node {}", parent.0));
    }
    let mut node = match catalog::get(kind) {
      Some(spec) => spec.build(document.ids.next()),
      None if kind.starts_with('@') => Node::new(document.ids.next(), kind),
      None => return Err(format!("no component called {kind}")),
    };
    if let Some(name) = name {
      node.name = Some(name.to_string());
    }
    for (key, value) in parsed_props {
      node.props.insert(key, value);
    }
    if let Some(style) = parsed_style {
      node.style = style;
    }
    if let Some(motion) = parsed_motion {
      node.motion = motion;
    }
    let id = document.insert(
      parent,
      slot.unwrap_or(DEFAULT_SLOT),
      index.unwrap_or(usize::MAX),
      node,
    );
    self.save()?;
    Ok(json!({ "id": id.0, "kind": kind, "parent": parent.0 }))
  }

  pub fn set_node(&mut self, doc: Option<&str>, node: u32, edit: Edit<'_>) -> Answer {
    let Edit {
      name,
      props,
      style,
      motion,
      hidden,
      locked,
    } = edit;
    let id = NodeId(node);
    let kind = self
      .doc(doc)?
      .node(id)
      .map(|node| node.kind.clone())
      .ok_or_else(|| format!("no node {node}"))?;
    let parsed_props = match props {
      Some(props) => props_from_json(&kind, props)?,
      None => Props::new(),
    };
    let parsed_style = match style {
      Some(style) => Some(style_from_json_onto(
        self.doc(doc)?.node(id).unwrap().style.clone(),
        style,
      )?),
      None => None,
    };
    let parsed_motion = match motion {
      Some(motion) => Some(motion_from_json_onto(
        self.doc(doc)?.node(id).unwrap().motion,
        motion,
      )?),
      None => None,
    };

    let document = self.doc_mut(doc)?;
    let target = document
      .node_mut(id)
      .ok_or_else(|| format!("no node {node}"))?;
    if let Some(name) = name {
      target.name = (!name.is_empty()).then(|| name.to_string());
    }
    for (key, value) in parsed_props {
      target.props.insert(key, value);
    }
    if let Some(style) = parsed_style {
      target.style = style;
    }
    if let Some(motion) = parsed_motion {
      target.motion = motion;
    }
    if let Some(hidden) = hidden {
      target.hidden = hidden;
    }
    if let Some(locked) = locked {
      target.locked = locked;
    }
    self.save()?;
    Ok(json!({ "id": node, "kind": kind }))
  }

  pub fn move_node(
    &mut self,
    doc: Option<&str>,
    node: u32,
    parent: u32,
    slot: Option<&str>,
    index: Option<usize>,
  ) -> Answer {
    let document = self.doc_mut(doc)?;
    let moved = document.move_node(
      NodeId(node),
      NodeId(parent),
      slot.unwrap_or(DEFAULT_SLOT),
      index.unwrap_or(usize::MAX),
    );
    if !moved {
      return Err("that move would put a node inside itself, or the node is the root".into());
    }
    self.save()?;
    Ok(json!({ "id": node, "parent": parent }))
  }

  pub fn remove_node(&mut self, doc: Option<&str>, node: u32) -> Answer {
    let document = self.doc_mut(doc)?;
    let removed = document.remove(NodeId(node));
    if removed.is_empty() {
      return Err(format!(
        "nothing removed: {node} is the root, or does not exist"
      ));
    }
    self.save()?;
    Ok(json!({ "removed": removed.len() }))
  }

  pub fn add_state(
    &mut self,
    doc: Option<&str>,
    name: &str,
    ty: &str,
    initial: Option<&str>,
  ) -> Answer {
    let ty = match ty {
      "bool" => VarType::Bool,
      "int" => VarType::Int,
      "float" => VarType::Float,
      "items" => VarType::Items,
      "text" | "" => VarType::Text,
      other => return Err(format!("no variable type called {other}")),
    };
    let document = self.doc_mut(doc)?;
    let name = document.unique_var_name(name);
    let mut var = StateVar::new(name.clone(), ty);
    var.initial = initial.unwrap_or("").to_string();
    document.state.push(var);
    self.save()?;
    Ok(json!({ "name": name, "type": ty.label(), "rust": ty.rust() }))
  }

  pub fn add_action(&mut self, doc: Option<&str>, name: &str) -> Answer {
    let document = self.doc_mut(doc)?;
    let name = document.unique_action_name(name);
    document.actions.push(ActionDef::new(name.clone()));
    self.save()?;
    Ok(json!({ "name": name }))
  }

  pub fn bind(&mut self, doc: Option<&str>, node: u32, prop: &str, var: &str) -> Answer {
    let document = self.doc_mut(doc)?;
    if document.var(var).is_none() {
      return Err(format!("no state variable called {var}"));
    }
    let target = document
      .node_mut(NodeId(node))
      .ok_or_else(|| format!("no node {node}"))?;
    target.set_prop(prop, PropValue::Binding(var.to_string()));
    self.save()?;
    Ok(json!({ "node": node, "prop": prop, "reads": var }))
  }

  pub fn connect(&mut self, doc: Option<&str>, node: u32, event: &str, action: &str) -> Answer {
    let document = self.doc_mut(doc)?;
    if !document.actions.iter().any(|a| a.name == action) {
      return Err(format!("no action called {action}"));
    }
    let target = document
      .node_mut(NodeId(node))
      .ok_or_else(|| format!("no node {node}"))?;
    target.events.insert(event.to_string(), action.to_string());
    self.save()?;
    Ok(json!({ "node": node, "event": event, "calls": action }))
  }

  pub fn set_theme(
    &mut self,
    scheme: Option<&str>,
    primary: Option<&str>,
    radius: Option<&str>,
    font: Option<&str>,
  ) -> Answer {
    let project = self.project_mut()?;
    if let Some(scheme) = scheme {
      project.theme.scheme = match scheme {
        "light" => Scheme::Light,
        "dark" => Scheme::Dark,
        other => return Err(format!("no scheme called {other}")),
      };
    }
    if let Some(primary) = primary {
      project.theme.primary =
        ColorToken::parse(primary).ok_or_else(|| format!("no palette colour called {primary}"))?;
    }
    if let Some(radius) = radius {
      project.theme.radius =
        SizeToken::parse(radius).ok_or_else(|| format!("no size called {radius}"))?;
    }
    if let Some(font) = font {
      project.theme.font = font.to_string();
    }
    self.save()?;
    self.overview()
  }
}

fn write_outline(doc: &Document, id: NodeId, depth: usize, out: &mut Vec<String>) {
  let Some(node) = doc.node(id) else { return };
  let title = node
    .name
    .clone()
    .or_else(|| node.component_ref().map(|name| name.to_string()))
    .or_else(|| catalog::get(&node.kind).map(|spec| spec.title.to_string()))
    .unwrap_or_else(|| node.kind.clone());
  let mut line = format!("{}#{} {} [{}]", "  ".repeat(depth), id.0, title, node.kind);
  if node.hidden {
    line.push_str(" (hidden)");
  }
  if node.locked {
    line.push_str(" (locked)");
  }
  out.push(line);

  let slots: Vec<String> = node.slots.keys().cloned().collect();
  for slot in slots {
    let children = node.slot(&slot).to_vec();
    if children.is_empty() {
      continue;
    }
    if slot != DEFAULT_SLOT {
      out.push(format!("{}  {slot}:", "  ".repeat(depth)));
    }
    for child in children {
      let extra = usize::from(slot != DEFAULT_SLOT);
      write_outline(doc, child, depth + 1 + extra, out);
    }
  }
}

fn type_name(ty: PropType) -> &'static str {
  match ty {
    PropType::Bool => "bool",
    PropType::Int => "int",
    PropType::Float => "float",
    PropType::Text => "text",
    PropType::MultilineText => "text",
    PropType::Choice => "choice",
    PropType::Color => "color",
    PropType::Size => "size",
    PropType::Variant => "variant",
    PropType::Icon => "icon",
    PropType::Items => "items",
    PropType::Numbers => "numbers",
  }
}

fn prop_to_json(value: &PropValue) -> Value {
  match value {
    PropValue::Bool(v) => json!(v),
    PropValue::Int(v) => json!(v),
    PropValue::Float(v) => json!(v),
    PropValue::Text(v) | PropValue::Choice(v) | PropValue::Icon(v) => json!(v),
    PropValue::Size(v) => json!(v.label()),
    PropValue::Variant(v) => json!(v.label()),
    PropValue::Color(ColorSpec::Named(v)) => json!(v.label()),
    PropValue::Color(ColorSpec::Custom(v)) => json!(v),
    PropValue::Items(v) => json!(v),
    PropValue::Numbers(v) => json!(v),
    PropValue::Binding(v) => json!({ "bind": v }),
  }
}

/// Turn a JSON object of props into typed values, guided by the catalog. This
/// is what lets a caller write `{"variant": "outline", "size": "lg"}` and have
/// it mean the right thing without knowing about `VariantToken`.
pub fn props_from_json(kind: &str, values: &Map<String, Value>) -> Result<Props, String> {
  let spec = catalog::get(kind);
  let mut out = Props::new();
  for (key, value) in values {
    let Some(spec) = spec else {
      return Err(format!("no component called {kind}"));
    };
    let prop = spec.prop(key).ok_or_else(|| {
      let known: Vec<&str> = spec.props.iter().map(|p| p.key).collect();
      format!("{kind} has no prop `{key}`; it has {}", known.join(", "))
    })?;
    out.insert(
      key.clone(),
      prop_from_json(prop.ty, prop.choices, key, value)?,
    );
  }
  Ok(out)
}

fn prop_from_json(
  ty: PropType,
  choices: &[&str],
  key: &str,
  value: &Value,
) -> Result<PropValue, String> {
  // `{"bind": "query"}` reads a state variable rather than a literal.
  if let Some(var) = value.get("bind").and_then(|v| v.as_str()) {
    return Ok(PropValue::Binding(var.to_string()));
  }
  let text = value.as_str();
  match ty {
    PropType::Bool => value
      .as_bool()
      .map(PropValue::Bool)
      .ok_or_else(|| format!("`{key}` wants true or false")),
    PropType::Int => value
      .as_i64()
      .map(PropValue::Int)
      .ok_or_else(|| format!("`{key}` wants a whole number")),
    PropType::Float => value
      .as_f64()
      .filter(|value| value.is_finite())
      .map(PropValue::Float)
      .ok_or_else(|| format!("`{key}` wants a finite number")),
    PropType::Text | PropType::MultilineText => {
      Ok(PropValue::Text(text.map(|t| t.to_string()).unwrap_or_else(
        || value.to_string().trim_matches('"').to_string(),
      )))
    }
    PropType::Icon => Ok(PropValue::Icon(text.unwrap_or_default().to_string())),
    PropType::Choice => {
      let text = text.ok_or_else(|| format!("`{key}` wants one of {}", choices.join(", ")))?;
      if !choices.is_empty() && !choices.contains(&text) {
        return Err(format!("`{key}` wants one of {}", choices.join(", ")));
      }
      Ok(PropValue::Choice(text.to_string()))
    }
    PropType::Size => {
      let text = text.ok_or_else(|| format!("`{key}` wants xs, sm, md, lg or xl"))?;
      SizeToken::parse(text)
        .map(PropValue::Size)
        .ok_or_else(|| format!("`{key}` wants xs, sm, md, lg or xl"))
    }
    PropType::Variant => {
      let text = text.ok_or_else(|| format!("`{key}` wants a variant name"))?;
      VariantToken::parse(text)
        .map(PropValue::Variant)
        .ok_or_else(|| format!("no variant called {text}"))
    }
    PropType::Color => {
      let text = text.ok_or_else(|| format!("`{key}` wants a palette name or a hex"))?;
      Ok(PropValue::Color(match ColorToken::parse(text) {
        Some(token) => ColorSpec::Named(token),
        None => ColorSpec::Custom(text.to_string()),
      }))
    }
    PropType::Items => {
      let items = value
        .as_array()
        .map(|values| {
          values
            .iter()
            .map(|v| {
              v.as_str()
                .map(|s| s.to_string())
                .unwrap_or_else(|| v.to_string())
            })
            .collect::<Vec<_>>()
        })
        .or_else(|| text.map(|t| t.lines().map(|l| l.trim().to_string()).collect()))
        .ok_or_else(|| format!("`{key}` wants a list of strings"))?;
      Ok(PropValue::Items(items))
    }
    PropType::Numbers => {
      let numbers = value
        .as_array()
        .map(|values| {
          values
            .iter()
            .filter_map(|v| v.as_f64())
            .filter(|v| v.is_finite())
            .collect::<Vec<_>>()
        })
        .ok_or_else(|| format!("`{key}` wants a list of numbers"))?;
      Ok(PropValue::Numbers(numbers))
    }
  }
}

/// Style from JSON, onto a fresh default.
/// A node's entrance from JSON, onto whatever it already had.
///
/// `enter: null` turns the animation off, which is the only way to say
/// "remove this" in a merge-shaped API.
pub fn motion_from_json_onto(
  mut motion: MotionProps,
  values: &Map<String, Value>,
) -> Result<MotionProps, String> {
  for (key, value) in values {
    let number = || value.as_f64().filter(|v| v.is_finite()).map(|v| v as f32);
    let text = value.as_str();
    match key.as_str() {
      "enter" => {
        motion.enter = match value {
          Value::Null => None,
          _ => Some(text.and_then(EnterToken::parse).ok_or_else(|| {
            "enter wants fade, slideup, slidedown, slideleft, slideright or null".to_string()
          })?),
        }
      }
      "ease" => {
        motion.ease = text
          .and_then(EaseToken::parse)
          .ok_or_else(|| format!("no easing called {text:?}"))?
      }
      "duration" => motion.duration = number().unwrap_or(0.0).max(0.0),
      "delay" => motion.delay = number().unwrap_or(0.0).max(0.0),
      "distance" => motion.distance = number().unwrap_or(0.0),
      "stagger" => motion.stagger = number().unwrap_or(0.0).max(0.0),
      "repeat" => {
        motion.repeat = text
          .and_then(LoopToken::parse)
          .ok_or_else(|| "repeat wants once or forever".to_string())?
      }
      "alternate" => motion.alternate = value.as_bool().unwrap_or(false),
      other => return Err(format!("no motion setting called {other}")),
    }
  }
  Ok(motion)
}

pub fn style_from_json(values: &Map<String, Value>) -> Result<StyleProps, String> {
  style_from_json_onto(StyleProps::default(), values)
}

/// Style from JSON, onto whatever the node already had.
pub fn style_from_json_onto(
  mut style: StyleProps,
  values: &Map<String, Value>,
) -> Result<StyleProps, String> {
  for (key, value) in values {
    let number = || value.as_f64().filter(|v| v.is_finite()).map(|v| v as f32);
    let text = value.as_str();
    match key.as_str() {
      "layout" => {
        style.layout = match text {
          Some("absolute") => LayoutMode::Absolute,
          Some("flow") | None => LayoutMode::Flow,
          Some(other) => return Err(format!("no layout called {other}")),
        }
      }
      "direction" => {
        style.direction = match text {
          Some("row") => Direction::Row,
          Some("column") => Direction::Column,
          other => return Err(format!("no direction called {other:?}")),
        }
      }
      "wrap" => style.wrap = value.as_bool().unwrap_or(false),
      "gap" => style.gap = number(),
      "align" => {
        style.align = text.and_then(AlignToken::parse);
        if style.align.is_none() {
          return Err("align wants start, center, end or stretch".into());
        }
      }
      "justify" => {
        style.justify = text.and_then(JustifyToken::parse);
        if style.justify.is_none() {
          return Err("justify wants start, center, end, between or around".into());
        }
      }
      "x" => style.x = number().unwrap_or(0.0),
      "y" => style.y = number().unwrap_or(0.0),
      "width" => style.width = dimension(value)?,
      "height" => style.height = dimension(value)?,
      "min_width" => style.min_width = number(),
      "max_width" => style.max_width = number(),
      "min_height" => style.min_height = number(),
      "max_height" => style.max_height = number(),
      "padding" => style.padding = edges(value)?,
      "margin" => style.margin = edges(value)?,
      "background" => style.background = Some(color(value)?),
      "text_color" => style.text_color = Some(color(value)?),
      "border_width" => style.border_width = number().unwrap_or(0.0),
      "border_color" => style.border_color = Some(color(value)?),
      "radius" => style.radius = number().unwrap_or(0.0),
      "shadow" => {
        style.shadow = ShadowToken::ALL
          .iter()
          .copied()
          .find(|token| Some(token.label()) == text)
          .ok_or_else(|| "shadow wants none, xs, sm, md, lg or xl".to_string())?
      }
      "opacity" => style.opacity = number().unwrap_or(1.0),
      "font_size" => style.font_size = number(),
      "font_weight" => style.font_weight = value.as_u64().map(|v| v as u16),
      "italic" => style.italic = value.as_bool().unwrap_or(false),
      other => return Err(format!("no style field called `{other}`")),
    }
  }
  Ok(style)
}

/// `200`, `"full"`, `"auto"`, or `"grow"`.
fn dimension(value: &Value) -> Result<Dimension, String> {
  if let Some(number) = value.as_f64().filter(|v| v.is_finite()) {
    return Ok(Dimension::Px(number as f32));
  }
  match value.as_str() {
    Some("auto") => Ok(Dimension::Auto),
    Some("full") => Ok(Dimension::Full),
    Some("grow") => Ok(Dimension::Grow(1.0)),
    _ => Err("a size wants a number, or \"auto\", \"full\" or \"grow\"".into()),
  }
}

/// `16`, or `{"top": 8, "left": 12}`.
fn edges(value: &Value) -> Result<Edges, String> {
  if let Some(all) = value.as_f64().filter(|v| v.is_finite()) {
    return Ok(Edges::all(all as f32));
  }
  let object = value
    .as_object()
    .ok_or_else(|| "spacing wants a number, or an object of top/right/bottom/left".to_string())?;
  let side = |key: &str| {
    object
      .get(key)
      .and_then(|v| v.as_f64())
      .filter(|v| v.is_finite())
      .unwrap_or(0.0) as f32
  };
  Ok(Edges {
    top: side("top"),
    right: side("right"),
    bottom: side("bottom"),
    left: side("left"),
  })
}

fn color(value: &Value) -> Result<ColorSpec, String> {
  let text = value
    .as_str()
    .ok_or_else(|| "a colour wants a name or a hex".to_string())?;
  Ok(match ColorToken::parse(text) {
    Some(token) => ColorSpec::Named(token),
    None => ColorSpec::Custom(text.to_string()),
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  fn map(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap()
  }

  #[test]
  fn motion_json_merges_onto_what_is_already_there() {
    let first = motion_from_json_onto(
      MotionProps::default(),
      &map(json!({ "enter": "slideup", "ease": "out-back", "duration": 320 })),
    )
    .unwrap();
    assert_eq!(first.enter, Some(EnterToken::SlideUp));
    assert_eq!(first.ease, EaseToken::OutBack);
    assert_eq!(first.duration, 320.0);

    // A later delay-only edit leaves the easing alone.
    let second = motion_from_json_onto(first, &map(json!({ "delay": 60 }))).unwrap();
    assert_eq!(second.ease, EaseToken::OutBack);
    assert_eq!(second.delay, 60.0);
  }

  #[test]
  fn null_is_how_an_entrance_is_taken_away() {
    let motion = motion_from_json_onto(
      MotionProps {
        enter: Some(EnterToken::Fade),
        ..Default::default()
      },
      &map(json!({ "enter": null })),
    )
    .unwrap();
    assert!(motion.is_off());
  }

  #[test]
  fn an_unknown_word_is_an_error_not_a_silent_default() {
    for bad in [
      json!({ "ease": "easeOutQuad" }),
      json!({ "enter": "zoom" }),
      json!({ "repeat": "twice" }),
      json!({ "wobble": 3 }),
    ] {
      assert!(
        motion_from_json_onto(MotionProps::default(), &map(bad.clone())).is_err(),
        "{bad} should have been refused"
      );
    }
  }

  #[test]
  fn unwritable_numbers_never_reach_the_document() {
    let motion = motion_from_json_onto(
      MotionProps::default(),
      &map(json!({ "duration": f64::INFINITY, "delay": -40 })),
    )
    .unwrap();
    assert!(!motion.has_non_finite());
    assert_eq!(motion.delay, 0.0, "a negative delay clamps to none");
  }
}
