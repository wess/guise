//! The hand-off from a decoder thread to the UI.
//!
//! Decoders run on their own thread and gpui entities live on the main one, so
//! frames cross through a one-slot mailbox: `send` overwrites whatever is
//! waiting. That is the right policy for live video — if the UI is behind, the
//! stale frame is worth nothing and queueing it only adds latency — and the
//! overwritten count is kept so a host can see how far behind it ran.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use super::VideoFrame;

struct Slot {
  latest: Mutex<Option<VideoFrame>>,
  sent: AtomicU64,
}

/// A cloneable, `Send` handle that feeds frames to one [`VideoView`](super::VideoView).
/// Get it from `VideoView::feed`.
#[derive(Clone)]
pub struct VideoFeed {
  slot: Arc<Slot>,
}

impl VideoFeed {
  pub(crate) fn new() -> Self {
    VideoFeed {
      slot: Arc::new(Slot {
        latest: Mutex::new(None),
        sent: AtomicU64::new(0),
      }),
    }
  }

  /// Offer a frame; replaces one the view hasn't picked up yet.
  pub fn send(&self, frame: VideoFrame) {
    self.slot.sent.fetch_add(1, Ordering::Relaxed);
    *self.slot.latest.lock().unwrap_or_else(|e| e.into_inner()) = Some(frame);
  }

  /// The waiting frame, if any.
  pub(crate) fn take(&self) -> Option<VideoFrame> {
    self
      .slot
      .latest
      .lock()
      .unwrap_or_else(|e| e.into_inner())
      .take()
  }

  /// How many frames have been sent, picked up or not.
  pub(crate) fn sent(&self) -> u64 {
    self.slot.sent.load(Ordering::Relaxed)
  }

  /// True once every clone but the view's own poller is gone.
  pub(crate) fn orphaned(&self) -> bool {
    Arc::strong_count(&self.slot) <= 1
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn frame(v: u8) -> VideoFrame {
    VideoFrame::bgra(1, 1, vec![v; 4]).unwrap()
  }

  #[test]
  fn the_newest_frame_wins() {
    let feed = VideoFeed::new();
    feed.send(frame(1));
    feed.send(frame(2));
    assert_eq!(feed.take().unwrap().as_bgra()[0], 2);
    assert!(feed.take().is_none());
    assert_eq!(feed.sent(), 2);
  }

  #[test]
  fn it_is_orphaned_when_only_the_poller_is_left() {
    let poller = VideoFeed::new();
    let sender = poller.clone();
    assert!(!poller.orphaned());
    drop(sender);
    assert!(poller.orphaned());
  }
}
