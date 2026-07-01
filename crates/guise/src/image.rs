//! `Image` — a themed wrapper around gpui's `img()` element.
//!
//! Shows a picture from a remote URI, an asset path, or a filesystem path,
//! with guise's sizing/radius vocabulary and an optional fallback slot that
//! renders while the source is loading or unavailable.
//!
//! Note: gpui also exports a *type* named `gpui::Image` — that one is raw
//! encoded bytes (an `Arc<gpui::Image>` is itself a valid source), not an
//! element. This component is the element.
//!
//! ```ignore
//! Image::new("https://example.com/cat.png")
//!     .width(240.0)
//!     .height(160.0)
//!     .radius(Size::Md)
//!     .fit(ObjectFit::Cover)
//!     .fallback(|| Text::new("no image").dimmed())
//! ```

use gpui::prelude::*;
use gpui::{img, px, AnyElement, App, ImageSource, IntoElement, Window};

pub use gpui::ObjectFit;

use crate::theme::{theme, Size};

/// An image element. The Mantine `Image`.
///
/// The source accepts anything gpui's [`ImageSource`] converts from: `&str` /
/// `String` (an `http(s)://` URI, else an embedded-asset path), a
/// `Path`/`PathBuf` (local file), or decoded/raw image data.
#[derive(IntoElement)]
pub struct Image {
    source: ImageSource,
    width: Option<f32>,
    height: Option<f32>,
    radius: Option<Size>,
    circle: bool,
    fit: ObjectFit,
    fallback: Option<Box<dyn Fn() -> AnyElement + 'static>>,
}

impl Image {
    pub fn new(source: impl Into<ImageSource>) -> Self {
        Image {
            source: source.into(),
            width: None,
            height: None,
            radius: None,
            circle: false,
            fit: ObjectFit::Cover,
            fallback: None,
        }
    }

    /// Fixed width in px. Give the element a size — an unsized image lays
    /// out at zero.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Fixed height in px.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// Corner radius from the theme scale (Mantine images are square-cornered
    /// by default).
    pub fn radius(mut self, radius: Size) -> Self {
        self.radius = Some(radius);
        self
    }

    /// Clip to a circle (an avatar). Pair with equal `width`/`height`.
    pub fn circle(mut self) -> Self {
        self.circle = true;
        self
    }

    /// How the picture fills its box (default [`ObjectFit::Cover`]).
    pub fn fit(mut self, fit: ObjectFit) -> Self {
        self.fit = fit;
        self
    }

    /// Element shown while the source is loading or failed to resolve.
    pub fn fallback<E: IntoElement>(mut self, fallback: impl Fn() -> E + 'static) -> Self {
        self.fallback = Some(Box::new(move || fallback().into_any_element()));
        self
    }
}

impl RenderOnce for Image {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let mut el = img(self.source).object_fit(self.fit);
        if let Some(fallback) = self.fallback {
            el = el.with_fallback(fallback);
        }
        if let Some(width) = self.width {
            el = el.w(px(width));
        }
        if let Some(height) = self.height {
            el = el.h(px(height));
        }
        if self.circle {
            el = el.rounded_full();
        } else if let Some(radius) = self.radius {
            el = el.rounded(px(t.radius(radius)));
        }
        el
    }
}
