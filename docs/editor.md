# Editor

`Editor` is a stateful entity — a multiline code editor with a line-number
gutter, syntax highlighting, selection, and undo/redo. It renders the pure,
headless `EditorModel`, and highlighting is line-based through the
`Highlighter` trait; `Language` ships built-in tokenizers for Rust, SQL, and
JSON.

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
| `language(Language)` | `None` | built-in `Rust` / `Sql` / `Json` tokenizers |
| `placeholder(text)` | none | dimmed hint while empty and unfocused |
| `read_only(bool)` | `false` | blocks edits; selection, copy, and ⌘Enter still work — cut degrades to copy, the caret hides |
| `line_numbers(bool)` | `true` | gutter; the active line's number brightens while focused |
| `tab_size(usize)` | `4` | spaces per tab stop (min 1) |
| `font_size(f32)` | `13.0` | buffer font size in px; line height is 1.5× |
| `rows(usize)` | none | minimum height, in visible lines |

Runtime: `editor.read(cx).text()` reads the buffer;
`editor.update(cx, |e, cx| e.set_text("…", cx))` replaces it (resetting
cursor, selection, and history); `focus_handle()` lets a host focus it on
open. Content scrolls on both axes once it outgrows the element, and edits
and movement auto-scroll the caret into view — give the editor a bounded
parent (or `rows(n)`) and the viewport comes free.

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

> **Note** `Editor::language` takes the built-in `Language` enum — a custom
> `Highlighter` can't be plugged into the entity yet. Implement the trait to
> tokenize additional languages for your own rendering: pair it with
> `EditorModel` for the text and `token_color` for the theme mapping.

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
