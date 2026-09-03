//! A `.tailor` file: every screen and component you are building, plus the
//! theme they are designed against and the settings the generator reads.
//!
//! One file rather than one-per-screen, because a component placed inside a
//! screen has to resolve, and cross-file references would mean a project
//! index anyway. The whole thing is small — it is a tree of props, not assets.

use crate::doc::{DocKind, Document};
use crate::tokens::{ColorToken, SizeToken};
use serde::{Deserialize, Serialize};

/// Bumped when the on-disk shape changes in a way a reader must know about.
/// A file from the future is refused rather than half-read.
pub const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scheme {
  #[default]
  Dark,
  Light,
}

impl Scheme {
  pub const ALL: &'static [Scheme] = &[Scheme::Dark, Scheme::Light];

  pub fn label(self) -> &'static str {
    match self {
      Scheme::Dark => "dark",
      Scheme::Light => "light",
    }
  }
}

/// The guise theme a project designs against. The canvas installs it so what
/// you see is what the generated app gets, and `theme.rs` is generated from it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThemeSpec {
  #[serde(default)]
  pub scheme: Scheme,
  #[serde(default = "default_primary")]
  pub primary: ColorToken,
  #[serde(default = "default_radius")]
  pub radius: SizeToken,
  #[serde(default = "default_font")]
  pub font: String,
  /// One of guise's prebuilt themes (see [`PRESETS`]) to start from instead of
  /// plain light/dark. Empty is the default and means "just the scheme".
  #[serde(default, skip_serializing_if = "String::is_empty")]
  pub preset: String,
  /// A guise theme file, stored *inline* rather than as a path. A `.tailor`
  /// project is one file you can mail to someone; a theme that lives beside it
  /// would be a second file to lose, and a path that only resolves on the
  /// machine that set it. The exporter writes it back out as `theme.json`.
  #[serde(default, skip_serializing_if = "String::is_empty")]
  pub json: String,
}

/// One of guise's prebuilt themes, as the document refers to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThemePreset {
  /// What the file stores, and what `Theme::preset(..)` looks up.
  pub id: &'static str,
  pub label: &'static str,
  /// The guise constructor the generator prints. Not always the id: the ids
  /// are run together and the Rust names are not.
  pub rust: &'static str,
  /// The scheme this preset is a variation of.
  pub scheme: Scheme,
}

const fn preset(
  id: &'static str,
  label: &'static str,
  rust: &'static str,
  scheme: Scheme,
) -> ThemePreset {
  ThemePreset {
    id,
    label,
    rust,
    scheme,
  }
}

/// The guise theme presets a project can start from.
///
/// Duplicated here rather than read out of guise, because the model is
/// deliberately guise-free — `presets_match_guise` in this file's tests reads
/// guise's own source and fails when the two lists drift.
pub const THEME_PRESETS: &[ThemePreset] = &[
  preset("catppuccin", "Catppuccin", "catppuccin", Scheme::Dark),
  preset("nord", "Nord", "nord", Scheme::Dark),
  preset("tokyonight", "Tokyo Night", "tokyonight", Scheme::Dark),
  preset("gruvbox", "Gruvbox", "gruvbox", Scheme::Dark),
  preset("dracula", "Dracula", "dracula", Scheme::Dark),
  preset(
    "solarizedlight",
    "Solarized Light",
    "solarized_light",
    Scheme::Light,
  ),
];

impl ThemeSpec {
  /// The named preset, when there is one and it is known.
  pub fn preset_entry(&self) -> Option<&'static ThemePreset> {
    THEME_PRESETS.iter().find(|preset| preset.id == self.preset)
  }

  /// The scheme the design actually renders in. A pasted theme file names its
  /// own, a preset is a variation of one, and otherwise it is the project's.
  ///
  /// Precedence is the same everywhere it matters — the canvas, the generator,
  /// and the inspector all call this rather than reading `scheme` directly.
  pub fn effective_scheme(&self) -> Scheme {
    if !self.json.is_empty() {
      return match json_scheme(&self.json) {
        Some(scheme) => scheme,
        // guise's theme files default to dark when they say nothing.
        None => Scheme::Dark,
      };
    }
    match self.preset_entry() {
      Some(preset) => preset.scheme,
      None => self.scheme,
    }
  }

  /// True when a preset or a theme file is doing the work, so the scheme and
  /// primary controls no longer decide anything.
  pub fn is_overridden(&self) -> bool {
    !self.json.is_empty() || self.preset_entry().is_some()
  }
}

/// The `scheme` slot of a guise theme file. The format is a flat object of
/// strings, so reading one key needs no schema.
fn json_scheme(source: &str) -> Option<Scheme> {
  let value: serde_json::Value = serde_json::from_str(source).ok()?;
  match value.get("scheme")?.as_str()? {
    "light" => Some(Scheme::Light),
    "dark" => Some(Scheme::Dark),
    _ => None,
  }
}

fn default_primary() -> ColorToken {
  ColorToken::Blue
}

fn default_radius() -> SizeToken {
  SizeToken::Md
}

fn default_font() -> String {
  ".SystemUIFont".into()
}

impl Default for ThemeSpec {
  fn default() -> Self {
    ThemeSpec {
      scheme: Scheme::default(),
      primary: default_primary(),
      radius: default_radius(),
      font: default_font(),
      preset: String::new(),
      json: String::new(),
    }
  }
}

/// Which flavour of guise the generator writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Flavor {
  /// Plain builder calls and gpui `Styled` methods. Always compiles, reads
  /// like the rest of an app.
  #[default]
  Plain,
  /// guise's layout macros (`col!`, `row!`) and `style! { … }` blocks.
  Macros,
}

impl Flavor {
  pub const ALL: &'static [Flavor] = &[Flavor::Plain, Flavor::Macros];

  pub fn label(self) -> &'static str {
    match self {
      Flavor::Plain => "plain",
      Flavor::Macros => "macros",
    }
  }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenSettings {
  #[serde(default)]
  pub flavor: Flavor,
  /// The crate-relative module the generated files go into.
  #[serde(default = "default_module")]
  pub module: String,
  /// Emit a `main.rs` and a `theme.rs` alongside the components, so the
  /// export is a crate you can run rather than a folder you have to wire up.
  #[serde(default = "yes")]
  pub emit_app: bool,
  /// Where the last export went, so *Open in Editor* knows which file on disk
  /// a node corresponds to without asking every time. Absolute, and absent
  /// until something has actually been exported.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub export_dir: Option<String>,
}

fn default_module() -> String {
  "ui".into()
}

fn yes() -> bool {
  true
}

impl Default for GenSettings {
  fn default() -> Self {
    GenSettings {
      flavor: Flavor::default(),
      module: default_module(),
      emit_app: true,
      export_dir: None,
    }
  }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
  #[serde(default = "current_version")]
  pub format: u32,
  pub name: String,
  pub docs: Vec<Document>,
  #[serde(default)]
  pub theme: ThemeSpec,
  #[serde(default)]
  pub gen: GenSettings,
}

fn current_version() -> u32 {
  FORMAT_VERSION
}

#[derive(Debug)]
pub enum LoadError {
  /// The file parsed, but was written by a newer Tailor.
  Newer(u32),
  Json(serde_json::Error),
}

impl std::fmt::Display for LoadError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      LoadError::Newer(v) => {
        write!(f, "this project was saved by a newer Tailor (format {v})")
      }
      LoadError::Json(err) => write!(f, "{err}"),
    }
  }
}

impl std::error::Error for LoadError {}

impl Project {
  /// A new project with one empty screen.
  pub fn new(name: impl Into<String>) -> Self {
    Project {
      format: FORMAT_VERSION,
      name: name.into(),
      docs: vec![Document::new("main", "MainScreen", DocKind::Screen)],
      theme: ThemeSpec::default(),
      gen: GenSettings::default(),
    }
  }

  pub fn doc(&self, id: &str) -> Option<&Document> {
    self.docs.iter().find(|d| d.id == id)
  }

  pub fn doc_mut(&mut self, id: &str) -> Option<&mut Document> {
    self.docs.iter_mut().find(|d| d.id == id)
  }

  /// A document by the name a component reference uses.
  pub fn doc_by_name(&self, name: &str) -> Option<&Document> {
    self.docs.iter().find(|d| d.name == name)
  }

  /// The components (not screens) another document may place.
  pub fn placeable(&self, excluding: &str) -> Vec<&Document> {
    self
      .docs
      .iter()
      .filter(|d| d.kind == DocKind::Component && d.id != excluding)
      .collect()
  }

  pub fn unique_doc_name(&self, base: &str) -> String {
    crate::doc::unique(base, |candidate| {
      self.docs.iter().any(|d| d.name == candidate)
    })
  }

  pub fn unique_doc_id(&self, base: &str) -> String {
    crate::doc::unique(base, |candidate| {
      self.docs.iter().any(|d| d.id == candidate)
    })
  }

  /// Would placing the component named `placing` inside the document named
  /// `host` create a cycle? Both arguments are document *names*, which is
  /// what a `@Name` reference carries. The visited set is not paranoia: a
  /// hand-edited file can already contain the cycle this exists to prevent.
  pub fn would_recurse(&self, host: &str, placing: &str) -> bool {
    fn walk<'a>(
      project: &'a Project,
      host: &str,
      placing: &'a str,
      seen: &mut Vec<&'a str>,
    ) -> bool {
      if host == placing {
        return true;
      }
      if seen.contains(&placing) {
        return true;
      }
      seen.push(placing);
      let Some(doc) = project.doc_by_name(placing) else {
        return false;
      };
      doc.nodes.values().any(|node| match node.component_ref() {
        Some(inner) => walk(project, host, inner, seen),
        None => false,
      })
    }
    walk(self, host, placing, &mut Vec::new())
  }

  /// The first number in the project that JSON cannot write, named well
  /// enough to fix. serde turns an infinity or a NaN into `null` rather than
  /// failing, and the file that results does not load again — so a save has
  /// to look before it writes.
  pub fn non_finite(&self) -> Option<String> {
    for doc in &self.docs {
      for node in doc.nodes.values() {
        if node.style.has_non_finite() {
          return Some(format!(
            "{}: node {} has an unwritable size",
            doc.name, node.id
          ));
        }
        if node.motion.has_non_finite() {
          return Some(format!(
            "{}: node {} has an unwritable animation timing",
            doc.name, node.id
          ));
        }
        for (key, value) in &node.props {
          if value.has_non_finite() {
            return Some(format!(
              "{}: node {}'s `{key}` is not a finite number",
              doc.name, node.id
            ));
          }
        }
      }
    }
    None
  }

  /// Serialize. Fallible on purpose — see [`Project::non_finite`].
  pub fn to_json(&self) -> Result<String, String> {
    if let Some(found) = self.non_finite() {
      return Err(found);
    }
    serde_json::to_string_pretty(self).map_err(|err| err.to_string())
  }

  pub fn from_json(text: &str) -> Result<Self, LoadError> {
    let mut project: Project = serde_json::from_str(text).map_err(LoadError::Json)?;
    if project.format > FORMAT_VERSION {
      return Err(LoadError::Newer(project.format));
    }
    for doc in &mut project.docs {
      doc.repair();
    }
    if project.docs.is_empty() {
      project
        .docs
        .push(Document::new("main", "MainScreen", DocKind::Screen));
    }
    Ok(project)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::node::DEFAULT_SLOT;

  #[test]
  fn a_new_project_round_trips() {
    let project = Project::new("Demo");
    let parsed = Project::from_json(&project.to_json().unwrap()).unwrap();
    assert_eq!(parsed, project);
  }

  #[test]
  fn a_non_finite_number_is_refused_rather_than_panicking() {
    use crate::node::DEFAULT_SLOT;
    let mut project = Project::new("Demo");
    let doc = &mut project.docs[0];
    let root = doc.root;
    let node = doc.create("progress");
    let id = doc.insert(root, DEFAULT_SLOT, 0, node);
    doc
      .node_mut(id)
      .unwrap()
      .set_prop("value", crate::props::PropValue::Float(f64::NAN));
    let err = project.to_json().unwrap_err();
    assert!(err.contains("finite"), "{err}");
  }

  #[test]
  fn a_newer_format_is_refused() {
    let mut project = Project::new("Demo");
    project.format = FORMAT_VERSION + 1;
    let err = Project::from_json(&project.to_json().unwrap()).unwrap_err();
    assert!(matches!(err, LoadError::Newer(_)));
  }

  #[test]
  fn loading_repairs_a_hand_edited_file() {
    let mut project = Project::new("Demo");
    let doc = &mut project.docs[0];
    let root = doc.root;
    doc
      .node_mut(root)
      .unwrap()
      .slot_mut(DEFAULT_SLOT)
      .push(crate::id::NodeId(77));
    let parsed = Project::from_json(&project.to_json().unwrap()).unwrap();
    assert!(parsed.docs[0].children_of(parsed.docs[0].root).is_empty());
  }

  #[test]
  fn a_component_that_contains_its_host_is_a_cycle() {
    let mut project = Project::new("Demo");
    project
      .docs
      .push(Document::new("card", "Card", DocKind::Component));
    project
      .docs
      .push(Document::new("row", "Row", DocKind::Component));

    // Row places Card.
    let node = project.doc_mut("row").unwrap().create("@Card");
    let root = project.doc("row").unwrap().root;
    project
      .doc_mut("row")
      .unwrap()
      .insert(root, DEFAULT_SLOT, 0, node);

    assert!(project.would_recurse("Card", "Row"));
    assert!(!project.would_recurse("Row", "Card"));
    assert!(project.would_recurse("Card", "Card"));
  }

  #[test]
  fn placeable_lists_components_but_not_the_host() {
    let mut project = Project::new("Demo");
    project
      .docs
      .push(Document::new("card", "Card", DocKind::Component));
    let names: Vec<&str> = project
      .placeable("main")
      .iter()
      .map(|d| d.name.as_str())
      .collect();
    assert_eq!(names, ["Card"]);
    assert!(project.placeable("card").is_empty());
  }

  #[test]
  fn a_preset_and_a_theme_file_each_decide_the_scheme() {
    let mut spec = ThemeSpec {
      scheme: Scheme::Light,
      ..ThemeSpec::default()
    };
    assert_eq!(spec.effective_scheme(), Scheme::Light);
    assert!(!spec.is_overridden());

    // A preset is a variation of one scheme, so it settles the question.
    spec.preset = "dracula".into();
    assert_eq!(spec.effective_scheme(), Scheme::Dark);
    assert_eq!(spec.preset_entry().unwrap().label, "Dracula");
    assert!(spec.is_overridden());

    // A pasted theme file outranks the preset and names its own.
    spec.json = r##"{"scheme": "light", "primary": "#268bd2"}"##.into();
    assert_eq!(spec.effective_scheme(), Scheme::Light);
    // guise defaults a theme file with no `scheme` key to dark.
    spec.json = r##"{"primary": "#268bd2"}"##.into();
    assert_eq!(spec.effective_scheme(), Scheme::Dark);

    // An unknown preset is ignored rather than fatal — a file written by a
    // newer Tailor still opens.
    spec.json = String::new();
    spec.preset = "nonesuch".into();
    assert_eq!(spec.effective_scheme(), Scheme::Light);
    assert!(!spec.is_overridden());
  }

  #[test]
  fn the_theme_survives_a_round_trip_and_older_files_still_open() {
    let mut project = Project::new("Demo");
    project.theme.preset = "nord".into();
    project.theme.json = r#"{"scheme": "dark"}"#.into();
    let text = project.to_json().unwrap();
    let back = Project::from_json(&text).unwrap();
    assert_eq!(back.theme, project.theme);

    // The two fields are skipped when empty, so a project that never set them
    // writes the file it always wrote — and one written before they existed
    // still loads.
    let plain = Project::new("Demo").to_json().unwrap();
    let value: serde_json::Value = serde_json::from_str(&plain).unwrap();
    let theme = value.get("theme").expect("a theme block");
    assert!(
      theme.get("preset").is_none(),
      "empty preset should not be written"
    );
    assert!(
      theme.get("json").is_none(),
      "empty theme file should not be written"
    );
    assert_eq!(
      Project::from_json(&plain).unwrap().theme,
      ThemeSpec::default()
    );
  }

  /// The model is guise-free on purpose, so [`PRESETS`] is a copy. This reads
  /// guise's own source and fails when the copy drifts — the same ratchet the
  /// catalog coverage test uses.
  #[test]
  fn presets_match_guise() {
    let path =
      std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../guise/src/theme/presets.rs");
    let source =
      std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));

    // `pub const PRESET_NAMES: [&str; 6] = [ "catppuccin", ... ];`
    let start = source
      .find("PRESET_NAMES")
      .expect("guise names its presets");
    let list = &source[start..];
    let open = list.find('[').expect("a list");
    let open = open + 1 + list[open + 1..].find('[').expect("the values");
    let close = list[open..].find(']').expect("a closed list") + open;
    let names: Vec<&str> = list[open + 1..close]
      .split(',')
      .map(|piece| piece.trim().trim_matches('"'))
      .filter(|piece| !piece.is_empty())
      .collect();

    let ours: Vec<&str> = THEME_PRESETS.iter().map(|preset| preset.id).collect();
    assert_eq!(
      ours, names,
      "PRESETS has drifted from guise's PRESET_NAMES — update the table"
    );

    // And each one's scheme, which is the half a name list cannot carry.
    for preset in THEME_PRESETS {
      let at = source
        .find(&format!("pub fn {}() -> Theme {{", preset.rust))
        .unwrap_or_else(|| panic!("guise no longer defines {}()", preset.rust));
      let body = &source[at..at + 80];
      let expected = match preset.scheme {
        Scheme::Dark => "Theme::dark()",
        Scheme::Light => "Theme::light()",
      };
      assert!(
        body.contains(expected),
        "{} is {:?} here but not in guise",
        preset.id,
        preset.scheme
      );
    }
  }
}
