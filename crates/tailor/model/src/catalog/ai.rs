//! The `ai/` module: a transcript and the parts one is made of.
//!
//! guise's AI components are transport-agnostic — the view owns the transcript
//! and the host owns the request — which is exactly what makes them designable:
//! there is no socket to fake, only content to lay out. So a designer places a
//! message, a tool call, a token meter, and the generated code hands them real
//! data later.
//!
//! The composite views (`AIChatView`, `AIComposer`, `AIModelPicker`,
//! `AISettings`) are entities, the way they are in a hand-written app.

use crate::props::{boolean, color_name, enums, int, multiline, size, text, Emit, PropValue};
use crate::tokens::{ColorToken, SizeToken};

use super::spec::{ComponentSpec, Ctor};

pub static SPECS: &[ComponentSpec] = &[
  comp!(
      "aimessage", "Message", "AIMessage", Ai, "message-square",
      "One turn in a transcript.",
      Ctor::Args(&["role", "body"]),
      props: &[
          enums("role", "Role", Emit::None, "AIRole",
              &["user", "assistant", "system", "tool"], || PropValue::Choice("assistant".into())),
          multiline("body", "Body", Emit::None),
          text("name", "Name", Emit::Method("name")),
          text("meta", "Meta", Emit::Method("meta")),
          text("error", "Error", Emit::Method("error")),
          boolean("streaming", "Streaming", Emit::Method("streaming"), false),
          boolean("avatar", "Avatar", Emit::Method("avatar"), true),
          size("size", "Size", Emit::Method("size"), SizeToken::Md),
      ],
  ),
  comp!(
      "aistreamingtext", "Streaming text", "AIStreamingText", Ai, "text-cursor",
      "Text as it arrives, with a caret.",
      Ctor::Arg("text"),
      props: &[
          multiline("text", "Text", Emit::None),
          boolean("caret", "Caret", Emit::Method("caret"), true),
          size("size", "Size", Emit::Method("size"), SizeToken::Md),
      ],
  ),
  comp!(
      "aithinking", "Thinking", "AIThinking", Ai, "loader",
      "The pause before the first token.",
      Ctor::Unit,
      props: &[
          text("label", "Label", Emit::Method("label")),
          color_name("color", "Color", Emit::Method("color"), ColorToken::Gray),
          size("size", "Size", Emit::Method("size"), SizeToken::Md),
      ],
  ),
  comp!(
      "aireasoning", "Reasoning", "AIReasoning", Ai, "brain",
      "Collapsed reasoning above an answer.",
      Ctor::IdAnd("text"),
      props: &[
          multiline("text", "Text", Emit::None),
          text("label", "Label", Emit::Method("label")),
          boolean("open", "Open", Emit::Method("open"), false),
          boolean("streaming", "Streaming", Emit::Method("streaming"), false),
          size("size", "Size", Emit::Method("size"), SizeToken::Md),
      ],
  ),
  comp!(
      "aitoolcall", "Tool call", "AIToolCall", Ai, "wrench",
      "A tool the model reached for, and what came back.",
      Ctor::IdAnd("name"),
      props: &[
          text("name", "Name", Emit::None),
          enums("status", "Status", Emit::Method("status"), "AIToolStatus",
              &["pending", "running", "ok", "error"], || PropValue::Choice("ok".into())),
          multiline("arguments", "Arguments", Emit::Method("arguments")),
          multiline("result", "Result", Emit::Method("result")),
          text("meta", "Meta", Emit::Method("meta")),
          boolean("open", "Open", Emit::Method("open"), false),
          boolean("expandable", "Expandable", Emit::Method("expandable"), true),
          size("size", "Size", Emit::Method("size"), SizeToken::Md),
      ],
  ),
  comp!(
      "aitokenmeter", "Token meter", "AITokenMeter", Ai, "gauge",
      "How much of the context window is gone.",
      Ctor::Args(&["used", "limit"]),
      props: &[
          int("used", "Used", Emit::None, || PropValue::Int(24_000)),
          int("limit", "Limit", Emit::None, || PropValue::Int(200_000)),
          text("label", "Label", Emit::Method("label")),
          boolean("bar", "Bar", Emit::Method("bar"), true),
          size("size", "Size", Emit::Method("size"), SizeToken::Md),
      ],
  ),
  comp!(
      "aicost", "Cost", "AICost", Ai, "circle-dollar-sign",
      "What the turn cost, from tokens and a price list.",
      Ctor::Special,
      props: &[
          int("input", "Input tokens", Emit::Custom, || PropValue::Int(12_000)),
          int("output", "Output tokens", Emit::Custom, || PropValue::Int(800)),
          text("input_price", "Input $/M", Emit::Custom),
          text("output_price", "Output $/M", Emit::Custom),
          text("label", "Label", Emit::Method("label")),
          boolean("breakdown", "Breakdown", Emit::Method("breakdown"), false),
          size("size", "Size", Emit::Method("size"), SizeToken::Md),
      ],
  ),
  comp!(
      "aicitation", "Citation", "AICitation", Ai, "superscript",
      "The numbered marker that points into the sources.",
      Ctor::IdAnd("index"),
      props: &[
          int("index", "Index", Emit::None, || PropValue::Int(1)),
          text("label", "Label", Emit::Method("label")),
          size("size", "Size", Emit::Method("size"), SizeToken::Sm),
      ],
  ),
  comp!(
      "aisources", "Sources", "AISources", Ai, "book-open",
      "What the answer was drawn from.",
      Ctor::Special,
      props: &[
          crate::props::items("sources", "Sources", Emit::Custom,
              || PropValue::Items(vec!["guise docs — docs/ai.md".into()])),
          text("title", "Title", Emit::Method("title")),
          boolean("excerpts", "Excerpts", Emit::Method("excerpts"), true),
          size("size", "Size", Emit::Method("size"), SizeToken::Md),
      ],
  ),
  comp!(
      "aichatview", "Chat view", "AIChatView", Ai, "messages-square",
      "The whole transcript, scrolled and composed.",
      Ctor::Entity,
      props: &[
          crate::props::items("turns", "Turns", Emit::Custom,
              || PropValue::Items(vec!["How do I theme this?".into(), "Read it from `theme(cx)`.".into()])),
      ],
  ),
  comp!(
      "aicomposer", "Composer", "AIComposer", Ai, "pencil-line",
      "The box you type the next turn into.",
      Ctor::Entity,
      props: &[
          text("hint", "Hint", Emit::Method("hint")),
          boolean("attachments", "Attachments", Emit::Method("attachments"), false),
          boolean("disabled", "Disabled", Emit::Method("disabled"), false),
          size("size", "Size", Emit::Method("size"), SizeToken::Md),
      ],
  ),
  comp!(
      "aimodelpicker", "Model picker", "AIModelPicker", Ai, "list-filter",
      "Which model answers.",
      Ctor::Entity,
      props: &[
          crate::props::items("models", "Models", Emit::Custom,
              || PropValue::Items(vec!["Opus".into(), "Sonnet".into(), "Haiku".into()])),
          text("label", "Label", Emit::Method("label")),
          boolean("disabled", "Disabled", Emit::Method("disabled"), false),
          size("size", "Size", Emit::Method("size"), SizeToken::Md),
      ],
  ),
  comp!(
      "aisettings", "AI settings", "AISettings", Ai, "sliders-horizontal",
      "Temperature, tokens, and the system prompt.",
      Ctor::Entity,
      props: &[size("size", "Size", Emit::Method("size"), SizeToken::Md)],
  ),
];
