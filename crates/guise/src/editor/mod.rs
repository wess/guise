//! Multiline text-editor building blocks.
//!
//! [`EditorModel`] is the pure editing model — document lines, a char-index
//! [`Pos`] cursor, anchor-based selection, and snapshot undo/redo. The
//! [`Editor`] entity renders it with a line-number gutter, syntax
//! highlighting (see [`Language`]/[`Highlighter`]), selection, and the full
//! keyboard map, emitting [`EditorEvent`] as the buffer changes.
//!
//! ```ignore
//! use guise::editor::{Editor, EditorEvent, Language};
//!
//! let editor = cx.new(|cx| {
//!     Editor::new(cx)
//!         .language(Language::Rust)
//!         .value("fn main() {}")
//! });
//! cx.subscribe(&editor, |_this, _editor, event: &EditorEvent, _cx| {
//!     if let EditorEvent::Run(source) = event { /* Cmd+Enter */ }
//! })
//! .detach();
//! ```

// The `Editor` entity lives in its own file, named for the component like
// every other module; the path doubling (`editor::editor`) never leaks since
// the type is re-exported here.
#[allow(clippy::module_inception)]
mod editor;
mod highlight;
mod model;

pub use editor::{Editor, EditorEvent};
pub use highlight::{token_color, Highlighter, Language, LineState, TokenKind};
pub use model::{EditorModel, Pos};
