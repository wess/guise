# File handling

Two components cover picking files: **`FileInput`**, a form-field entity that
opens the platform file dialog, and **`Dropzone`**, a stateless drop target
that accepts OS file drags (and clicks to browse). Both filter by extension
with the same `accept` list — entries like `"png"` or `".JPG"`, compared
case-insensitively; an empty list accepts everything.

Neither component reads the files — they hand the parent `PathBuf`s and stay
out of I/O.

## FileInput

A field-styled trigger. Create like any stateful input, subscribe for
selections; the event carries the chosen paths (empty = cleared via the ×
button).

```rust
let avatar = cx.new(|cx| {
    FileInput::new(cx)
        .label("Avatar")
        .accept(["png", "jpg", "jpeg"])
});
cx.subscribe(&avatar, |_, FileInputEvent(paths), _| {
    // paths: Vec<PathBuf>; empty means the user cleared the field
}).detach();
```

| Method | Default | Notes |
| --- | --- | --- |
| `multiple()` | single | multi-select dialog, shows "N files" |
| `directories()` | files | pick a folder instead |
| `accept(exts)` | any | selections not passing are ignored |
| `placeholder` / `label` / `size` / `disabled` | — | field chrome |
| `paths()` | — | current selection accessor |

## Dropzone

A bordered, dashed target. Drag files over it and the border/tint switch to
the theme primary; drop delivers the accepted paths. Clicking opens the file
dialog unless `.no_click()`.

```rust
Dropzone::new("uploads")
    .label("Drop images here")
    .hint("PNG or JPG")
    .accept(["png", "jpg"])
    .on_files(|paths, _cx| {
        // every drop or dialog pick that passed the filter
    })
```

| Method | Default | Notes |
| --- | --- | --- |
| `label(text)` | "Drop files here" | main line |
| `hint(text)` | none | dimmed second line |
| `icon(IconName)` | `CloudUpload` | |
| `accept(exts)` | any | non-passing dropped paths are silently ignored |
| `single()` | multiple | keep only the first path |
| `no_click()` | clickable | disable click-to-browse |
| `height(px)` | `140.0` | |
| `on_files(fn(Vec<PathBuf>, &mut App))` | — | never called with an empty vec |

> **Note** Directory drops arrive like any other path; if you need to reject
> folders, check `path.is_dir()` in your handler. The drag highlight uses
> gpui's `drag_over::<ExternalPaths>` styling, so it only reacts to OS file
> drags — internal guise drags (tabs, panes) don't trigger it.
