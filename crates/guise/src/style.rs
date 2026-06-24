//! Shared visual variants. Mantine resolves a `(color, variant)` pair into a
//! background / foreground / border triple that every interactive component
//! (Button, Badge, ActionIcon, ...) draws from. This is that resolver.

use gpui::Hsla;

use crate::theme::{ColorName, Size, Theme};

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

/// Resolve a color + variant against the theme into drawable colors.
pub fn surface(theme: &Theme, name: ColorName, variant: Variant) -> Surface {
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
