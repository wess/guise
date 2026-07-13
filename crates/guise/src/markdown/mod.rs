//! Obsidian-style live-preview markdown editing.
//!
//! [`MarkdownEditor`] is the component: every line renders formatted
//! (headings sized, lists as bullets/checkboxes, fenced code highlighted,
//! links clickable) while the cursor line reveals its markdown syntax for
//! editing. Under it sit three pure, unit-tested passes — [`block`] classifies
//! lines, [`inline`] parses emphasis/code/links, and [`layout`] plans what
//! each row shows and how its bytes map back to the source.
//!
//! ```ignore
//! use guise::markdown::{MarkdownEditor, MarkdownEditorEvent};
//!
//! let editor = cx.new(|cx| {
//!     MarkdownEditor::new(cx)
//!         .value("# Notes\n\n- [ ] ship it")
//!         .placeholder("Start writing…")
//! });
//! cx.subscribe(&editor, |_this, _editor, event: &MarkdownEditorEvent, _cx| {
//!     match event {
//!         MarkdownEditorEvent::Change(text) => { /* persist */ }
//!         MarkdownEditorEvent::LinkClick(target) => { /* open */ }
//!     }
//! })
//! .detach();
//! ```

pub mod block;
pub mod inline;
pub mod layout;

// The `MarkdownEditor` entity lives in its own file, named for the component
// like every other module; the path doubling never leaks since the type is
// re-exported here.
mod editor;

pub use editor::{MarkdownEditor, MarkdownEditorEvent, MarkdownStyle};
