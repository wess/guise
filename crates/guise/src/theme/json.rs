//! JSON theme files: `Theme::from_json(source)`.
//!
//! The format is a **flat JSON object of string values** — every key is a
//! theme slot, every value a CSS color (any form `css()` accepts) or token.
//! Flat-on-purpose: it keeps the parser dependency-free and the files
//! diff-friendly.
//!
//! ```json
//! {
//!   "name": "midnight",
//!   "scheme": "dark",
//!   "primary": "#7aa2f7",
//!   "body": "#1a1b26",
//!   "surface": "#16161e",
//!   "surfacehover": "#292e42",
//!   "text": "#c0caf5",
//!   "dimmed": "#565f89",
//!   "border": "#3b4261",
//!   "success": "rgb(158, 206, 106)",
//!   "warning": "#e0af68",
//!   "danger": "#f7768e",
//!   "info": "#7dcfff",
//!   "fontfamily": "Inter",
//!   "radius": "md"
//! }
//! ```
//!
//! Every key is optional except that unknown keys are rejected (they're
//! almost always typos). `scheme` defaults to `dark`.

use std::fmt;

use super::css::css;
use super::{ColorScheme, Size, Theme};

/// Why a theme file failed to load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThemeJsonError {
    /// Malformed JSON, with a byte offset and a short reason.
    Syntax(usize, &'static str),
    /// A key that isn't a theme slot (probably a typo).
    UnknownKey(String),
    /// A value that didn't parse for its key.
    BadValue(String, String),
}

impl fmt::Display for ThemeJsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ThemeJsonError::Syntax(at, why) => write!(f, "theme json: {why} at byte {at}"),
            ThemeJsonError::UnknownKey(key) => write!(f, "theme json: unknown key {key:?}"),
            ThemeJsonError::BadValue(key, value) => {
                write!(f, "theme json: bad value {value:?} for {key:?}")
            }
        }
    }
}

impl std::error::Error for ThemeJsonError {}

/// Parse a flat `{"string": "string", ...}` object. Nested values, arrays,
/// numbers, and booleans are syntax errors — the format is deliberately flat.
fn parse_flat(src: &str) -> Result<Vec<(String, String)>, ThemeJsonError> {
    let bytes = src.as_bytes();
    let mut i = 0;
    let mut pairs = Vec::new();

    let skip_ws = |i: &mut usize| {
        while *i < bytes.len() && bytes[*i].is_ascii_whitespace() {
            *i += 1;
        }
    };

    fn parse_string(bytes: &[u8], i: &mut usize) -> Result<String, ThemeJsonError> {
        if bytes.get(*i) != Some(&b'"') {
            return Err(ThemeJsonError::Syntax(*i, "expected a string"));
        }
        *i += 1;
        let mut out = String::new();
        loop {
            match bytes.get(*i) {
                None => return Err(ThemeJsonError::Syntax(*i, "unterminated string")),
                Some(b'"') => {
                    *i += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    *i += 1;
                    match bytes.get(*i) {
                        Some(b'"') => out.push('"'),
                        Some(b'\\') => out.push('\\'),
                        Some(b'/') => out.push('/'),
                        Some(b'n') => out.push('\n'),
                        Some(b't') => out.push('\t'),
                        Some(b'r') => out.push('\r'),
                        Some(b'u') => {
                            let hex = bytes
                                .get(*i + 1..*i + 5)
                                .and_then(|h| std::str::from_utf8(h).ok())
                                .and_then(|h| u32::from_str_radix(h, 16).ok())
                                .and_then(char::from_u32)
                                .ok_or(ThemeJsonError::Syntax(*i, "bad \\u escape"))?;
                            out.push(hex);
                            *i += 4;
                        }
                        _ => return Err(ThemeJsonError::Syntax(*i, "bad escape")),
                    }
                    *i += 1;
                }
                Some(_) => {
                    // Push the full UTF-8 character, not just one byte.
                    let rest = &src_from(bytes, *i);
                    let ch = rest.chars().next().unwrap_or('\u{fffd}');
                    out.push(ch);
                    *i += ch.len_utf8();
                }
            }
        }
    }

    fn src_from(bytes: &[u8], i: usize) -> &str {
        std::str::from_utf8(&bytes[i..]).unwrap_or("")
    }

    skip_ws(&mut i);
    if bytes.get(i) != Some(&b'{') {
        return Err(ThemeJsonError::Syntax(i, "expected '{'"));
    }
    i += 1;
    skip_ws(&mut i);
    if bytes.get(i) == Some(&b'}') {
        i += 1;
    } else {
        loop {
            skip_ws(&mut i);
            let key = parse_string(bytes, &mut i)?;
            skip_ws(&mut i);
            if bytes.get(i) != Some(&b':') {
                return Err(ThemeJsonError::Syntax(i, "expected ':'"));
            }
            i += 1;
            skip_ws(&mut i);
            let value = parse_string(bytes, &mut i)?;
            pairs.push((key, value));
            skip_ws(&mut i);
            match bytes.get(i) {
                Some(b',') => i += 1,
                Some(b'}') => {
                    i += 1;
                    break;
                }
                _ => return Err(ThemeJsonError::Syntax(i, "expected ',' or '}'")),
            }
        }
    }
    skip_ws(&mut i);
    if i != bytes.len() {
        return Err(ThemeJsonError::Syntax(i, "trailing content"));
    }
    Ok(pairs)
}

fn size_token(value: &str) -> Option<Size> {
    match value {
        "xs" => Some(Size::Xs),
        "sm" => Some(Size::Sm),
        "md" => Some(Size::Md),
        "lg" => Some(Size::Lg),
        "xl" => Some(Size::Xl),
        _ => None,
    }
}

impl Theme {
    /// Build a theme from a JSON string (see the module docs for the format).
    /// Starts from [`Theme::light`]/[`Theme::dark`] per the `scheme` key
    /// (default dark) and applies each slot as an override.
    pub fn from_json(source: &str) -> Result<Theme, ThemeJsonError> {
        let pairs = parse_flat(source)?;

        let scheme = match pairs.iter().find(|(k, _)| k == "scheme") {
            None => ColorScheme::Dark,
            Some((_, v)) => match v.as_str() {
                "light" => ColorScheme::Light,
                "dark" => ColorScheme::Dark,
                other => return Err(ThemeJsonError::BadValue("scheme".into(), other.into())),
            },
        };
        let mut theme = match scheme {
            ColorScheme::Light => Theme::light(),
            ColorScheme::Dark => Theme::dark(),
        };

        for (key, value) in pairs {
            let color =
                || css(&value).map_err(|_| ThemeJsonError::BadValue(key.clone(), value.clone()));
            theme = match key.as_str() {
                "name" | "$schema" | "scheme" => theme,
                "primary" => theme.with_primary(color()?),
                "body" => theme.with_body(color()?),
                "surface" => theme.with_surface(color()?),
                "surfacehover" => theme.with_surface_hover(color()?),
                "text" => theme.with_text(color()?),
                "dimmed" => theme.with_dimmed(color()?),
                "border" => theme.with_border(color()?),
                "success" => theme.with_success(color()?),
                "warning" => theme.with_warning(color()?),
                "danger" => theme.with_danger(color()?),
                "info" => theme.with_info(color()?),
                "fontfamily" => {
                    theme.font_family = value.clone().into();
                    theme
                }
                "radius" => {
                    theme.default_radius = size_token(&value)
                        .ok_or_else(|| ThemeJsonError::BadValue(key.clone(), value.clone()))?;
                    theme
                }
                _ => return Err(ThemeJsonError::UnknownKey(key)),
            };
        }
        Ok(theme)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r##"{
        "name": "midnight",
        "scheme": "dark",
        "primary": "#7aa2f7",
        "body": "#1a1b26",
        "surfacehover": "rgb(41, 46, 66)",
        "danger": "hsl(349, 89%, 72%)",
        "fontfamily": "Inter",
        "radius": "lg"
    }"##;

    #[test]
    fn parses_a_full_theme() {
        let theme = Theme::from_json(SAMPLE).unwrap();
        assert!(theme.scheme.is_dark());
        assert_eq!(theme.font_family.as_ref(), "Inter");
        assert_eq!(theme.default_radius, Size::Lg);
        assert!(theme.overrides.primary.is_some());
        assert!(theme.overrides.body.is_some());
        assert!(theme.overrides.surface_hover.is_some());
        assert!(theme.overrides.danger.is_some());
        // Unset slots stay on scheme defaults.
        assert!(theme.overrides.text.is_none());
        assert_ne!(theme.primary(), Theme::dark().primary());
    }

    #[test]
    fn scheme_defaults_to_dark_and_light_works() {
        assert!(Theme::from_json("{}").unwrap().scheme.is_dark());
        let light = Theme::from_json(r#"{"scheme": "light"}"#).unwrap();
        assert!(!light.scheme.is_dark());
    }

    #[test]
    fn unknown_keys_are_rejected() {
        let err = Theme::from_json(r##"{"primry": "#fff"}"##).unwrap_err();
        assert_eq!(err, ThemeJsonError::UnknownKey("primry".into()));
    }

    #[test]
    fn bad_values_name_the_key() {
        let err = Theme::from_json(r#"{"primary": "not-a-color"}"#).unwrap_err();
        assert_eq!(
            err,
            ThemeJsonError::BadValue("primary".into(), "not-a-color".into())
        );
        let err = Theme::from_json(r#"{"radius": "huge"}"#).unwrap_err();
        assert_eq!(
            err,
            ThemeJsonError::BadValue("radius".into(), "huge".into())
        );
        let err = Theme::from_json(r#"{"scheme": "sepia"}"#).unwrap_err();
        assert_eq!(
            err,
            ThemeJsonError::BadValue("scheme".into(), "sepia".into())
        );
    }

    #[test]
    fn syntax_errors_carry_a_reason() {
        assert!(matches!(
            Theme::from_json("[]"),
            Err(ThemeJsonError::Syntax(_, "expected '{'"))
        ));
        assert!(matches!(
            Theme::from_json(r#"{"a": 1}"#),
            Err(ThemeJsonError::Syntax(_, "expected a string"))
        ));
        assert!(matches!(
            Theme::from_json(r#"{"a": "b"} extra"#),
            Err(ThemeJsonError::Syntax(_, "trailing content"))
        ));
        assert!(matches!(
            Theme::from_json(r#"{"a": "b" "c": "d"}"#),
            Err(ThemeJsonError::Syntax(_, "expected ',' or '}'"))
        ));
    }

    #[test]
    fn string_escapes_round_trip() {
        let theme = Theme::from_json(r#"{"fontfamily": "JetBrains \"Mono\"!"}"#).unwrap();
        assert_eq!(theme.font_family.as_ref(), "JetBrains \"Mono\"!");
    }
}
