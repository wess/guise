//! The editor's own settings — not the project's.
//!
//! Everything here is about how Tailor behaves while you work: which scheme the
//! chrome uses, whether the canvas snaps, which panels are open and how wide.
//! A project carries its own theme and generator settings; those travel with
//! the file.

use serde::{Deserialize, Serialize};
use tailor_model::{Flavor, Scheme};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CanvasMode {
  /// Real components, the way the app will look.
  #[default]
  Design,
  /// Outlines and names only — Android Studio's blueprint view. Useful when
  /// the design is dense enough that the content hides the structure.
  Blueprint,
  /// Design on the left, generated code on the right.
  Split,
  /// Components are live and the canvas stops intercepting clicks.
  Preview,
}

impl CanvasMode {
  pub const ALL: &'static [CanvasMode] = &[
    CanvasMode::Design,
    CanvasMode::Blueprint,
    CanvasMode::Split,
    CanvasMode::Preview,
  ];

  pub fn label(self) -> &'static str {
    match self {
      CanvasMode::Design => "Design",
      CanvasMode::Blueprint => "Blueprint",
      CanvasMode::Split => "Split",
      CanvasMode::Preview => "Preview",
    }
  }

  pub fn icon(self) -> &'static str {
    match self {
      CanvasMode::Design => "layout-dashboard",
      CanvasMode::Blueprint => "grid-2x2",
      CanvasMode::Split => "columns-2",
      CanvasMode::Preview => "play",
    }
  }

  /// Whether the canvas hands clicks to the components rather than using
  /// them for selection.
  pub fn interactive(self) -> bool {
    self == CanvasMode::Preview
  }
}

/// Which panel a splitter resizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Panel {
  Palette,
  Outline,
  Inspector,
  Code,
  Problems,
}

impl Panel {
  pub const ALL: &'static [Panel] = &[
    Panel::Palette,
    Panel::Outline,
    Panel::Inspector,
    Panel::Code,
    Panel::Problems,
  ];

  pub fn label(self) -> &'static str {
    match self {
      Panel::Palette => "Library",
      Panel::Outline => "Outline",
      Panel::Inspector => "Inspector",
      Panel::Code => "Code",
      Panel::Problems => "Problems",
    }
  }

  pub fn icon(self) -> &'static str {
    match self {
      Panel::Palette => "shapes",
      Panel::Outline => "list-tree",
      Panel::Inspector => "sliders-horizontal",
      Panel::Code => "file-code",
      Panel::Problems => "triangle-alert",
    }
  }

  /// The smallest and largest a drag may make it.
  pub fn range(self) -> (f32, f32) {
    match self {
      Panel::Palette => (180.0, 480.0),
      Panel::Outline => (170.0, 440.0),
      Panel::Inspector => (220.0, 560.0),
      Panel::Code => (320.0, 900.0),
      Panel::Problems => (90.0, 520.0),
    }
  }

  /// Whether the splitter runs down the side (a width) or across (a height).
  pub fn vertical(self) -> bool {
    self != Panel::Problems
  }

  /// Which way the pointer has to move to make the panel bigger. Panels on
  /// the right and the bottom grow as the pointer moves *back* towards them.
  pub fn grows_negative(self) -> bool {
    matches!(self, Panel::Inspector | Panel::Code | Panel::Problems)
  }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
  /// The scheme the start screen uses, and the default for a new project.
  pub scheme: Scheme,
  /// Take the start screen's scheme from the OS instead of `scheme`. Off by
  /// default: it is new, and an existing config that never mentioned it meant
  /// the scheme it did name.
  pub follow_system: bool,
  pub canvas_mode: CanvasMode,
  /// Snap free-form drags to the grid.
  pub snap: bool,
  /// Snap them to siblings' edges and centres, drawing a guide where they
  /// catch. Separate from the grid: wanting one and not the other is the
  /// normal case, not an edge case.
  pub snap_objects: bool,
  pub grid: f32,
  /// How far one arrow-key press moves. Shift-arrow uses the grid.
  pub nudge: f32,
  /// New frames place their children at explicit x/y rather than in a flow.
  pub free_form: bool,
  /// Draw the grid behind the artboard.
  pub show_grid: bool,
  /// Outline every node, not only the selected one.
  pub show_outlines: bool,
  /// The flavour new projects generate in.
  pub flavor: Flavor,
  /// Save the open project whenever it changes.
  pub autosave: bool,
  /// Open the live window with the inspector already showing. Off by
  /// default: that window exists to show the design at its real size, and
  /// the inspector is the one thing that takes room away from it.
  pub live_devtools: bool,
  /// Which editor *Open in Editor* jumps to, as a key into [`EDITORS`].
  #[serde(default = "default_editor")]
  pub editor: String,

  // Panel layout. Open/closed and size both persist: having to re-collapse
  // three panels every launch is the kind of friction that makes a tool feel
  // unfinished.
  pub palette_open: bool,
  pub outline_open: bool,
  pub inspector_open: bool,
  pub problems_open: bool,
  pub palette_width: f32,
  pub outline_width: f32,
  pub inspector_width: f32,
  pub code_width: f32,
  pub problems_height: f32,
  /// Inspector sections the user has folded away, by key.
  pub folded: Vec<String>,
}

/// The editors Tailor knows how to jump into, and the command each takes.
///
/// `{file}`, `{line}` and `{column}` are filled in. They are all the same
/// shape — a path and a position — which is why this is a table and not a
/// trait: adding an editor is a row.
pub const EDITORS: &[(&str, &str, &str)] = &[
  ("zed", "Zed", "zed {file}:{line}:{column}"),
  ("vscode", "VS Code", "code --goto {file}:{line}:{column}"),
  ("sublime", "Sublime Text", "subl {file}:{line}:{column}"),
  (
    "intellij",
    "IntelliJ",
    "idea --line {line} --column {column} {file}",
  ),
  ("emacs", "Emacs", "emacsclient +{line}:{column} {file}"),
  ("vim", "Neovim", "nvim +{line} {file}"),
];

fn default_editor() -> String {
  "zed".into()
}

/// The command line for an editor key, or Zed's if the key is unknown.
pub fn editor_command(key: &str) -> &'static str {
  EDITORS
    .iter()
    .find(|(name, _, _)| *name == key)
    .or_else(|| EDITORS.first())
    .map(|(_, _, command)| *command)
    .unwrap_or("zed {file}:{line}:{column}")
}

/// What to call it in a menu.
pub fn editor_title(key: &str) -> &'static str {
  EDITORS
    .iter()
    .find(|(name, _, _)| *name == key)
    .map(|(_, title, _)| *title)
    .unwrap_or("your editor")
}

impl Default for Settings {
  fn default() -> Self {
    Settings {
      scheme: Scheme::Dark,
      follow_system: false,
      canvas_mode: CanvasMode::Design,
      snap: true,
      snap_objects: true,
      grid: 8.0,
      nudge: 1.0,
      free_form: false,
      show_grid: true,
      show_outlines: false,
      flavor: Flavor::Plain,
      autosave: false,
      live_devtools: false,
      editor: default_editor(),
      palette_open: true,
      outline_open: true,
      inspector_open: true,
      problems_open: false,
      palette_width: 260.0,
      outline_width: 248.0,
      inspector_width: 300.0,
      code_width: 520.0,
      problems_height: 180.0,
      folded: Vec::new(),
    }
  }
}

impl Settings {
  pub fn load() -> Self {
    std::fs::read_to_string(crate::paths::settings_file())
      .ok()
      .and_then(|text| serde_json::from_str(&text).ok())
      .unwrap_or_default()
  }

  /// Write the settings back. A failure here is not worth interrupting the
  /// user over — the setting stays applied for this session either way.
  pub fn save(&self) {
    crate::paths::ensure_config_dir();
    if let Ok(text) = serde_json::to_string_pretty(self) {
      let _ = std::fs::write(crate::paths::settings_file(), text);
    }
  }

  pub fn is_open(&self, panel: Panel) -> bool {
    match panel {
      Panel::Palette => self.palette_open,
      Panel::Outline => self.outline_open,
      Panel::Inspector => self.inspector_open,
      Panel::Problems => self.problems_open,
      // The code panel is not a panel you open; Split mode is.
      Panel::Code => self.canvas_mode == CanvasMode::Split,
    }
  }

  pub fn set_open(&mut self, panel: Panel, open: bool) {
    match panel {
      Panel::Palette => self.palette_open = open,
      Panel::Outline => self.outline_open = open,
      Panel::Inspector => self.inspector_open = open,
      Panel::Problems => self.problems_open = open,
      Panel::Code => {
        self.canvas_mode = if open {
          CanvasMode::Split
        } else {
          CanvasMode::Design
        }
      }
    }
  }

  pub fn size(&self, panel: Panel) -> f32 {
    match panel {
      Panel::Palette => self.palette_width,
      Panel::Outline => self.outline_width,
      Panel::Inspector => self.inspector_width,
      Panel::Code => self.code_width,
      Panel::Problems => self.problems_height,
    }
  }

  /// Set a panel's size, clamped to what its splitter allows.
  pub fn set_size(&mut self, panel: Panel, value: f32) {
    let (min, max) = panel.range();
    let value = value.clamp(min, max);
    match panel {
      Panel::Palette => self.palette_width = value,
      Panel::Outline => self.outline_width = value,
      Panel::Inspector => self.inspector_width = value,
      Panel::Code => self.code_width = value,
      Panel::Problems => self.problems_height = value,
    }
  }

  /// Clamp everything a drag or a hand-edited file could have pushed out of
  /// range, so no panel is ever too narrow to grab.
  pub fn sanitized(mut self) -> Self {
    for panel in Panel::ALL {
      let size = self.size(*panel);
      self.set_size(*panel, size);
    }
    self.grid = self.grid.clamp(1.0, 64.0);
    self.nudge = self.nudge.clamp(0.5, 64.0);
    self
  }

  /// Whether an inspector section is folded away.
  pub fn is_folded(&self, key: &str) -> bool {
    self.folded.iter().any(|folded| folded == key)
  }

  pub fn toggle_folded(&mut self, key: &str) {
    match self.folded.iter().position(|folded| folded == key) {
      Some(index) => {
        self.folded.remove(index);
      }
      None => self.folded.push(key.to_string()),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn settings_round_trip() {
    let settings = Settings {
      snap: false,
      canvas_mode: CanvasMode::Blueprint,
      ..Settings::default()
    };
    let text = serde_json::to_string(&settings).unwrap();
    assert_eq!(serde_json::from_str::<Settings>(&text).unwrap(), settings);
  }

  #[test]
  fn a_partial_file_keeps_the_defaults() {
    let settings: Settings = serde_json::from_str(r#"{"snap": false}"#).unwrap();
    assert!(!settings.snap);
    assert!(settings.snap_objects);
    assert_eq!(settings.grid, 8.0);
    assert_eq!(settings.nudge, 1.0);
    assert!(!settings.free_form);
    assert!(settings.palette_open);
  }

  #[test]
  fn panel_sizes_are_clamped_to_their_range() {
    let settings = Settings {
      palette_width: 4000.0,
      problems_height: 1.0,
      grid: 0.0,
      ..Settings::default()
    };
    let settings = settings.sanitized();
    assert_eq!(settings.palette_width, Panel::Palette.range().1);
    assert_eq!(settings.problems_height, Panel::Problems.range().0);
    assert_eq!(settings.grid, 1.0);
  }

  #[test]
  fn a_panel_reads_and_writes_through_its_enum() {
    let mut settings = Settings::default();
    for panel in Panel::ALL {
      settings.set_size(*panel, 10_000.0);
      assert_eq!(settings.size(*panel), panel.range().1);
    }
    assert!(settings.is_open(Panel::Palette));
    settings.set_open(Panel::Palette, false);
    assert!(!settings.is_open(Panel::Palette));

    // The code panel is Split mode wearing a panel's clothes.
    settings.set_open(Panel::Code, true);
    assert_eq!(settings.canvas_mode, CanvasMode::Split);
    assert!(settings.is_open(Panel::Code));
  }

  #[test]
  fn folded_sections_toggle_and_persist() {
    let mut settings = Settings::default();
    assert!(!settings.is_folded("style"));
    settings.toggle_folded("style");
    assert!(settings.is_folded("style"));

    let text = serde_json::to_string(&settings).unwrap();
    let parsed: Settings = serde_json::from_str(&text).unwrap();
    assert!(parsed.is_folded("style"));

    settings.toggle_folded("style");
    assert!(!settings.is_folded("style"));
  }

  #[test]
  fn only_preview_hands_clicks_to_the_components() {
    assert!(CanvasMode::Preview.interactive());
    assert!(!CanvasMode::Design.interactive());
    assert!(!CanvasMode::Split.interactive());
  }
}

#[cfg(test)]
mod editor_tests {
  use super::*;

  /// The picker in the app mirrors this table. A row without a usable command
  /// is a menu item that silently does nothing.
  #[test]
  fn every_editor_has_a_command_and_the_default_resolves() {
    for (key, title, command) in EDITORS {
      assert!(!title.is_empty(), "{key} has no title");
      assert!(command.contains("{file}"), "{key} never names the file");
      assert!(command.contains("{line}"), "{key} never names the line");
      assert_eq!(editor_command(key), *command);
      assert_eq!(editor_title(key), *title);
    }
    // An unknown key falls back rather than opening nothing at all.
    assert_eq!(editor_command("nano"), EDITORS[0].2);
    assert_eq!(editor_command(&Settings::default().editor), EDITORS[0].2);
  }
}
