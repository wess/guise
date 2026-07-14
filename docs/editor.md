# Editor

`Editor` is a stateful entity — a multiline code editor with a line-number
gutter, syntax highlighting, selection, and undo/redo. It renders the pure,
headless `EditorModel`. Highlighting has two backends: the zero-dependency
line-based `Language` tokenizers (the default), and whole-document
[`DocumentHighlighter`](#tree-sitter-treesitter-feature) backends — the
`treesitter` feature ships an adapter that plugs any tree-sitter grammar in.

## Editor (entity)

Create it with `cx.new` and keep the `Entity`. It emits
`EditorEvent::Change` on every edit and `EditorEvent::Run` on ⌘Enter, so a
host can execute the buffer (a query console, a REPL).

```rust
let editor = cx.new(|cx| {
    Editor::new(cx)
        .language(Language::Rust)
        .rows(8)
        .placeholder("Type some Rust…")
        .value("fn main() {\n    println!(\"hi\");\n}")
});

cx.subscribe(&editor, |_this, _editor, event: &EditorEvent, _cx| match event {
    EditorEvent::Change(text) => { /* every edit — the full new text */ }
    EditorEvent::Run(source) => { /* ⌘Enter — execute `source` */ }
})
.detach();
```

| Method | Default | Notes |
| --- | --- | --- |
| `new(cx)` | — | construct inside `cx.new(\|cx\| ...)` |
| `value(&str)` | `""` | initial text (`text()` is the getter, like `TextInput`) |
| `language(Language)` | `None` | built-in tokenizers — see [Languages](#languages) |
| `highlighter(impl DocumentHighlighter)` | none | whole-document backend (tree-sitter); overrides `language` while set |
| `placeholder(text)` | none | dimmed hint while empty and unfocused |
| `read_only(bool)` | `false` | blocks edits; selection, copy, and ⌘Enter still work — cut degrades to copy, the caret hides |
| `line_numbers(bool)` | `true` | gutter; the active line's number brightens while focused |
| `tab_size(usize)` | `4` | spaces per tab stop (min 1) |
| `font_size(f32)` | `13.0` | buffer font size in px; line height is 1.5× |
| `rows(usize)` | none | minimum height, in visible lines |
| `token_colors([Hsla; 8])` | theme | override the syntax palette, one color per `TokenKind` in `TokenKind::ALL` order |
| `style(EditorStyle)` | theme | per-editor visual overrides — see [Styling](#styling) |

Runtime: `editor.read(cx).text()` reads the buffer;
`editor.update(cx, |e, cx| e.set_text("…", cx))` replaces it (resetting
cursor, selection, and history); `set_language(lang, cx)` and
`set_highlighter(Some(Box::new(hl)), cx)` switch highlighting (a file-type
change); `focus_handle()` lets a host focus it on open. Content scrolls on both axes once it outgrows the element, and edits
and movement auto-scroll the caret into view — give the editor a bounded
parent (or `rows(n)`) and the viewport comes free.

### Diagnostics

LSP-shaped diagnostics from a compiler, linter, or language server:
`Diagnostic { line, cols, severity, message }` with `Severity::{Error,
Warning, Info, Hint}`. Affected lines get a severity-colored **gutter dot**,
the char range gets an **underline** (an empty `cols` range covers the whole
line — `Diagnostic::line_wide` builds one), and when the caret sits on a
diagnosed line its worst message shows in a **strip under the buffer** — no
hover needed, and text selection is unaffected.

```rust
editor.update(cx, |e, cx| {
    e.set_diagnostics(vec![
        Diagnostic::new(3, 8..13, Severity::Error, "cannot find value `nmae`"),
        Diagnostic::line_wide(7, Severity::Warning, "unused import"),
    ], cx);
});
// later: e.clear_diagnostics(cx); e.diagnostics() reads back.
```

Severity colors ride the theme's feedback accents
([`danger`/`warning`/`info`](theming.md#semantic-colors-scheme-aware), hints
use dimmed), so they restyle with the theme.

```rust
pub enum EditorEvent {
    Change(String), // the document changed; carries the full new text
    Run(String),    // ⌘Enter; carries the current text
}
```

### Two-way binding

`Editor::bind` ties the buffer to a [`Signal<String>`](reactive.md#signal).
The signal is the source of truth: the editor adopts its value immediately,
edits write back, and signal writes replace the text — equality guards on
both directions prevent update loops.

```rust
let source = use_state(cx, String::new());
Editor::bind(&editor, &source, cx);
```

### Styling

Every visual comes from the active theme by default. Pass an `EditorStyle` to
override individual pieces — unset fields fall back to the theme, so an empty
style changes nothing. `bare` drops the frame border and corner radius, for an
editor embedded as a strip inside other chrome.

```rust
use guise::{Editor, EditorStyle, Language};

let editor = cx.new(|cx| {
    Editor::new(cx)
        .language(Language::Sql)
        .style(EditorStyle { bare: true, ..Default::default() })
});
```

`EditorStyle` is `Copy` and `Default`. Its fields — every one except `bare` an
`Option<Hsla>` — are `bare`, `bg`, `text`, `caret`, `selection`, `active_line`,
`gutter_fg`, `gutter_fg_active`, and `placeholder`. Swap the whole style at
runtime (a theme toggle) with `editor.update(cx, |e, cx| e.set_style(next, cx))`.
To recolor only the syntax tokens, `token_colors([Hsla; 8])` replaces the
per-`TokenKind` palette in `TokenKind::ALL` order.

### Building on the buffer

For features that live above the editor — autocomplete, find/replace, a modal
keymap — the entity exposes its model and geometry:

- `model() -> &EditorModel` reads the cursor, selection, and lines.
- `edit(window, cx, |m| …)` mutates the [`EditorModel`](#editormodel-headless)
  directly and runs the same bookkeeping as a built-in edit — emits
  `EditorEvent::Change`, keeps the caret visible, and repaints.
- `set_highlights(Vec<(Pos, Pos, Hsla)>, cx)` paints background rectangles under
  the text — search matches, occurrence highlights. Ranges are document
  positions; a multi-line range paints like a selection.
- `caret_origin(window) -> Point<Pixels>` is where to anchor a completion popup
  or inline widget, and `line_height()` is the painted height of one line.

```rust
// anchor a completion popup just below the caret
let at = editor.read(cx).caret_origin(window);
let row_h = editor.read(cx).line_height();
// … position a floating list at (at.x, at.y + row_h) …
```

### Key map

Movement follows the macOS conventions; ⇧ extends the selection on every
movement key.

| Keys | Action |
| --- | --- |
| ←/→ | move by char; ⌥ by word (crossing lines); ⌘ to line start/end |
| ↑/↓ | move by line, keeping a sticky column; ⌘ to document start/end |
| Home / End | line start/end; ⌘ document start/end |
| Backspace / Delete | delete backward/forward; ⌥ deletes a word; ⌘ clears to the line edge |
| Enter | newline, copying the current line's leading indent |
| ⌘Enter | emit `EditorEvent::Run` with the buffer |
| Tab | insert spaces to the next tab stop |
| ⌘A | select all |
| ⌘C / ⌘X / ⌘V | copy / cut / paste via the OS clipboard |
| ⌘Z / ⇧⌘Z | undo / redo — runs of single-char typing coalesce into one step |
| Escape | clear the selection, then bubble (dialogs still close on it) |

⌘Tab, read-only Tab/Enter, and any unhandled chord bubble to the host, so
focus management keeps working. Mouse: click places the caret, ⇧-click
extends, drag selects, double-click selects a word, triple-click the line.
Two editors in one window never react to each other's drags — the drag
payload is tagged with the owning entity.

> **Note** Caret, selection boxes, and mouse hit-testing all go through
> gpui's line shaping, so double-width glyphs (CJK, emoji fallback) and
> literal tabs line up with what is painted. Input is keystroke-driven (gpui
> `KeyDownEvent`, no marked-text integration), which means IME composition
> doesn't work; ⌥-composed glyphs land because the OS delivers them as a
> finished character.

## Languages

`Language` is the built-in `Highlighter`, a `Copy` enum of small
keyword/scanner tokenizers:

| Variant | Tokenizes |
| --- | --- |
| `Language::None` | nothing — every line is plain text (the default) |
| `Language::Rust` | `//` and *nesting* `/* */` comments, `"…"` with backslash escapes, keywords, `0x`/decimal/exponent numbers, uppercase idents as types, `name(` / `name!(` as calls |
| `Language::Sql` | case-insensitive keywords and column types, `--` and non-nesting `/* */` comments, `'…'` strings with doubled-quote escaping |
| `Language::Json` | `"…"` strings, numbers, `true` / `false` / `null` as keywords |
| `Language::Toml` | `#` comments, `"…"`/`'…'` strings, `true`/`false` |
| `Language::Python` | `#` comments, `"…"`/`'…'` strings, full keyword set, builtin types, uppercase idents as classes |
| `Language::JavaScript` | `//` + `/* */`, `"…"`/`'…'`/`` `…` `` strings, keywords, uppercase idents as types |
| `Language::TypeScript` | JavaScript plus `interface`/`type`/`declare`/… and the primitive type names |
| `Language::Go` | `//` + `/* */`, `"…"`/`` `…` `` strings, keywords, builtin types |
| `Language::C` | `//` + `/* */`, `"…"`/`'…'`, keywords, builtin types |
| `Language::Markdown` | line-structural: `#` headings, `>` quotes, list markers, `` `code` `` spans, and ``` fenced blocks (fence state carries across lines) |

Tokens are classified as one of eight `TokenKind`s (`Keyword`, `Ident`,
`Number`, `StringLit`, `Comment`, `Punct`, `Type`, `Function`);
`token_color(kind, theme)` maps each kind onto the active theme, light/dark
aware, and `TokenKind::ALL` / `TokenKind::index()` let a renderer resolve the
whole palette into an array once per frame.

## Highlighter (trait)

Highlighting is line-based and pluggable. A `Highlighter` tokenizes one line
at a time into byte-range spans, threading a `LineState` through consecutive
lines so block comments carry across them:

```rust
use guise::editor::{Highlighter, Language, LineState, TokenKind};

let mut state = LineState::default(); // start each document fresh
for line in source.lines() {
    let tokens = Language::Sql.line(line, &mut state);
    // tokens: Vec<(Range<usize>, TokenKind)>
}
```

The contract: `line` receives a single line (no `\n`); returned ranges are
**byte** offsets into it, ascending, non-overlapping, and aligned to char
boundaries (multibyte-safe — ready for gpui `TextRun` lengths). Uncovered
gaps render unstyled. Feed lines in document order so `LineState` (the open
block-comment depth) stays correct.

Inside the entity, `Language` output is cached per line
(`guise::editor::HighlightCache`) and revalidated against the text and the
entering `LineState` — a keystroke re-tokenizes only the edited line, and
later lines only when a block construct actually changed the state reaching
them. Nothing re-tokenizes frame to frame.

> **Note** `Editor::language` takes the built-in `Language` enum — a custom
> line-based `Highlighter` still can't be plugged into the entity. For a
> custom backend, implement `DocumentHighlighter` (below) instead; the
> line-based trait remains for tokenizing with `EditorModel` +
> `token_color` in your own rendering.

## DocumentHighlighter (trait)

The whole-document seam. Where `Highlighter` re-scans line by line, a
`DocumentHighlighter` parses the full buffer once per edit and serves
per-line tokens from that parse — the shape a parse-tree backend wants:

```rust
pub trait DocumentHighlighter {
    fn update(&mut self, text: &str);                            // reparse; called once per edit
    fn tokens(&self, line: usize) -> &[(Range<usize>, TokenKind)]; // same shape as Highlighter::line
}
```

Plug one in with `.highlighter(hl)` at build time or
`set_highlighter(Some(Box::new(hl)), cx)` at runtime; it overrides
`language` while set, and `set_highlighter(None, cx)` falls back. The editor
calls `update` only when the text changed — never per frame.

## Tree-sitter (`treesitter` feature)

`TreeSitterHighlighter` is a `DocumentHighlighter` over
[tree-sitter](https://tree-sitter.github.io/) — real parse-tree highlighting
(context-aware, injection-capable grammars) instead of keyword scanning.
guise ships **no grammars**: you pass a grammar crate's language and
highlight query, so only the languages your app uses get compiled in.

```toml
guise-ui = { version = "…", features = ["treesitter"] }
tree-sitter-rust = "0.24"   # each grammar is its own crate
```

```rust
let rust = guise::TreeSitterHighlighter::new(
    tree_sitter_rust::LANGUAGE.into(),
    tree_sitter_rust::HIGHLIGHTS_QUERY,
)?; // errors = query/grammar version mismatch

let editor = cx.new(|cx| Editor::new(cx).highlighter(rust).value(source));
```

Capture names from the query (`keyword`, `function.method`,
`punctuation.bracket`, …) map onto the eight `TokenKind`s by their root
segment, so the theme palette and `token_colors` overrides apply unchanged;
unrecognized captures render unstyled. The gallery's editor showcase runs on
this backend when built with `cargo run -p gallery --features treesitter`.

## EditorModel (headless)

The pure editing model behind `Editor`: the document as a `Vec<String>` of
lines, a char-index cursor, an anchor-based selection, and snapshot undo/redo.
No UI and no gpui — fully unit-testable, and usable on its own for a custom
editor surface.

```rust
use guise::editor::{EditorModel, Pos};

let mut model = EditorModel::new("fn main() {}");
model.move_to(0, 3, false);
model.word_right(true);                 // extend the selection over "main"
assert_eq!(model.selected_text().as_deref(), Some("main"));
model.insert("start");                  // replaces the selection
assert!(model.undo());
```

`Pos` is a `(line, col)` pair where `col` is a **char** index (not bytes), so
multibyte text (é, 日本語) edits correctly; ordering is document order. The
methods, by group:

- **Document** — `text()`, `set_text(&str)` (resets cursor/selection/history),
  `lines()`, `line(i)`, `line_count()`, `is_empty()`, `cursor()`,
  `tab_size()` / `set_tab_size(n)`.
- **Editing** — `insert(&str)` (CRLF normalized, embedded newlines split
  lines), `backspace()`, `delete()` (both join lines at the edges and return
  whether anything changed), `newline()` (auto-indents), `tab()`.
- **Movement** (every method takes `extend: bool` — shift semantics) —
  `move_left/right/up/down`, `home` / `end`, `doc_start` / `doc_end`,
  `word_left` / `word_right`, and `move_to(line, col, extend)` /
  `pos_for_click(line, col)` for mouse input (both clamp out-of-range values).
- **Selection** — `selection() -> Option<(Pos, Pos)>` (normalized to document
  order), `select_all()`, `select_word()`, `select_line()`,
  `clear_selection()`, `selected_text()`, `delete_selection()`.
- **Clipboard halves** — `cut()` / `copy()` return the selected text; the
  entity layer talks to the OS clipboard.
- **History** — `undo()` / `redo()` / `can_undo()` / `can_redo()`. Runs of
  single-char typing coalesce into one undo step; any other edit or movement
  breaks the run.

The single-line counterpart is [`TextEdit`](inputs.md#driving-a-field-yourself)
— same char-index cursor and word-boundary semantics, minus selections and
history.
