# WebView

`WebView` embeds a real operating-system web view inside a gpui window —
WKWebView on macOS, WebView2 on Windows, WebKitGTK on Linux — using
[`wry`](https://crates.io/crates/wry). The native view is parented to the gpui
window and repositioned every frame to track the component's layout bounds, so
it sits inside normal `guise` layout like any other element.

It is a **stateful entity** (`Render` + `EventEmitter`). Create it with
`cx.new` and subscribe for [`WebViewEvent`]s.

```rust
use guise::prelude::*;

let view = cx.new(|cx| {
    WebView::new(cx)
        .url("https://example.com")
        .height(360.0)
});

cx.subscribe(&view, |_this, _view, event: &WebViewEvent, _cx| {
    match event {
        WebViewEvent::TitleChanged(title) => println!("title: {title}"),
        WebViewEvent::UrlChanged(url)     => println!("→ {url}"),
        WebViewEvent::LoadStarted         => {}
        WebViewEvent::LoadFinished        => {}
    }
})
.detach();
```

Render the entity wherever you want the page to appear:

```rust
Stack::new().child(view.clone())
```

## Inline HTML

```rust
WebView::new(cx).html("<h1>Hello from wry</h1>")
```

## Driving it at runtime

Because the parent holds an `Entity<WebView>`, navigate it after creation with
`update`:

```rust
self.view.update(cx, |wv, cx| wv.load_url("https://docs.rs", cx));
self.view.update(cx, |wv, cx| wv.load_html("<p>offline</p>", cx));
wv.evaluate_script("document.body.style.zoom = '1.25'");
```

## Builder methods

| Method | Default | Notes |
| --- | --- | --- |
| `new(cx)` | — | Lazily builds the native view on the first render with a window. |
| `url(into)` | — | Initial URL to load. |
| `html(into)` | — | Initial inline HTML (mutually exclusive with `url`). |
| `width(f32)` | fill parent | Fixed width in pixels. |
| `height(f32)` | fill parent | Fixed height in pixels. |
| `bordered(bool)` | `true` | Themed border + rounded frame around the view. |
| `radius(Size)` | theme default | Corner radius when bordered. |
| `transparent(bool)` | `false` | Let the page background show through. |

## Runtime methods

| Method | Notes |
| --- | --- |
| `load_url(into, cx)` | Navigate the live view and update the stored source. |
| `load_html(into, cx)` | Replace the live view with inline HTML. |
| `evaluate_script(js)` | Run JavaScript in the page. No-op until the view exists. |

## Events

`WebViewEvent` is emitted as the page changes:

- `TitleChanged(SharedString)` — the document title updated.
- `UrlChanged(SharedString)` — the view navigated.
- `LoadStarted` / `LoadFinished` — page load lifecycle.

Handler callbacks from the native view are marshalled back onto the entity by a
small drain loop, so you always receive them inside the normal gpui update cycle
(safe to call `cx.emit` / mutate state).

## The `webview` feature

The native backend lives behind the **default-on** `webview` Cargo feature,
which pulls in `wry`. Two things to know:

- **Linux** needs the system WebKitGTK dev libraries at build time
  (`libwebkit2gtk-4.1-dev` on Debian/Ubuntu).
- For **headless or docs-only builds**, disable it:

  ```toml
  guise = { version = "0.1", default-features = false }
  ```

  With the feature off, `WebView` still exists and keeps the exact same API, but
  renders a themed placeholder instead of a live page — so code that constructs
  and subscribes to it keeps compiling.

## Notes & limitations

- The native view paints **above** gpui content in its rectangle; gpui elements
  drawn over the same region won't show through it. Treat it as an opaque pane.
- It is created lazily on the first frame that has a window handle, so a brand
  new `WebView` has no live page for one frame.
- One native view per `WebView` entity; dropping the entity tears the view down.
