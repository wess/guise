//! `Icon` — a themed glyph. A single home for the symbols guise components draw
//! (chevrons, checks, close, …) so they stay visually and tonally consistent.
//!
//! Icons are Unicode glyphs rather than SVG assets: no asset pipeline, and they
//! inherit the surrounding text color by default (pass [`Icon::color`] to tint).

use gpui::prelude::*;
use gpui::{div, px, App, IntoElement, SharedString, Window};

use crate::theme::{theme, ColorName, Size};

/// A named icon. The glyph is resolved by [`IconName::glyph`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconName {
    Check,
    Close,
    Minus,
    Plus,
    ChevronDown,
    ChevronUp,
    ChevronLeft,
    ChevronRight,
    Search,
    Dot,
    Info,
    Warning,
    Star,
    Copy,
    Menu,
    Ellipsis,
    ArrowRight,
    ArrowLeft,
    Eye,
    EyeOff,
}

impl IconName {
    /// The Unicode glyph drawn for this icon.
    pub fn glyph(self) -> &'static str {
        match self {
            IconName::Check => "\u{2713}",
            IconName::Close => "\u{00d7}",
            IconName::Minus => "\u{2212}",
            IconName::Plus => "+",
            IconName::ChevronDown => "\u{25be}",
            IconName::ChevronUp => "\u{25b4}",
            IconName::ChevronLeft => "\u{25c2}",
            IconName::ChevronRight => "\u{25b8}",
            IconName::Search => "\u{2315}",
            IconName::Dot => "\u{2022}",
            IconName::Info => "\u{2139}",
            IconName::Warning => "\u{26a0}",
            IconName::Star => "\u{2605}",
            IconName::Copy => "\u{29c9}",
            IconName::Menu => "\u{2630}",
            IconName::Ellipsis => "\u{2026}",
            IconName::ArrowRight => "\u{2192}",
            IconName::ArrowLeft => "\u{2190}",
            IconName::Eye => "\u{25ce}",
            IconName::EyeOff => "\u{2298}",
        }
    }
}

/// A themed icon glyph. Inherits the parent's text color unless [`color`] is set.
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
        let t = theme(cx);
        let tint = self.color.map(|c| t.color(c, t.primary_shade()).hsla());
        let mut el = div()
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(self.glyph_px()))
            .child(SharedString::new_static(self.name.glyph()));
        if let Some(tint) = tint {
            el = el.text_color(tint);
        }
        el
    }
}
