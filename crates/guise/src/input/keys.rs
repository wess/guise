//! Keyboard handling for single-line text fields.
//!
//! Shared by [`TextInput`](super::TextInput) and by hosts that drive a
//! [`TextEdit`](super::TextEdit) directly — inline fields that render their own
//! chrome (a search bar, a palette) rather than embedding the full component.
//! macOS/Linux conventions: Option = word-wise, Cmd = line-wise, plus the
//! Emacs-style Ctrl+A / Ctrl+E / Ctrl+K.

use crate::chord::Chord;
use gpui::Keystroke;

use super::edit::TextEdit;

/// What a keystroke did to a single-line field, so the host can react.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyOutcome {
  /// Enter — the host should commit.
  Submit,
  /// Escape — the host should dismiss.
  Cancel,
  /// The field changed; redraw.
  Edited,
  /// Not handled here; the host may act on it (e.g. Tab, Cmd+W).
  Pass,
}

/// Apply `ks` to `edit`, returning what the host should do. `platform` is Cmd
/// on macOS; `alt` is Option.
///
/// This is the whole keyboard, including typing the printable character. The
/// components use [`apply_nav`] instead and leave text entry to the platform's
/// input handler, which is what makes IME and dead keys work; this entry point
/// stays for hosts that drive a [`TextEdit`] themselves and only get raw key
/// events.
pub fn apply_key(edit: &mut TextEdit, ks: &Keystroke) -> KeyOutcome {
  let outcome = apply_nav(edit, ks);
  if outcome != KeyOutcome::Pass {
    return outcome;
  }
  // Printable input: never on Cmd/Ctrl chords (those are shortcuts);
  // Option+key is allowed so composed glyphs land. Control characters are
  // filtered out — the platform reports a `\t` for Tab and a `\n` for
  // Enter, and a text field must not type either.
  if !ks.modifiers.platform && !ks.modifiers.control {
    if let Some(text) = ks.key_char.as_deref() {
      let text: String = text.chars().filter(|c| !c.is_control()).collect();
      if !text.is_empty() {
        edit.insert(&text);
        return KeyOutcome::Edited;
      }
    }
  }
  KeyOutcome::Pass
}

/// Navigation, selection, and deletion only — everything a text field does
/// with a key that isn't typing a character. Returns [`KeyOutcome::Pass`] for
/// anything it doesn't recognise, including every printable key.
pub fn apply_nav(edit: &mut TextEdit, ks: &Keystroke) -> KeyOutcome {
  let m = &ks.modifiers;
  match ks.key.as_str() {
    "enter" => return KeyOutcome::Submit,
    "escape" => return KeyOutcome::Cancel,
    // Cmd/Super+A selects the whole field (Ctrl+A stays Emacs line-start).
    "a" if m.cmd() => {
      edit.select_all();
      return KeyOutcome::Edited;
    }
    "left" => {
      if !m.shift && !m.line() && !m.word() && edit.collapse_selection_start() {
        return KeyOutcome::Edited;
      }
      edit.pre_move(m.shift);
      if m.line() {
        edit.home();
      } else if m.word() {
        edit.word_left();
      } else {
        edit.left();
      }
      return KeyOutcome::Edited;
    }
    "right" => {
      if !m.shift && !m.line() && !m.word() && edit.collapse_selection_end() {
        return KeyOutcome::Edited;
      }
      edit.pre_move(m.shift);
      if m.line() {
        edit.end();
      } else if m.word() {
        edit.word_right();
      } else {
        edit.right();
      }
      return KeyOutcome::Edited;
    }
    // Single-line: vertical keys collapse to the line edges.
    "up" | "home" => {
      edit.pre_move(m.shift);
      edit.home();
      return KeyOutcome::Edited;
    }
    "down" | "end" => {
      edit.pre_move(m.shift);
      edit.end();
      return KeyOutcome::Edited;
    }
    "backspace" => {
      if m.line() {
        edit.delete_to_start();
      } else if m.word() {
        edit.delete_word_back();
      } else {
        edit.backspace();
      }
      return KeyOutcome::Edited;
    }
    "delete" => {
      if m.line() {
        edit.delete_to_end();
      } else if m.word() {
        edit.delete_word_forward();
      } else {
        edit.delete();
      }
      return KeyOutcome::Edited;
    }
    "k" if m.emacs() => {
      edit.delete_to_end();
      return KeyOutcome::Edited;
    }
    "a" if m.emacs() => {
      edit.home();
      return KeyOutcome::Edited;
    }
    "e" if m.emacs() => {
      edit.end();
      return KeyOutcome::Edited;
    }
    _ => {}
  }
  KeyOutcome::Pass
}
