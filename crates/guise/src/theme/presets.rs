//! Prebuilt themes — well-known palettes expressed as override sets over the
//! base light/dark themes. Grab one directly (`Theme::dracula().init(cx)`) or
//! look it up by name with [`Theme::preset`].

use super::{Color, Theme};

/// The names [`Theme::preset`] recognizes.
pub const PRESET_NAMES: [&str; 6] = [
    "catppuccin",
    "nord",
    "tokyonight",
    "gruvbox",
    "dracula",
    "solarizedlight",
];

fn hex(value: &'static str) -> Color {
    Color::hex(value)
}

impl Theme {
    /// Look up a prebuilt theme by name (see [`PRESET_NAMES`]).
    pub fn preset(name: &str) -> Option<Theme> {
        match name {
            "catppuccin" => Some(Theme::catppuccin()),
            "nord" => Some(Theme::nord()),
            "tokyonight" => Some(Theme::tokyonight()),
            "gruvbox" => Some(Theme::gruvbox()),
            "dracula" => Some(Theme::dracula()),
            "solarizedlight" => Some(Theme::solarized_light()),
            _ => None,
        }
    }

    /// Catppuccin Mocha (dark).
    pub fn catppuccin() -> Theme {
        Theme::dark()
            .with_primary(hex("#89b4fa"))
            .with_body(hex("#1e1e2e"))
            .with_surface(hex("#181825"))
            .with_surface_hover(hex("#313244"))
            .with_text(hex("#cdd6f4"))
            .with_dimmed(hex("#a6adc8"))
            .with_border(hex("#45475a"))
            .with_success(hex("#a6e3a1"))
            .with_warning(hex("#f9e2af"))
            .with_danger(hex("#f38ba8"))
            .with_info(hex("#89dceb"))
    }

    /// Nord (dark).
    pub fn nord() -> Theme {
        Theme::dark()
            .with_primary(hex("#88c0d0"))
            .with_body(hex("#2e3440"))
            .with_surface(hex("#3b4252"))
            .with_surface_hover(hex("#434c5e"))
            .with_text(hex("#eceff4"))
            .with_dimmed(hex("#d8dee9"))
            .with_border(hex("#4c566a"))
            .with_success(hex("#a3be8c"))
            .with_warning(hex("#ebcb8b"))
            .with_danger(hex("#bf616a"))
            .with_info(hex("#81a1c1"))
    }

    /// Tokyo Night (dark).
    pub fn tokyonight() -> Theme {
        Theme::dark()
            .with_primary(hex("#7aa2f7"))
            .with_body(hex("#1a1b26"))
            .with_surface(hex("#16161e"))
            .with_surface_hover(hex("#292e42"))
            .with_text(hex("#c0caf5"))
            .with_dimmed(hex("#565f89"))
            .with_border(hex("#3b4261"))
            .with_success(hex("#9ece6a"))
            .with_warning(hex("#e0af68"))
            .with_danger(hex("#f7768e"))
            .with_info(hex("#7dcfff"))
    }

    /// Gruvbox (dark, hard contrast accents).
    pub fn gruvbox() -> Theme {
        Theme::dark()
            .with_primary(hex("#fe8019"))
            .with_body(hex("#282828"))
            .with_surface(hex("#3c3836"))
            .with_surface_hover(hex("#504945"))
            .with_text(hex("#ebdbb2"))
            .with_dimmed(hex("#a89984"))
            .with_border(hex("#665c54"))
            .with_success(hex("#b8bb26"))
            .with_warning(hex("#fabd2f"))
            .with_danger(hex("#fb4934"))
            .with_info(hex("#83a598"))
    }

    /// Dracula (dark).
    pub fn dracula() -> Theme {
        Theme::dark()
            .with_primary(hex("#bd93f9"))
            .with_body(hex("#282a36"))
            .with_surface(hex("#21222c"))
            .with_surface_hover(hex("#44475a"))
            .with_text(hex("#f8f8f2"))
            .with_dimmed(hex("#6272a4"))
            .with_border(hex("#44475a"))
            .with_success(hex("#50fa7b"))
            .with_warning(hex("#f1fa8c"))
            .with_danger(hex("#ff5555"))
            .with_info(hex("#8be9fd"))
    }

    /// Solarized (light).
    pub fn solarized_light() -> Theme {
        Theme::light()
            .with_primary(hex("#268bd2"))
            .with_body(hex("#fdf6e3"))
            .with_surface(hex("#eee8d5"))
            .with_surface_hover(hex("#e4dcc4"))
            .with_text(hex("#657b83"))
            .with_dimmed(hex("#93a1a1"))
            .with_border(hex("#d5cdb4"))
            .with_success(hex("#859900"))
            .with_warning(hex("#b58900"))
            .with_danger(hex("#dc322f"))
            .with_info(hex("#2aa198"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_name_resolves_and_overrides_all_slots() {
        for name in PRESET_NAMES {
            let theme = Theme::preset(name).unwrap_or_else(|| panic!("missing preset {name}"));
            let o = &theme.overrides;
            for (slot, value) in [
                ("primary", o.primary),
                ("body", o.body),
                ("surface", o.surface),
                ("surface_hover", o.surface_hover),
                ("text", o.text),
                ("dimmed", o.dimmed),
                ("border", o.border),
                ("success", o.success),
                ("warning", o.warning),
                ("danger", o.danger),
                ("info", o.info),
            ] {
                assert!(value.is_some(), "{name} leaves {slot} unset");
            }
        }
        assert!(Theme::preset("nope").is_none());
    }

    #[test]
    fn presets_pick_sensible_schemes() {
        assert!(Theme::catppuccin().scheme.is_dark());
        assert!(!Theme::solarized_light().scheme.is_dark());
    }
}
