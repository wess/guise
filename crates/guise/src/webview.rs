//! `WebView` — a native web view embedded in a gpui window (stateful entity).
//!
//! Backed by [`wry`](https://crates.io/crates/wry), which parents a real OS
//! web view (WKWebView on macOS, WebView2 on Windows, WebKitGTK on Linux) as a
//! child of the gpui window. The native view is positioned every frame to track
//! the bounds of this component, so it composes inside normal `guise` layout.
//!
//! Create with `cx.new(|cx| WebView::new(cx).url("https://example.com"))` and
//! subscribe for [`WebViewEvent`]s. Because the underlying view owns OS
//! resources, it is built lazily on first render (when a window handle exists).
//!
//! The native backend lives behind the default-on `webview` feature. Disable it
//! (`default-features = false`) for headless or docs-only builds; the component
//! then renders a themed placeholder while keeping the same public API.

use gpui::prelude::*;
use gpui::{div, px, Context, EventEmitter, FocusHandle, IntoElement, SharedString, Window};

use crate::theme::{theme, Size};

#[cfg(feature = "webview")]
use {
    gpui::{canvas, Bounds, Pixels},
    std::{cell::RefCell, rc::Rc, time::Duration},
    wry::{
        dpi::{LogicalPosition, LogicalSize},
        PageLoadEvent, Rect, WebViewBuilder,
    },
};

/// Emitted as the embedded page loads and changes.
#[derive(Debug, Clone)]
pub enum WebViewEvent {
    /// The document title changed. Carries the new title.
    TitleChanged(SharedString),
    /// The view navigated to a new URL. Carries the destination.
    UrlChanged(SharedString),
    /// A page began loading.
    LoadStarted,
    /// A page finished loading.
    LoadFinished,
    /// The page posted a message to the host via `window.ipc.postMessage(...)`.
    /// Carries the raw string payload; the host decides how to interpret it.
    Message(SharedString),
}

/// What the view should display.
#[derive(Clone)]
enum Source {
    /// Nothing requested yet.
    Empty,
    /// Load a remote or local URL.
    Url(SharedString),
    /// Load an inline HTML string.
    Html(SharedString),
}

/// A native web view. Create with `cx.new(|cx| WebView::new(cx))`.
pub struct WebView {
    source: Source,
    focus: FocusHandle,
    radius: Option<Size>,
    bordered: bool,
    transparent: bool,
    width: Option<f32>,
    height: Option<f32>,
    /// JavaScript injected at document start (before page scripts run). Hosts
    /// use it to expose a native API the page can call via
    /// `window.ipc.postMessage(...)`. Only applied when the `webview` feature is
    /// on; the placeholder ignores it.
    #[cfg_attr(not(feature = "webview"), allow(dead_code))]
    init_script: Option<SharedString>,
    /// A directory served over an internal `guise://` origin (see [`WebView::serve`]).
    #[cfg_attr(not(feature = "webview"), allow(dead_code))]
    serve_dir: Option<std::path::PathBuf>,

    #[cfg(feature = "webview")]
    inner: Option<Rc<wry::WebView>>,
    #[cfg(feature = "webview")]
    queue: Rc<RefCell<Vec<WebViewEvent>>>,
    #[cfg(feature = "webview")]
    draining: bool,
}

impl EventEmitter<WebViewEvent> for WebView {}

impl WebView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        WebView {
            source: Source::Empty,
            focus: cx.focus_handle(),
            radius: None,
            bordered: true,
            transparent: false,
            width: None,
            height: None,
            init_script: None,
            serve_dir: None,

            #[cfg(feature = "webview")]
            inner: None,
            #[cfg(feature = "webview")]
            queue: Rc::new(RefCell::new(Vec::new())),
            #[cfg(feature = "webview")]
            draining: false,
        }
    }

    /// Inject JavaScript that runs at document start, before the page's own
    /// scripts. Combined with [`WebViewEvent::Message`] (delivered when the page
    /// calls `window.ipc.postMessage(str)`), this lets a host expose a native
    /// API to the embedded page. No-op under the placeholder build.
    pub fn init_script(mut self, js: impl Into<SharedString>) -> Self {
        self.init_script = Some(js.into());
        self
    }

    /// Load a URL (`https://…`, `file://…`, etc.).
    pub fn url(mut self, url: impl Into<SharedString>) -> Self {
        self.source = Source::Url(url.into());
        self
    }

    /// Load an inline HTML document.
    pub fn html(mut self, html: impl Into<SharedString>) -> Self {
        self.source = Source::Html(html.into());
        self
    }

    /// Serve files from `dir` over an internal `guise://localhost/` origin and
    /// load `entry` from it. Prefer this over a `file://` [`WebView::url`] for
    /// local content: `file://` pages are treated as an opaque/null origin, so
    /// the JS bridge (`window.ipc.postMessage`) is dropped and ES modules /
    /// `fetch` are blocked. A real origin fixes both.
    pub fn serve(mut self, dir: impl Into<std::path::PathBuf>, entry: impl AsRef<str>) -> Self {
        self.serve_dir = Some(dir.into());
        self.source = Source::Url(
            format!("guise://localhost/{}", entry.as_ref().trim_start_matches('/')).into(),
        );
        self
    }

    /// Override the corner radius (defaults to the theme radius).
    pub fn radius(mut self, radius: Size) -> Self {
        self.radius = Some(radius);
        self
    }

    /// Draw a border + rounded frame around the view (default `true`).
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    /// Let the page background show through (default `false`).
    pub fn transparent(mut self, transparent: bool) -> Self {
        self.transparent = transparent;
        self
    }

    /// Fix the width in pixels. Defaults to filling the parent.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Fix the height in pixels. Defaults to filling the parent.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// Navigate the live view to `url`, updating the stored source.
    pub fn load_url(&mut self, url: impl Into<SharedString>, cx: &mut Context<Self>) {
        let url = url.into();
        #[cfg(feature = "webview")]
        if let Some(inner) = &self.inner {
            let _ = inner.load_url(&url);
        }
        self.source = Source::Url(url);
        cx.notify();
    }

    /// Replace the live view with inline HTML, updating the stored source.
    pub fn load_html(&mut self, html: impl Into<SharedString>, cx: &mut Context<Self>) {
        let html = html.into();
        #[cfg(feature = "webview")]
        if let Some(inner) = &self.inner {
            let _ = inner.load_html(&html);
        }
        self.source = Source::Html(html);
        cx.notify();
    }

    /// Run JavaScript in the live view. No-op until the view exists.
    pub fn evaluate_script(&self, _js: &str) {
        #[cfg(feature = "webview")]
        if let Some(inner) = &self.inner {
            let _ = inner.evaluate_script(_js);
        }
    }

    /// Show or hide the native surface. The surface tracks its layout bounds only
    /// while it is painted, so a host that stops rendering this view (e.g. a
    /// collapsed drawer or a hidden tab) must hide it explicitly — otherwise the
    /// OS view lingers on screen at its last position. A painted view re-shows
    /// itself. No-op until the view exists.
    pub fn set_visible(&mut self, _visible: bool) {
        #[cfg(feature = "webview")]
        if let Some(inner) = &self.inner {
            let _ = inner.set_visible(_visible);
        }
    }

    /// Build the native view once a window handle is available, then start the
    /// loop that drains events from the wry handlers back onto the entity.
    #[cfg(feature = "webview")]
    fn ensure_view(&mut self, window: &mut Window, cx: &mut Context<Self>, bounds: Bounds<Pixels>) {
        if self.inner.is_some() {
            return;
        }

        let queue = self.queue.clone();
        let (q_title, q_nav, q_load, q_ipc) =
            (queue.clone(), queue.clone(), queue.clone(), queue.clone());

        let mut builder = WebViewBuilder::new()
            .with_bounds(rect_from(bounds))
            .with_transparent(self.transparent)
            .with_document_title_changed_handler(move |title| {
                q_title
                    .borrow_mut()
                    .push(WebViewEvent::TitleChanged(title.into()));
            })
            .with_navigation_handler(move |url| {
                q_nav
                    .borrow_mut()
                    .push(WebViewEvent::UrlChanged(url.into()));
                true
            })
            .with_on_page_load_handler(move |event, _url| {
                q_load.borrow_mut().push(match event {
                    PageLoadEvent::Started => WebViewEvent::LoadStarted,
                    PageLoadEvent::Finished => WebViewEvent::LoadFinished,
                });
            })
            // JS -> native: `window.ipc.postMessage(str)` in the page lands here.
            .with_ipc_handler(move |req| {
                q_ipc
                    .borrow_mut()
                    .push(WebViewEvent::Message(req.into_body().into()));
            });

        if let Some(js) = &self.init_script {
            builder = builder.with_initialization_script(js.to_string());
        }

        // Serve `serve_dir` over the `guise://` scheme used by `WebView::serve`.
        if let Some(dir) = self.serve_dir.clone() {
            builder = builder.with_custom_protocol("guise".to_string(), move |_id, request| {
                serve_local(&dir, request.uri().path())
            });
        }

        builder = match &self.source {
            Source::Url(url) => builder.with_url(url.as_ref()),
            Source::Html(html) => builder.with_html(html.as_ref()),
            Source::Empty => builder,
        };

        match builder.build_as_child(&*window) {
            Ok(view) => self.inner = Some(Rc::new(view)),
            Err(err) => {
                eprintln!("guise: failed to create webview: {err}");
                return;
            }
        }

        if !self.draining {
            self.draining = true;
            cx.spawn(async move |this, cx| loop {
                cx.background_executor()
                    .timer(Duration::from_millis(40))
                    .await;
                let drained: Vec<WebViewEvent> = queue.borrow_mut().drain(..).collect();
                let pushed = this.update(cx, |_this, cx| {
                    let any = !drained.is_empty();
                    for event in drained {
                        cx.emit(event);
                    }
                    if any {
                        cx.notify();
                    }
                });
                if pushed.is_err() {
                    break;
                }
            })
            .detach();
        }
    }
}

#[cfg(feature = "webview")]
fn rect_from(bounds: Bounds<Pixels>) -> Rect {
    Rect {
        position: LogicalPosition::new(bounds.origin.x.to_f64(), bounds.origin.y.to_f64()).into(),
        size: LogicalSize::new(bounds.size.width.to_f64(), bounds.size.height.to_f64()).into(),
    }
}

impl Render for WebView {
    #[cfg(feature = "webview")]
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Build the native view on the first frame that has a window handle.
        // It is created at a best-guess size; the `canvas` paint below snaps it
        // to the real layout bounds on this same frame.
        if self.inner.is_none() {
            let w = self.width.unwrap_or(800.0);
            let h = self.height.unwrap_or(600.0);
            let initial = Bounds {
                origin: gpui::point(px(0.0), px(0.0)),
                size: gpui::size(px(w), px(h)),
            };
            self.ensure_view(window, cx, initial);
        }

        let t = theme(cx);
        let radius = t.radius(self.radius.unwrap_or(t.default_radius));
        let border = t.border().hsla();
        let bg = t.surface().hsla();

        // Sized region the native view tracks. `canvas` hands us the painted
        // bounds in window coordinates each frame; we forward them to wry.
        let view = self.inner.clone();
        let surface = canvas(
            move |_bounds, _window, _app| {},
            move |bounds, _state, _window, _app| {
                if let Some(view) = &view {
                    let _ = view.set_bounds(rect_from(bounds));
                    // Being painted means we're on screen; re-assert visibility so
                    // a view that was hidden while unmounted shows again.
                    let _ = view.set_visible(true);
                }
            },
        )
        .size_full();

        frame(self.bordered, radius, border, bg, self.width, self.height)
            .track_focus(&self.focus)
            .child(surface)
    }

    #[cfg(not(feature = "webview"))]
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        let radius = t.radius(self.radius.unwrap_or(t.default_radius));
        let border = t.border().hsla();
        let bg = t.surface().hsla();
        let dimmed = t.dimmed().hsla();
        let label = match &self.source {
            Source::Url(url) => url.clone(),
            Source::Html(html) => SharedString::from(format!("inline HTML ({} bytes)", html.len())),
            Source::Empty => SharedString::from("no source"),
        };

        frame(self.bordered, radius, border, bg, self.width, self.height)
            .track_focus(&self.focus)
            .items_center()
            .justify_center()
            .text_color(dimmed)
            .child(SharedString::from(format!("WebView (disabled): {label}")))
    }
}

/// The themed container shared by both render paths.
fn frame(
    bordered: bool,
    radius: f32,
    border: gpui::Hsla,
    bg: gpui::Hsla,
    width: Option<f32>,
    height: Option<f32>,
) -> gpui::Stateful<gpui::Div> {
    let mut root = div().id("guise-webview").flex().overflow_hidden().bg(bg);
    root = match width {
        Some(w) => root.w(px(w)),
        None => root.w_full(),
    };
    root = match height {
        Some(h) => root.h(px(h)),
        None => root.h_full(),
    };
    if bordered {
        root = root.border_1().border_color(border).rounded(px(radius));
    }
    root
}

/// Serve a file from `dir` for a `guise://localhost/<path>` request. Rejects
/// paths that try to escape `dir`; unknown files return 404.
#[cfg(feature = "webview")]
fn serve_local(
    dir: &std::path::Path,
    url_path: &str,
) -> wry::http::Response<std::borrow::Cow<'static, [u8]>> {
    use std::borrow::Cow;
    use wry::http::{Response, StatusCode};

    let not_found = || {
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Cow::Borrowed(&b"not found"[..]))
            .unwrap()
    };

    let rel = url_path.trim_start_matches('/');
    let rel = if rel.is_empty() { "index.html" } else { rel };
    // No traversal or absolute escapes; only simple forward paths.
    if rel.split('/').any(|c| c.is_empty() || c == "." || c == "..") {
        return not_found();
    }
    match std::fs::read(dir.join(rel)) {
        Ok(bytes) => Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", content_type(rel))
            .header("Access-Control-Allow-Origin", "*")
            .body(Cow::Owned(bytes))
            .unwrap(),
        Err(_) => not_found(),
    }
}

/// A best-effort content type from a file's extension.
#[cfg(feature = "webview")]
fn content_type(rel: &str) -> &'static str {
    match rel.rsplit('.').next() {
        Some("html" | "htm") => "text/html; charset=utf-8",
        Some("js" | "mjs") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("ico") => "image/x-icon",
        Some("woff2") => "font/woff2",
        Some("woff") => "font/woff",
        Some("ttf") => "font/ttf",
        Some("wasm") => "application/wasm",
        Some("map") => "application/json; charset=utf-8",
        _ => "application/octet-stream",
    }
}
