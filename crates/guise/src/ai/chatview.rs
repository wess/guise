//! `AIChatView` — the transcript.
//!
//! This owns the conversation so a host doesn't have to re-derive one from its
//! own state every frame: push a user turn, open an assistant turn, feed it
//! deltas as they arrive, close it. The host keeps the network; this keeps the
//! list, the scroll position, and the per-turn disclosure state.
//!
//! The one behaviour worth naming is stick-to-bottom. A transcript that
//! auto-scrolls unconditionally rips the page away from someone reading back
//! through it; one that never scrolls leaves the newest text off-screen. So it
//! follows the tail only while the view is already at the tail, and a scroll
//! away from the bottom detaches it until the user comes back or sends
//! something.
//!
//! ```ignore
//! let chat = cx.new(|cx| AIChatView::new(cx));
//! chat.update(cx, |chat, cx| {
//!     chat.push(AITurn::user(prompt), cx);
//!     chat.begin_reply(cx);
//! });
//! // …as tokens arrive
//! chat.update(cx, |chat, cx| chat.push_delta(&token, cx));
//! chat.update(cx, |chat, cx| chat.end_reply(cx));
//! ```

use gpui::prelude::*;
use gpui::{
  div, px, Context, EventEmitter, FocusHandle, IntoElement, Pixels, ScrollHandle, SharedString,
  Window,
};

use super::{AICitation, AIMessage, AIReasoning, AIRole, AISource, AISources, AIThinking};
use super::{AIToolCall, AIToolStatus};
use crate::devtools::Probed;
use crate::theme::{theme, Size};

/// How close to the end counts as "at the bottom", in pixels. A couple of
/// lines of slack, so a small overscroll or a resize doesn't detach the view.
const FOLLOW_SLACK: f32 = 48.0;

/// One tool invocation inside a turn.
#[derive(Debug, Clone, Default)]
pub struct AITurnTool {
  pub name: String,
  pub status: AIToolStatus,
  pub arguments: Option<String>,
  pub result: Option<String>,
  pub meta: Option<String>,
  /// Whether the card is expanded. Owned here so scrolling away and back
  /// doesn't reset it.
  pub open: bool,
}

impl AITurnTool {
  pub fn new(name: impl Into<String>) -> Self {
    AITurnTool {
      name: name.into(),
      ..Default::default()
    }
  }

  pub fn status(mut self, status: AIToolStatus) -> Self {
    self.status = status;
    self
  }

  pub fn arguments(mut self, arguments: impl Into<String>) -> Self {
    self.arguments = Some(arguments.into());
    self
  }

  pub fn result(mut self, result: impl Into<String>) -> Self {
    self.result = Some(result.into());
    self
  }
}

/// One turn of the conversation.
#[derive(Debug, Clone, Default)]
pub struct AITurn {
  pub role: AIRole,
  pub body: String,
  /// Extended-thinking output, folded away by default.
  pub reasoning: Option<String>,
  pub reasoning_open: bool,
  pub tools: Vec<AITurnTool>,
  pub sources: Vec<AISource>,
  /// Still being written to.
  pub streaming: bool,
  /// The turn failed; the text stays and this is shown under it.
  pub error: Option<String>,
  /// Overrides the role's name in the header — a model id, say.
  pub name: Option<String>,
  /// Trailing header detail: a timestamp, a token count.
  pub meta: Option<String>,
}

impl AITurn {
  pub fn new(role: AIRole, body: impl Into<String>) -> Self {
    AITurn {
      role,
      body: body.into(),
      ..Default::default()
    }
  }

  pub fn user(body: impl Into<String>) -> Self {
    AITurn::new(AIRole::User, body)
  }

  pub fn assistant(body: impl Into<String>) -> Self {
    AITurn::new(AIRole::Assistant, body)
  }

  pub fn system(body: impl Into<String>) -> Self {
    AITurn::new(AIRole::System, body)
  }

  pub fn name(mut self, name: impl Into<String>) -> Self {
    self.name = Some(name.into());
    self
  }

  pub fn meta(mut self, meta: impl Into<String>) -> Self {
    self.meta = Some(meta.into());
    self
  }

  pub fn reasoning(mut self, reasoning: impl Into<String>) -> Self {
    self.reasoning = Some(reasoning.into());
    self
  }

  pub fn sources(mut self, sources: impl IntoIterator<Item = AISource>) -> Self {
    self.sources = sources.into_iter().collect();
    self
  }

  pub fn tools(mut self, tools: impl IntoIterator<Item = AITurnTool>) -> Self {
    self.tools = tools.into_iter().collect();
    self
  }
}

/// What the transcript asks the host to do.
#[derive(Debug, Clone)]
pub enum AIChatViewEvent {
  /// A source was clicked: `(turn index, source index)`.
  OpenSource(usize, usize),
}

/// A scrolling conversation.
pub struct AIChatView {
  turns: Vec<AITurn>,
  scroll: ScrollHandle,
  focus: FocusHandle,
  /// Whether new text should pull the view down. Cleared when the reader
  /// scrolls up, restored when they return to the bottom or send.
  follow: bool,
  /// Shown after the last turn while waiting for the first token.
  pending: Option<SharedString>,
  empty: Option<SharedString>,
  size: Size,
  max_width: Option<f32>,
  /// Height each turn measured at, from the last frame that drew it in full.
  /// A turn scrolled well outside the viewport is replaced by a spacer of
  /// this height — see [`Self::render`].
  heights: Vec<Pixels>,
  /// The viewport width those heights were measured at. A resize reflows
  /// every turn, so it invalidates all of them.
  measured_width: Pixels,
  /// Whether to skip building turns that are far off screen.
  virtualize: bool,
}

impl EventEmitter<AIChatViewEvent> for AIChatView {}

impl AIChatView {
  pub fn new(cx: &mut Context<Self>) -> Self {
    AIChatView {
      turns: Vec::new(),
      scroll: ScrollHandle::new(),
      focus: cx.focus_handle(),
      follow: true,
      pending: None,
      empty: None,
      size: Size::Sm,
      max_width: None,
      heights: Vec::new(),
      measured_width: px(0.0),
      virtualize: true,
    }
  }

  /// Seed the transcript — restoring a saved conversation.
  pub fn turns(mut self, turns: impl IntoIterator<Item = AITurn>) -> Self {
    self.turns = turns.into_iter().collect();
    self
  }

  /// What to show before anything has been said.
  pub fn empty_message(mut self, message: impl Into<SharedString>) -> Self {
    self.empty = Some(message.into());
    self
  }

  pub fn size(mut self, size: Size) -> Self {
    self.size = size;
    self
  }

  /// Cap the reading width and center it. Long lines are hard to read, and a
  /// transcript in a wide window is the usual way to get them.
  pub fn max_width(mut self, width: f32) -> Self {
    self.max_width = Some(width);
    self
  }

  /// Build every turn every frame, however long the conversation gets.
  ///
  /// On by default, virtualizing means a turn scrolled more than a screen
  /// away is drawn as a spacer of the height it last measured, because
  /// building it means re-parsing its markdown — which is linear in the size
  /// of the whole transcript and lands on every frame. Turn it off if you
  /// need every turn's element tree live at all times (an in-place find, a
  /// screenshot of the full history).
  pub fn virtualize(mut self, virtualize: bool) -> Self {
    self.virtualize = virtualize;
    self
  }

  pub fn focus_handle(&self) -> FocusHandle {
    self.focus.clone()
  }

  pub fn turn_count(&self) -> usize {
    self.turns.len()
  }

  pub fn all(&self) -> &[AITurn] {
    &self.turns
  }

  pub fn turn(&self, index: usize) -> Option<&AITurn> {
    self.turns.get(index)
  }

  /// Edit a turn in place — attaching a tool result, marking an error.
  pub fn update_turn(
    &mut self,
    index: usize,
    edit: impl FnOnce(&mut AITurn),
    cx: &mut Context<Self>,
  ) {
    if let Some(turn) = self.turns.get_mut(index) {
      edit(turn);
      cx.notify();
    }
  }

  /// Append a turn and return its index. Sending always re-attaches the
  /// view to the bottom: the user just acted, so they want to see the result.
  pub fn push(&mut self, turn: AITurn, cx: &mut Context<Self>) -> usize {
    self.turns.push(turn);
    self.follow = true;
    cx.notify();
    self.turns.len() - 1
  }

  /// Open an empty assistant turn to stream into, and return its index.
  pub fn begin_reply(&mut self, cx: &mut Context<Self>) -> usize {
    let mut turn = AITurn::assistant(String::new());
    turn.streaming = true;
    self.pending = None;
    self.push(turn, cx)
  }

  /// Append to the open assistant turn. Does nothing if none is open, so a
  /// late-arriving delta after a cancel can't resurrect a finished turn.
  pub fn push_delta(&mut self, delta: &str, cx: &mut Context<Self>) {
    if let Some(turn) = self.streaming_turn() {
      turn.body.push_str(delta);
      cx.notify();
    }
  }

  /// Append to the open turn's reasoning block.
  pub fn push_reasoning(&mut self, delta: &str, cx: &mut Context<Self>) {
    if let Some(turn) = self.streaming_turn() {
      turn
        .reasoning
        .get_or_insert_with(String::new)
        .push_str(delta);
      cx.notify();
    }
  }

  /// Close the open assistant turn.
  pub fn end_reply(&mut self, cx: &mut Context<Self>) {
    if let Some(turn) = self.streaming_turn() {
      turn.streaming = false;
      cx.notify();
    }
  }

  /// Close the open turn with a failure. Whatever text arrived is kept —
  /// a truncated reply is still evidence of what went wrong.
  pub fn fail_reply(&mut self, error: impl Into<String>, cx: &mut Context<Self>) {
    let error = error.into();
    if let Some(turn) = self.streaming_turn() {
      turn.streaming = false;
      turn.error = Some(error);
      cx.notify();
    }
  }

  /// Show a "working on it" line under the transcript.
  pub fn set_pending(&mut self, label: Option<impl Into<SharedString>>, cx: &mut Context<Self>) {
    self.pending = label.map(Into::into);
    self.follow = true;
    cx.notify();
  }

  /// Drop every turn.
  pub fn clear(&mut self, cx: &mut Context<Self>) {
    self.turns.clear();
    self.pending = None;
    self.follow = true;
    cx.notify();
  }

  /// Re-attach to the bottom and scroll there.
  pub fn scroll_to_bottom(&mut self, cx: &mut Context<Self>) {
    self.follow = true;
    cx.notify();
  }

  /// Whether new text is currently pulling the view down.
  pub fn is_following(&self) -> bool {
    self.follow
  }

  /// How far the transcript can scroll, in pixels. Exposed for the test that
  /// proves virtualizing doesn't change the layout.
  #[cfg(test)]
  pub(crate) fn scroll_extent(&self) -> Pixels {
    self.scroll.max_offset().y
  }

  fn streaming_turn(&mut self) -> Option<&mut AITurn> {
    self.turns.iter_mut().rev().find(|turn| turn.streaming)
  }

  /// Refresh the per-turn heights from the last frame and decide which turns
  /// to build this one. Returns one flag per turn.
  ///
  /// A turn is built when it is anywhere near the viewport, when its height
  /// has never been measured, or when virtualizing is off. Everything else
  /// is a spacer — which is the whole point, because building a turn parses
  /// its markdown, and doing that for a long transcript on every frame costs
  /// more than the frame has.
  fn measure(&mut self) -> Vec<bool> {
    let count = self.turns.len();
    let viewport = self.scroll.bounds();
    // A resize reflows every turn, so nothing measured at the old width can
    // be trusted. Clearing the heights is not enough on its own — an
    // off-screen turn's bounds are its *spacer's*, so the stale height
    // would be read straight back. Everything is drawn for one frame
    // instead, which is what re-measures it.
    // A width of zero means "not laid out yet", not "resized to nothing";
    // the first real width is the baseline, not a change from it.
    let resized =
      self.measured_width > px(0.0) && (viewport.size.width - self.measured_width).abs() > px(0.5);
    if resized || self.measured_width <= px(0.0) {
      self.measured_width = viewport.size.width;
      if resized {
        self.heights.clear();
      }
    }
    self.heights.resize(count, px(0.0));

    // Measure first, decide second. A turn that was a spacer last frame
    // reports the spacer's height, which is the same number — but a turn
    // drawn in full reports the real one, which is how a height first gets
    // recorded at all.
    let mut drawn = vec![true; count];
    let overscan = viewport.size.height.max(px(600.0));
    let (top, bottom) = (viewport.top() - overscan, viewport.bottom() + overscan);
    for (index, (height, drawn)) in self.heights.iter_mut().zip(drawn.iter_mut()).enumerate() {
      let Some(bounds) = self.scroll.bounds_for_item(index) else {
        continue;
      };
      if bounds.size.height > px(0.0) {
        *height = bounds.size.height;
      }
      // A resize reflows everything, so every turn is drawn once at the
      // new width — clearing the heights alone would not do it, since an
      // off-screen turn's bounds are its spacer's and would just be read
      // straight back. Never stand in for a turn whose height isn't
      // known either: the spacer would collapse and take the scroll
      // position with it.
      if resized || !self.virtualize || *height <= px(0.0) {
        continue;
      }
      *drawn = bounds.bottom() >= top && bounds.top() <= bottom;
    }
    drawn
  }

  /// How many turns were built on the last frame, for the test that proves
  /// virtualizing skips the ones off screen without moving anything.
  #[cfg(test)]
  pub(crate) fn drawn_count(&mut self) -> usize {
    self.measure().iter().filter(|drawn| **drawn).count()
  }

  /// How far the viewport sits above the end of the content, in pixels.
  /// gpui's scroll offset runs negative as content moves up, so the bottom
  /// is where `offset.y` reaches `-max_offset.height`.
  fn distance_from_bottom(&self) -> f32 {
    let offset = self.scroll.offset().y;
    let max = self.scroll.max_offset().y;
    f32::from(max + offset).max(0.0)
  }

  /// Re-decide whether to follow, after the reader moved the view
  /// themselves. Scrolling back to within a line or two of the end
  /// re-attaches, which is what makes "catch up" a scroll rather than a
  /// button hunt.
  fn on_scroll(
    &mut self,
    _event: &gpui::ScrollWheelEvent,
    _window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    let following = self.distance_from_bottom() <= FOLLOW_SLACK;
    if following != self.follow {
      self.follow = following;
      cx.notify();
    }
  }
}

impl Render for AIChatView {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let t = theme(cx);
    let dimmed = t.dimmed().hsla();
    let font = t.font_size(self.size);
    let empty = self.turns.is_empty() && self.pending.is_none();
    let max_width = self.max_width;
    let drawn = self.measure();

    // Turns are direct children of the scrolling box rather than of an
    // inner list, so gpui tracks each one's bounds and `scroll_to_item`
    // can reach the last of them using the current frame's measurements.
    let mut rows: Vec<gpui::AnyElement> = Vec::with_capacity(self.turns.len() + 1);

    for (index, turn) in self.turns.iter().enumerate() {
      // A turn that is far off screen stands in as a spacer of the
      // height it last measured, so the scroll extent and every
      // position in it stay exactly where they were.
      if !drawn[index] {
        rows.push(row(max_width).h(self.heights[index]).into_any_element());
        continue;
      }
      let mut message = AIMessage::new(turn.role, turn.body.clone())
        .streaming(turn.streaming)
        .size(self.size);
      if let Some(name) = &turn.name {
        message = message.name(name.clone());
      }
      if let Some(meta) = &turn.meta {
        message = message.meta(meta.clone());
      }
      if let Some(error) = &turn.error {
        message = message.error(error.clone());
      }

      if let Some(reasoning) = &turn.reasoning {
        // `AIReasoning` draws the text only when open, and reasoning is
        // routinely longer than the answer — so a collapsed block was
        // copying the largest string in the turn every frame to throw
        // it away.
        let text = if turn.reasoning_open {
          reasoning.clone()
        } else {
          String::new()
        };
        message = message.child(
          div().mt(px(8.0)).child(
            AIReasoning::new(("guise-ai-reasoning", index), text)
              .open(turn.reasoning_open)
              .streaming(turn.streaming)
              .size(self.size)
              .on_toggle(cx.listener(move |this, _event, _window, cx| {
                this.update_turn(index, |turn| turn.reasoning_open = !turn.reasoning_open, cx);
              })),
          ),
        );
      }

      for (slot, tool) in turn.tools.iter().enumerate() {
        let mut card = AIToolCall::new(("guise-ai-tool", index * 64 + slot), tool.name.clone())
          .status(tool.status)
          .open(tool.open)
          .expandable(tool.arguments.is_some() || tool.result.is_some())
          .size(self.size)
          .on_toggle(cx.listener(move |this, _event, _window, cx| {
            this.update_turn(
              index,
              |turn| {
                if let Some(tool) = turn.tools.get_mut(slot) {
                  tool.open = !tool.open;
                }
              },
              cx,
            );
          }));
        // Only a folded-open card draws these, and a tool result runs
        // to tens of kilobytes.
        if tool.open {
          if let Some(arguments) = &tool.arguments {
            card = card.arguments(arguments.clone());
          }
          if let Some(result) = &tool.result {
            card = card.result(result.clone());
          }
        }
        if let Some(meta) = &tool.meta {
          card = card.meta(meta.clone());
        }
        message = message.child(div().mt(px(8.0)).child(card));
      }

      if !turn.sources.is_empty() {
        let chips = div().flex().flex_row().flex_wrap().gap(px(4.0)).children(
          turn.sources.iter().enumerate().map(|(slot, source)| {
            AICitation::new(("guise-ai-cite", index * 64 + slot), slot + 1)
              .label(source.title.clone())
              .on_click(cx.listener(move |_this, _event, _window, cx| {
                cx.emit(AIChatViewEvent::OpenSource(index, slot));
              }))
          }),
        );
        // `on_open` reports an index rather than an event, so it
        // can't go through `cx.listener`; a weak handle re-enters the
        // entity to emit.
        let view = cx.entity().downgrade();
        message =
          message
            .child(div().mt(px(8.0)).child(chips))
            .child(div().mt(px(6.0)).child(
              AISources::new(turn.sources.clone()).excerpts(true).on_open(
                move |slot, _window, cx| {
                  view
                    .update(cx, |_this, cx| {
                      cx.emit(AIChatViewEvent::OpenSource(index, slot));
                    })
                    .ok();
                },
              ),
            ));
      }

      rows.push(row(max_width).child(message).into_any_element());
    }

    if let Some(pending) = self.pending.clone() {
      rows.push(
        row(max_width)
          .child(AIThinking::new().label(pending).size(self.size))
          .into_any_element(),
      );
    }

    // Ask for the last row before painting; the scroll container resolves
    // it against this frame's bounds, so a growing reply doesn't lag a
    // frame behind the caret.
    if self.follow && !rows.is_empty() {
      self.scroll.scroll_to_item(rows.len() - 1);
    }

    div()
      .id("guise-ai-chatview")
      .track_focus(&self.focus)
      .flex()
      .flex_col()
      .items_center()
      .gap(px(18.0))
      .size_full()
      .overflow_y_scroll()
      .track_scroll(&self.scroll)
      .on_scroll_wheel(cx.listener(Self::on_scroll))
      .p(px(16.0))
      .text_size(px(font))
      .when(empty, |view| {
        view
          .justify_center()
          .child(div().text_color(dimmed).children(self.empty.clone()))
      })
      .when(!empty, |view| view.children(rows))
      .probe("AIChatView")
  }
}

/// One transcript row: full width, capped and centered when the view asks for
/// a reading width.
fn row(max_width: Option<f32>) -> gpui::Div {
  let row = div().w_full();
  match max_width {
    Some(max) => row.max_w(px(max)),
    None => row,
  }
}
