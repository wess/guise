# Drag & drop

gpui's drag system is **typed**: a drag carries a value, and only targets
expecting that value's type react. `guise::dnd` wraps the idiom into three
reusable pieces — a themed drag chip, hover highlighting, and reorder math
included — generic over any `Clone` payload.

- **`Draggable<T>`** — makes its child the source of a typed drag.
- **`DropTarget<T>`** — accepts drags of the matching payload type.
- **`SortableList`** — drag-to-reorder rows over a parent-owned list.

Component-internal drags (panegroup tabs, table column reorder, splitter
handles) stay specialized; this module is the app-facing surface.

## Draggable + DropTarget

Payloads are plain values (`Copy`/`Clone` structs, ids, enums). While a
matching drag hovers a target, the target shows a primary border + tint.

```rust
#[derive(Clone, Copy, PartialEq)]
struct CardId(usize);

// Source: dragging the card carries its id; a themed chip follows the pointer.
Draggable::new("card-3", CardId(3))
    .label("Q3 report")
    .child(Card::new().child(summary))

// Target: only reacts to CardId drags.
DropTarget::<CardId>::new("done-lane")
    .on_drop(|card, _window, _cx| move_to_done(*card))
    .child(lane_content)
```

| | Method | Notes |
| --- | --- | --- |
| `Draggable` | `new(id, payload)` | payload: `Clone + 'static` |
| | `label(text)` | chip text under the pointer (default "…") |
| | `child(el)` | the draggable content |
| `DropTarget` | `new(id)` | type-annotate the payload: `DropTarget::<CardId>::new(..)` |
| | `on_drop(fn(&T, &mut Window, &mut App))` | receives the payload |
| | `plain()` | disable the built-in drag-over highlight |
| | `child(el)` | |

Different payload types never interfere — a `CardId` drag passes over a
`DropTarget<FileId>` without highlighting it.

## SortableList

Rows drag to reorder; the parent owns the items and applies the move.
Dropping row `from` onto row `to` means "place it where I dropped it" —
exactly what `apply_reorder` implements.

```rust
let view = cx.entity().downgrade();
SortableList::new("queue", self.tracks.len(), {
    let tracks = self.tracks.clone();
    move |i, _window, _cx| Text::new(tracks[i].clone()).into_any_element()
})
.label_of(|i| format!("Track {}", i + 1).into())
.on_reorder(move |from, to, _window, cx| {
    view.update(cx, |this, cx| {
        guise::dnd::apply_reorder(&mut this.tracks, from, to);
        cx.notify();
    })
    .ok();
})
```

| Method | Default | Notes |
| --- | --- | --- |
| `new(id, count, item_fn)` | — | `item_fn(usize, ..) -> impl IntoElement`, re-invoked every frame |
| `label_of(fn(usize) -> SharedString)` | "Item N" | drag chip text |
| `gap(px)` | `4.0` | row spacing |
| `on_reorder(fn(from, to, ..))` | — | fires only for drops from the same list |

The hovered row shows a primary top border as the insert marker. The list id
doubles as a **group guard**: rows only accept drags from the same
`SortableList`, so two lists side by side don't cross-talk.

`apply_reorder(&mut vec, from, to)` is the matching pure helper: remove at
`from`, insert at `to`; out-of-range indices are a no-op.
