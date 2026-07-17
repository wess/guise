//! Keyboard handling for single-line text fields.
//!
//! Shared by [`TextInput`](super::TextInput) and by hosts that drive a
//! [`TextEdit`](super::TextEdit) directly — inline fields that render their own
//! chrome (a search bar, a palette) rather than embedding the full component.
//! macOS/Linux conventions: Option = word-wise, Cmd = line-wise, plus the
//! Emacs-style Ctrl+A / Ctrl+E / Ctrl+K.

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
pub fn apply_key(edit: &mut TextEdit, ks: &Keystroke) -> KeyOutcome {
    let m = &ks.modifiers;
    match ks.key.as_str() {
        "enter" => return KeyOutcome::Submit,
        "escape" => return KeyOutcome::Cancel,
        // Cmd/Super+A selects the whole field (Ctrl+A stays Emacs line-start).
        "a" if m.platform => {
            edit.select_all();
            return KeyOutcome::Edited;
        }
        "left" => {
            if !m.shift && !m.platform && !m.alt && edit.collapse_selection_start() {
                return KeyOutcome::Edited;
            }
            edit.pre_move(m.shift);
            if m.platform {
                edit.home();
            } else if m.alt {
                edit.word_left();
            } else {
                edit.left();
            }
            return KeyOutcome::Edited;
        }
        "right" => {
            if !m.shift && !m.platform && !m.alt && edit.collapse_selection_end() {
                return KeyOutcome::Edited;
            }
            edit.pre_move(m.shift);
            if m.platform {
                edit.end();
            } else if m.alt {
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
            if m.platform {
                edit.delete_to_start();
            } else if m.alt {
                edit.delete_word_back();
            } else {
                edit.backspace();
            }
            return KeyOutcome::Edited;
        }
        "delete" => {
            if m.platform {
                edit.delete_to_end();
            } else if m.alt {
                edit.delete_word_forward();
            } else {
                edit.delete();
            }
            return KeyOutcome::Edited;
        }
        "k" if m.control => {
            edit.delete_to_end();
            return KeyOutcome::Edited;
        }
        "a" if m.control => {
            edit.home();
            return KeyOutcome::Edited;
        }
        "e" if m.control => {
            edit.end();
            return KeyOutcome::Edited;
        }
        _ => {}
    }
    // Printable input: never on Cmd/Ctrl chords (those are shortcuts);
    // Option+key is allowed so composed glyphs land.
    if !m.platform && !m.control {
        if let Some(t) = ks.key_char.as_deref().filter(|t| !t.is_empty()) {
            edit.insert(t);
            return KeyOutcome::Edited;
        }
    }
    KeyOutcome::Pass
}
