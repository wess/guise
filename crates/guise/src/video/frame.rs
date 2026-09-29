//! One decoded picture.
//!
//! gpui's image path wants BGRA, so that is what a frame stores. Decoders hand
//! over RGBA or planar YUV; the constructors convert once, here, off the render
//! path. Every constructor returns `None` for a buffer the wrong size for its
//! dimensions — a decoder bug should surface as a missing frame, not as a
//! panic inside the paint pass.

use std::sync::Arc;

use gpui::RenderImage;
use image::{Frame, RgbaImage};

/// A decoded video frame, ready to show.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VideoFrame {
  width: u32,
  height: u32,
  bgra: Vec<u8>,
}

impl VideoFrame {
  /// From tightly packed 8-bit RGBA (`width * height * 4` bytes). Swaps the
  /// red and blue channels in place.
  pub fn rgba(width: u32, height: u32, mut data: Vec<u8>) -> Option<Self> {
    if !fits(width, height, data.len(), 4) {
      return None;
    }
    for px in data.chunks_exact_mut(4) {
      px.swap(0, 2);
    }
    Some(VideoFrame {
      width,
      height,
      bgra: data,
    })
  }

  /// From tightly packed 8-bit BGRA — no conversion, the cheapest way in when
  /// the source can produce it (most capture APIs can).
  pub fn bgra(width: u32, height: u32, data: Vec<u8>) -> Option<Self> {
    if !fits(width, height, data.len(), 4) {
      return None;
    }
    Some(VideoFrame {
      width,
      height,
      bgra: data,
    })
  }

  /// From planar 4:2:0 YUV (I420, what most software decoders emit), BT.601
  /// limited range. `y` is `width * height` bytes; `u` and `v` are each
  /// `ceil(width / 2) * ceil(height / 2)`. Strides must equal the widths.
  pub fn i420(width: u32, height: u32, y: &[u8], u: &[u8], v: &[u8]) -> Option<Self> {
    let (w, h) = (width as usize, height as usize);
    let (cw, ch) = (w.div_ceil(2), h.div_ceil(2));
    if w == 0 || h == 0 || y.len() != w * h || u.len() != cw * ch || v.len() != cw * ch {
      return None;
    }
    let mut bgra = Vec::with_capacity(w * h * 4);
    for row in 0..h {
      for col in 0..w {
        let luma = i32::from(y[row * w + col]) - 16;
        let ci = (row / 2) * cw + col / 2;
        let d = i32::from(u[ci]) - 128;
        let e = i32::from(v[ci]) - 128;
        let c = 298 * luma;
        let clamp = |x: i32| (x >> 8).clamp(0, 255) as u8;
        bgra.extend_from_slice(&[
          clamp(c + 516 * d + 128),
          clamp(c - 100 * d - 208 * e + 128),
          clamp(c + 409 * e + 128),
          255,
        ]);
      }
    }
    Some(VideoFrame {
      width,
      height,
      bgra,
    })
  }

  pub fn width(&self) -> u32 {
    self.width
  }

  pub fn height(&self) -> u32 {
    self.height
  }

  /// The pixels, BGRA, tightly packed.
  pub fn as_bgra(&self) -> &[u8] {
    &self.bgra
  }

  /// Hand the pixels to gpui. Each call makes a new image id, which is what
  /// tells the sprite atlas this is a different texture.
  pub(crate) fn into_image(self) -> Option<Arc<RenderImage>> {
    let buffer = RgbaImage::from_raw(self.width, self.height, self.bgra)?;
    Some(Arc::new(RenderImage::new(vec![Frame::new(buffer)])))
  }
}

fn fits(width: u32, height: u32, len: usize, bytes: usize) -> bool {
  width > 0 && height > 0 && len == width as usize * height as usize * bytes
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn rgba_is_stored_as_bgra() {
    let f = VideoFrame::rgba(2, 1, vec![1, 2, 3, 255, 10, 20, 30, 255]).unwrap();
    assert_eq!(f.as_bgra(), &[3, 2, 1, 255, 30, 20, 10, 255]);
  }

  #[test]
  fn bgra_is_kept_as_is() {
    let f = VideoFrame::bgra(1, 1, vec![9, 8, 7, 6]).unwrap();
    assert_eq!(f.as_bgra(), &[9, 8, 7, 6]);
  }

  #[test]
  fn a_wrong_sized_buffer_is_refused() {
    assert!(VideoFrame::rgba(2, 2, vec![0; 15]).is_none());
    assert!(VideoFrame::bgra(0, 4, vec![]).is_none());
    assert!(VideoFrame::i420(2, 2, &[0; 4], &[0; 1], &[0; 2]).is_none());
  }

  #[test]
  fn i420_maps_the_limited_range_extremes() {
    // Y=16 is black and Y=235 is white with neutral chroma.
    let black = VideoFrame::i420(2, 2, &[16; 4], &[128], &[128]).unwrap();
    assert_eq!(&black.as_bgra()[..4], &[0, 0, 0, 255]);
    let white = VideoFrame::i420(2, 2, &[235; 4], &[128], &[128]).unwrap();
    assert_eq!(&white.as_bgra()[..4], &[255, 255, 255, 255]);
  }

  #[test]
  fn i420_puts_red_chroma_in_the_red_channel() {
    // BT.601 red is roughly Y=81, U=90, V=240.
    let f = VideoFrame::i420(2, 2, &[81; 4], &[90], &[240]).unwrap();
    let [b, g, r, a] = f.as_bgra()[..4] else {
      unreachable!()
    };
    assert!(r > 240 && g < 20 && b < 20 && a == 255, "{r} {g} {b}");
  }

  #[test]
  fn i420_takes_odd_dimensions() {
    let f = VideoFrame::i420(3, 3, &[16; 9], &[128; 4], &[128; 4]).unwrap();
    assert_eq!(f.as_bgra().len(), 3 * 3 * 4);
  }
}
