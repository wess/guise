//! The theme: palette + sizing tokens + color scheme, exposed as a gpui
//! `Global` so any component can resolve themed values during render.
//!
//! Mirrors Mantine's `MantineProvider` model — a single source of truth for
//! colors, spacing, radius and typography, with semantic colors derived from
//! the active light/dark [`ColorScheme`].

mod color;
mod palette;
mod tokens;

pub use color::Color;
pub use palette::{mantine, ColorName, Palette, Shades};
pub use tokens::{Scale, Size};

use gpui::{App, Global, SharedString};

/// Light or dark surface treatment. Drives every semantic color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorScheme {
    Light,
    Dark,
}

impl ColorScheme {
    pub fn is_dark(self) -> bool {
        matches!(self, ColorScheme::Dark)
    }

    /// The opposite scheme.
    pub fn toggled(self) -> Self {
        match self {
            ColorScheme::Light => ColorScheme::Dark,
            ColorScheme::Dark => ColorScheme::Light,
        }
    }
}

/// The active theme. Install once with [`Theme::init`], read with [`theme`].
#[derive(Debug, Clone)]
pub struct Theme {
    pub scheme: ColorScheme,
    pub palette: Palette,
    /// The color used for default-variant filled controls.
    pub primary_color: ColorName,
    /// Shade index of the primary color in light / dark mode.
    pub primary_shade_light: usize,
    pub primary_shade_dark: usize,
    pub white: Color,
    pub black: Color,
    pub spacing: Scale,
    pub radius: Scale,
    pub font_size: Scale,
    /// Default corner radius for components that don't specify one.
    pub default_radius: Size,
    pub line_height: f32,
    pub font_family: SharedString,
}

impl Global for Theme {}

impl Default for Theme {
    fn default() -> Self {
        Theme::light()
    }
}

impl Theme {
    pub fn light() -> Self {
        Theme {
            scheme: ColorScheme::Light,
            palette: mantine(),
            primary_color: ColorName::Blue,
            primary_shade_light: 6,
            primary_shade_dark: 8,
            white: Color::hex("#ffffff"),
            black: Color::hex("#000000"),
            spacing: Scale::spacing(),
            radius: Scale::radius(),
            font_size: Scale::font_size(),
            default_radius: Size::Sm,
            line_height: 1.55,
            font_family: SharedString::new_static("Helvetica"),
        }
    }

    pub fn dark() -> Self {
        Theme {
            scheme: ColorScheme::Dark,
            ..Theme::light()
        }
    }

    /// Install the theme as the app-global, replacing any existing one.
    pub fn init(self, cx: &mut App) {
        cx.set_global(self);
    }

    // --- Token lookups -----------------------------------------------------

    pub fn spacing(&self, size: Size) -> f32 {
        self.spacing.get(size)
    }

    pub fn radius(&self, size: Size) -> f32 {
        self.radius.get(size)
    }

    pub fn font_size(&self, size: Size) -> f32 {
        self.font_size.get(size)
    }

    // --- Color resolution --------------------------------------------------

    /// A single shade of a named color.
    pub fn color(&self, name: ColorName, shade: usize) -> Color {
        self.palette.get(name, shade)
    }

    /// The active primary-color shade for the current scheme.
    pub fn primary_shade(&self) -> usize {
        match self.scheme {
            ColorScheme::Light => self.primary_shade_light,
            ColorScheme::Dark => self.primary_shade_dark,
        }
    }

    /// The primary color at its scheme-appropriate shade.
    pub fn primary(&self) -> Color {
        self.color(self.primary_color, self.primary_shade())
    }

    // --- Semantic colors (scheme-aware) ------------------------------------

    /// The app/window background.
    pub fn body(&self) -> Color {
        match self.scheme {
            ColorScheme::Light => self.white,
            ColorScheme::Dark => self.color(ColorName::Dark, 7),
        }
    }

    /// Raised surface background (Paper, Card, Menu, ...).
    pub fn surface(&self) -> Color {
        match self.scheme {
            ColorScheme::Light => self.white,
            ColorScheme::Dark => self.color(ColorName::Dark, 6),
        }
    }

    /// A subtly recessed/hover fill.
    pub fn surface_hover(&self) -> Color {
        match self.scheme {
            ColorScheme::Light => self.color(ColorName::Gray, 0),
            ColorScheme::Dark => self.color(ColorName::Dark, 5),
        }
    }

    /// Primary body text.
    pub fn text(&self) -> Color {
        match self.scheme {
            ColorScheme::Light => self.color(ColorName::Dark, 9),
            ColorScheme::Dark => self.color(ColorName::Dark, 0),
        }
    }

    /// Secondary / dimmed text.
    pub fn dimmed(&self) -> Color {
        match self.scheme {
            ColorScheme::Light => self.color(ColorName::Gray, 6),
            ColorScheme::Dark => self.color(ColorName::Dark, 2),
        }
    }

    /// Default border / divider color.
    pub fn border(&self) -> Color {
        match self.scheme {
            ColorScheme::Light => self.color(ColorName::Gray, 3),
            ColorScheme::Dark => self.color(ColorName::Dark, 4),
        }
    }
}

/// Read the active theme. Panics if [`Theme::init`] was never called.
pub fn theme(cx: &App) -> &Theme {
    cx.global::<Theme>()
}
