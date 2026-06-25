//! 24-bit RGB color with hex parsing and conversion into gpui's `Hsla`.
//!
//! Colors are stored as plain RGB triples (the Mantine/open-color palette is
//! authored as hex). `hsla`/`alpha` bridge into the gpui rendering types at the
//! point of use.

use gpui::{Hsla, Rgba};

/// An opaque 24-bit color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b }
    }

    /// Parse a `#rrggbb` / `rrggbb` / `#rgb` / `rgb` hex string.
    ///
    /// Panics on malformed input — callers pass static palette literals, so a
    /// bad value is a programming error that should fail loudly at startup.
    pub const fn hex(s: &str) -> Self {
        let bytes = s.as_bytes();
        let hex = if !bytes.is_empty() && bytes[0] == b'#' {
            split_at(bytes, 1)
        } else {
            bytes
        };
        match hex.len() {
            3 => Color::new(
                nibble(hex[0]) * 17,
                nibble(hex[1]) * 17,
                nibble(hex[2]) * 17,
            ),
            6 => Color::new(
                nibble(hex[0]) * 16 + nibble(hex[1]),
                nibble(hex[2]) * 16 + nibble(hex[3]),
                nibble(hex[4]) * 16 + nibble(hex[5]),
            ),
            _ => panic!("guise: color hex must be 3 or 6 digits"),
        }
    }

    /// Build an opaque `Color` from a gpui `Hsla`, dropping any alpha. Used so a
    /// CSS color (which parses to `Hsla`) can populate the opaque theme fields.
    pub fn from_hsla(c: Hsla) -> Color {
        let (r, g, b) = hsl_to_rgb(c.h, c.s, c.l);
        Color::new(r, g, b)
    }

    /// As an opaque gpui `Rgba`.
    pub fn rgba(self) -> Rgba {
        Rgba {
            r: self.r as f32 / 255.0,
            g: self.g as f32 / 255.0,
            b: self.b as f32 / 255.0,
            a: 1.0,
        }
    }

    /// As an opaque gpui `Hsla`.
    pub fn hsla(self) -> Hsla {
        self.rgba().into()
    }

    /// As a gpui `Hsla` with the given alpha in `0.0..=1.0`.
    pub fn alpha(self, a: f32) -> Hsla {
        let mut hsla: Hsla = self.rgba().into();
        hsla.a = a.clamp(0.0, 1.0);
        hsla
    }

    /// Relative luminance in `0.0..=1.0` (Rec. 709 weights). Used to pick a
    /// readable foreground (black vs. white) over a filled color.
    pub fn luminance(self) -> f32 {
        (0.2126 * self.r as f32 + 0.7152 * self.g as f32 + 0.0722 * self.b as f32) / 255.0
    }

    /// A foreground color (black or white) with adequate contrast over `self`.
    pub fn contrasting(self) -> Color {
        if self.luminance() > 0.55 {
            Color::new(0, 0, 0)
        } else {
            Color::new(255, 255, 255)
        }
    }
}

impl From<Color> for Hsla {
    fn from(c: Color) -> Hsla {
        c.hsla()
    }
}

/// HSL (h,s,l all in `0.0..=1.0`, the gpui convention) to 8-bit RGB.
fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let to_u8 = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    if s <= 0.0 {
        let v = to_u8(l);
        return (v, v, v);
    }
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    let hue = |mut t: f32| {
        if t < 0.0 {
            t += 1.0;
        }
        if t > 1.0 {
            t -= 1.0;
        }
        if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 1.0 / 2.0 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        }
    };
    (
        to_u8(hue(h + 1.0 / 3.0)),
        to_u8(hue(h)),
        to_u8(hue(h - 1.0 / 3.0)),
    )
}

const fn split_at(bytes: &[u8], at: usize) -> &[u8] {
    // const-friendly slice from `at` to end.
    bytes.split_at(at).1
}

const fn nibble(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => panic!("guise: invalid hex digit in color"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_long_and_short_forms() {
        assert_eq!(Color::hex("#228be6"), Color::new(0x22, 0x8b, 0xe6));
        assert_eq!(Color::hex("228be6"), Color::new(0x22, 0x8b, 0xe6));
        assert_eq!(Color::hex("#fff"), Color::new(255, 255, 255));
        assert_eq!(Color::hex("000"), Color::new(0, 0, 0));
    }

    #[test]
    fn contrasting_picks_readable_foreground() {
        assert_eq!(Color::hex("#ffffff").contrasting(), Color::new(0, 0, 0));
        assert_eq!(Color::hex("#1864ab").contrasting(), Color::new(255, 255, 255));
    }
}
