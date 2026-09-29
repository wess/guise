//! How a frame's rectangle maps into the view's box.
//!
//! Pure so the letterbox and crop arithmetic can be tested without a window.

/// How a frame fills a `VideoView`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum VideoFit {
  /// Keep the aspect ratio and show the whole frame, letterboxing the rest.
  #[default]
  Contain,
  /// Keep the aspect ratio and fill the box, cropping what overflows.
  Cover,
  /// Scale each axis independently to fill the box.
  Stretch,
}

/// Where a `src`-sized frame lands in a `dst`-sized box, as `(x, y, w, h)` with
/// the origin at the box's top-left. Degenerate sizes give an empty rect.
pub fn fit_rect(src: (f32, f32), dst: (f32, f32), fit: VideoFit) -> (f32, f32, f32, f32) {
  let (sw, sh) = src;
  let (dw, dh) = dst;
  if sw <= 0.0 || sh <= 0.0 || dw <= 0.0 || dh <= 0.0 {
    return (0.0, 0.0, 0.0, 0.0);
  }
  let (w, h) = match fit {
    VideoFit::Stretch => (dw, dh),
    VideoFit::Contain => {
      let scale = (dw / sw).min(dh / sh);
      (sw * scale, sh * scale)
    }
    VideoFit::Cover => {
      let scale = (dw / sw).max(dh / sh);
      (sw * scale, sh * scale)
    }
  };
  ((dw - w) * 0.5, (dh - h) * 0.5, w, h)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn contain_letterboxes_a_wide_frame_in_a_square() {
    let (x, y, w, h) = fit_rect((200.0, 100.0), (100.0, 100.0), VideoFit::Contain);
    assert_eq!((x, y, w, h), (0.0, 25.0, 100.0, 50.0));
  }

  #[test]
  fn cover_crops_a_wide_frame_in_a_square() {
    let (x, y, w, h) = fit_rect((200.0, 100.0), (100.0, 100.0), VideoFit::Cover);
    assert_eq!((x, y, w, h), (-50.0, 0.0, 200.0, 100.0));
  }

  #[test]
  fn stretch_ignores_the_aspect_ratio() {
    let r = fit_rect((200.0, 100.0), (100.0, 300.0), VideoFit::Stretch);
    assert_eq!(r, (0.0, 0.0, 100.0, 300.0));
  }

  #[test]
  fn a_matching_box_is_left_alone() {
    let r = fit_rect((640.0, 360.0), (640.0, 360.0), VideoFit::Contain);
    assert_eq!(r, (0.0, 0.0, 640.0, 360.0));
  }

  #[test]
  fn degenerate_sizes_paint_nothing() {
    let empty = (0.0, 0.0, 0.0, 0.0);
    assert_eq!(
      fit_rect((0.0, 10.0), (10.0, 10.0), VideoFit::Contain),
      empty
    );
    assert_eq!(fit_rect((10.0, 10.0), (0.0, 10.0), VideoFit::Cover), empty);
  }
}
