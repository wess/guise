//! The text, caret, and selection of a [`TextArea`], drawn as wrapped lines
//! shaped through the text system.
//!
//! `TextArea` used to draw its value as divs — one per line, the selected
//! part in a tinted box — which put the text on screen but left nothing to
//! point at: a click could only focus the field, never place the caret, and
//! there was no way to select with the mouse at all. This is the multi-line
//! sibling of [`line`](super::line)'s `Line`: it keeps the shaped layout from
//! the last paint so a window-space point maps back to a char, and painting
//! registers an [`ElementInputHandler`] so IME, dead keys, and press-and-hold
//! reach the field.
//!
//! Text wraps at the field's width. Each logical line (split on `\n`) is one
//! `WrappedLine`; the byte offset where it starts is kept beside it so the
//! two coordinate systems — the model's chars and the layout's per-line
//! bytes — can be crossed in either direction.

use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;

use gpui::{
  fill, point, px, size, App, AvailableSpace, Bounds, ElementInputHandler, Entity, GlobalElementId,
  Hsla, LayoutId, PaintQuad, Pixels, Point, ScrollHandle, SharedString, Style, TextAlign, TextRun,
  UnderlineStyle, Window, WrappedLine,
};

use super::textarea::TextArea;
use crate::theme::theme;

/// Width of the sliver painted for a selected line break, so a selection
/// that spans an empty line still shows it is covered.
const NEWLINE_WIDTH: f32 = 4.0;

/// Geometry from the last paint, written by the element and read by the
/// field's mouse and IME handling.
#[derive(Default)]
pub(crate) struct AreaState {
  /// Each logical line with the byte offset into the value where it starts.
  lines: Rc<Vec<(usize, WrappedLine)>>,
  bounds: Option<Bounds<Pixels>>,
  line_height: Pixels,
  /// Whether the last paint showed the placeholder rather than a value.
  empty: bool,
  /// The IME's in-progress composition, as a char range into the value.
  pub(crate) marked: Option<Range<usize>>,
  /// True between mouse-down and mouse-up, while a drag extends a selection.
  pub(crate) selecting: bool,
  /// Set when the caret moved by keyboard or typing, so the next paint
  /// scrolls it into view. A flag rather than every frame: scrolling to the
  /// caret unconditionally would fight the user's own scroll wheel.
  pub(crate) reveal: bool,
  /// Where ↑/↓ aim, horizontally: the caret's x when the first vertical
  /// move started, kept across a run of them so the column doesn't drift
  /// through short rows. Cleared by anything else that moves the caret.
  pub(crate) goal_x: Option<Pixels>,
}

impl AreaState {
  /// The byte offset into the value for a window-space point, or `None`
  /// before the first paint.
  pub(crate) fn byte_at(&self, position: Point<Pixels>) -> Option<usize> {
    let bounds = self.bounds?;
    if self.empty {
      return Some(0);
    }
    let lh = self.line_height;
    let mut y = position.y - bounds.top();
    let last = self.lines.len().checked_sub(1)?;
    for (ix, (start, line)) in self.lines.iter().enumerate() {
      let height = line.size(lh).height;
      if y < height || ix == last {
        let local = point(
          position.x - bounds.left(),
          y.max(px(0.0)).min(height - px(1.0)),
        );
        let index = match line.closest_index_for_position(local, lh) {
          Ok(index) | Err(index) => index,
        };
        return Some(start + index.min(line.len()));
      }
      y -= height;
    }
    None
  }

  /// Where the given byte offset sits on screen: the top of its row, in
  /// window space.
  pub(crate) fn point_for(&self, byte: usize) -> Option<Point<Pixels>> {
    let bounds = self.bounds?;
    position(&self.lines, self.line_height, byte).map(|p| bounds.origin + p)
  }

  /// The byte one visual row above or below `byte`, aiming at `goal_x`, and
  /// the x that was aimed at. Rows are what the user sees, so a long line
  /// that wraps takes several presses to cross — the way every native text
  /// view moves. Past the first or last row it goes to the start or end.
  pub(crate) fn vertical(
    &self,
    byte: usize,
    goal_x: Option<Pixels>,
    down: bool,
  ) -> Option<(usize, Pixels)> {
    let bounds = self.bounds?;
    if self.empty {
      return Some((0, px(0.0)));
    }
    let at = self.point_for(byte)?;
    let x = goal_x.unwrap_or(at.x - bounds.left());
    let lh = self.line_height;
    // Aim at the middle of the neighbouring row.
    let y = if down {
      at.y + lh * 1.5
    } else {
      at.y - lh * 0.5
    };
    if y < bounds.top() {
      return Some((0, x));
    }
    if y >= bounds.bottom() {
      let (start, line) = self.lines.last()?;
      return Some((start + line.len(), x));
    }
    Some((self.byte_at(point(bounds.left() + x, y))?, x))
  }

  pub(crate) fn line_height(&self) -> Pixels {
    self.line_height
  }
}

/// The offset of `byte` from the top-left of the text, given its lines.
fn position(lines: &[(usize, WrappedLine)], lh: Pixels, byte: usize) -> Option<Point<Pixels>> {
  let mut y = px(0.0);
  for (start, line) in lines {
    if byte <= start + line.len() {
      let local = line.position_for_index(byte.saturating_sub(*start), lh)?;
      return Some(point(local.x, y + local.y));
    }
    y += line.size(lh).height;
  }
  None
}

/// Draws a [`TextArea`]'s value. Build it in the field's `render` with the
/// field's own entity and the placeholder; visuals come from the theme.
pub(crate) struct AreaText {
  field: Entity<TextArea>,
  placeholder: SharedString,
  scroll: Option<ScrollHandle>,
}

impl AreaText {
  pub(crate) fn new(field: Entity<TextArea>, placeholder: SharedString) -> Self {
    AreaText {
      field,
      placeholder,
      scroll: None,
    }
  }

  /// The scroll container the field sits in, when it caps its height, so the
  /// caret can be kept in view.
  pub(crate) fn scroll(mut self, scroll: Option<ScrollHandle>) -> Self {
    self.scroll = scroll;
    self
  }
}

/// The value as shaped at some wrap width, shared between the layout
/// measure callback and `prepaint`.
struct Shaped {
  text: SharedString,
  runs: Vec<TextRun>,
  font_size: Pixels,
  line_height: Pixels,
  wrap_width: Option<Pixels>,
  /// Shared with the field's [`AreaState`] after paint: this gpui's
  /// `WrappedLine` isn't `Clone`.
  lines: Rc<Vec<(usize, WrappedLine)>>,
  height: Pixels,
}

impl Shaped {
  fn shape(&mut self, wrap_width: Option<Pixels>, window: &Window) {
    let shaped = window
      .text_system()
      .shape_text(
        self.text.clone(),
        self.font_size,
        &self.runs,
        wrap_width,
        None,
      )
      .unwrap_or_default();
    let mut start = 0;
    let mut height = px(0.0);
    self.lines = Rc::new(
      shaped
        .into_iter()
        .map(|line| {
          let at = start;
          start += line.len() + 1;
          height += line.size(self.line_height).height;
          (at, line)
        })
        .collect(),
    );
    self.height = height.max(self.line_height);
    self.wrap_width = wrap_width;
  }
}

pub(crate) struct AreaLayout {
  shaped: Rc<RefCell<Shaped>>,
  focused: bool,
  empty: bool,
}

pub(crate) struct AreaPrepaint {
  caret: Option<PaintQuad>,
  newlines: Vec<PaintQuad>,
}

impl gpui::IntoElement for AreaText {
  type Element = Self;

  fn into_element(self) -> Self::Element {
    self
  }
}

impl gpui::Element for AreaText {
  type RequestLayoutState = AreaLayout;
  type PrepaintState = AreaPrepaint;

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
  ) -> (LayoutId, AreaLayout) {
    let t = theme(cx);
    let text_color = t.text().hsla();
    let dimmed = t.dimmed().hsla();
    let selection_color = t.selection();

    let field = self.field.read(cx);
    // The right-click menu takes focus while it is up, but the selection
    // it acts on should stay visible under it.
    let focused = (field.focus.is_focused(window) || field.menu_open(cx)) && !field.disabled;
    let edit = &field.edit;
    let empty = edit.is_empty();

    let style = window.text_style();
    let font = style.font();
    let font_size = style.font_size.to_pixels(window.rem_size());
    let line_height = window.line_height();

    let (text, runs) = if empty {
      let text = self.placeholder.clone();
      let run = run(&font, text.len(), dimmed, None, false);
      (text, vec![run])
    } else {
      let text = SharedString::from(edit.text());
      let len = text.len();
      let clamp = |byte: usize| byte.min(len);
      let selection = edit
        .selection()
        .filter(|_| focused)
        .map(|(s, e)| clamp(edit.byte_of(s))..clamp(edit.byte_of(e)));
      let marked = field
        .area
        .marked
        .clone()
        .map(|m| clamp(edit.byte_of(m.start))..clamp(edit.byte_of(m.end)));
      // Cut the value at every edge of the selection and the composition,
      // then style each piece by which of the two it falls in. Runs must
      // cover the string exactly — the text system slices by their sum.
      let mut edges = vec![0, len];
      for r in selection.iter().chain(marked.iter()) {
        edges.extend([r.start, r.end]);
      }
      edges.sort_unstable();
      edges.dedup();
      let within =
        |r: &Option<Range<usize>>, at: usize| r.as_ref().is_some_and(|r| r.contains(&at));
      let runs = edges
        .windows(2)
        .filter(|w| w[1] > w[0])
        .map(|w| {
          let background = within(&selection, w[0]).then_some(selection_color);
          run(
            &font,
            w[1] - w[0],
            text_color,
            background,
            within(&marked, w[0]),
          )
        })
        .collect();
      (text, runs)
    };

    let shaped = Rc::new(RefCell::new(Shaped {
      text,
      runs,
      font_size,
      line_height,
      wrap_width: None,
      lines: Rc::default(),
      height: line_height,
    }));

    let mut style = Style::default();
    style.size.width = gpui::relative(1.0).into();
    let measured = shaped.clone();
    let layout_id = window.request_measured_layout(style, move |known, available, window, _cx| {
      let wrap = known.width.or(match available.width {
        AvailableSpace::Definite(width) => Some(width),
        _ => None,
      });
      let mut shaped = measured.borrow_mut();
      if shaped.lines.is_empty() || shaped.wrap_width != wrap {
        shaped.shape(wrap, window);
      }
      let width = wrap.unwrap_or_else(|| {
        shaped
          .lines
          .iter()
          .map(|(_, line)| line.width())
          .fold(px(0.0), Pixels::max)
      });
      size(width, shaped.height)
    });

    (
      layout_id,
      AreaLayout {
        shaped,
        focused,
        empty,
      },
    )
  }

  fn prepaint(
    &mut self,
    _id: Option<&GlobalElementId>,
    _inspector: Option<&gpui::InspectorElementId>,
    bounds: Bounds<Pixels>,
    layout: &mut AreaLayout,
    window: &mut Window,
    cx: &mut App,
  ) -> AreaPrepaint {
    let t = theme(cx);
    let caret_color = t.primary().hsla();
    let selection_color = t.selection();

    let mut shaped = layout.shaped.borrow_mut();
    if shaped.lines.is_empty() || shaped.wrap_width != Some(bounds.size.width) {
      shaped.shape(Some(bounds.size.width), window);
    }
    let lh = shaped.line_height;

    let field = self.field.read(cx);
    let edit = &field.edit;
    let mut prepaint = AreaPrepaint {
      caret: None,
      newlines: Vec::new(),
    };
    if !layout.focused {
      return prepaint;
    }

    let cursor = if layout.empty {
      0
    } else {
      edit.byte_of(edit.cursor())
    };
    let caret_at = position(&shaped.lines, lh, cursor).map(|p| bounds.origin + p);

    match edit.selection().filter(|_| !layout.empty) {
      // A selected line break has no glyph to paint a background behind,
      // so it gets a sliver at the end of its line instead.
      Some((start, end)) => {
        let (start, end) = (edit.byte_of(start), edit.byte_of(end));
        let mut y = px(0.0);
        for (line_start, line) in shaped.lines.iter() {
          let newline = line_start + line.len();
          if newline >= start && newline < end {
            if let Some(end_at) = line.position_for_index(line.len(), lh) {
              prepaint.newlines.push(fill(
                Bounds::new(
                  bounds.origin + point(end_at.x, y + end_at.y),
                  size(px(NEWLINE_WIDTH), lh),
                ),
                selection_color,
              ));
            }
          }
          y += line.size(lh).height;
        }
      }
      None => {
        prepaint.caret = caret_at.map(|at| fill(Bounds::new(at, size(px(1.0), lh)), caret_color));
      }
    }

    let reveal = field.area.reveal;
    drop(shaped);
    if reveal {
      if let (Some(scroll), Some(at)) = (self.scroll.as_ref(), caret_at) {
        if reveal_caret(scroll, at, lh) {
          window.on_next_frame(|window, _| window.refresh());
        }
      }
      self.field.update(cx, |field, _| field.area.reveal = false);
    }
    prepaint
  }

  fn paint(
    &mut self,
    _id: Option<&GlobalElementId>,
    _inspector: Option<&gpui::InspectorElementId>,
    bounds: Bounds<Pixels>,
    layout: &mut AreaLayout,
    prepaint: &mut AreaPrepaint,
    window: &mut Window,
    cx: &mut App,
  ) {
    let field = self.field.read(cx);
    if !field.disabled {
      let focus = field.focus.clone();
      window.handle_input(
        &focus,
        ElementInputHandler::new(bounds, self.field.clone()),
        cx,
      );
    }

    let shaped = layout.shaped.borrow();
    let lh = shaped.line_height;
    let mut y = px(0.0);
    for (_, line) in shaped.lines.iter() {
      let origin = bounds.origin + point(px(0.0), y);
      line
        .paint_background(origin, lh, TextAlign::Left, None, window, cx)
        .ok();
      y += line.size(lh).height;
    }
    for quad in prepaint.newlines.drain(..) {
      window.paint_quad(quad);
    }
    let mut y = px(0.0);
    for (_, line) in shaped.lines.iter() {
      let origin = bounds.origin + point(px(0.0), y);
      line
        .paint(origin, lh, TextAlign::Left, None, window, cx)
        .ok();
      y += line.size(lh).height;
    }
    if let Some(caret) = prepaint.caret.take() {
      window.paint_quad(caret);
    }

    let lines = shaped.lines.clone();
    let empty = layout.empty;
    self.field.update(cx, |field, _| {
      field.area.lines = lines;
      field.area.bounds = Some(bounds);
      field.area.line_height = lh;
      field.area.empty = empty;
    });
  }
}

fn run(
  font: &gpui::Font,
  len: usize,
  color: Hsla,
  background: Option<Hsla>,
  marked: bool,
) -> TextRun {
  TextRun {
    len,
    font: font.clone(),
    color,
    background_color: background,
    // The IME underlines what it is still composing, so the user can see
    // which characters are provisional.
    underline: marked.then_some(UnderlineStyle {
      color: Some(color),
      thickness: px(1.0),
      wavy: false,
    }),
    strikethrough: None,
  }
}

/// Scroll so a caret whose row starts at `at` is inside the viewport.
/// Returns whether the offset changed.
fn reveal_caret(scroll: &ScrollHandle, at: Point<Pixels>, lh: Pixels) -> bool {
  let view = scroll.bounds();
  let offset = scroll.offset();
  let shift = if at.y < view.top() {
    view.top() - at.y
  } else if at.y + lh > view.bottom() {
    view.bottom() - (at.y + lh)
  } else {
    return false;
  };
  let max = scroll.max_offset().y;
  let y = (offset.y + shift).min(px(0.0)).max(-max);
  if y == offset.y {
    return false;
  }
  scroll.set_offset(point(offset.x, y));
  true
}
