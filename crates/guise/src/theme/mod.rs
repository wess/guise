//! The theme: palette + sizing tokens + color scheme, exposed as a gpui
//! `Global` so any component can resolve themed values during render.
//!
//! Mirrors Mantine's `MantineProvider` model — a single source of truth for
//! colors, spacing, radius and typography, with semantic colors derived from
//! the active light/dark [`ColorScheme`].

mod color;
mod css;
mod palette;
mod tokens;

pub use color::Color;
pub use css::{css, hsl, hsla, rgb, rgba, CssColorError};
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
    /// Optional CSS-color overrides for the scheme-derived semantic colors and
    /// the primary accent. When set, the matching getter returns the override.
    /// Set them with the `with_*` builders (e.g. `Theme::dark().with_body(..)`).
    pub overrides: Overrides,
}

/// Per-theme semantic color overrides (opaque). `None` means "use the
/// scheme-derived default". Populated via the `Theme::with_*` builders.
#[derive(Debug, Clone, Copy, Default)]
pub struct Overrides {
    pub primary: Option<Color>,
    pub body: Option<Color>,
    pub surface: Option<Color>,
    pub surface_hover: Option<Color>,
    pub text: Option<Color>,
    pub dimmed: Option<Color>,
    pub border: Option<Color>,
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
            overrides: Overrides::default(),
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

    // --- CSS-color overrides (builder) -------------------------------------
    //
    // Each accepts anything convertible to an `Hsla` — notably the `color!`
    // macro and `css(..)` output, but also a palette `Color`. Alpha is dropped
    // (semantic colors are opaque). Named `with_*` to avoid clashing with the
    // same-named getters.

    /// Override the primary accent color.
    pub fn with_primary(mut self, color: impl Into<gpui::Hsla>) -> Self {
        self.overrides.primary = Some(Color::from_hsla(color.into()));
        self
    }

    /// Override the app/window background.
    pub fn with_body(mut self, color: impl Into<gpui::Hsla>) -> Self {
        self.overrides.body = Some(Color::from_hsla(color.into()));
        self
    }

    /// Override the raised-surface background.
    pub fn with_surface(mut self, color: impl Into<gpui::Hsla>) -> Self {
        self.overrides.surface = Some(Color::from_hsla(color.into()));
        self
    }

    /// Override the subtle hover/recessed fill.
    pub fn with_surface_hover(mut self, color: impl Into<gpui::Hsla>) -> Self {
        self.overrides.surface_hover = Some(Color::from_hsla(color.into()));
        self
    }

    /// Override the primary body-text color.
    pub fn with_text(mut self, color: impl Into<gpui::Hsla>) -> Self {
        self.overrides.text = Some(Color::from_hsla(color.into()));
        self
    }

    /// Override the secondary/dimmed text color.
    pub fn with_dimmed(mut self, color: impl Into<gpui::Hsla>) -> Self {
        self.overrides.dimmed = Some(Color::from_hsla(color.into()));
        self
    }

    /// Override the default border/divider color.
    pub fn with_border(mut self, color: impl Into<gpui::Hsla>) -> Self {
        self.overrides.border = Some(Color::from_hsla(color.into()));
        self
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
        self.overrides
            .primary
            .unwrap_or_else(|| self.color(self.primary_color, self.primary_shade()))
    }

    // --- Semantic colors (scheme-aware) ------------------------------------

    /// The app/window background.
    pub fn body(&self) -> Color {
        self.overrides.body.unwrap_or_else(|| match self.scheme {
            ColorScheme::Light => self.white,
            ColorScheme::Dark => self.color(ColorName::Dark, 7),
        })
    }

    /// Raised surface background (Paper, Card, Menu, ...).
    pub fn surface(&self) -> Color {
        self.overrides.surface.unwrap_or_else(|| match self.scheme {
            ColorScheme::Light => self.white,
            ColorScheme::Dark => self.color(ColorName::Dark, 6),
        })
    }

    /// A subtly recessed/hover fill.
    pub fn surface_hover(&self) -> Color {
        self.overrides
            .surface_hover
            .unwrap_or_else(|| match self.scheme {
                ColorScheme::Light => self.color(ColorName::Gray, 0),
                ColorScheme::Dark => self.color(ColorName::Dark, 5),
            })
    }

    /// Primary body text.
    pub fn text(&self) -> Color {
        self.overrides.text.unwrap_or_else(|| match self.scheme {
            ColorScheme::Light => self.color(ColorName::Dark, 9),
            ColorScheme::Dark => self.color(ColorName::Dark, 0),
        })
    }

    /// Secondary / dimmed text.
    pub fn dimmed(&self) -> Color {
        self.overrides.dimmed.unwrap_or_else(|| match self.scheme {
            ColorScheme::Light => self.color(ColorName::Gray, 6),
            ColorScheme::Dark => self.color(ColorName::Dark, 2),
        })
    }

    /// Default border / divider color.
    pub fn border(&self) -> Color {
        self.overrides.border.unwrap_or_else(|| match self.scheme {
            ColorScheme::Light => self.color(ColorName::Gray, 3),
            ColorScheme::Dark => self.color(ColorName::Dark, 4),
        })
    }
}

/// Read the active theme. Panics if [`Theme::init`] was never called.
pub fn theme(cx: &App) -> &Theme {
    cx.global::<Theme>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_override_replaces_scheme_default() {
        let base = Theme::light();
        let themed = Theme::light()
            .with_primary(css::rgb(200, 10, 10))
            .with_body(css::css("#0b0b0f").unwrap());

        assert!(themed.overrides.primary.is_some());
        assert_ne!(themed.primary(), base.primary());
        assert_ne!(themed.body(), base.body());
        // Untouched semantics still fall back to the scheme default.
        assert_eq!(themed.text(), base.text());
    }
}
