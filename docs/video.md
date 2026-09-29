# Video

`VideoView` shows video frames your app decodes. guise links no codec, opens no
socket and owns no clock — like the [AI](ai.md) components it is transport
agnostic: you decode (ffmpeg, a camera or screen-capture API, a WebRTC track),
guise puts the newest picture on screen through gpui's GPU image path and fits
it into its box.

It is a picture surface, not a player. Play, pause and seek belong to whatever
is decoding; controls are ordinary components you put around the view.

```rust
use guise::prelude::*;

let video = cx.new(|cx| {
    VideoView::new(cx)
        .height(360.0)
        .fit(VideoFit::Contain)
        .radius(Size::Md)
});
```

## Getting frames in

A `VideoFrame` is one decoded picture. Build it from whichever layout your
decoder produces; each constructor returns `None` if the buffer is the wrong
size for the dimensions, so a decoder bug is a missing frame rather than a
panic in the paint pass.

| Constructor | Input | Cost |
| --- | --- | --- |
| `VideoFrame::bgra(w, h, bytes)` | packed 8-bit BGRA | none — gpui's native layout |
| `VideoFrame::rgba(w, h, bytes)` | packed 8-bit RGBA | swaps two channels in place |
| `VideoFrame::i420(w, h, y, u, v)` | planar 4:2:0 YUV, BT.601 limited range | one conversion pass |

From the UI thread, push directly:

```rust
video.update(cx, |v, cx| v.push_frame(frame, cx));
```

From a decoder thread, ask the view for a `VideoFeed` — a cloneable, `Send`
handle — and send into it:

```rust
let feed = video.update(cx, |v, cx| v.feed(cx));

std::thread::spawn(move || {
    for decoded in decoder {
        if let Some(frame) = VideoFrame::i420(decoded.w, decoded.h, &decoded.y, &decoded.u, &decoded.v) {
            feed.send(frame);
        }
    }
});
```

The feed is a **one-slot mailbox**: `send` replaces a frame the view hasn't
picked up yet. For live video that is the right policy — a stale frame is worth
nothing and queueing it only adds latency. The view polls it every 8 ms and
stops when every clone of the feed is dropped. `view.stats()` reports `shown`
and `dropped`, so you can see how far behind the UI ran.

## Fitting

| `VideoFit` | |
| --- | --- |
| `Contain` (default) | keep the aspect ratio, show the whole frame, letterbox the rest |
| `Cover` | keep the aspect ratio, fill the box, crop the overflow |
| `Stretch` | scale each axis independently |

Letterbox bars take `.background(color)`, defaulting to the theme surface.
`fit_rect(src, dst, fit)` is the pure arithmetic if you need the same mapping
for an overlay.

## Builder and methods

| | |
| --- | --- |
| `.fit(VideoFit)` | how frames fill the box |
| `.width(px)` / `.height(px)` | fixed size; default fills the parent |
| `.radius(Size)` | corner radius token; the picture is rounded only when it reaches the box edges |
| `.background(color)` | letterbox colour |
| `.placeholder(text)` | shown before the first frame and after `clear` (default "No signal") |
| `push_frame(frame, cx)` | show a frame (UI thread) |
| `feed(cx) -> VideoFeed` | a `Send` handle for a decoder thread |
| `clear(cx)` | drop the picture, show the placeholder |
| `frame_size()` | `Option<(u32, u32)>` of the current picture |
| `stats()` | `VideoStats { shown, dropped }` |

Subscribe for `VideoEvent::Resized { width, height }`, emitted on the first
frame and whenever the dimensions change — the moment to resize a container or
re-run layout around the view:

```rust
cx.subscribe(&video, |_this, _v, event: &VideoEvent, _cx| {
    let VideoEvent::Resized { width, height } = event;
})
.detach();
```

## Notes

- Every frame is a new gpui image and gpui keeps an image's atlas slot until
  told to drop it. The view retires each replaced frame and drops it at the
  start of the next render, so a long stream does not fill the atlas.
- The gallery's *VideoView* section runs a generated colour-bar pattern through
  a `VideoFeed` from a plain thread at 30 fps — the same path a real decoder
  takes.
- macOS could avoid the CPU copy with gpui's `surface()` and a `CVPixelBuffer`;
  `VideoView` does not use it, to stay on the portable image path that gpui
  0.2.2 offers on every platform.
