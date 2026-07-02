//! `Blockquote` — a quoted passage behind a left accent border, with an
//! optional icon and citation.
//!
//! ```ignore
//! Blockquote::new()
//!     .icon(IconName::Info)
//!     .text("Life is like an npm install — you never know what you are going to get.")
//!     .cite("– Forrest Gump")
//! ```

use gpui::prelude::*;
use gpui::{div, px, AnyElement, App, IntoElement, SharedString, Window};

use crate::icon::{Icon, IconName};
use crate::theme::{theme, ColorName, Size};

/// A quote block. The Mantine `Blockquote`.
///
/// Content is either [`Blockquote::text`], `ParentElement` children
/// (`.child(..)`), or both — text renders first.
#[derive(IntoElement)]
pub struct Blockquote {
    children: Vec<AnyElement>,
    text: Option<SharedString>,
    color: ColorName,
    cite: Option<SharedString>,
    icon: Option<IconName>,
    padding: Size,
    radius: Option<Size>,
}

impl Blockquote {
    pub fn new() -> Self {
        Blockquote {
            children: Vec::new(),
            text: None,
            color: ColorName::Blue,
            cite: None,
            icon: None,
            padding: Size::Lg,
            radius: None,
        }
    }

    /// The quoted text (shorthand for a single themed text child).
    pub fn text(mut self, text: impl Into<SharedString>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// The accent color for the border, icon, and background wash.
    pub fn color(mut self, color: ColorName) -> Self {
        self.color = color;
        self
    }

    /// Attribution line, rendered dimmed below the quote (include your own
    /// dash, e.g. `"– Forrest Gump"`).
    pub fn cite(mut self, cite: impl Into<SharedString>) -> Self {
        self.cite = Some(cite.into());
        self
    }

    /// A glyph shown above the quote in the accent color.
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn padding(mut self, padding: Size) -> Self {
        self.padding = padding;
        self
    }

    pub fn radius(mut self, radius: Size) -> Self {
        self.radius = Some(radius);
        self
    }
}

impl Default for Blockquote {
    fn default() -> Self {
        Blockquote::new()
    }
}

impl ParentElement for Blockquote {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Blockquote {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let dark = t.scheme.is_dark();
        let accent = t.color(self.color, if dark { 4 } else { 6 }).hsla();
        let wash = t.color(self.color, if dark { 5 } else { 6 }).alpha(0.06);
        let text_color = t.text().hsla();
        let dimmed = t.dimmed().hsla();
        let padding = t.spacing(self.padding);
        let gap = t.spacing(Size::Sm);
        let radius = t.radius(self.radius.unwrap_or(t.default_radius));
        let font_md = t.font_size(Size::Md);
        let font_sm = t.font_size(Size::Sm);

        let mut el = div()
            .flex()
            .flex_col()
            .gap(px(gap))
            .p(px(padding))
            .border_l(px(3.0))
            .border_color(accent)
            .rounded_r(px(radius))
            .bg(wash)
            .text_size(px(font_md))
            .text_color(text_color);

        if let Some(icon) = self.icon {
            el = el.child(
                div()
                    .flex()
                    .text_color(accent)
                    .child(Icon::new(icon).size(Size::Sm)),
            );
        }
        if let Some(text) = self.text {
            el = el.child(div().child(text));
        }
        el = el.children(self.children);
        if let Some(cite) = self.cite {
            el = el.child(div().text_size(px(font_sm)).text_color(dimmed).child(cite));
        }
        el
    }
}
