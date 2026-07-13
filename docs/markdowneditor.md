# Markdown editor

`MarkdownEditor` is a stateful entity — a live-preview markdown editor in the
style of Obsidian. Every line renders formatted (headings sized, emphasis
styled, list markers drawn as bullets and checkboxes, fenced code
highlighted, links clickable) while the cursor line — and any line the
selection touches — *reveals* its markdown syntax for editing, with the
markers dimmed. Text soft-wraps to the editor width, and list items keep a
hanging indent across wrapped lines.

It shares its text model with [`Editor`](editor.md) — the pure, unit-tested
`EditorModel` — so cursor, selection, clipboard, and undo behave identically.
The markdown understanding lives in three pure, unit-tested passes under
`guise::markdown`: `block` (line classification), `inline` (emphasis, code,
links), and `layout` (what each row shows, and how its bytes map back to the
source).

## MarkdownEditor (entity)

Create it with `cx.new` and keep the `Entity`. It emits
`MarkdownEditorEvent::Change` on every edit and
`MarkdownEditorEvent::LinkClick` when a link is activated — ⌘click normally,
plain click when read-only. Wikilink targets (`[[Page]]`) arrive as the page
name; url links as the url.

```rust
use guise::markdown::{MarkdownEditor, MarkdownEditorEvent};

let editor = cx.new(|cx| {
    MarkdownEditor::new(cx)
        .value("# Notes\n\n- [ ] try **guise**")
        .placeholder("Start writing…")
});

cx.subscribe(&editor, |_this, _editor, event: &MarkdownEditorEvent, _cx| {
    match event {
        MarkdownEditorEvent::Change(text) => { /* persist the document */ }
        MarkdownEditorEvent::LinkClick(target) => { /* open the target */ }
    }
})
.detach();
```

| Method | Default | Notes |
| --- | --- | --- |
| `new(cx)` | — | construct inside `cx.new(\|cx\| ...)` |
| `value(&str)` | `""` | initial markdown (`text()` is the getter) |
| `placeholder(text)` | none | dimmed hint while empty and unfocused |
| `read_only(bool)` | `false` | pure preview: no caret, no edits, no syntax reveal; selection, copy, and plain-click links still work |
| `font_size(f32)` | `15.0` | base size in px; headings scale from it |
| `rows(usize)` | none | minimum height, in paragraph lines |
| `tab_size(usize)` | `4` | spaces per list indent level |
| `style(MarkdownStyle)` | theme | per-editor overrides (`bare`, `bg`, `text`, `caret`, `selection`, `accent`, `code_bg`, `placeholder`) |

Runtime API: `text()`, `set_text(value, cx)`, `focus_handle()`, `model()`
(read the underlying `EditorModel`), `edit(cx, |model| …)` (mutate it with
change-event and scroll bookkeeping), `toggle_task(line, cx)`, `set_style`,
and `MarkdownEditor::bind(&entity, &signal, cx)` for two-way binding to a
`Signal<String>` (see [Reactive state](reactive.md)).

## What renders

- **Headings** `#`–`######`, scaled and bold, with extra space above.
- **Emphasis** — `**bold**`, `*italic*` / `_italic_`, `***both***`,
  `~~strikethrough~~`, `==highlight==`, and `` `inline code` ``.
- **Links** — `[text](url)`, `[[WikiLink]]`, `[[Page|alias]]`; underlined in
  the accent color with the target hidden until revealed.
- **Lists** — `-`/`*`/`+` bullets, `1.`/`1)` numbers, and `- [ ]`/`- [x]`
  task checkboxes, nested by indentation with hanging indents. Checked tasks
  dim their text.
- **Quotes** — `>` per level, drawn as accent bars.
- **Fenced code** — ``` blocks on a tinted surface, with Rust / SQL / JSON
  highlighting from the fence's info string (the [`Editor`](editor.md)
  `Language` tokenizers).
- **Thematic breaks** — `---` / `***` / `___` as a rule.
- YAML frontmatter and tables render as dimmed/plain monospace source.

## Editing behavior

- **Live reveal** — the cursor line (and every line a selection touches)
  shows its raw markdown with dimmed markers; leaving the line re-renders it.
  List markers stay rendered even on the cursor line; the caret parks at the
  content start.
- **Enter** continues lists and quotes (`- `, `- [ ] `, `4. `, `> `) and
  clears the marker instead when the item is empty. ⇧ isn't special — use
  Backspace at the content start to strip a marker.
- **Tab / ⇧Tab** indent and outdent list items from anywhere on the line.
- **⌘B / ⌘I** toggle `**` / `*` around the selection or the word at the
  caret; **⌘K** wraps the selection as `[selection]()` with the caret in the
  parentheses.
- **⌘Enter** toggles the checkbox on a task line; clicking the box does too.
- **Arrows** move by *visual* row through soft-wrapped text, with a sticky
  goal column. ⌘←/→ jump to line edges, ⌥←/→ by word, ⌘↑/↓ to the document
  edges — the same map as [`Editor`](editor.md), including select-all,
  copy/cut/paste, and undo/redo.

## Example

```sh
cargo run -p guise-ui --example markdown
```

opens a small document exercising everything above.
