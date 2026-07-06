//! `PinInput` — segmented one-character code boxes (gpui entity).
//!
//! Owns its slots, cursor, and focus; renders N single-character boxes and
//! emits [`PinInputEvent`] as the code changes or completes. Typing advances,
//! backspace clears and retreats, arrows move, and Cmd+V fills the boxes from
//! the clipboard.
//!
//! ```ignore
//! let pin = cx.new(|cx| PinInput::new(cx).length(6).mask(true));
//! cx.subscribe(&pin, |_this, _pin, event: &PinInputEvent, _cx| {
//!     if let PinInputEvent::Complete(code) = event { /* verify */ }
//! })
//! .detach();
//! ```

use gpui::prelude::*;
use gpui::{
    div, px, App, Context, Entity, EventEmitter, FocusHandle, IntoElement, KeyDownEvent,
    MouseButton, SharedString, Window,
};

use super::control_metrics;
use crate::reactive::Signal;
use crate::theme::{theme, Size};

/// Emitted as the user edits the code.
#[derive(Debug, Clone)]
pub enum PinInputEvent {
    /// The code changed. Carries the filled characters in order.
    Change(String),
    /// Every box is filled. Carries the full code (a `Change` fires first).
    Complete(String),
}

/// The pure editing model: N single-character slots plus an active-slot cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PinModel {
    slots: Vec<Option<char>>,
    cursor: usize,
}

impl PinModel {
    fn new(length: usize) -> Self {
        PinModel {
            slots: vec![None; length.max(1)],
            cursor: 0,
        }
    }

    fn len(&self) -> usize {
        self.slots.len()
    }

    /// The filled characters, in slot order.
    fn value(&self) -> String {
        self.slots.iter().flatten().collect()
    }

    fn is_complete(&self) -> bool {
        self.slots.iter().all(|slot| slot.is_some())
    }

    /// Type one character into the active slot and advance. Whitespace and
    /// control characters are rejected. Returns whether the code changed —
    /// re-typing the character a slot already holds only moves the cursor, so
    /// a full pin never re-emits `Complete` on a no-op keystroke.
    fn insert(&mut self, ch: char) -> bool {
        if ch.is_whitespace() || ch.is_control() {
            return false;
        }
        let changed = self.slots[self.cursor] != Some(ch);
        self.slots[self.cursor] = Some(ch);
        if self.cursor + 1 < self.len() {
            self.cursor += 1;
        }
        changed
    }

    /// Clear the active slot; if it was already empty, retreat and clear that
    /// one instead. Returns whether anything changed.
    fn backspace(&mut self) -> bool {
        if self.slots[self.cursor].is_some() {
            self.slots[self.cursor] = None;
            true
        } else if self.cursor > 0 {
            self.cursor -= 1;
            self.slots[self.cursor] = None;
            true
        } else {
            false
        }
    }

    fn left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    fn right(&mut self) {
        if self.cursor + 1 < self.len() {
            self.cursor += 1;
        }
    }

    /// Replace the code with pasted text: non-whitespace characters fill the
    /// slots from the start. Returns whether the code changed — re-pasting
    /// the identical text is a no-op and must not re-emit `Complete`.
    fn paste(&mut self, text: &str) -> bool {
        let len = self.len();
        let chars: Vec<char> = text
            .chars()
            .filter(|c| !c.is_whitespace() && !c.is_control())
            .take(len)
            .collect();
        if chars.is_empty() {
            return false;
        }
        let before = self.slots.clone();
        self.slots.fill(None);
        for (i, ch) in chars.iter().enumerate() {
            self.slots[i] = Some(*ch);
        }
        self.cursor = chars.len().min(len - 1);
        self.slots != before
    }

    /// Programmatic replace (used by `set_text`/`bind`).
    fn set_value(&mut self, value: &str) {
        let len = self.len();
        self.slots.fill(None);
        for (i, ch) in value.chars().take(len).enumerate() {
            self.slots[i] = Some(ch);
        }
        self.cursor = self
            .slots
            .iter()
            .position(|slot| slot.is_none())
            .unwrap_or(len - 1);
    }

    fn resize(&mut self, length: usize) {
        self.slots.resize(length.max(1), None);
        self.cursor = self.cursor.min(self.slots.len() - 1);
    }
}

/// A one-time-code field. Create with `cx.new(|cx| PinInput::new(cx))`.
pub struct PinInput {
    model: PinModel,
    focus: FocusHandle,
    mask: bool,
    size: Size,
    disabled: bool,
}

impl EventEmitter<PinInputEvent> for PinInput {}

impl PinInput {
    pub fn new(cx: &mut Context<Self>) -> Self {
        PinInput {
            model: PinModel::new(4),
            focus: cx.focus_handle(),
            mask: false,
            size: Size::Sm,
            disabled: false,
        }
    }

    /// Number of boxes (default 4).
    pub fn length(mut self, length: usize) -> Self {
        self.model.resize(length);
        self
    }

    /// Render filled boxes as bullets instead of the typed characters.
    pub fn mask(mut self, mask: bool) -> Self {
        self.mask = mask;
        self
    }

    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Initial code (builder). Extra characters beyond `length` are dropped.
    pub fn value(mut self, value: &str) -> Self {
        self.model.set_value(value);
        self
    }

    /// The field's focus handle, so a host can focus it on open.
    pub fn focus_handle(&self) -> FocusHandle {
        self.focus.clone()
    }

    /// The current code — the filled characters in order.
    pub fn text(&self) -> String {
        self.model.value()
    }

    /// Replace the code programmatically.
    pub fn set_text(&mut self, value: &str, cx: &mut Context<Self>) {
        self.model.set_value(value);
        cx.notify();
    }

    /// Two-way bind this field's code to a `Signal<String>`. The signal is
    /// the source of truth: the field adopts its value now, edits write back
    /// through [`Signal::set_if_changed`], and signal writes replace the code.
    /// Equality guards on both directions prevent update loops.
    pub fn bind(entity: &Entity<PinInput>, signal: &Signal<String>, cx: &mut App) {
        let initial = signal.get(cx);
        entity.update(cx, |this, cx| {
            if this.text() != initial {
                this.set_text(&initial, cx);
            }
        });
        let sink = signal.clone();
        cx.subscribe(entity, move |_pin, event: &PinInputEvent, cx| {
            if let PinInputEvent::Change(text) = event {
                sink.set_if_changed(cx, text.clone());
            }
        })
        .detach();
        let pin = entity.downgrade();
        cx.observe(signal.entity(), move |observed, cx| {
            let value = observed.read(cx).clone();
            pin.update(cx, |this, cx| {
                if this.text() != value {
                    this.set_text(&value, cx);
                }
            })
            .ok();
        })
        .detach();
    }

    /// Emit `Change` (and `Complete` when full), repaint, and consume the key.
    fn emit_edit(&mut self, cx: &mut Context<Self>) {
        let value = self.model.value();
        cx.emit(PinInputEvent::Change(value.clone()));
        if self.model.is_complete() {
            cx.emit(PinInputEvent::Complete(value));
        }
        cx.notify();
        cx.stop_propagation();
    }

    fn on_key(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        let ks = &event.keystroke;
        let m = &ks.modifiers;
        match ks.key.as_str() {
            "left" => {
                self.model.left();
                cx.notify();
                cx.stop_propagation();
            }
            "right" => {
                self.model.right();
                cx.notify();
                cx.stop_propagation();
            }
            "backspace" => {
                if self.model.backspace() {
                    self.emit_edit(cx);
                } else {
                    cx.stop_propagation();
                }
            }
            "v" if m.platform => {
                if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                    if self.model.paste(&text) {
                        self.emit_edit(cx);
                    }
                }
                cx.stop_propagation();
            }
            _ => {
                // Printable input: never on Cmd/Ctrl chords; Option+key is
                // allowed so composed glyphs land (same rule as TextInput).
                if !m.platform && !m.control {
                    if let Some(typed) = ks.key_char.as_deref().filter(|t| !t.is_empty()) {
                        let cursor_before = self.model.cursor;
                        let mut changed = false;
                        for ch in typed.chars() {
                            changed |= self.model.insert(ch);
                        }
                        if changed {
                            self.emit_edit(cx);
                        } else if self.model.cursor != cursor_before {
                            // Same character over a filled slot: the cursor
                            // advanced but the code is untouched — repaint,
                            // no Change/Complete.
                            cx.notify();
                            cx.stop_propagation();
                        }
                    }
                }
            }
        }
    }
}

impl Render for PinInput {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        let (height, _pad_x, font) = control_metrics(self.size);
        let radius = t.radius(t.default_radius);
        let focused = self.focus.is_focused(window) && !self.disabled;

        let border = t.border().hsla();
        let active_border = t.primary().hsla();
        let surface = t.surface().hsla();
        let text_color = t.text().hsla();

        let cursor = self.model.cursor;
        let mask = self.mask;

        let mut row = div()
            .id("guise-pininput")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key))
            .flex()
            .items_center()
            .gap(px(8.0));

        for (i, slot) in self.model.slots.iter().enumerate() {
            let ch = slot.map(|c| if mask { '\u{2022}' } else { c });
            let active = focused && i == cursor;
            let mut cell = div()
                .id(("guise-pin-box", i))
                .flex()
                .items_center()
                .justify_center()
                .w(px(height))
                .h(px(height))
                .rounded(px(radius))
                .border_1()
                .border_color(if active { active_border } else { border })
                .bg(surface)
                .text_size(px(font))
                .text_color(text_color)
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _ev, window, cx| {
                        window.focus(&this.focus, cx);
                        this.model.cursor = i.min(this.model.len() - 1);
                        cx.notify();
                    }),
                );
            if let Some(ch) = ch {
                cell = cell.child(SharedString::from(ch.to_string()));
            }
            row = row.child(cell);
        }

        if self.disabled {
            row.opacity(0.6)
        } else {
            row
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PinModel;

    #[test]
    fn typing_advances_and_stops_at_the_last_box() {
        let mut pin = PinModel::new(4);
        assert!(pin.insert('1'));
        assert!(pin.insert('2'));
        assert_eq!(pin.value(), "12");
        assert_eq!(pin.cursor, 2);

        pin.insert('3');
        pin.insert('4');
        assert_eq!(pin.cursor, 3, "cursor parks on the last box");
        assert!(pin.is_complete());

        // Typing again overwrites the last box in place.
        assert!(pin.insert('9'));
        assert_eq!(pin.value(), "1239");
        assert_eq!(pin.cursor, 3);
    }

    #[test]
    fn retyping_the_same_char_reports_no_change() {
        let mut pin = PinModel::new(4);
        pin.paste("1234");
        // Double-pressing the final digit: complete pin, parked cursor.
        assert!(!pin.insert('4'));
        assert_eq!(pin.value(), "1234");
        // Mid-pin, the cursor still advances but the code is unchanged.
        pin.left();
        assert!(!pin.insert('3'));
        assert_eq!(pin.cursor, 3);
        assert_eq!(pin.value(), "1234");
    }

    #[test]
    fn identical_paste_reports_no_change() {
        let mut pin = PinModel::new(4);
        assert!(pin.paste("1234"));
        assert!(!pin.paste("1234"));
        assert!(pin.paste("129"), "different content still reports a change");
        assert_eq!(pin.value(), "129");
    }

    #[test]
    fn whitespace_and_control_chars_are_rejected() {
        let mut pin = PinModel::new(4);
        assert!(!pin.insert(' '));
        assert!(!pin.insert('\t'));
        assert!(!pin.insert('\u{7}'));
        assert_eq!(pin.value(), "");
        assert_eq!(pin.cursor, 0);
    }

    #[test]
    fn backspace_clears_current_then_retreats() {
        let mut pin = PinModel::new(4);
        pin.insert('1');
        pin.insert('2');
        // cursor sits on the empty third box: retreat and clear box 2.
        assert!(pin.backspace());
        assert_eq!(pin.value(), "1");
        assert_eq!(pin.cursor, 1);
        // Now box 1 (empty after the clear)... clear box 0 next.
        assert!(pin.backspace());
        assert_eq!(pin.value(), "");
        assert_eq!(pin.cursor, 0);
        // Empty at the first box: nothing to do.
        assert!(!pin.backspace());
    }

    #[test]
    fn backspace_clears_a_filled_box_in_place() {
        let mut pin = PinModel::new(4);
        pin.paste("1234");
        assert_eq!(pin.cursor, 3);
        assert!(pin.backspace());
        assert_eq!(pin.value(), "123");
        assert_eq!(pin.cursor, 3, "clears the filled box without retreating");
    }

    #[test]
    fn arrows_clamp_to_the_boxes() {
        let mut pin = PinModel::new(3);
        pin.left();
        assert_eq!(pin.cursor, 0);
        pin.right();
        pin.right();
        pin.right();
        assert_eq!(pin.cursor, 2);
        pin.left();
        assert_eq!(pin.cursor, 1);
    }

    #[test]
    fn paste_fills_from_the_start_and_skips_whitespace() {
        let mut pin = PinModel::new(4);
        pin.insert('9');
        assert!(pin.paste(" 12 34 56 "));
        assert_eq!(pin.value(), "1234", "replaces old content, truncates");
        assert!(pin.is_complete());
        assert_eq!(pin.cursor, 3);
    }

    #[test]
    fn short_paste_leaves_the_cursor_on_the_next_empty_box() {
        let mut pin = PinModel::new(6);
        assert!(pin.paste("12"));
        assert_eq!(pin.value(), "12");
        assert_eq!(pin.cursor, 2);
        assert!(!pin.paste("   "), "whitespace-only paste is a no-op");
    }

    #[test]
    fn set_value_places_the_cursor_at_the_first_empty_box() {
        let mut pin = PinModel::new(4);
        pin.set_value("12");
        assert_eq!(pin.cursor, 2);
        pin.set_value("123456");
        assert_eq!(pin.value(), "1234", "extra characters are dropped");
        assert_eq!(pin.cursor, 3);
        pin.set_value("");
        assert_eq!(pin.value(), "");
        assert_eq!(pin.cursor, 0);
    }

    #[test]
    fn resize_preserves_slots_and_clamps_the_cursor() {
        let mut pin = PinModel::new(6);
        pin.paste("123456");
        pin.resize(3);
        assert_eq!(pin.value(), "123");
        assert_eq!(pin.cursor, 2);
        pin.resize(5);
        assert_eq!(pin.value(), "123");
        assert_eq!(pin.len(), 5);
        // Zero-length requests are clamped to one box.
        pin.resize(0);
        assert_eq!(pin.len(), 1);
        assert_eq!(pin.cursor, 0);
    }
}
