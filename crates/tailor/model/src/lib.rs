//! Tailor's document model.
//!
//! Everything Tailor knows about a design, with no gpui in sight: the component
//! catalog, the node tree, the tokens a node can carry, the state and actions
//! that make a document a real component, and the `.tailor` file format.
//!
//! Keeping this crate free of the UI is what lets the tree be unit-tested — the
//! reparent rules, the cycle checks, the undo stack, and the file round-trip are
//! all plain-data logic, and they are where a builder actually goes wrong.

pub mod catalog;
pub mod doc;
pub mod history;
pub mod id;
pub mod lint;
pub mod motion;
pub mod node;
pub mod project;
pub mod props;
pub mod state;
pub mod style;
pub mod tokens;

pub use catalog::{Category, ComponentSpec, Ctor, SlotRef, SlotSpec};
pub use doc::{Canvas, DocKind, Document, PRESETS};
pub use history::History;
pub use id::{IdGen, NodeId};
pub use lint::{Problem, Severity};
pub use motion::{MotionProps, Resolved as ResolvedMotion};
pub use node::{EventSpec, Node, COMPONENT_PREFIX, DEFAULT_SLOT};
pub use project::{
  Flavor, GenSettings, LoadError, Project, Scheme, ThemePreset, ThemeSpec, FORMAT_VERSION,
  THEME_PRESETS,
};
pub use props::{Emit, PropSpec, PropType, PropValue, Props};
pub use state::{ActionDef, StateVar, VarType};
pub use style::{
  Dimension, Direction, Edges, LayoutMode, Overflow, ShadowToken, StyleProps, TextAlign,
};
pub use tokens::{
  AlignToken, ColorSpec, ColorToken, EaseToken, EnterToken, JustifyToken, LoopToken, SizeToken,
  VariantToken,
};

/// Turn a display name into a Rust type name: `main screen` becomes
/// `MainScreen`. Shared by the generator and by the app's rename validation, so
/// what the inspector shows as the generated type is what gets generated.
pub fn pascal_case(input: &str) -> String {
  let mut out = String::with_capacity(input.len());
  let mut capitalize = true;
  for ch in input.chars() {
    if ch.is_alphanumeric() {
      if capitalize {
        out.extend(ch.to_uppercase());
        capitalize = false;
      } else {
        out.push(ch);
      }
    } else {
      capitalize = true;
    }
  }
  if out
    .chars()
    .next()
    .map(|c| c.is_ascii_digit())
    .unwrap_or(true)
  {
    out.insert(0, 'X');
  }
  // `Self` is the only keyword a PascalCase name can land on, and it lands on
  // it from something as ordinary as a document called "self".
  if out == "Self" {
    out.push('_');
  }
  out
}

/// Turn a display name into a Rust identifier: `Email Address` becomes
/// `email_address`. Runs of capitals stay together (`URLField` → `url_field`).
pub fn snake_case(input: &str) -> String {
  let chars: Vec<char> = input.chars().collect();
  let mut out = String::with_capacity(input.len() + 4);
  for (index, ch) in chars.iter().enumerate() {
    if ch.is_alphanumeric() {
      if ch.is_uppercase() {
        let prev_lower = index > 0 && chars[index - 1].is_lowercase();
        let next_lower = chars
          .get(index + 1)
          .map(|c| c.is_lowercase())
          .unwrap_or(false);
        let starts_word =
          prev_lower || (index > 0 && chars[index - 1].is_uppercase() && next_lower);
        if starts_word && !out.ends_with('_') && !out.is_empty() {
          out.push('_');
        }
        out.extend(ch.to_lowercase());
      } else {
        out.push(*ch);
      }
    } else if !out.is_empty() && !out.ends_with('_') {
      out.push('_');
    }
  }
  let trimmed = out.trim_end_matches('_').to_string();
  if trimmed.is_empty() || trimmed.chars().next().unwrap().is_ascii_digit() {
    format!("x{trimmed}")
  } else if is_keyword(&trimmed) {
    format!("{trimmed}_")
  } else {
    trimmed
  }
}

/// The Rust keywords a generated identifier must not collide with. Not the full
/// list — only what a component or variable name plausibly lands on.
fn is_keyword(word: &str) -> bool {
  matches!(
    word,
    "as"
      | "box"
      | "break"
      | "const"
      | "continue"
      | "crate"
      | "else"
      | "enum"
      | "false"
      | "fn"
      | "for"
      | "if"
      | "impl"
      | "in"
      | "let"
      | "loop"
      | "match"
      | "mod"
      | "move"
      | "mut"
      | "ref"
      | "return"
      | "self"
      | "static"
      | "struct"
      | "super"
      | "trait"
      | "true"
      | "type"
      | "unsafe"
      | "use"
      | "where"
      | "while"
      | "async"
      | "await"
      | "dyn"
  )
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn pascal_case_makes_a_type_name() {
    assert_eq!(pascal_case("main screen"), "MainScreen");
    assert_eq!(pascal_case("login-form"), "LoginForm");
    assert_eq!(pascal_case("LoginForm"), "LoginForm");
    assert_eq!(pascal_case("2fa"), "X2fa");
    assert_eq!(pascal_case(""), "X");
    assert_eq!(pascal_case("self"), "Self_");
  }

  #[test]
  fn snake_case_makes_an_identifier() {
    assert_eq!(snake_case("Email Address"), "email_address");
    assert_eq!(snake_case("firstName"), "first_name");
    assert_eq!(snake_case("URLField"), "url_field");
    assert_eq!(snake_case("type"), "type_");
    assert_eq!(snake_case("2nd"), "x2nd");
    assert_eq!(snake_case("  "), "x");
  }
}
