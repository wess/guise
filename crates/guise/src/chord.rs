//! Which physical modifier a text-editing gesture means on this OS.
//!
//! The key handlers were written against macOS, where Cmd is the shortcut key,
//! Option moves by word and Cmd+arrow jumps to a line edge. Elsewhere the same
//! gestures are Ctrl+key, Ctrl+arrow and the Home/End keys — and Ctrl+A is
//! select-all, not Emacs line-start. Handlers ask for the gesture (`cmd`,
//! `word`, `line`, `emacs`) instead of reading `platform`/`alt`/`control`, so
//! the mapping lives in one place. The `*_on` functions take the OS as a
//! parameter so both tables are testable from one machine.

use gpui::Modifiers;

/// Shortcut key: Cmd on macOS, Ctrl elsewhere (gpui's `secondary`).
pub(crate) const fn cmd_on(m: &Modifiers, mac: bool) -> bool {
  if mac {
    m.platform
  } else {
    m.control
  }
}

/// Word-wise movement and deletion: Option on macOS, Ctrl elsewhere.
pub(crate) const fn word_on(m: &Modifiers, mac: bool) -> bool {
  if mac {
    m.alt
  } else {
    m.control
  }
}

/// Jump to the line edge with an arrow: Cmd+←/→ on macOS. Other platforms have
/// Home/End for that, so this is never true there.
pub(crate) const fn line_on(m: &Modifiers, mac: bool) -> bool {
  mac && m.platform
}

/// Emacs-style Ctrl+A/E/K. macOS text fields honour them; elsewhere Ctrl+A is
/// select-all, so they are off.
pub(crate) const fn emacs_on(m: &Modifiers, mac: bool) -> bool {
  mac && m.control
}

/// A clean shortcut chord: the shortcut key with no Option, and on macOS no
/// Ctrl either (Cmd+Ctrl+key is the system's, not ours).
pub(crate) const fn shortcut_on(m: &Modifiers, mac: bool) -> bool {
  cmd_on(m, mac) && !m.alt && (!mac || !m.control)
}

/// The mapping for the OS this was compiled for.
pub(crate) trait Chord {
  fn cmd(&self) -> bool;
  fn word(&self) -> bool;
  fn line(&self) -> bool;
  fn emacs(&self) -> bool;
  fn shortcut(&self) -> bool;
}

impl Chord for Modifiers {
  fn cmd(&self) -> bool {
    cmd_on(self, cfg!(target_os = "macos"))
  }
  fn word(&self) -> bool {
    word_on(self, cfg!(target_os = "macos"))
  }
  fn line(&self) -> bool {
    line_on(self, cfg!(target_os = "macos"))
  }
  fn emacs(&self) -> bool {
    emacs_on(self, cfg!(target_os = "macos"))
  }
  fn shortcut(&self) -> bool {
    shortcut_on(self, cfg!(target_os = "macos"))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn mods(platform: bool, control: bool, alt: bool) -> Modifiers {
    Modifiers {
      platform,
      control,
      alt,
      ..Default::default()
    }
  }

  #[test]
  fn cmd_is_the_platform_key_on_mac_and_control_elsewhere() {
    assert!(cmd_on(&mods(true, false, false), true));
    assert!(!cmd_on(&mods(false, true, false), true));
    assert!(cmd_on(&mods(false, true, false), false));
    assert!(!cmd_on(&mods(true, false, false), false));
  }

  #[test]
  fn word_is_option_on_mac_and_control_elsewhere() {
    assert!(word_on(&mods(false, false, true), true));
    assert!(!word_on(&mods(false, true, false), true));
    assert!(word_on(&mods(false, true, false), false));
    assert!(!word_on(&mods(false, false, true), false));
  }

  #[test]
  fn line_edges_and_emacs_keys_are_mac_only() {
    assert!(line_on(&mods(true, false, false), true));
    assert!(!line_on(&mods(true, false, false), false));
    assert!(emacs_on(&mods(false, true, false), true));
    assert!(!emacs_on(&mods(false, true, false), false));
  }

  #[test]
  fn shortcut_rejects_option_and_mac_ctrl() {
    assert!(shortcut_on(&mods(true, false, false), true));
    assert!(!shortcut_on(&mods(true, false, true), true));
    assert!(!shortcut_on(&mods(true, true, false), true));
    assert!(shortcut_on(&mods(false, true, false), false));
    assert!(!shortcut_on(&mods(false, true, true), false));
  }
}
