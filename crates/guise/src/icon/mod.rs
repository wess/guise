//! `Icon` — a themed glyph drawn from the embedded [Lucide](https://lucide.dev)
//! icon font. Lucide is the default icon set for guise: every icon in the set
//! is an [`IconName`] variant, and any component that accepts a [`Glyph`] takes
//! an `IconName` directly.
//!
//! The font ships inside the crate and registers itself with gpui's text
//! system on first render — no asset pipeline or app setup required. Icons
//! inherit the surrounding text color by default (pass [`Icon::color`] to
//! tint).

mod lucide;

pub use lucide::{IconName, LUCIDE_VERSION};

use std::borrow::Cow;
use std::sync::atomic::{AtomicBool, Ordering};

use gpui::prelude::*;
use gpui::{div, px, App, IntoElement, SharedString, Window};

use crate::theme::{theme, ColorName, Size};

/// The family name baked into the embedded Lucide font.
pub(crate) const FONT_FAMILY: &str = "lucide";

static FONT_BYTES: &[u8] = include_bytes!("../../assets/lucide/lucide.ttf");
static FONT_REGISTERED: AtomicBool = AtomicBool::new(false);

/// Register the embedded Lucide font with gpui's text system. Idempotent and
/// cheap after the first call; every glyph-drawing render path goes through it.
pub(crate) fn ensure_font(cx: &App) {
    if !FONT_REGISTERED.swap(true, Ordering::Relaxed) {
        cx.text_system()
            .add_fonts(vec![Cow::Borrowed(FONT_BYTES)])
            .expect("failed to register the embedded Lucide icon font");
    }
}

/// Icon content for components with an icon slot: a Lucide icon or a short
/// piece of text (an emoji, "+", "</>", …). Both render inline and inherit
/// the surrounding text size and color.
#[derive(Debug, Clone, IntoElement)]
pub enum Glyph {
    Lucide(IconName),
    Text(SharedString),
}

impl From<IconName> for Glyph {
    fn from(name: IconName) -> Self {
        Glyph::Lucide(name)
    }
}

impl From<&'static str> for Glyph {
    fn from(text: &'static str) -> Self {
        Glyph::Text(SharedString::new_static(text))
    }
}

impl From<String> for Glyph {
    fn from(text: String) -> Self {
        Glyph::Text(text.into())
    }
}

impl From<SharedString> for Glyph {
    fn from(text: SharedString) -> Self {
        Glyph::Text(text)
    }
}

impl RenderOnce for Glyph {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        match self {
            Glyph::Lucide(name) => {
                ensure_font(cx);
                div()
                    .font_family(FONT_FAMILY)
                    .child(SharedString::new_static(name.glyph()))
            }
            Glyph::Text(text) => div().child(text),
        }
    }
}

/// A themed icon glyph. Inherits the parent's text color unless [`color`] is
/// set.
///
/// [`color`]: Icon::color
#[derive(IntoElement)]
pub struct Icon {
    name: IconName,
    size: Size,
    color: Option<ColorName>,
}

impl Icon {
    pub fn new(name: IconName) -> Self {
        Icon {
            name,
            size: Size::Md,
            color: None,
        }
    }

    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Tint the glyph with a palette color (defaults to inheriting text color).
    pub fn color(mut self, color: ColorName) -> Self {
        self.color = Some(color);
        self
    }

    fn glyph_px(&self) -> f32 {
        match self.size {
            Size::Xs => 14.0,
            Size::Sm => 16.0,
            Size::Md => 20.0,
            Size::Lg => 26.0,
            Size::Xl => 32.0,
        }
    }
}

impl RenderOnce for Icon {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        ensure_font(cx);
        let t = theme(cx);
        let tint = self.color.map(|c| t.color(c, t.primary_shade()).hsla());
        let size = px(self.glyph_px());
        let mut el = div()
            .flex()
            .items_center()
            .justify_center()
            .font_family(FONT_FAMILY)
            .text_size(size)
            .line_height(size)
            .child(SharedString::new_static(self.name.glyph()));
        if let Some(tint) = tint {
            el = el.text_color(tint);
        }
        el
    }
}
