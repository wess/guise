//! Shared visual variants. Mantine resolves a `(color, variant)` pair into a
//! background / foreground / border triple that every interactive component
//! (Button, Badge, ActionIcon, ...) draws from. This is that resolver.

use gpui::{Div, Hsla, Styled};

use crate::theme::{ColorName, Size, Theme};

/// Apply an element transform — notably a [`style!`](crate::style) block — to
/// any styled element: `div().apply(style! { … })`.
pub trait StyleExt: Sized {
    /// Run `f` on `self` and return the result. `style!` produces exactly the
    /// `FnOnce(Self) -> Self` this expects.
    fn apply(self, f: impl FnOnce(Self) -> Self) -> Self {
        f(self)
    }
}

impl<T: Styled> StyleExt for T {}

/// Flex helpers that reach into gpui's [`StyleRefinement`] for what the
/// crates.io 0.2.2 `Styled` trait doesn't expose: arbitrary grow/shrink
/// factors, `align-items: stretch`, and `justify-content: space-evenly`.
pub(crate) trait FlexExt: Sized {
    /// Set an arbitrary `flex-grow` factor.
    fn grow(self, factor: f32) -> Self;
    /// Set an arbitrary `flex-shrink` factor.
    fn shrink(self, factor: f32) -> Self;
    /// `align-items: stretch`.
    fn items_stretch(self) -> Self;
    /// `justify-content: space-evenly`.
    fn justify_evenly(self) -> Self;
}

impl FlexExt for Div {
    fn grow(mut self, factor: f32) -> Self {
        self.style().flex_grow = Some(factor);
        self
    }

    fn shrink(mut self, factor: f32) -> Self {
        self.style().flex_shrink = Some(factor);
        self
    }

    fn items_stretch(mut self) -> Self {
        self.style().align_items = Some(gpui::AlignItems::Stretch);
        self
    }

    fn justify_evenly(mut self) -> Self {
        self.style().justify_content = Some(gpui::JustifyContent::SpaceEvenly);
        self
    }
}

/// Shared square dimension (px) for icon-style controls (ActionIcon,
/// CloseButton) across the size scale.
pub(crate) fn icon_size(size: Size) -> f32 {
    match size {
        Size::Xs => 18.0,
        Size::Sm => 22.0,
        Size::Md => 28.0,
        Size::Lg => 34.0,
        Size::Xl => 44.0,
    }
}

/// How a colored component is filled. Matches Mantine's `variant` prop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Variant {
    /// Solid fill in the color (the default for Button).
    #[default]
    Filled,
    /// Tinted background, colored text.
    Light,
    /// Transparent background, colored border + text.
    Outline,
    /// Transparent until hovered, colored text.
    Subtle,
    /// Neutral surface with a border (the gray button).
    Default,
    /// No background or border, colored text only.
    Transparent,
    /// White background, colored text.
    White,
}

/// Resolved colors for one `(color, variant)` pairing.
#[derive(Debug, Clone, Copy)]
pub struct Surface {
    pub bg: Hsla,
    pub bg_hover: Hsla,
    pub fg: Hsla,
    pub border: Option<Hsla>,
}

/// A color a component can be tinted with: either a palette family (which the
/// variant resolver expands into shades) or a single explicit color — e.g. from
/// the [`color!`](crate::color) macro / [`css`](crate::theme::css).
///
/// `ColorName` and `Hsla` both `Into<ColorValue>`, so `.color(ColorName::Blue)`
/// and `.color(color!(rgba(34, 139, 230, 0.5)))` both work.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColorValue {
    /// A named palette family with its full shade ramp.
    Named(ColorName),
    /// One explicit color; variant shades are derived from it.
    Custom(Hsla),
}

impl Default for ColorValue {
    fn default() -> Self {
        ColorValue::Named(ColorName::Blue)
    }
}

impl From<ColorName> for ColorValue {
    fn from(name: ColorName) -> Self {
        ColorValue::Named(name)
    }
}

impl From<Hsla> for ColorValue {
    fn from(color: Hsla) -> Self {
        ColorValue::Custom(color)
    }
}

impl ColorValue {
    /// The single accent color (for components that don't go through variants).
    pub fn accent(self, theme: &Theme) -> Hsla {
        match self {
            ColorValue::Named(name) => theme.color(name, theme.primary_shade()).hsla(),
            ColorValue::Custom(c) => c,
        }
    }

    /// The soft (lightly tinted) background used by selected/checked states.
    pub fn soft(self, theme: &Theme) -> Hsla {
        let dark = theme.scheme.is_dark();
        match self {
            ColorValue::Named(name) if dark => theme.color(name, 5).alpha(0.20),
            ColorValue::Named(name) => theme.color(name, 0).hsla(),
            ColorValue::Custom(c) => with_alpha(c, if dark { 0.22 } else { 0.12 }),
        }
    }
}

fn shift_l(c: Hsla, delta: f32) -> Hsla {
    Hsla {
        l: (c.l + delta).clamp(0.0, 1.0),
        ..c
    }
}

fn with_alpha(c: Hsla, a: f32) -> Hsla {
    Hsla { a, ..c }
}

/// A readable foreground (near-black or near-white) over `c`.
fn readable_on(c: Hsla) -> Hsla {
    let v = if c.l > 0.6 { 0.0 } else { 1.0 };
    Hsla {
        h: 0.0,
        s: 0.0,
        l: v,
        a: 1.0,
    }
}

/// Resolve a color + variant against the theme into drawable colors. Accepts a
/// palette [`ColorName`] or an explicit [`ColorValue`] (e.g. a `color!(..)`).
pub fn surface(theme: &Theme, color: impl Into<ColorValue>, variant: Variant) -> Surface {
    match color.into() {
        ColorValue::Named(name) => surface_named(theme, name, variant),
        ColorValue::Custom(c) => surface_custom(theme, c, variant),
    }
}

/// Variant resolution for a single explicit color (no palette shades to draw
/// on, so hover/tints are derived algorithmically).
fn surface_custom(theme: &Theme, c: Hsla, variant: Variant) -> Surface {
    let dark = theme.scheme.is_dark();
    let transparent = gpui::transparent_black();
    let hover_fill = if dark {
        shift_l(c, 0.06)
    } else {
        shift_l(c, -0.06)
    };
    match variant {
        Variant::Filled => Surface {
            bg: c,
            bg_hover: hover_fill,
            fg: readable_on(c),
            border: None,
        },
        Variant::Light => Surface {
            bg: with_alpha(c, if dark { 0.20 } else { 0.12 }),
            bg_hover: with_alpha(c, if dark { 0.28 } else { 0.20 }),
            fg: c,
            border: None,
        },
        Variant::Outline => Surface {
            bg: transparent,
            bg_hover: with_alpha(c, 0.08),
            fg: c,
            border: Some(c),
        },
        Variant::Subtle => Surface {
            bg: transparent,
            bg_hover: with_alpha(c, if dark { 0.15 } else { 0.08 }),
            fg: c,
            border: None,
        },
        Variant::Default => Surface {
            bg: theme.surface().hsla(),
            bg_hover: theme.surface_hover().hsla(),
            fg: theme.text().hsla(),
            border: Some(theme.border().hsla()),
        },
        Variant::Transparent => Surface {
            bg: transparent,
            bg_hover: transparent,
            fg: c,
            border: None,
        },
        Variant::White => Surface {
            bg: theme.white.hsla(),
            bg_hover: theme.color(ColorName::Gray, 0).hsla(),
            fg: c,
            border: None,
        },
    }
}

/// Variant resolution for a named palette color (the original Mantine mapping).
fn surface_named(theme: &Theme, name: ColorName, variant: Variant) -> Surface {
    let dark = theme.scheme.is_dark();
    let transparent = gpui::transparent_black();
    let shade = theme.primary_shade();
    let filled = theme.color(name, shade);
    let filled_hover = theme.color(name, (shade + 1).min(9));
    // The accent shade used for text/border in non-filled variants.
    let accent = theme.color(name, if dark { 4 } else { 6 });

    match variant {
        Variant::Filled => Surface {
            bg: filled.hsla(),
            bg_hover: filled_hover.hsla(),
            fg: filled.contrasting().hsla(),
            border: None,
        },
        Variant::Light => {
            let (bg, bg_hover, fg) = if dark {
                (
                    theme.color(name, 5).alpha(0.20),
                    theme.color(name, 5).alpha(0.30),
                    theme.color(name, 2).hsla(),
                )
            } else {
                (
                    theme.color(name, 0).hsla(),
                    theme.color(name, 1).hsla(),
                    theme.color(name, 8).hsla(),
                )
            };
            Surface {
                bg,
                bg_hover,
                fg,
                border: None,
            }
        }
        Variant::Outline => Surface {
            bg: transparent,
            bg_hover: accent.alpha(0.08),
            fg: accent.hsla(),
            border: Some(accent.hsla()),
        },
        Variant::Subtle => Surface {
            bg: transparent,
            bg_hover: accent.alpha(if dark { 0.15 } else { 0.08 }),
            fg: accent.hsla(),
            border: None,
        },
        Variant::Default => Surface {
            bg: theme.surface().hsla(),
            bg_hover: theme.surface_hover().hsla(),
            fg: theme.text().hsla(),
            border: Some(theme.border().hsla()),
        },
        Variant::Transparent => Surface {
            bg: transparent,
            bg_hover: transparent,
            fg: accent.hsla(),
            border: None,
        },
        Variant::White => Surface {
            bg: theme.white.hsla(),
            bg_hover: theme.color(ColorName::Gray, 0).hsla(),
            fg: theme.color(name, shade).hsla(),
            border: None,
        },
    }
}
