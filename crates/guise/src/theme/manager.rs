//! `ThemeManager` — the registry, the selection, and the OS-appearance follow.
//!
//! Switching a theme was always a one-liner: write the [`Theme`] global, ask
//! for a redraw, and every component restyles because it reads `theme(cx)` at
//! paint time. That is why guise shipped for so long without a manager. What
//! the one-liner does not answer is everything *around* the switch — which
//! themes exist, which one the user picked last time, what "follow the system"
//! means in an app that ships four dark themes — and every consumer ended up
//! writing that part again.
//!
//! So this owns exactly that, and nothing about how a theme looks:
//!
//! - a **registry** of `id -> `[`ThemeEntry`], seeded with `light` and `dark`,
//!   extended with the prebuilt presets, host themes, or a directory of JSON
//!   theme files,
//! - a **choice** ([`ThemeChoice`]) that is either one theme by id or *follow
//!   the system*, which resolves through the registry's light/dark pair,
//! - the **window appearance watch** that keeps that resolution honest when the
//!   OS flips at sundown.
//!
//! ```ignore
//! use guise::prelude::*;
//!
//! ThemeManager::new()
//!     .with_presets()
//!     .with_dir(config_dir.join("themes"))     // *.json, see `Theme::from_json`
//!     .choice(saved_choice.parse().unwrap())   // "system" | "theme:dracula"
//!     .install(cx);
//!
//! // once per window, so `ThemeChoice::System` tracks the OS:
//! ThemeManager::watch(window, cx);
//! ```
//!
//! Persistence stays with the host, the way [`crate::settings`] leaves the
//! schema with the host: [`ThemeChoice`] is `Display` + `FromStr`, so it round
//! trips through one string in whatever config file the app already writes.
//! Reading a *themes directory* is the one thing here that touches the disk —
//! an app that would rather bring its own bytes (an `include_str!`, a file it
//! already read) uses [`ThemeManager::register_json`] instead.

use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use gpui::{App, Global, SharedString, Subscription, Window, WindowAppearance};

use super::{ColorScheme, Theme, ThemeJsonError, PRESET_NAMES};

/// Where a registered theme came from. Carried for the UI: a picker can group
/// the app's own themes apart from the ones a user dropped in a folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThemeSource {
  /// `Theme::light()` / `Theme::dark()` or one of the prebuilt presets.
  Builtin,
  /// Registered by the host in code.
  Custom,
  /// Parsed from a JSON theme file.
  File(PathBuf),
}

/// One theme in the registry: the theme itself plus the identity a picker and
/// a config file need.
#[derive(Debug, Clone)]
pub struct ThemeEntry {
  /// Stable key — what a saved [`ThemeChoice`] refers to. Lowercase by
  /// convention; a file's stem becomes its id.
  pub id: SharedString,
  /// Display name. Derived from the id when nothing better is given.
  pub name: SharedString,
  pub source: ThemeSource,
  pub theme: Theme,
}

impl ThemeEntry {
  /// A host theme, named after its id (`"midnight blue"` from `midnight-blue`).
  pub fn new(id: impl Into<SharedString>, theme: Theme) -> Self {
    let id = id.into();
    let name = title_case(&id).into();
    ThemeEntry {
      id,
      name,
      source: ThemeSource::Custom,
      theme,
    }
  }

  pub fn name(mut self, name: impl Into<SharedString>) -> Self {
    self.name = name.into();
    self
  }

  pub fn source(mut self, source: ThemeSource) -> Self {
    self.source = source;
    self
  }

  /// The scheme this theme is a variation of — what "follow the system" and a
  /// light/dark toggle sort on.
  pub fn scheme(&self) -> ColorScheme {
    self.theme.scheme
  }
}

/// What the app is wearing.
///
/// `Display`/`FromStr` round trip through one string so a host can persist it
/// in its own config: `"system"`, or `"theme:<id>"`. A bare id parses too, so a
/// config that just says `dracula` works.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ThemeChoice {
  /// Follow the OS appearance through the registry's light/dark pair.
  #[default]
  System,
  /// One theme, by id. Stays put when the OS flips.
  Fixed(SharedString),
}

impl fmt::Display for ThemeChoice {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      ThemeChoice::System => f.write_str("system"),
      ThemeChoice::Fixed(id) => write!(f, "theme:{id}"),
    }
  }
}

impl FromStr for ThemeChoice {
  type Err = std::convert::Infallible;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let s = s.trim();
    Ok(match s {
      "" | "system" | "auto" => ThemeChoice::System,
      _ => ThemeChoice::Fixed(s.strip_prefix("theme:").unwrap_or(s).to_string().into()),
    })
  }
}

/// A theme file the registry could not load. [`ThemeManager::load_dir`] returns
/// these rather than failing the whole directory — one bad file in a user's
/// themes folder should not cost them the other nine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThemeLoadError {
  /// The file could not be read.
  Io(PathBuf, String),
  /// The file was read but did not parse.
  Json(PathBuf, ThemeJsonError),
}

impl ThemeLoadError {
  pub fn path(&self) -> &Path {
    match self {
      ThemeLoadError::Io(path, _) | ThemeLoadError::Json(path, _) => path,
    }
  }
}

impl fmt::Display for ThemeLoadError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      ThemeLoadError::Io(path, why) => write!(f, "{}: {why}", path.display()),
      ThemeLoadError::Json(path, why) => write!(f, "{}: {why}", path.display()),
    }
  }
}

impl std::error::Error for ThemeLoadError {}

/// The theme registry and the active selection, installed as a gpui global
/// alongside [`Theme`] itself. See the [module docs](self).
pub struct ThemeManager {
  entries: Vec<ThemeEntry>,
  choice: ThemeChoice,
  light: SharedString,
  dark: SharedString,
  system: ColorScheme,
  watches: Vec<Subscription>,
}

impl Global for ThemeManager {}

impl fmt::Debug for ThemeManager {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("ThemeManager")
      .field("entries", &self.entries.len())
      .field("choice", &self.choice)
      .field("light", &self.light)
      .field("dark", &self.dark)
      .field("system", &self.system)
      .finish()
  }
}

impl Default for ThemeManager {
  fn default() -> Self {
    ThemeManager::new()
  }
}

impl ThemeManager {
  /// A registry holding just `light` and `dark`, following the system.
  ///
  /// Those two ids are the light/dark pair `ThemeChoice::System` resolves
  /// through, so an app with its own two-theme identity can re-register them
  /// (same ids, its own themes and names) and get the follow behaviour for
  /// free. To use a *different* pair, see [`ThemeManager::pair`].
  pub fn new() -> Self {
    ThemeManager {
      entries: vec![
        ThemeEntry::new("light", Theme::light())
          .name("Light")
          .source(ThemeSource::Builtin),
        ThemeEntry::new("dark", Theme::dark())
          .name("Dark")
          .source(ThemeSource::Builtin),
      ],
      choice: ThemeChoice::System,
      light: "light".into(),
      dark: "dark".into(),
      system: ColorScheme::Light,
      watches: Vec::new(),
    }
  }

  /// Add the six prebuilt presets (see [`PRESET_NAMES`]).
  pub fn with_presets(mut self) -> Self {
    for id in PRESET_NAMES {
      if let Some(theme) = Theme::preset(id) {
        self.register(
          ThemeEntry::new(id, theme)
            .name(preset_name(id))
            .source(ThemeSource::Builtin),
        );
      }
    }
    self
  }

  /// Add one theme (builder form of [`ThemeManager::register`]).
  pub fn with(mut self, entry: ThemeEntry) -> Self {
    self.register(entry);
    self
  }

  /// Load a directory of `*.json` theme files, ignoring the ones that fail.
  /// Use [`ThemeManager::load_dir`] when you want to report them.
  pub fn with_dir(mut self, dir: impl AsRef<Path>) -> Self {
    self.load_dir(dir);
    self
  }

  /// Set the choice (builder form of [`ThemeManager::set_choice`]).
  pub fn choice(mut self, choice: ThemeChoice) -> Self {
    self.choice = choice;
    self
  }

  /// Point [`ThemeChoice::System`] at a different pair of registered themes.
  pub fn pair(mut self, light: impl Into<SharedString>, dark: impl Into<SharedString>) -> Self {
    self.light = light.into();
    self.dark = dark.into();
    self
  }

  // --- registry ----------------------------------------------------------

  /// Add a theme, replacing any entry with the same id.
  pub fn register(&mut self, entry: ThemeEntry) {
    match self.entries.iter_mut().find(|e| e.id == entry.id) {
      Some(existing) => *existing = entry,
      None => self.entries.push(entry),
    }
  }

  /// Register a theme from JSON the host already has in hand — an
  /// `include_str!`, a downloaded file, a string from its own config.
  pub fn register_json(
    &mut self,
    id: impl Into<SharedString>,
    source: &str,
  ) -> Result<(), ThemeJsonError> {
    let theme = Theme::from_json(source)?;
    let id = id.into();
    let name = super::json::json_name(source).unwrap_or_else(|| title_case(&id));
    self.register(ThemeEntry::new(id, theme).name(name));
    Ok(())
  }

  /// Load every `*.json` file in `dir` as a theme, keyed by file stem, and
  /// return the files that failed. A missing directory is not an error — apps
  /// point this at a folder the user may never create.
  pub fn load_dir(&mut self, dir: impl AsRef<Path>) -> Vec<ThemeLoadError> {
    let mut failures = Vec::new();
    let Ok(read) = std::fs::read_dir(dir.as_ref()) else {
      return failures;
    };

    // Sorted, so a directory listing's arbitrary order doesn't reorder a picker
    // between launches.
    let mut paths: Vec<PathBuf> = read
      .filter_map(|entry| entry.ok())
      .map(|entry| entry.path())
      .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
      .collect();
    paths.sort();

    for path in paths {
      let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
        continue;
      };
      let source = match std::fs::read_to_string(&path) {
        Ok(source) => source,
        Err(e) => {
          failures.push(ThemeLoadError::Io(path.clone(), e.to_string()));
          continue;
        }
      };
      match Theme::from_json(&source) {
        Ok(theme) => {
          let id: SharedString = stem.to_lowercase().into();
          let name = super::json::json_name(&source).unwrap_or_else(|| title_case(stem));
          self.register(
            ThemeEntry::new(id, theme)
              .name(name)
              .source(ThemeSource::File(path.clone())),
          );
        }
        Err(e) => failures.push(ThemeLoadError::Json(path.clone(), e)),
      }
    }
    failures
  }

  /// Every registered theme, in registration order.
  pub fn entries(&self) -> &[ThemeEntry] {
    &self.entries
  }

  pub fn entry(&self, id: &str) -> Option<&ThemeEntry> {
    self.entries.iter().find(|e| e.id == id)
  }

  // --- selection ---------------------------------------------------------

  /// The active choice. Persist it with `to_string()`.
  pub fn selection(&self) -> &ThemeChoice {
    &self.choice
  }

  /// The last OS appearance a watched window reported.
  pub fn system_scheme(&self) -> ColorScheme {
    self.system
  }

  /// The entry the current choice resolves to. Falls back to the light/dark
  /// pair, then to the first entry, so a stale id in a config file degrades to
  /// a sane theme instead of a panic.
  pub fn resolved(&self) -> &ThemeEntry {
    let id = match &self.choice {
      ThemeChoice::Fixed(id) => id.clone(),
      ThemeChoice::System => self.pair_id(self.system),
    };
    self
      .entry(&id)
      .or_else(|| self.entry(&self.pair_id(self.system)))
      .unwrap_or_else(|| {
        self
          .entries
          .first()
          .expect("a ThemeManager always holds at least one theme")
      })
  }

  /// The id of the theme currently in use.
  pub fn resolved_id(&self) -> &SharedString {
    &self.resolved().id
  }

  fn pair_id(&self, scheme: ColorScheme) -> SharedString {
    match scheme {
      ColorScheme::Light => self.light.clone(),
      ColorScheme::Dark => self.dark.clone(),
    }
  }

  // --- installed-global operations ---------------------------------------

  /// Install as the global and apply the resolved theme.
  pub fn install(self, cx: &mut App) {
    cx.set_global(self);
    ThemeManager::apply(cx);
  }

  /// Write the resolved theme into the [`Theme`] global and redraw. Every
  /// mutator below ends here; call it directly after editing the manager
  /// through `cx.global_mut::<ThemeManager>()`.
  pub fn apply(cx: &mut App) {
    let theme = match cx.try_global::<ThemeManager>() {
      Some(manager) => manager.resolved().theme.clone(),
      None => return,
    };
    cx.set_global(theme);
    cx.refresh_windows();
  }

  /// Wear one registered theme. Unknown ids are ignored (and reported as
  /// `false`) so a picker can't strand the app on a theme that isn't there.
  pub fn select(cx: &mut App, id: impl Into<SharedString>) -> bool {
    let id = id.into();
    let known = cx
      .try_global::<ThemeManager>()
      .is_some_and(|m| m.entry(&id).is_some());
    if known {
      ThemeManager::set_choice(cx, ThemeChoice::Fixed(id));
    }
    known
  }

  /// Follow the OS appearance again.
  pub fn follow_system(cx: &mut App) {
    ThemeManager::set_choice(cx, ThemeChoice::System);
  }

  /// Set the choice and apply it.
  pub fn set_choice(cx: &mut App, choice: ThemeChoice) {
    if cx.has_global::<ThemeManager>() {
      cx.global_mut::<ThemeManager>().choice = choice;
      ThemeManager::apply(cx);
    }
  }

  /// Swap to the light/dark counterpart of what is showing — what a
  /// light/dark toggle button does. Leaves [`ThemeChoice::System`]: an explicit
  /// toggle is an explicit choice.
  pub fn toggle(cx: &mut App) {
    let Some(manager) = cx.try_global::<ThemeManager>() else {
      return;
    };
    let next = manager.pair_id(manager.resolved().scheme().toggled());
    ThemeManager::set_choice(cx, ThemeChoice::Fixed(next));
  }

  /// Record the OS appearance, restyling if the choice follows it. Usually
  /// driven by [`ThemeManager::watch`]; call it directly when the host learns
  /// the appearance some other way.
  pub fn set_system_scheme(cx: &mut App, scheme: ColorScheme) {
    let Some(manager) = cx.try_global::<ThemeManager>() else {
      return;
    };
    if manager.system == scheme {
      return;
    }
    cx.global_mut::<ThemeManager>().system = scheme;
    if cx.global::<ThemeManager>().choice == ThemeChoice::System {
      ThemeManager::apply(cx);
    }
  }

  /// Track a window's appearance, so [`ThemeChoice::System`] follows the OS.
  /// Call once per window, from the window's init. The subscription is held by
  /// the manager global, which outlives every window.
  pub fn watch(window: &mut Window, cx: &mut App) {
    if !cx.has_global::<ThemeManager>() {
      return;
    }
    ThemeManager::set_system_scheme(cx, scheme_of(window.appearance()));
    let watch = window.observe_window_appearance(|window, cx| {
      ThemeManager::set_system_scheme(cx, scheme_of(window.appearance()));
    });
    cx.global_mut::<ThemeManager>().watches.push(watch);
  }
}

/// gpui's four appearances collapse to two: vibrancy is a material, not a
/// scheme, and guise has no separate palette for it.
pub fn scheme_of(appearance: WindowAppearance) -> ColorScheme {
  match appearance {
    WindowAppearance::Dark | WindowAppearance::VibrantDark => ColorScheme::Dark,
    WindowAppearance::Light | WindowAppearance::VibrantLight => ColorScheme::Light,
  }
}

/// The installed manager, if there is one. Unlike [`theme`](super::theme) this
/// is optional: the manager is opt-in, and a picker rendered without one should
/// draw nothing rather than panic.
pub fn manager(cx: &App) -> Option<&ThemeManager> {
  cx.try_global::<ThemeManager>()
}

/// `tokyo-night` -> `Tokyo Night`, for entries that carry no name of their own.
fn title_case(id: &str) -> String {
  let mut out = String::with_capacity(id.len());
  for word in id.split(['-', '_', ' ']).filter(|w| !w.is_empty()) {
    if !out.is_empty() {
      out.push(' ');
    }
    let mut chars = word.chars();
    if let Some(first) = chars.next() {
      out.extend(first.to_uppercase());
      out.push_str(chars.as_str());
    }
  }
  if out.is_empty() {
    id.to_string()
  } else {
    out
  }
}

/// Display names for the presets, whose ids are run together.
fn preset_name(id: &str) -> &'static str {
  match id {
    "catppuccin" => "Catppuccin",
    "nord" => "Nord",
    "tokyonight" => "Tokyo Night",
    "gruvbox" => "Gruvbox",
    "dracula" => "Dracula",
    "solarizedlight" => "Solarized Light",
    _ => "Theme",
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("guise-theme-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
  }

  #[test]
  fn new_seeds_the_light_dark_pair() {
    let manager = ThemeManager::new();
    assert_eq!(manager.entries().len(), 2);
    assert_eq!(manager.entry("dark").unwrap().name, "Dark");
    // Following the system, and the system starts light.
    assert_eq!(manager.resolved_id().as_ref(), "light");
  }

  #[test]
  fn presets_land_with_readable_names() {
    let manager = ThemeManager::new().with_presets();
    assert_eq!(manager.entries().len(), 2 + PRESET_NAMES.len());
    assert_eq!(manager.entry("tokyonight").unwrap().name, "Tokyo Night");
    assert_eq!(manager.entry("nord").unwrap().source, ThemeSource::Builtin);
  }

  #[test]
  fn registering_a_known_id_replaces_it() {
    let mut manager = ThemeManager::new();
    manager.register(ThemeEntry::new("dark", Theme::dracula()).name("Midnight"));
    assert_eq!(manager.entries().len(), 2);
    assert_eq!(manager.entry("dark").unwrap().name, "Midnight");
    // ...and the replacement is what "follow the system into dark" resolves to.
    manager.system = ColorScheme::Dark;
    assert!(manager.resolved().theme.overrides.primary.is_some());
  }

  #[test]
  fn system_choice_follows_the_scheme_and_a_fixed_one_does_not() {
    let mut manager = ThemeManager::new().with_presets();
    manager.system = ColorScheme::Dark;
    assert_eq!(manager.resolved_id().as_ref(), "dark");
    manager.system = ColorScheme::Light;
    assert_eq!(manager.resolved_id().as_ref(), "light");

    manager.choice = ThemeChoice::Fixed("dracula".into());
    manager.system = ColorScheme::Dark;
    assert_eq!(manager.resolved_id().as_ref(), "dracula");
  }

  #[test]
  fn a_custom_pair_redirects_the_system_choice() {
    let manager = ThemeManager::new()
      .with_presets()
      .pair("solarizedlight", "nord");
    assert_eq!(manager.resolved_id().as_ref(), "solarizedlight");
  }

  #[test]
  fn a_stale_id_falls_back_instead_of_panicking() {
    let mut manager = ThemeManager::new();
    manager.choice = ThemeChoice::Fixed("deleted-by-the-user".into());
    manager.system = ColorScheme::Dark;
    assert_eq!(manager.resolved_id().as_ref(), "dark");
  }

  #[test]
  fn choice_round_trips_through_a_string() {
    let cases = [
      (ThemeChoice::System, "system"),
      (ThemeChoice::Fixed("dracula".into()), "theme:dracula"),
    ];
    for (choice, text) in cases {
      assert_eq!(choice.to_string(), text);
      assert_eq!(text.parse::<ThemeChoice>().unwrap(), choice);
    }
    // A hand-written config can be terser than what we emit.
    assert_eq!(
      "dracula".parse::<ThemeChoice>().unwrap(),
      ThemeChoice::Fixed("dracula".into())
    );
    assert_eq!("".parse::<ThemeChoice>().unwrap(), ThemeChoice::System);
    assert_eq!(
      " auto ".parse::<ThemeChoice>().unwrap(),
      ThemeChoice::System
    );
  }

  #[test]
  fn load_dir_keys_by_stem_and_reports_only_the_bad_files() {
    let dir = scratch("load");
    std::fs::write(
      dir.join("Midnight.json"),
      r##"{"name": "Midnight Blue", "scheme": "dark", "primary": "#7aa2f7"}"##,
    )
    .unwrap();
    std::fs::write(dir.join("plain-jane.json"), r#"{"scheme": "light"}"#).unwrap();
    std::fs::write(dir.join("broken.json"), r##"{"primry": "#fff"}"##).unwrap();
    std::fs::write(dir.join("notes.txt"), "not a theme").unwrap();

    let mut manager = ThemeManager::new();
    let failures = manager.load_dir(&dir);

    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].path(), dir.join("broken.json"));
    assert!(matches!(failures[0], ThemeLoadError::Json(_, _)));

    assert_eq!(manager.entries().len(), 4);
    let midnight = manager.entry("midnight").unwrap();
    assert_eq!(midnight.name, "Midnight Blue");
    assert_eq!(midnight.scheme(), ColorScheme::Dark);
    assert_eq!(
      midnight.source,
      ThemeSource::File(dir.join("Midnight.json"))
    );
    // No `name` key, so the stem is title-cased.
    assert_eq!(manager.entry("plain-jane").unwrap().name, "Plain Jane");

    let _ = std::fs::remove_dir_all(&dir);
  }

  #[test]
  fn a_missing_themes_dir_is_not_an_error() {
    let manager = ThemeManager::new().with_dir("/nonexistent/guise/themes");
    assert_eq!(manager.entries().len(), 2);
  }

  #[test]
  fn register_json_names_from_the_file_then_the_id() {
    let mut manager = ThemeManager::new();
    manager
      .register_json(
        "midnight",
        r##"{"name": "Midnight", "primary": "#7aa2f7"}"##,
      )
      .unwrap();
    manager
      .register_json("plain-jane", r#"{"scheme": "light"}"#)
      .unwrap();
    assert_eq!(manager.entry("midnight").unwrap().name, "Midnight");
    assert_eq!(manager.entry("plain-jane").unwrap().name, "Plain Jane");
    assert!(manager.register_json("bad", r#"{"nope": "x"}"#).is_err());
  }

  #[test]
  fn appearances_collapse_to_two_schemes() {
    assert_eq!(scheme_of(WindowAppearance::VibrantDark), ColorScheme::Dark);
    assert_eq!(
      scheme_of(WindowAppearance::VibrantLight),
      ColorScheme::Light
    );
  }
}
