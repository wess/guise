# Size and performance

What guise costs, where it goes, and the two settings that matter most. Every
number here is measured on the gallery — the whole library, every component,
on macOS arm64.

## Binary size

```
cargo build --release -p gallery
```

| | |
| --- | --- |
| Before the size pass | 13.86 MB |
| After | **7.82 MB** |

Most of that came from the release profile, which is a **choice the final
binary makes, not the library** — Cargo only reads the profile in the
top-level workspace. Copy this into your app's `Cargo.toml`:

```toml
[profile.release]
lto = "fat"
codegen-units = 1
panic = "abort"
strip = "symbols"
```

What each one is worth here:

| Setting | Saves | Cost |
| --- | --- | --- |
| `strip = "symbols"` | ~2.5 MB | Backtraces lose function names |
| `panic = "abort"` | ~1.5 MB | None in practice — gpui only catches panics in its test harness, and Cargo forces unwinding back on for tests |
| `lto = "fat"` + `codegen-units = 1` | ~1.7 MB | Slower release builds (~2 min for the gallery) |

## Debug builds

`cargo run` on an app built from guise feels slow in debug and fine in release,
because gpui does layout, text shaping and scene building every frame and
opt-level 0 makes all of it slow. Optimise the dependencies and leave your own
crates at 0:

```toml
[profile.dev.package."*"]
opt-level = 2
```

This repo's workspace does it, so the gallery is smooth under `cargo run`. The
cost is a slower first build; dependencies only rebuild when the lockfile
changes, so your edit loop is unaffected. Like the release profile above it is
read from the top-level workspace, so an app has to add it itself.

## What guise itself contributes

Of the 7.8 MB, gpui and its dependency tree (image decoding, SVG, text shaping,
the GPU backend) are about 4 MB and fixed. guise adds roughly:

| | |
| --- | --- |
| Code | ~825 KB across ~200 component `render` functions |
| Lucide font | 764 KB |
| Icon name/glyph tables | ~100 KB |

The font is the single largest item and it is unconditional — `IconName` covers
all 1991 Lucide icons and the font has to hold every glyph one of them might
name. Two things were shaved off it:

- **GSUB and `post` are stripped** (`scripts/stripfont.py`, run by
  `bun scripts/icons.ts`). guise addresses glyphs by private-use codepoint, so
  the ligature table and the glyph-name table are never read: −78 KB.
- **`IconName`'s `Debug` is written out** rather than derived. A derived one is
  a match with 1991 arms, to print a string the name table already holds.
  `Debug` therefore shows the kebab-case name (`arrow-up`).

If you need the font gone entirely, that is the honest ceiling of what a
bundled icon set can do — a subsetted font is an app-level decision, since only
the app knows which icons it uses.

## Per-frame work

gpui rebuilds the element tree every frame, so anything a `render` does is on
the frame budget. Three things in guise are shaped around that.

**Text fields shape once per frame, not per keystroke.** The single-line core
([`input/line.rs`](inputs.md#what-a-field-does)) builds one `SharedString` for
the value and measures the caret and selection against the same allocation. The
platform's input handler asks for the selection and for text ranges several
times per keystroke; those read the char buffer directly instead of
materialising a `String` each time.

**Long transcripts don't re-parse.** Rendering a message parses its markdown,
which is linear in the message. Left alone, a conversation re-parses *every
turn on every frame*:

| Transcript | Parse cost per frame |
| --- | --- |
| 2.3 KB | 0.11 ms |
| 11.5 KB | 0.48 ms |
| 46 KB | 1.25 ms |

At 46 KB that is 15% of a 120 Hz frame, and it keeps growing. So
[`AIChatView`](ai.md#aichatview-entity) virtualizes: a turn more than a screen
away is drawn as a spacer of the height it last measured, which makes the cost
proportional to the viewport instead of the conversation. In a 60-turn
transcript, 16 turns get built.

The spacer carries the *measured* height, so the scroll extent and every
position in it are unchanged — a resize invalidates the measurements and
everything is drawn again at the new width. Turn it off with
`.virtualize(false)` if you need every turn's element tree live.

**Animation costs frames, not ticks.** [`Motion`](transitions.md) sampling is
pure: `sample(t)` walks a handful of tracks and allocates one small `Vec`, and
nothing in the system holds a timer or mutates per frame. An `Animator` stores
a clock *anchor* and derives the playhead from `Instant::now()`, so a paused
animation costs exactly zero.

What costs something is the repaint. An animation keeps the window redrawing by
asking for the next frame, and gpui's `request_animation_frame` notifies the
whole **view** — so one spinner re-renders every component in the view that
holds it, every frame, for as long as it is mounted.

Measured on an M-series laptop, release builds:

| Window | Idle CPU |
| --- | --- |
| `--example checklist`, entrances settled | 0.6% |
| `--example motion`, entrances settled | 0.5% |
| `cargo run -p gallery` | 84% |

A one-shot stops asking for frames the moment it settles, which is why the two
examples cost nothing at rest. The gallery is the other end: 130 components in
one view, three of which (`Loader`, `Skeleton`, and the streaming caret) loop
forever by their nature, so every frame rebuilds all of it.

The lesson is not "don't use a spinner" — it is that a looping animation is a
**per-view** cost, not a per-element one. Keep the loop scoped to the thing it
describes and drop it when that thing is done; `animate_when(condition, ..)` is
there for exactly that. Splitting a busy screen so the looping part sits in its
own entity keeps the repaint off everything else.

## Memory

**Undo history is bounded by what it retains, not by step count.** A step is a
whole copy of the buffer. Capping the depth alone means 128 steps of a 200 KB
`TextArea` is 100 MB, so the history is also capped at 256k chars and drops the
oldest steps first. `set_text` clears it — a programmatic replacement isn't
something the user can undo back through.

**`TextEdit` holds `Vec<char>`**, which is 4 bytes per character. That buys
O(1) char indexing for the caret, selection, and word navigation, and costs 4×
the text for a large document. It is the right trade for fields and text areas;
the code [`Editor`](editor.md) has its own line-based model for real documents.

## Measuring your own build

```sh
cargo install cargo-bloat
cargo bloat --release --crates -n 20     # by crate
cargo bloat --release -n 40              # by function
size -m target/release/<binary>          # by section (macOS)
```

`cargo bloat` needs symbols, so drop `strip` from the profile while you use it.
