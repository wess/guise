//! CSS-style color literals: a runtime parser and constructor functions that
//! produce gpui [`Hsla`] (which carries alpha). Backs the [`color!`] macro.
//!
//! Supported notations:
//! - hex — `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`
//! - `rgb(r, g, b)` / `rgba(r, g, b, a)` — channels `0..=255` (or `%`), alpha `0..=1` (or `%`)
//! - `hsl(h, s%, l%)` / `hsla(h, s%, l%, a)` — hue degrees, sat/lum percent
//! - CSS named colors — `red`, `teal`, `rebeccapurple`, …
//!
//! [`color!`]: crate::color

use std::fmt;

use gpui::{Hsla, Rgba};

/// Returned when a CSS color string can't be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CssColorError(String);

impl fmt::Display for CssColorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "guise: invalid CSS color `{}`", self.0)
    }
}

impl std::error::Error for CssColorError {}

// --- Constructors (also the macro's expansion targets) ---------------------

fn rgba_f(r: f32, g: f32, b: f32, a: f32) -> Hsla {
    Rgba {
        r: (r / 255.0).clamp(0.0, 1.0),
        g: (g / 255.0).clamp(0.0, 1.0),
        b: (b / 255.0).clamp(0.0, 1.0),
        a: a.clamp(0.0, 1.0),
    }
    .into()
}

/// An opaque color from 8-bit channels.
pub fn rgb(r: u8, g: u8, b: u8) -> Hsla {
    rgba(r, g, b, 1.0)
}

/// A color from 8-bit channels plus alpha in `0.0..=1.0`.
pub fn rgba(r: u8, g: u8, b: u8, a: f32) -> Hsla {
    rgba_f(r as f32, g as f32, b as f32, a)
}

/// An opaque color from hue (degrees) and saturation/lightness (`0..=100`).
pub fn hsl(h: f32, s: f32, l: f32) -> Hsla {
    hsla(h, s, l, 1.0)
}

/// A color from hue (degrees), saturation/lightness (`0..=100`), and alpha.
pub fn hsla(h: f32, s: f32, l: f32, a: f32) -> Hsla {
    Hsla {
        h: (h / 360.0).rem_euclid(1.0),
        s: (s / 100.0).clamp(0.0, 1.0),
        l: (l / 100.0).clamp(0.0, 1.0),
        a: a.clamp(0.0, 1.0),
    }
}

// --- Parser ----------------------------------------------------------------

/// Parse a CSS color string into an [`Hsla`].
pub fn css(input: &str) -> Result<Hsla, CssColorError> {
    let err = || CssColorError(input.to_string());
    let s = input.trim();

    if let Some(hex) = s.strip_prefix('#') {
        return parse_hex(hex).ok_or_else(err);
    }
    let lower = s.to_ascii_lowercase();
    if let Some(inner) = wrapped(&lower, "rgba").or_else(|| wrapped(&lower, "rgb")) {
        return parse_rgb(inner).ok_or_else(err);
    }
    if let Some(inner) = wrapped(&lower, "hsla").or_else(|| wrapped(&lower, "hsl")) {
        return parse_hsl(inner).ok_or_else(err);
    }
    named(&lower).map(|(r, g, b)| rgb(r, g, b)).ok_or_else(err)
}

/// Strip a `name( … )` wrapper, returning the inner argument text.
fn wrapped<'a>(s: &'a str, name: &str) -> Option<&'a str> {
    let rest = s.strip_prefix(name)?.trim_start();
    let rest = rest.strip_prefix('(')?;
    rest.strip_suffix(')')
}

fn nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn hex2(bytes: &[u8], i: usize) -> Option<f32> {
    Some((nibble(bytes[i])? * 16 + nibble(bytes[i + 1])?) as f32)
}

fn parse_hex(hex: &str) -> Option<Hsla> {
    let b = hex.as_bytes();
    match b.len() {
        3 => Some(rgba_f(
            (nibble(b[0])? * 17) as f32,
            (nibble(b[1])? * 17) as f32,
            (nibble(b[2])? * 17) as f32,
            1.0,
        )),
        4 => Some(rgba_f(
            (nibble(b[0])? * 17) as f32,
            (nibble(b[1])? * 17) as f32,
            (nibble(b[2])? * 17) as f32,
            (nibble(b[3])? * 17) as f32 / 255.0,
        )),
        6 => Some(rgba_f(hex2(b, 0)?, hex2(b, 2)?, hex2(b, 4)?, 1.0)),
        8 => Some(rgba_f(
            hex2(b, 0)?,
            hex2(b, 2)?,
            hex2(b, 4)?,
            hex2(b, 6)? / 255.0,
        )),
        _ => None,
    }
}

fn tokens(inner: &str) -> Vec<&str> {
    inner
        .split(|c| c == ',' || c == ' ' || c == '/')
        .filter(|t| !t.is_empty())
        .collect()
}

fn num(tok: &str) -> Option<f32> {
    tok.trim().parse().ok()
}

/// A color channel: `0..=255` or a percentage of 255.
fn channel(tok: &str) -> Option<f32> {
    let t = tok.trim();
    match t.strip_suffix('%') {
        Some(p) => Some(num(p)? / 100.0 * 255.0),
        None => num(t),
    }
}

/// An alpha value: `0..=1` or a percentage.
fn alpha(tok: &str) -> Option<f32> {
    let t = tok.trim();
    match t.strip_suffix('%') {
        Some(p) => Some(num(p)? / 100.0),
        None => num(t),
    }
}

fn parse_rgb(inner: &str) -> Option<Hsla> {
    let parts = tokens(inner);
    let a = match parts.len() {
        3 => 1.0,
        4 => alpha(parts[3])?,
        _ => return None,
    };
    Some(rgba_f(
        channel(parts[0])?,
        channel(parts[1])?,
        channel(parts[2])?,
        a,
    ))
}

fn parse_hsl(inner: &str) -> Option<Hsla> {
    let parts = tokens(inner);
    let a = match parts.len() {
        3 => 1.0,
        4 => alpha(parts[3])?,
        _ => return None,
    };
    let h = num(parts[0].trim().trim_end_matches("deg"))?;
    let s = num(parts[1].trim().trim_end_matches('%'))?;
    let l = num(parts[2].trim().trim_end_matches('%'))?;
    Some(hsla(h, s, l, a))
}

/// The standard CSS named colors.
fn named(name: &str) -> Option<(u8, u8, u8)> {
    let rgb = match name {
        "transparent" => return Some((0, 0, 0)),
        "aliceblue" => (240, 248, 255),
        "antiquewhite" => (250, 235, 215),
        "aqua" | "cyan" => (0, 255, 255),
        "aquamarine" => (127, 255, 212),
        "azure" => (240, 255, 255),
        "beige" => (245, 245, 220),
        "bisque" => (255, 228, 196),
        "black" => (0, 0, 0),
        "blanchedalmond" => (255, 235, 205),
        "blue" => (0, 0, 255),
        "blueviolet" => (138, 43, 226),
        "brown" => (165, 42, 42),
        "burlywood" => (222, 184, 135),
        "cadetblue" => (95, 158, 160),
        "chartreuse" => (127, 255, 0),
        "chocolate" => (210, 105, 30),
        "coral" => (255, 127, 80),
        "cornflowerblue" => (100, 149, 237),
        "cornsilk" => (255, 248, 220),
        "crimson" => (220, 20, 60),
        "darkblue" => (0, 0, 139),
        "darkcyan" => (0, 139, 139),
        "darkgoldenrod" => (184, 134, 11),
        "darkgray" | "darkgrey" => (169, 169, 169),
        "darkgreen" => (0, 100, 0),
        "darkkhaki" => (189, 183, 107),
        "darkmagenta" => (139, 0, 139),
        "darkolivegreen" => (85, 107, 47),
        "darkorange" => (255, 140, 0),
        "darkorchid" => (153, 50, 204),
        "darkred" => (139, 0, 0),
        "darksalmon" => (233, 150, 122),
        "darkseagreen" => (143, 188, 143),
        "darkslateblue" => (72, 61, 139),
        "darkslategray" | "darkslategrey" => (47, 79, 79),
        "darkturquoise" => (0, 206, 209),
        "darkviolet" => (148, 0, 211),
        "deeppink" => (255, 20, 147),
        "deepskyblue" => (0, 191, 255),
        "dimgray" | "dimgrey" => (105, 105, 105),
        "dodgerblue" => (30, 144, 255),
        "firebrick" => (178, 34, 34),
        "floralwhite" => (255, 250, 240),
        "forestgreen" => (34, 139, 34),
        "fuchsia" | "magenta" => (255, 0, 255),
        "gainsboro" => (220, 220, 220),
        "ghostwhite" => (248, 248, 255),
        "gold" => (255, 215, 0),
        "goldenrod" => (218, 165, 32),
        "gray" | "grey" => (128, 128, 128),
        "green" => (0, 128, 0),
        "greenyellow" => (173, 255, 47),
        "honeydew" => (240, 255, 240),
        "hotpink" => (255, 105, 180),
        "indianred" => (205, 92, 92),
        "indigo" => (75, 0, 130),
        "ivory" => (255, 255, 240),
        "khaki" => (240, 230, 140),
        "lavender" => (230, 230, 250),
        "lavenderblush" => (255, 240, 245),
        "lawngreen" => (124, 252, 0),
        "lemonchiffon" => (255, 250, 205),
        "lightblue" => (173, 216, 230),
        "lightcoral" => (240, 128, 128),
        "lightcyan" => (224, 255, 255),
        "lightgoldenrodyellow" => (250, 250, 210),
        "lightgray" | "lightgrey" => (211, 211, 211),
        "lightgreen" => (144, 238, 144),
        "lightpink" => (255, 182, 193),
        "lightsalmon" => (255, 160, 122),
        "lightseagreen" => (32, 178, 170),
        "lightskyblue" => (135, 206, 250),
        "lightslategray" | "lightslategrey" => (119, 136, 153),
        "lightsteelblue" => (176, 196, 222),
        "lightyellow" => (255, 255, 224),
        "lime" => (0, 255, 0),
        "limegreen" => (50, 205, 50),
        "linen" => (250, 240, 230),
        "maroon" => (128, 0, 0),
        "mediumaquamarine" => (102, 205, 170),
        "mediumblue" => (0, 0, 205),
        "mediumorchid" => (186, 85, 211),
        "mediumpurple" => (147, 112, 219),
        "mediumseagreen" => (60, 179, 113),
        "mediumslateblue" => (123, 104, 238),
        "mediumspringgreen" => (0, 250, 154),
        "mediumturquoise" => (72, 209, 204),
        "mediumvioletred" => (199, 21, 133),
        "midnightblue" => (25, 25, 112),
        "mintcream" => (245, 255, 250),
        "mistyrose" => (255, 228, 225),
        "moccasin" => (255, 228, 181),
        "navajowhite" => (255, 222, 173),
        "navy" => (0, 0, 128),
        "oldlace" => (253, 245, 230),
        "olive" => (128, 128, 0),
        "olivedrab" => (107, 142, 35),
        "orange" => (255, 165, 0),
        "orangered" => (255, 69, 0),
        "orchid" => (218, 112, 214),
        "palegoldenrod" => (238, 232, 170),
        "palegreen" => (152, 251, 152),
        "paleturquoise" => (175, 238, 238),
        "palevioletred" => (219, 112, 147),
        "papayawhip" => (255, 239, 213),
        "peachpuff" => (255, 218, 185),
        "peru" => (205, 133, 63),
        "pink" => (255, 192, 203),
        "plum" => (221, 160, 221),
        "powderblue" => (176, 224, 230),
        "purple" => (128, 0, 128),
        "rebeccapurple" => (102, 51, 153),
        "red" => (255, 0, 0),
        "rosybrown" => (188, 143, 143),
        "royalblue" => (65, 105, 225),
        "saddlebrown" => (139, 69, 19),
        "salmon" => (250, 128, 114),
        "sandybrown" => (244, 164, 96),
        "seagreen" => (46, 139, 87),
        "seashell" => (255, 245, 238),
        "sienna" => (160, 82, 45),
        "silver" => (192, 192, 192),
        "skyblue" => (135, 206, 235),
        "slateblue" => (106, 90, 205),
        "slategray" | "slategrey" => (112, 128, 144),
        "snow" => (255, 250, 250),
        "springgreen" => (0, 255, 127),
        "steelblue" => (70, 130, 180),
        "tan" => (210, 180, 140),
        "teal" => (0, 128, 128),
        "thistle" => (216, 191, 216),
        "tomato" => (255, 99, 71),
        "turquoise" => (64, 224, 208),
        "violet" => (238, 130, 238),
        "wheat" => (245, 222, 179),
        "white" => (255, 255, 255),
        "whitesmoke" => (245, 245, 245),
        "yellow" => (255, 255, 0),
        "yellowgreen" => (154, 205, 50),
        _ => return None,
    };
    Some(rgb)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_forms() {
        assert_eq!(css("#228be6").unwrap(), rgb(0x22, 0x8b, 0xe6));
        assert_eq!(css("228be6").is_err(), true); // needs the leading '#'
        assert_eq!(css("#fff").unwrap(), rgb(255, 255, 255));
        assert_eq!(css("#ff000080").unwrap(), rgba(255, 0, 0, 0x80 as f32 / 255.0));
    }

    #[test]
    fn functional_forms() {
        assert_eq!(css("rgb(34, 139, 230)").unwrap(), rgb(34, 139, 230));
        assert_eq!(css("rgba(0,0,0,0.5)").unwrap(), rgba(0, 0, 0, 0.5));
        assert_eq!(css("hsl(210, 80%, 52%)").unwrap(), hsl(210.0, 80.0, 52.0));
        assert_eq!(css("hsla(210,80%,52%,0.5)").unwrap(), hsla(210.0, 80.0, 52.0, 0.5));
    }

    #[test]
    fn named_colors() {
        assert_eq!(css("teal").unwrap(), rgb(0, 128, 128));
        assert_eq!(css("RebeccaPurple").unwrap(), rgb(102, 51, 153));
        assert!(css("notacolor").is_err());
    }
}
