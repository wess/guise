//! The edit actions every text surface answers: copy, cut, paste, select
//! all, undo, redo.
//!
//! The fields already handle the keystrokes themselves, so an app that does
//! nothing gets the clipboard for free. These exist for the app that does
//! something: gpui matches keymap bindings *before* any `on_key_down` runs, so
//! the moment a host binds cmd-c to its own action — which it must, to show
//! the shortcut in an Edit menu — that binding swallows the key and every
//! field goes deaf to it. Binding these instead routes the key (and a click
//! on the menu item) to whichever field holds focus. When no field does, the
//! action finds no handler and falls through to the host's own.
//!
//! Not in the prelude on purpose: a glob import of `Copy` would shadow the
//! std trait of the same name.
//!
//! ```ignore
//! cx.bind_keys(guise::actions::key_bindings());
//! cx.set_menus(vec![Menu {
//!   name: "Edit".into(),
//!   items: vec![
//!     MenuItem::action("Cut", guise::actions::Cut),
//!     MenuItem::action("Copy", guise::actions::Copy),
//!     MenuItem::action("Paste", guise::actions::Paste),
//!   ],
//! }]);
//! ```

use gpui::KeyBinding;

gpui::actions!(guise, [Copy, Cut, Paste, SelectAll, Undo, Redo]);

/// The platform's usual chords for the six actions: cmd on macOS, ctrl
/// elsewhere (gpui's `secondary` modifier). Redo takes both conventions,
/// shift+z and y.
pub fn key_bindings() -> Vec<KeyBinding> {
  vec![
    KeyBinding::new("secondary-c", Copy, None),
    KeyBinding::new("secondary-x", Cut, None),
    KeyBinding::new("secondary-v", Paste, None),
    KeyBinding::new("secondary-a", SelectAll, None),
    KeyBinding::new("secondary-z", Undo, None),
    KeyBinding::new("secondary-shift-z", Redo, None),
    KeyBinding::new("secondary-y", Redo, None),
  ]
}
