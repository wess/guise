//! The shared guts of every single-line text field.
//!
//! Before this existed each field drew its value as three sibling divs —
//! `before`, a 1px caret, `after` — which meant the caret landed wherever the
//! layout engine happened to break the boxes, the mouse could not point at a
//! character at all, and a value longer than the box was simply clipped. This
//! module replaces that with a real gpui [`Element`] that shapes the line
//! through the text system, so:
//!
//! - the caret sits on a glyph boundary, and the mouse can hit-test into the
//!   text (click to place, drag to select, double-click a word, triple-click
//!   the line),
//! - a long value scrolls horizontally to keep the caret in view instead of
//!   disappearing under the border,
//! - painting registers an [`ElementInputHandler`], which is the only way to
//!   get IME composition, dead keys, press-and-hold accents, the macOS
//!   character palette, and the system's own text services. A plain
//!   `on_key_down` handler cannot see any of them.
//!
//! Text entry therefore does **not** run through key handling: the platform
//! delivers it to [`EntityInputHandler::replace_text_in_range`] after the key
//! handler declines it. Key handling covers navigation, deletion, the
//! clipboard, undo, and focus movement only.
//!
//! A field opts in by implementing [`LineEditor`] and calling
//! [`line_input_handler!`] to get the platform trait for free.

use crate::chord::Chord;
use std::ops::Range;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{
  fill, point, px, size, App, Bounds, ClipboardItem, Context, ElementInputHandler, Entity,
  FocusHandle, GlobalElementId, Hsla, KeyDownEvent, LayoutId, MouseDownEvent, MouseMoveEvent,
  MouseUpEvent, PaintQuad, Pixels, Point, ShapedLine, SharedString, Style, TextRun, UnderlineStyle,
  Window,
};

use super::edit::TextEdit;
use super::editmenu::{self, EditMenu};
use super::{apply_nav, KeyOutcome};
use crate::actions;
use crate::overlay::ContextMenu;
use crate::theme::theme;

/// What a masked field shows instead of its characters.
const MASK: char = '\u{2022}';
/// The same bullet as a string, so masking is one allocation rather than two.
const MASK_STR: &str = "\u{2022}";

/// How long the caret stays visible, then hidden. Matches the platform's own
/// text fields closely enough that the two don't visibly beat against each
/// other when both are on screen.
const BLINK: Duration = Duration::from_millis(530);

/// Breathing room kept between the caret and the field edge while scrolling,
/// so the caret never sits flush against the border.
const SCROLL_PAD: f32 = 2.0;

/// Per-field state owned by the host entity but written by the element: the
/// shaped line and bounds from the last paint (which is what hit-testing and
/// the platform's `bounds_for_range` need), the horizontal scroll offset, the
/// in-progress IME composition, and the caret blink.
#[derive(Debug, Default)]
pub struct LineState {
  /// The line as it was last shaped — masked text for a password field,
  /// the placeholder when the value is empty.
  pub(crate) shaped: Option<ShapedLine>,
  pub(crate) bounds: Option<Bounds<Pixels>>,
  /// How far the text is scrolled left, in pixels, to keep the caret visible.
  pub(crate) scroll: Pixels,
  /// The IME's in-progress composition, as a char range into the real text.
  pub(crate) marked: Option<Range<usize>>,
  /// True between mouse-down and mouse-up, while a drag extends a selection.
  pub(crate) selecting: bool,
  /// Whether the last paint masked the text, so index mapping can undo it.
  pub(crate) masked: bool,
  /// Whether the last paint showed the placeholder rather than a value.
  pub(crate) empty: bool,
  pub(crate) focused: bool,
  pub(crate) caret_on: bool,
  /// Whether a blink task is already running for this field.
  pub(crate) blinking: bool,
  /// The right-click menu, built on each right-click.
  pub(crate) menu: Option<Entity<ContextMenu>>,
}

impl LineState {
  pub fn new() -> Self {
    LineState {
      caret_on: true,
      ..Default::default()
    }
  }

  /// Byte offset into the *shaped* line for a char index into the real text.
  /// The two differ when the field is masked, because one bullet stands in
  /// for one char but takes three bytes.
  fn shaped_byte(&self, edit: &TextEdit, index: usize) -> usize {
    if self.masked {
      index.min(edit.len()) * MASK.len_utf8()
    } else {
      edit.byte_of(index)
    }
  }

  /// The inverse of [`shaped_byte`](Self::shaped_byte).
  fn char_index(&self, edit: &TextEdit, byte: usize) -> usize {
    if self.masked {
      (byte / MASK.len_utf8()).min(edit.len())
    } else {
      edit.char_of(byte)
    }
  }

  /// The char index the given window-space point falls on. Returns `None`
  /// when the field hasn't been painted yet or is showing its placeholder,
  /// in which case there is nowhere to point but the start.
  pub(crate) fn index_at(&self, edit: &TextEdit, position: Point<Pixels>) -> Option<usize> {
    let (bounds, shaped) = (self.bounds?, self.shaped.as_ref()?);
    if self.empty {
      return Some(0);
    }
    let x = position.x - bounds.left() + self.scroll;
    Some(self.char_index(edit, shaped.closest_index_for_x(x)))
  }

  /// Note that something happened worth showing a solid caret for.
  fn wake(&mut self) {
    self.caret_on = true;
  }
}

/// A field that can be drawn and driven by this module's `Line` element.
///
/// Implementors keep the buffer and the [`LineState`]; everything else — glyph
/// layout, hit-testing, scrolling, IME, the clipboard — is shared.
/// The platform trait comes along for the ride: a field that can be drawn by
/// `Line` is by definition one the OS can drive text into, and every
/// implementor gets it from the crate's `line_input_handler!` macro.
pub trait LineEditor: 'static + Sized + gpui::EntityInputHandler {
  fn edit(&self) -> &TextEdit;
  fn edit_mut(&mut self) -> &mut TextEdit;
  fn line(&self) -> &LineState;
  fn line_mut(&mut self) -> &mut LineState;
  fn line_focus(&self) -> &FocusHandle;

  /// Show bullets instead of characters, and refuse to copy or cut.
  fn line_masked(&self) -> bool {
    false
  }

  /// Accept focus, selection, and copy, but reject every mutation.
  fn line_read_only(&self) -> bool {
    false
  }

  /// Cap on the value's length in chars, enforced on typing and on paste the
  /// way an `<input maxlength>` is.
  fn line_max_length(&self) -> Option<usize> {
    None
  }

  /// Narrow what may be entered, the way `<input type="number">` does.
  /// Returns the text to actually insert; dropping every char rejects the
  /// input. Applied to typing, IME, drops, and paste alike, so a field can't
  /// be filled with something it rejects by any route.
  fn line_filter(&self, text: String) -> String {
    text
  }

  /// Called after any mutation so the field can emit its change event. The
  /// element never calls `cx.notify` on the host's behalf; do it here.
  fn line_changed(&mut self, cx: &mut Context<Self>);
}

/// Implement gpui's [`EntityInputHandler`] for a [`LineEditor`].
///
/// The trait can't be blanket-implemented — it's foreign, and the orphan rules
/// reject an uncovered type parameter — so each field opts in by name. Every
/// body here is the same mechanical translation between the platform's UTF-16
/// offsets and the model's char indices.
macro_rules! line_input_handler {
  ($ty:ty) => {
    impl ::gpui::EntityInputHandler for $ty {
      fn text_for_range(
        &mut self,
        range_utf16: ::std::ops::Range<usize>,
        actual: &mut Option<::std::ops::Range<usize>>,
        _window: &mut ::gpui::Window,
        _cx: &mut ::gpui::Context<Self>,
      ) -> Option<String> {
        let range = $crate::input::line::from_utf16(self.edit(), &range_utf16);
        actual.replace($crate::input::line::to_utf16(self.edit(), &range));
        Some($crate::input::line::slice(self.edit(), &range))
      }

      fn selected_text_range(
        &mut self,
        _ignore_disabled: bool,
        _window: &mut ::gpui::Window,
        _cx: &mut ::gpui::Context<Self>,
      ) -> Option<::gpui::UTF16Selection> {
        Some($crate::input::line::utf16_selection(self.edit()))
      }

      fn marked_text_range(
        &self,
        _window: &mut ::gpui::Window,
        _cx: &mut ::gpui::Context<Self>,
      ) -> Option<::std::ops::Range<usize>> {
        let marked = self.line().marked.clone()?;
        Some($crate::input::line::to_utf16(self.edit(), &marked))
      }

      fn unmark_text(&mut self, _window: &mut ::gpui::Window, _cx: &mut ::gpui::Context<Self>) {
        self.line_mut().marked = None;
      }

      fn replace_text_in_range(
        &mut self,
        range_utf16: Option<::std::ops::Range<usize>>,
        text: &str,
        _window: &mut ::gpui::Window,
        cx: &mut ::gpui::Context<Self>,
      ) {
        $crate::input::line::replace(self, range_utf16, text, None, cx);
      }

      fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<::std::ops::Range<usize>>,
        text: &str,
        selected_utf16: Option<::std::ops::Range<usize>>,
        _window: &mut ::gpui::Window,
        cx: &mut ::gpui::Context<Self>,
      ) {
        $crate::input::line::replace(self, range_utf16, text, Some(selected_utf16), cx);
      }

      fn bounds_for_range(
        &mut self,
        range_utf16: ::std::ops::Range<usize>,
        bounds: ::gpui::Bounds<::gpui::Pixels>,
        _window: &mut ::gpui::Window,
        _cx: &mut ::gpui::Context<Self>,
      ) -> Option<::gpui::Bounds<::gpui::Pixels>> {
        $crate::input::line::range_bounds(self, range_utf16, bounds)
      }

      fn character_index_for_point(
        &mut self,
        point: ::gpui::Point<::gpui::Pixels>,
        _window: &mut ::gpui::Window,
        _cx: &mut ::gpui::Context<Self>,
      ) -> Option<usize> {
        let index = self.line().index_at(self.edit(), point)?;
        Some($crate::input::line::to_utf16(self.edit(), &(0..index)).end)
      }
    }
  };
}

pub(crate) use line_input_handler;

// UTF-16 translation
//
// The platform addresses text in UTF-16 code units; the model counts chars.
// These four helpers are the only place that conversion happens.

pub(crate) fn to_utf16(edit: &TextEdit, range: &Range<usize>) -> Range<usize> {
  // Over `chars()` rather than `text()`: the platform calls these several
  // times per keystroke, and `text()` builds a fresh `String` each time.
  let buffer = edit.chars();
  let units = |chars: usize| {
    buffer[..chars.min(buffer.len())]
      .iter()
      .map(|c| c.len_utf16())
      .sum::<usize>()
  };
  units(range.start)..units(range.end)
}

pub(crate) fn from_utf16(edit: &TextEdit, range: &Range<usize>) -> Range<usize> {
  let buffer = edit.chars();
  let chars = |units: usize| {
    let mut seen = 0;
    for (index, c) in buffer.iter().enumerate() {
      if seen >= units {
        return index;
      }
      seen += c.len_utf16();
    }
    buffer.len()
  };
  chars(range.start)..chars(range.end)
}

pub(crate) fn slice(edit: &TextEdit, range: &Range<usize>) -> String {
  let buffer = edit.chars();
  let start = range.start.min(buffer.len());
  let end = range.end.clamp(start, buffer.len());
  buffer[start..end].iter().collect()
}

pub(crate) fn utf16_selection(edit: &TextEdit) -> gpui::UTF16Selection {
  let (start, end) = edit.selection().unwrap_or((edit.cursor(), edit.cursor()));
  let reversed = edit.cursor() == start && start != end;
  gpui::UTF16Selection {
    range: to_utf16(edit, &(start..end)),
    reversed,
  }
}

/// A single-line field never holds a line break, exactly like `<input>`, which
/// flattens them out of anything pasted or dropped into it. Other control
/// characters would render as nothing useful, so they go too — and this is
/// also what makes it impossible for a stray `\t` to be typed.
fn flatten(text: &str) -> String {
  text
    .chars()
    .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
    .filter(|c| !c.is_control())
    .collect()
}

/// The shared body of `replace_text_in_range` and its marked-text sibling.
/// `marking` is `Some(selection)` when the IME is mid-composition.
pub(crate) fn replace<V: LineEditor>(
  this: &mut V,
  range_utf16: Option<Range<usize>>,
  text: &str,
  marking: Option<Option<Range<usize>>>,
  cx: &mut Context<V>,
) {
  if this.line_read_only() {
    return;
  }
  let text = this.line_filter(flatten(text));

  let range = range_utf16
    .map(|r| from_utf16(this.edit(), &r))
    .or_else(|| this.line().marked.clone())
    .unwrap_or_else(|| {
      this
        .edit()
        .selection()
        .map(|(s, e)| s..e)
        .unwrap_or_else(|| this.edit().cursor()..this.edit().cursor())
    });

  // `maxlength` counts what would remain, so a replacement that swaps a
  // selection for something the same size always fits.
  let text = match this.line_max_length() {
    Some(max) => {
      let kept = this.edit().len() - range.len().min(this.edit().len());
      text.chars().take(max.saturating_sub(kept)).collect()
    }
    None => text,
  };

  let start = range.start;
  this.edit_mut().replace_range(range, &text);
  match marking {
    Some(selection) => {
      let end = start + text.chars().count();
      this.line_mut().marked = (!text.is_empty()).then_some(start..end);
      if let Some(selection) = selection {
        let selection = from_utf16(this.edit(), &selection);
        this
          .edit_mut()
          .set_selection(start + selection.start, start + selection.end);
      }
    }
    None => this.line_mut().marked = None,
  }
  this.line_mut().wake();
  this.line_changed(cx);
}

/// Where a char range sits on screen, for the IME's candidate window.
pub(crate) fn range_bounds<V: LineEditor>(
  this: &V,
  range_utf16: Range<usize>,
  bounds: Bounds<Pixels>,
) -> Option<Bounds<Pixels>> {
  let shaped = this.line().shaped.as_ref()?;
  let range = from_utf16(this.edit(), &range_utf16);
  let x = |index: usize| {
    bounds.left() + shaped.x_for_index(this.line().shaped_byte(this.edit(), index))
      - this.line().scroll
  };
  Some(Bounds::from_corners(
    point(x(range.start), bounds.top()),
    point(x(range.end), bounds.bottom()),
  ))
}

// mouse

/// Focus the field and place (or extend) the selection where the user clicked.
/// Click counts follow the platform convention a browser also uses: one places
/// the caret, two takes the word, three takes the whole value.
pub(crate) fn mouse_down<V: LineEditor>(
  this: &mut V,
  event: &MouseDownEvent,
  window: &mut Window,
  cx: &mut Context<V>,
) {
  window.focus(this.line_focus());
  let Some(index) = this.line().index_at(this.edit(), event.position) else {
    cx.notify();
    return;
  };
  match event.click_count {
    1 if event.modifiers.shift => this.edit_mut().extend_to(index),
    1 => this.edit_mut().set_cursor(index),
    2 => {
      let (start, end) = this.edit().word_at(index);
      this.edit_mut().set_selection(start, end);
    }
    _ => this.edit_mut().select_all(),
  }
  this.line_mut().selecting = true;
  this.line_mut().wake();
  cx.notify();
}

/// Open the Cut / Copy / Paste menu. A right-click outside the selection
/// moves the caret there first, the way a native field does; inside it, the
/// selection stays so the menu acts on it.
pub(crate) fn context_menu<V: LineEditor>(
  this: &mut V,
  event: &MouseDownEvent,
  window: &mut Window,
  cx: &mut Context<V>,
) {
  window.focus(this.line_focus());
  if let Some(index) = this.line().index_at(this.edit(), event.position) {
    let inside = this
      .edit()
      .selection()
      .is_some_and(|(start, end)| index >= start && index <= end);
    if !inside {
      this.edit_mut().set_cursor(index);
    }
  }
  let menu = EditMenu::new(
    this.edit().has_selection(),
    this.line_masked(),
    this.line_read_only(),
  );
  let focus = this.line_focus().clone();
  let mut slot = this.line_mut().menu.take();
  editmenu::open(&mut slot, menu, &focus, event.position, window, cx);
  this.line_mut().menu = slot;
  cx.stop_propagation();
  cx.notify();
}

/// Extend the selection while the button is held.
pub(crate) fn mouse_move<V: LineEditor>(
  this: &mut V,
  event: &MouseMoveEvent,
  _window: &mut Window,
  cx: &mut Context<V>,
) {
  if !this.line().selecting {
    return;
  }
  if let Some(index) = this.line().index_at(this.edit(), event.position) {
    this.edit_mut().extend_to(index);
    cx.notify();
  }
}

pub(crate) fn mouse_up<V: LineEditor>(
  this: &mut V,
  _event: &MouseUpEvent,
  _window: &mut Window,
  cx: &mut Context<V>,
) {
  if this.line().selecting {
    this.line_mut().selecting = false;
    cx.notify();
  }
}

/// Attach the mouse handling every field shares, answer the
/// [`actions`](crate::actions) a host's Edit menu sends, and mark it a text
/// surface.
///
/// The four handlers have to travel together — a selection drag that only
/// registers `on_mouse_up` and not `on_mouse_up_out` never ends when the
/// pointer leaves the field — so they are applied in one place rather than
/// copied into each field's `render`. Fields that need more (a picker that
/// also opens its list) add their own handler after; gpui runs both.
pub(crate) fn wire<V: LineEditor>(
  element: gpui::Stateful<gpui::Div>,
  focus: &FocusHandle,
  state: &LineState,
  cx: &mut Context<V>,
) -> gpui::Stateful<gpui::Div> {
  element
    .track_focus(focus)
    .on_mouse_down(gpui::MouseButton::Right, cx.listener(context_menu))
    .children(editmenu::slot(&state.menu))
    .cursor(gpui::CursorStyle::IBeam)
    .on_mouse_down(gpui::MouseButton::Left, cx.listener(mouse_down))
    .on_mouse_move(cx.listener(mouse_move))
    .on_mouse_up(gpui::MouseButton::Left, cx.listener(mouse_up))
    .on_mouse_up_out(gpui::MouseButton::Left, cx.listener(mouse_up))
    .on_action(cx.listener(|this: &mut V, _: &actions::Copy, _, cx| {
      copy(this, cx);
    }))
    .on_action(cx.listener(|this: &mut V, _: &actions::Cut, _, cx| {
      let outcome = cut(this, cx);
      settle(this, outcome, cx);
    }))
    .on_action(cx.listener(|this: &mut V, _: &actions::Paste, _, cx| {
      let outcome = paste(this, cx);
      settle(this, outcome, cx);
    }))
    .on_action(cx.listener(|this: &mut V, _: &actions::Undo, _, cx| {
      let outcome = history(this, true, cx);
      settle(this, outcome, cx);
    }))
    .on_action(cx.listener(|this: &mut V, _: &actions::Redo, _, cx| {
      let outcome = history(this, false, cx);
      settle(this, outcome, cx);
    }))
    .on_action(cx.listener(|this: &mut V, _: &actions::SelectAll, _, cx| {
      this.edit_mut().select_all();
      this.line_mut().wake();
      cx.notify();
    }))
}

/// What a field's own key handler does with an outcome, for the action path,
/// which has no key handler to hand it back to.
fn settle<V: LineEditor>(this: &mut V, outcome: KeyOutcome, cx: &mut Context<V>) {
  if outcome == KeyOutcome::Edited {
    this.line_changed(cx);
  } else {
    cx.notify();
  }
}

/// Give a field the Tab-order and focus accessors every form control needs.
///
/// These were copy-pasted into three fields and simply absent from the other
/// four, which left half the inputs in the Tab ring with no way for a host to
/// order them or focus them on open.
macro_rules! line_focus_builders {
  ($ty:ty) => {
    impl $ty {
      /// Where this field sits in the window's Tab order. Fields default
      /// to 0, which walks them in render order; set this only to
      /// override that.
      pub fn tab_index(mut self, index: isize) -> Self {
        self.focus = self.focus.clone().tab_index(index);
        self
      }

      /// Leave the field out of the Tab order without disabling it.
      pub fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.focus = self.focus.clone().tab_stop(tab_stop);
        self
      }

      /// The field's focus handle, so a host can focus it on open.
      pub fn focus_handle(&self) -> ::gpui::FocusHandle {
        self.focus.clone()
      }
    }
  };
}

pub(crate) use line_focus_builders;

// keyboard

/// The keys a single-line field handles itself: the clipboard, undo, focus
/// movement, then navigation and deletion via
/// [`apply_nav`](super::apply_nav).
///
/// Printable input is deliberately absent. Returning [`KeyOutcome::Pass`] for
/// it lets the platform hand the key to the input handler instead, which is
/// what makes dead keys, IME composition, and press-and-hold work.
pub(crate) fn keys<V: LineEditor>(
  this: &mut V,
  event: &KeyDownEvent,
  window: &mut Window,
  cx: &mut Context<V>,
) -> KeyOutcome {
  if editmenu::is_open(&this.line().menu, cx) {
    return KeyOutcome::Pass;
  }
  let ks = &event.keystroke;
  let m = &ks.modifiers;

  // Tab is focus movement, never a character. Without this the platform
  // reports a `\t` for it and the field would type one, which is the one
  // behaviour every HTML form gets right for free.
  if ks.key == "tab" && !m.platform && !m.control {
    if m.shift {
      window.focus_prev();
    } else {
      window.focus_next();
    }
    cx.stop_propagation();
    return KeyOutcome::Edited;
  }

  if m.shortcut() {
    match ks.key.as_str() {
      "c" => return copy(this, cx),
      "x" => return cut(this, cx),
      "v" => return paste(this, cx),
      "z" if m.shift => return history(this, false, cx),
      "z" => return history(this, true, cx),
      "y" => return history(this, false, cx),
      _ => {}
    }
  }

  if this.line_read_only() && mutates(ks.key.as_str()) {
    return KeyOutcome::Pass;
  }

  let outcome = apply_nav(this.edit_mut(), ks);
  if outcome == KeyOutcome::Edited {
    this.line_mut().wake();
  }
  outcome
}

/// Whether a key would change the text, for the read-only check.
fn mutates(key: &str) -> bool {
  matches!(key, "backspace" | "delete" | "k")
}

fn copy<V: LineEditor>(this: &mut V, cx: &mut Context<V>) -> KeyOutcome {
  if !this.line_masked() {
    if let Some(text) = this.edit().selected_text() {
      cx.write_to_clipboard(ClipboardItem::new_string(text));
    }
  }
  cx.stop_propagation();
  KeyOutcome::Pass
}

fn cut<V: LineEditor>(this: &mut V, cx: &mut Context<V>) -> KeyOutcome {
  if this.line_masked() || this.line_read_only() {
    cx.stop_propagation();
    return KeyOutcome::Pass;
  }
  let Some(text) = this.edit().selected_text() else {
    cx.stop_propagation();
    return KeyOutcome::Pass;
  };
  cx.write_to_clipboard(ClipboardItem::new_string(text));
  this.edit_mut().delete_selection();
  this.line_mut().wake();
  cx.stop_propagation();
  KeyOutcome::Edited
}

fn paste<V: LineEditor>(this: &mut V, cx: &mut Context<V>) -> KeyOutcome {
  if this.line_read_only() {
    cx.stop_propagation();
    return KeyOutcome::Pass;
  }
  let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
    cx.stop_propagation();
    return KeyOutcome::Pass;
  };
  let text = this.line_filter(flatten(&text));
  let text = match this.line_max_length() {
    Some(max) => {
      let selected = this.edit().selection().map_or(0, |(s, e)| e - s);
      let room = max.saturating_sub(this.edit().len() - selected);
      text.chars().take(room).collect()
    }
    None => text,
  };
  this.edit_mut().break_undo();
  this.edit_mut().insert(&text);
  this.edit_mut().break_undo();
  this.line_mut().wake();
  cx.stop_propagation();
  KeyOutcome::Edited
}

fn history<V: LineEditor>(this: &mut V, undo: bool, cx: &mut Context<V>) -> KeyOutcome {
  if this.line_read_only() {
    cx.stop_propagation();
    return KeyOutcome::Pass;
  }
  let changed = if undo {
    this.edit_mut().undo()
  } else {
    this.edit_mut().redo()
  };
  this.line_mut().wake();
  cx.stop_propagation();
  if changed {
    KeyOutcome::Edited
  } else {
    KeyOutcome::Pass
  }
}

// the element

/// The text, caret, and selection of a single-line field.
///
/// Give it the entity that owns the buffer and the colors already resolved
/// from the theme; it handles the rest. Put it inside the field's chrome —
/// it fills the width it is given and is one line tall.
pub struct Line<V: LineEditor> {
  field: Entity<V>,
  placeholder: SharedString,
  /// Only the placeholder's color varies between fields — a picker that has
  /// a value shows it in the text color rather than dimmed. The rest is the
  /// theme's, resolved at paint.
  placeholder_color: Option<Hsla>,
}

impl<V: LineEditor> Line<V> {
  pub fn new(field: Entity<V>) -> Self {
    Line {
      field,
      placeholder: SharedString::default(),
      placeholder_color: None,
    }
  }

  /// Text to show while the field is empty, and the color to show it in.
  pub fn placeholder(mut self, placeholder: impl Into<SharedString>, color: Hsla) -> Self {
    self.placeholder = placeholder.into();
    self.placeholder_color = Some(color);
    self
  }
}

/// What `prepaint` worked out for `paint` to draw.
pub struct LinePrepaint {
  shaped: Option<ShapedLine>,
  caret: Option<PaintQuad>,
  selection: Option<PaintQuad>,
  scroll: Pixels,
}

impl<V: LineEditor> IntoElement for Line<V> {
  type Element = Self;

  fn into_element(self) -> Self::Element {
    self
  }
}

impl<V: LineEditor> Element for Line<V> {
  type RequestLayoutState = ();
  type PrepaintState = LinePrepaint;

  fn id(&self) -> Option<gpui::ElementId> {
    None
  }

  fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
    None
  }

  fn request_layout(
    &mut self,
    _id: Option<&GlobalElementId>,
    _inspector: Option<&gpui::InspectorElementId>,
    window: &mut Window,
    cx: &mut App,
  ) -> (LayoutId, ()) {
    let mut style = Style::default();
    style.size.width = gpui::relative(1.0).into();
    style.size.height = window.line_height().into();
    (window.request_layout(style, [], cx), ())
  }

  fn prepaint(
    &mut self,
    _id: Option<&GlobalElementId>,
    _inspector: Option<&gpui::InspectorElementId>,
    bounds: Bounds<Pixels>,
    _layout: &mut (),
    window: &mut Window,
    cx: &mut App,
  ) -> LinePrepaint {
    // The field's visuals are the theme's, so they are read here rather
    // than threaded through `Line::new` by seven callers that all passed
    // the same three values.
    let t = theme(cx);
    let text_color = t.text().hsla();
    let caret_color = t.primary().hsla();
    let selection_color = t.selection();
    let dimmed = t.dimmed().hsla();

    // The right-click menu takes focus while it is up, but the selection
    // it acts on should stay visible under it.
    let focused = {
      let field = self.field.read(cx);
      field.line_focus().is_focused(window) || editmenu::is_open(&field.line().menu, cx)
    };
    let field = self.field.read(cx);
    let masked = field.line_masked();
    let empty = field.edit().is_empty();
    let cursor = field.edit().cursor();
    let selection = field.edit().selection();
    let marked = field.line().marked.clone();
    let chars = field.edit().len();
    let caret_on = field.line().caret_on;

    // Built per branch rather than up front: a masked field would otherwise
    // allocate and copy the whole cleartext buffer every frame only to
    // throw it away, and an empty one would build an empty `String`.
    let display: SharedString = if empty {
      self.placeholder.clone()
    } else if masked {
      SharedString::from(MASK_STR.repeat(chars))
    } else {
      SharedString::from(field.edit().text())
    };

    let style = window.text_style();
    let font_size = style.font_size.to_pixels(window.rem_size());
    let color = if empty {
      self.placeholder_color.unwrap_or(dimmed)
    } else {
      text_color
    };
    let run = TextRun {
      len: display.len(),
      font: style.font(),
      color,
      background_color: None,
      underline: None,
      strikethrough: None,
    };

    // The IME underlines what it is still composing, so the user can see
    // which characters are provisional.
    let runs = match marked.filter(|_| !empty) {
      Some(marked) => {
        // Runs have to cover the shaped string exactly — the text
        // system sums their lengths and slices the string by the
        // total, so an over-long run is a panic, not a mis-draw. The
        // marked range is the IME's view of the text and can outlive
        // an edit that shortened it, so clamp rather than trust it.
        let limit = display.len();
        let byte = |index: usize| {
          if masked {
            index.saturating_mul(MASK.len_utf8())
          } else {
            byte_of(&display, index)
          }
          .min(limit)
        };
        let start = byte(marked.start);
        let end = byte(marked.end).max(start);
        vec![
          TextRun {
            len: start,
            ..run.clone()
          },
          TextRun {
            len: end.saturating_sub(start),
            underline: Some(UnderlineStyle {
              color: Some(color),
              thickness: px(1.0),
              wavy: false,
            }),
            ..run.clone()
          },
          TextRun {
            len: display.len().saturating_sub(end),
            ..run
          },
        ]
        .into_iter()
        .filter(|run| run.len > 0)
        .collect()
      }
      None => vec![run],
    };

    // Cloning a `SharedString` is a refcount bump, so the shaped copy and
    // the one the offsets are measured against are the same allocation.
    let shaped = window
      .text_system()
      .shape_line(display.clone(), font_size, &runs, None);

    // Scroll so the caret stays inside the field. Anchoring on the caret
    // rather than the text means a long value slides under the border on
    // whichever side the user is not looking at.
    // Measured against the shaped string, which is the value itself when
    // it isn't masked — so no second copy of it has to be kept alive.
    let byte = |index: usize| {
      if masked {
        index.saturating_mul(MASK.len_utf8())
      } else {
        byte_of(&display, index)
      }
    };
    let caret_x = if empty {
      px(0.0)
    } else {
      shaped.x_for_index(byte(cursor))
    };
    let width = bounds.size.width;
    let pad = px(SCROLL_PAD);
    let mut scroll = self.field.read(cx).line().scroll;
    // Never scroll past the end: shrinking the value should pull the text
    // back rather than leave the field looking empty.
    scroll = scroll.min((shaped.width - width + pad).max(px(0.0)));
    if caret_x - scroll > width - pad {
      scroll = caret_x - width + pad;
    }
    if caret_x - scroll < px(0.0) {
      scroll = caret_x;
    }
    scroll = scroll.max(px(0.0));

    let quads = if focused && !empty {
      match selection {
        Some((start, end)) => (
          None,
          Some(fill(
            Bounds::from_corners(
              point(
                bounds.left() + shaped.x_for_index(byte(start)) - scroll,
                bounds.top(),
              ),
              point(
                bounds.left() + shaped.x_for_index(byte(end)) - scroll,
                bounds.bottom(),
              ),
            ),
            selection_color,
          )),
        ),
        None => (caret_quad(bounds, caret_x - scroll, caret_color), None),
      }
    } else if focused {
      (caret_quad(bounds, caret_x - scroll, caret_color), None)
    } else {
      (None, None)
    };

    self.field.update(cx, |field, cx| {
      let state = field.line_mut();
      state.scroll = scroll;
      state.masked = masked;
      state.empty = empty;
      if state.focused != focused {
        state.focused = focused;
        state.caret_on = true;
      }
      // One blink task per focused field, started the first frame it
      // holds focus and stopped by the task itself when it loses it.
      if focused && !state.blinking {
        state.blinking = true;
        blink(cx);
      }
    });

    LinePrepaint {
      shaped: Some(shaped),
      caret: quads.0.filter(|_| caret_on),
      selection: quads.1,
      scroll,
    }
  }

  fn paint(
    &mut self,
    _id: Option<&GlobalElementId>,
    _inspector: Option<&gpui::InspectorElementId>,
    bounds: Bounds<Pixels>,
    _layout: &mut (),
    prepaint: &mut LinePrepaint,
    window: &mut Window,
    cx: &mut App,
  ) {
    let focus = self.field.read(cx).line_focus().clone();
    window.handle_input(
      &focus,
      ElementInputHandler::new(bounds, self.field.clone()),
      cx,
    );

    let shaped = prepaint.shaped.take().unwrap_or_default();
    let origin = point(bounds.origin.x - prepaint.scroll, bounds.origin.y);
    // Keep the horizontal viewport tight without using the line box as a
    // vertical mask. Fallback glyphs, accents, and emoji can paint outside
    // that box even when their advance metrics fit it; the surrounding
    // control (or window) already supplies the correct vertical clip.
    let parent_mask = window.content_mask().bounds;
    let mask = Bounds::from_corners(
      point(bounds.left(), parent_mask.top()),
      point(bounds.right(), parent_mask.bottom()),
    );
    window.with_content_mask(Some(gpui::ContentMask { bounds: mask }), |window| {
      if let Some(selection) = prepaint.selection.take() {
        window.paint_quad(selection);
      }
      shaped.paint(origin, window.line_height(), window, cx).ok();
      if let Some(caret) = prepaint.caret.take() {
        window.paint_quad(caret);
      }
    });

    self.field.update(cx, |field, _| {
      let state = field.line_mut();
      state.shaped = Some(shaped);
      state.bounds = Some(bounds);
    });
  }
}

fn caret_quad(bounds: Bounds<Pixels>, x: Pixels, color: Hsla) -> Option<PaintQuad> {
  Some(fill(
    Bounds::new(
      point(bounds.left() + x, bounds.top()),
      size(px(1.0), bounds.size.height),
    ),
    color,
  ))
}

/// Byte offset of a char index into `text`.
fn byte_of(text: &str, index: usize) -> usize {
  text
    .char_indices()
    .nth(index)
    .map(|(byte, _)| byte)
    .unwrap_or(text.len())
}

/// Toggle the caret while the field holds focus. The task ends itself the
/// first tick after focus is lost, so nothing has to be cancelled.
fn blink<V: LineEditor>(cx: &mut Context<V>) {
  cx.spawn(async move |field, cx| loop {
    cx.background_executor().timer(BLINK).await;
    let running = field
      .update(cx, |field, cx| {
        let state = field.line_mut();
        if !state.focused {
          state.blinking = false;
          state.caret_on = true;
          cx.notify();
          return false;
        }
        state.caret_on = !state.caret_on;
        cx.notify();
        true
      })
      .unwrap_or(false);
    if !running {
      break;
    }
  })
  .detach();
}
