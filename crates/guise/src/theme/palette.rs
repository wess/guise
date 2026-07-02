//! The Mantine / open-color palette: 14 named colors, each a 10-step shade
//! ramp from lightest (index 0) to darkest (index 9).

use super::color::Color;

/// One of the named colors in the theme palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorName {
    Dark,
    Gray,
    Red,
    Pink,
    Grape,
    Violet,
    Indigo,
    Blue,
    Cyan,
    Teal,
    Green,
    Lime,
    Yellow,
    Orange,
}

impl ColorName {
    pub const ALL: [ColorName; 14] = [
        ColorName::Dark,
        ColorName::Gray,
        ColorName::Red,
        ColorName::Pink,
        ColorName::Grape,
        ColorName::Violet,
        ColorName::Indigo,
        ColorName::Blue,
        ColorName::Cyan,
        ColorName::Teal,
        ColorName::Green,
        ColorName::Lime,
        ColorName::Yellow,
        ColorName::Orange,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ColorName::Dark => "dark",
            ColorName::Gray => "gray",
            ColorName::Red => "red",
            ColorName::Pink => "pink",
            ColorName::Grape => "grape",
            ColorName::Violet => "violet",
            ColorName::Indigo => "indigo",
            ColorName::Blue => "blue",
            ColorName::Cyan => "cyan",
            ColorName::Teal => "teal",
            ColorName::Green => "green",
            ColorName::Lime => "lime",
            ColorName::Yellow => "yellow",
            ColorName::Orange => "orange",
        }
    }
}

/// A 10-step shade ramp for a single named color.
#[derive(Debug, Clone, Copy)]
pub struct Shades(pub [Color; 10]);

impl Shades {
    /// Shade by index, clamped into `0..=9`.
    pub fn get(&self, shade: usize) -> Color {
        self.0[shade.min(9)]
    }
}

/// The full set of named colors. Resolved by `ColorName`.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    shades: [Shades; 14],
}

impl Palette {
    /// The shade ramp for a named color.
    pub fn shades(&self, name: ColorName) -> Shades {
        self.shades[name as usize]
    }

    /// A single shade of a named color.
    pub fn get(&self, name: ColorName, shade: usize) -> Color {
        self.shades(name).get(shade)
    }

    /// Replace the shade ramp for one named color — e.g. re-pin `Dark` to an
    /// app's custom scale before `Theme::init`.
    pub fn set_shades(&mut self, name: ColorName, shades: Shades) {
        self.shades[name as usize] = shades;
    }
}

impl Default for Palette {
    fn default() -> Self {
        mantine()
    }
}

/// The default Mantine palette.
pub fn mantine() -> Palette {
    let ramp = |hexes: [&str; 10]| Shades(hexes.map(Color::hex));
    Palette {
        shades: [
            // Dark
            ramp([
                "#C9C9C9", "#b8b8b8", "#828282", "#696969", "#424242", "#3b3b3b", "#2e2e2e",
                "#242424", "#1f1f1f", "#141414",
            ]),
            // Gray
            ramp([
                "#f8f9fa", "#f1f3f5", "#e9ecef", "#dee2e6", "#ced4da", "#adb5bd", "#868e96",
                "#495057", "#343a40", "#212529",
            ]),
            // Red
            ramp([
                "#fff5f5", "#ffe3e3", "#ffc9c9", "#ffa8a8", "#ff8787", "#ff6b6b", "#fa5252",
                "#f03e3e", "#e03131", "#c92a2a",
            ]),
            // Pink
            ramp([
                "#fff0f6", "#ffdeeb", "#fcc2d7", "#faa2c1", "#f783ac", "#f06595", "#e64980",
                "#d6336c", "#c2255c", "#a61e4d",
            ]),
            // Grape
            ramp([
                "#f8f0fc", "#f3d9fa", "#eebefa", "#e599f7", "#da77f2", "#cc5de8", "#be4bdb",
                "#ae3ec9", "#9c36b5", "#862e9c",
            ]),
            // Violet
            ramp([
                "#f3f0ff", "#e5dbff", "#d0bfff", "#b197fc", "#9775fa", "#845ef7", "#7950f2",
                "#7048e8", "#6741d9", "#5f3dc4",
            ]),
            // Indigo
            ramp([
                "#edf2ff", "#dbe4ff", "#bac8ff", "#91a7ff", "#748ffc", "#5c7cfa", "#4c6ef5",
                "#4263eb", "#3b5bdb", "#364fc7",
            ]),
            // Blue
            ramp([
                "#e7f5ff", "#d0ebff", "#a5d8ff", "#74c0fc", "#4dabf7", "#339af0", "#228be6",
                "#1c7ed6", "#1971c2", "#1864ab",
            ]),
            // Cyan
            ramp([
                "#e3fafc", "#c5f6fa", "#99e9f2", "#66d9e8", "#3bc9db", "#22b8cf", "#15aabf",
                "#1098ad", "#0c8599", "#0b7285",
            ]),
            // Teal
            ramp([
                "#e6fcf5", "#c3fae8", "#96f2d7", "#63e6be", "#38d9a9", "#20c997", "#12b886",
                "#0ca678", "#099268", "#087f5b",
            ]),
            // Green
            ramp([
                "#ebfbee", "#d3f9d8", "#b2f2bb", "#8ce99a", "#69db7c", "#51cf66", "#40c057",
                "#37b24d", "#2f9e44", "#2b8a3e",
            ]),
            // Lime
            ramp([
                "#f4fce3", "#e9fac8", "#d8f5a2", "#c0eb75", "#a9e34b", "#94d82d", "#82c91e",
                "#74b816", "#66a80f", "#5c940d",
            ]),
            // Yellow
            ramp([
                "#fff9db", "#fff3bf", "#ffec99", "#ffe066", "#ffd43b", "#fcc419", "#fab005",
                "#f59f00", "#f08c00", "#e67700",
            ]),
            // Orange
            ramp([
                "#fff4e6", "#ffe8cc", "#ffd8a8", "#ffc078", "#ffa94d", "#ff922b", "#fd7e14",
                "#f76707", "#e8590c", "#d9480f",
            ]),
        ],
    }
}
