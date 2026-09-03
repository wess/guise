//! The theme.
//!
//! One theme, not two. guise reads its colours from an app-wide global at the
//! moment a component paints, not at the moment you build it — so there is no
//! way to scope a second theme to the canvas subtree without it leaking. Rather
//! than fight that, Tailor wears the project's theme: switch the project to
//! light and the editor goes light with it, which is also the most honest
//! preview a builder can give you.
//!
//! The chrome keeps its own graphite surface ramp on top, so the panels around
//! the canvas never read as part of the design inside it.

use gpui::{App, Hsla, Window};
use guise::prelude::*;
use guise::theme::{Color, Shades};
use tailor_model::{Scheme, ThemeSpec};
use tailor_store::Settings;

/// A neutral graphite ramp for the editor chrome. Deliberately not the
/// project's palette: the panels around the canvas should never be mistaken
/// for the design inside it.
const GRAPHITE: [&str; 10] = [
  "#C9CBD1", "#AEB1B8", "#93969E", "#6B6E76", "#494C54", "#383A41", "#2A2C32", "#1E2025",
  "#17181C", "#101114",
];

/// The theme Tailor's own interface uses.
pub fn chrome(scheme: Scheme) -> Theme {
  let mut theme = match scheme {
    Scheme::Dark => Theme::dark(),
    Scheme::Light => Theme::light(),
  };
  theme
    .palette
    .set_shades(ColorName::Dark, Shades(GRAPHITE.map(Color::hex)));
  theme.primary_color = ColorName::Blue;
  theme.default_radius = Size::Sm;
  theme.font_family = ".SystemUIFont".into();
  theme
}

/// The theme for an open project: the project's scheme, primary colour,
/// radius, and font, over Tailor's graphite surfaces.
///
/// A preset or a pasted theme file owns the scheme and the colours — that is
/// the whole of what one *is* — so the primary token stops applying when
/// either is set. Radius and font are orthogonal to a palette and stay the
/// project's either way.
pub fn project_theme(spec: &ThemeSpec) -> Theme {
  let mut theme = base(spec);
  if !spec.is_overridden() {
    theme.primary_color = color_of(spec.primary);
  }
  theme.default_radius = size_of(spec.radius);
  if !spec.font.is_empty() {
    theme.font_family = spec.font.clone().into();
  }
  theme
}

/// What the project's theme starts from, in the order [`ThemeSpec`] resolves:
/// a pasted theme file, then a preset, then plain light/dark.
///
/// Whatever wins keeps Tailor's graphite ramp for `ColorName::Dark`, because
/// that ramp is what the *chrome* reads. A preset's semantic colours are
/// overrides and still win for the canvas; the panels around it stay graphite.
fn base(spec: &ThemeSpec) -> Theme {
  let ramp = |mut theme: Theme| {
    theme
      .palette
      .set_shades(ColorName::Dark, Shades(GRAPHITE.map(Color::hex)));
    theme.font_family = ".SystemUIFont".into();
    theme
  };
  if !spec.json.is_empty() {
    if let Ok(theme) = Theme::from_json(&spec.json) {
      return ramp(theme);
    }
  }
  if let Some(preset) = spec.preset_entry() {
    if let Some(theme) = Theme::preset(preset.id) {
      return ramp(theme);
    }
  }
  chrome(spec.scheme)
}

/// Why a project's theme file was ignored, for the inspector to show. `None`
/// when there is no file or it parsed.
pub fn json_error(spec: &ThemeSpec) -> Option<String> {
  if spec.json.trim().is_empty() {
    return None;
  }
  Theme::from_json(&spec.json).err().map(|e| e.to_string())
}

/// The registry ids. `light` and `dark` are the pair `ThemeChoice::System`
/// resolves through, which is why Tailor's own chrome takes those names rather
/// than inventing its own — re-registering the pair is what buys the follow.
pub const LIGHT: &str = "light";
pub const DARK: &str = "dark";
pub const PROJECT: &str = "project";

/// Install the manager that owns the `Theme` global, seeded with Tailor's two
/// chrome themes. Called once at launch, before the window opens.
///
/// Everything after this point goes through the manager rather than writing
/// `Theme` directly: two writers of one global is how a picker and a toggle
/// end up disagreeing about what the app is wearing.
pub fn install_manager(settings: &Settings, cx: &mut App) {
  ThemeManager::new()
    .with(ThemeEntry::new(LIGHT, chrome(Scheme::Light)).name("Light"))
    .with(ThemeEntry::new(DARK, chrome(Scheme::Dark)).name("Dark"))
    .choice(chrome_choice(settings))
    .install(cx);
}

/// Track the window's appearance so `follow_system` means something.
pub fn watch(window: &mut Window, cx: &mut App) {
  ThemeManager::watch(window, cx);
}

/// What the start screen wears: the OS, or the scheme the settings name.
pub fn chrome_choice(settings: &Settings) -> ThemeChoice {
  if settings.follow_system {
    ThemeChoice::System
  } else {
    ThemeChoice::Fixed(id_of(settings.scheme).into())
  }
}

fn id_of(scheme: Scheme) -> &'static str {
  match scheme {
    Scheme::Light => LIGHT,
    Scheme::Dark => DARK,
  }
}

/// Install the project's theme app-wide. Called whenever a project is opened
/// and whenever its theme is edited.
pub fn install(spec: &ThemeSpec, cx: &mut App) {
  let theme = project_theme(spec);
  if cx.has_global::<ThemeManager>() {
    cx.global_mut::<ThemeManager>()
      .register(ThemeEntry::new(PROJECT, theme).name("Project"));
    ThemeManager::select(cx, PROJECT);
  } else {
    // No manager: the entity tests install a bare theme and never open a
    // window. Wearing it directly keeps them working.
    theme.init(cx);
  }
}

/// Back to Tailor's own chrome — the start screen, after a project closes.
pub fn wear_chrome(settings: &Settings, cx: &mut App) {
  if cx.has_global::<ThemeManager>() {
    ThemeManager::set_choice(cx, chrome_choice(settings));
  } else {
    chrome(settings.scheme).init(cx);
  }
}

pub fn color_of(token: tailor_model::ColorToken) -> ColorName {
  use tailor_model::ColorToken as T;
  match token {
    T::Dark => ColorName::Dark,
    T::Gray => ColorName::Gray,
    T::Red => ColorName::Red,
    T::Pink => ColorName::Pink,
    T::Grape => ColorName::Grape,
    T::Violet => ColorName::Violet,
    T::Indigo => ColorName::Indigo,
    T::Blue => ColorName::Blue,
    T::Cyan => ColorName::Cyan,
    T::Teal => ColorName::Teal,
    T::Green => ColorName::Green,
    T::Lime => ColorName::Lime,
    T::Yellow => ColorName::Yellow,
    T::Orange => ColorName::Orange,
  }
}

pub fn size_of(token: tailor_model::SizeToken) -> Size {
  use tailor_model::SizeToken as T;
  match token {
    T::Xs => Size::Xs,
    T::Sm => Size::Sm,
    T::Md => Size::Md,
    T::Lg => Size::Lg,
    T::Xl => Size::Xl,
  }
}

/// The resolved chrome colours a panel needs. Read once at the top of a render,
/// because every one of them is wanted before the first `cx.listener`.
#[derive(Clone, Copy)]
pub struct Chrome {
  pub body: Hsla,
  pub surface: Hsla,
  pub raised: Hsla,
  pub border: Hsla,
  pub text: Hsla,
  pub dimmed: Hsla,
  pub accent: Hsla,
  pub accent_soft: Hsla,
  pub danger: Hsla,
  pub warning: Hsla,
}

pub fn colors(cx: &App) -> Chrome {
  let theme = theme(cx);
  let shade = |index: usize| theme.color(ColorName::Dark, index).hsla();
  let light = theme.scheme == ColorScheme::Light;
  Chrome {
    body: theme.body().hsla(),
    surface: theme.surface().hsla(),
    raised: if light {
      shade(0).opacity(0.4)
    } else {
      shade(6)
    },
    border: theme.border().hsla(),
    text: theme.text().hsla(),
    dimmed: theme.dimmed().hsla(),
    accent: theme.color(ColorName::Blue, 5).hsla(),
    accent_soft: theme.color(ColorName::Blue, 5).alpha(0.16),
    danger: theme.color(ColorName::Red, 6).hsla(),
    warning: theme.color(ColorName::Yellow, 6).hsla(),
  }
}

/// The monospace family the code panel and the numeric fields use.
pub const MONO: &str = "Menlo";

trait Opacity {
  fn opacity(self, alpha: f32) -> Hsla;
}

impl Opacity for Hsla {
  fn opacity(mut self, alpha: f32) -> Hsla {
    self.a = alpha;
    self
  }
}
