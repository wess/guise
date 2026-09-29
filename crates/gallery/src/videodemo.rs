//! A synthetic camera for the `VideoView` showcase.
//!
//! The gallery links no codec, so the "stream" is generated: colour bars with a
//! bouncing square, sent from a plain thread at 30 fps through a `VideoFeed` —
//! the same path a real decoder thread takes.

use std::thread;
use std::time::{Duration, Instant};

use guise::prelude::*;

const W: u32 = 320;
const H: u32 = 180;

const BARS: [[u8; 3]; 7] = [
  [235, 235, 235],
  [235, 235, 16],
  [16, 235, 235],
  [16, 235, 16],
  [235, 16, 235],
  [235, 16, 16],
  [16, 16, 235],
];

/// One frame of the test pattern at `t` seconds.
pub fn pattern(t: f32) -> VideoFrame {
  let mut px = Vec::with_capacity((W * H * 4) as usize);
  // the square bounces on a triangle wave so it never jumps
  let wave = |speed: f32, span: f32| {
    let p = (t * speed).rem_euclid(2.0);
    (if p > 1.0 { 2.0 - p } else { p }) * span
  };
  let (sx, sy) = (wave(0.6, (W - 40) as f32), wave(0.9, (H - 40) as f32));
  for y in 0..H {
    for x in 0..W {
      let inside =
        x as f32 >= sx && (x as f32) < sx + 40.0 && y as f32 >= sy && (y as f32) < sy + 40.0;
      let [r, g, b] = if inside {
        [20, 20, 20]
      } else {
        BARS[(x * 7 / W) as usize]
      };
      px.extend_from_slice(&[r, g, b, 255]);
    }
  }
  VideoFrame::rgba(W, H, px).expect("pattern is W*H*4 bytes")
}

/// Send the pattern at 30 fps until the view goes away.
pub fn run(feed: VideoFeed) {
  thread::spawn(move || {
    let start = Instant::now();
    loop {
      feed.send(pattern(start.elapsed().as_secs_f32()));
      thread::sleep(Duration::from_millis(33));
    }
  });
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_pattern_is_a_full_frame() {
    let f = pattern(0.0);
    assert_eq!((f.width(), f.height()), (W, H));
  }
}
