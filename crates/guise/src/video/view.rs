//! `VideoView` — shows the newest frame the host gives it.
//!
//! A stateful entity because it holds the current texture. Push frames from the
//! UI thread with [`push_frame`](VideoView::push_frame), or call
//! [`feed`](VideoView::feed) once and give the returned [`VideoFeed`] to a
//! decoder thread.
//!
//! Every frame is a new gpui image, and gpui keeps an image's atlas slot until
//! told to drop it. The view retires each replaced frame and drops it at the
//! top of the next render, so a stream doesn't fill the atlas.

use std::sync::Arc;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{
  canvas, div, px, size, Bounds, ContentMask, Context, Corners, EventEmitter, Hsla, IntoElement,
  RenderImage, SharedString, Window,
};

use super::{fit_rect, VideoFeed, VideoFit, VideoFrame};
use crate::devtools::Probed;
use crate::theme::{theme, Size};

/// How often the feed's poller looks for a new frame. Under a frame at 120 fps,
/// so it never adds a visible frame of latency.
const POLL: Duration = Duration::from_millis(8);

/// Emitted when the picture's dimensions change (including the first frame).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VideoEvent {
  Resized { width: u32, height: u32 },
}

/// Frames shown and frames a [`VideoFeed`] overwrote before they were shown.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VideoStats {
  pub shown: u64,
  pub dropped: u64,
}

/// The video surface. Create with `cx.new(|cx| VideoView::new(cx))`.
pub struct VideoView {
  image: Option<Arc<RenderImage>>,
  dims: Option<(u32, u32)>,
  retired: Vec<Arc<RenderImage>>,
  fit: VideoFit,
  background: Option<Hsla>,
  width: Option<f32>,
  height: Option<f32>,
  radius: Option<Size>,
  placeholder: SharedString,
  shown: u64,
  feeds: Vec<VideoFeed>,
}

impl EventEmitter<VideoEvent> for VideoView {}

impl VideoView {
  pub fn new(_cx: &mut Context<Self>) -> Self {
    VideoView {
      image: None,
      dims: None,
      retired: Vec::new(),
      fit: VideoFit::default(),
      background: None,
      width: None,
      height: None,
      radius: None,
      placeholder: "No signal".into(),
      shown: 0,
      feeds: Vec::new(),
    }
  }

  /// How frames fill the box. Default `Contain`.
  pub fn fit(mut self, fit: VideoFit) -> Self {
    self.fit = fit;
    self
  }

  /// The colour behind a letterboxed frame. Default: the theme's surface.
  pub fn background(mut self, color: impl Into<Hsla>) -> Self {
    self.background = Some(color.into());
    self
  }

  /// Fixed width in px; the default fills the parent.
  pub fn width(mut self, width: f32) -> Self {
    self.width = Some(width);
    self
  }

  /// Fixed height in px; the default fills the parent.
  pub fn height(mut self, height: f32) -> Self {
    self.height = Some(height);
    self
  }

  /// Corner radius token.
  pub fn radius(mut self, radius: Size) -> Self {
    self.radius = Some(radius);
    self
  }

  /// What to show before the first frame and after `clear`.
  pub fn placeholder(mut self, text: impl Into<SharedString>) -> Self {
    self.placeholder = text.into();
    self
  }

  /// Show `frame`. Call from the UI thread.
  pub fn push_frame(&mut self, frame: VideoFrame, cx: &mut Context<Self>) {
    let dims = (frame.width(), frame.height());
    let Some(image) = frame.into_image() else {
      return;
    };
    if let Some(old) = self.image.replace(image) {
      self.retired.push(old);
    }
    self.shown += 1;
    if self.dims.replace(dims) != Some(dims) {
      cx.emit(VideoEvent::Resized {
        width: dims.0,
        height: dims.1,
      });
    }
    cx.notify();
  }

  /// Drop the picture and show the placeholder again.
  pub fn clear(&mut self, cx: &mut Context<Self>) {
    if let Some(old) = self.image.take() {
      self.retired.push(old);
    }
    self.dims = None;
    cx.notify();
  }

  /// The current picture's size, if one is showing.
  pub fn frame_size(&self) -> Option<(u32, u32)> {
    self.dims
  }

  /// Frames shown, and frames feeds overwrote before they were shown.
  pub fn stats(&self) -> VideoStats {
    let sent: u64 = self.feeds.iter().map(VideoFeed::sent).sum();
    VideoStats {
      shown: self.shown,
      dropped: sent.saturating_sub(self.shown),
    }
  }

  /// A handle a decoder thread can `send` frames into. The view polls it and
  /// shows the newest; it stops when every clone is dropped.
  pub fn feed(&mut self, cx: &mut Context<Self>) -> VideoFeed {
    let feed = VideoFeed::new();
    let poller = feed.clone();
    self.feeds.push(feed.clone());
    cx.spawn(async move |this, cx| loop {
      cx.background_executor().timer(POLL).await;
      if poller.orphaned() {
        break;
      }
      let Some(frame) = poller.take() else {
        continue;
      };
      if this
        .update(cx, |view, cx| view.push_frame(frame, cx))
        .is_err()
      {
        break;
      }
    })
    .detach();
    feed
  }
}

impl Render for VideoView {
  fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    for old in self.retired.drain(..) {
      let _ = window.drop_image(old);
    }
    let t = theme(cx);
    let bg = self.background.unwrap_or_else(|| t.surface().hsla());
    let dimmed = t.dimmed().hsla();
    let radius = t.radius(self.radius.unwrap_or(t.default_radius));

    let mut root = div()
      .id("guise-videoview")
      .flex()
      .items_center()
      .justify_center()
      .overflow_hidden()
      .bg(bg)
      .rounded(px(radius));
    root = match self.width {
      Some(w) => root.w(px(w)),
      None => root.w_full(),
    };
    root = match self.height {
      Some(h) => root.h(px(h)),
      None => root.h_full(),
    };

    let (image, dims) = (self.image.clone(), self.dims);
    let fit = self.fit;
    let root = match (image, dims) {
      (Some(image), Some((w, h))) => root.child(
        canvas(
          |_, _, _| (),
          move |bounds, _, window, _| {
            let dst = (f32::from(bounds.size.width), f32::from(bounds.size.height));
            let (x, y, fw, fh) = fit_rect((w as f32, h as f32), dst, fit);
            let target = Bounds::new(
              bounds.origin + gpui::point(px(x), px(y)),
              size(px(fw), px(fh)),
            );
            // Rounded corners only read right when the picture reaches the
            // box's edges; a letterboxed one keeps square corners.
            let corners = if fw >= dst.0 - 0.5 && fh >= dst.1 - 0.5 {
              Corners::all(px(radius))
            } else {
              Corners::default()
            };
            window.with_content_mask(Some(ContentMask { bounds }), |window| {
              let _ = window.paint_image(target, corners, image.clone(), 0, false);
            });
          },
        )
        .size_full(),
      ),
      _ => root.text_color(dimmed).child(self.placeholder.clone()),
    };
    root
      .probe("VideoView")
      .attr("fit", format!("{:?}", self.fit))
      .attr("frames", format!("{}", self.shown))
  }
}
