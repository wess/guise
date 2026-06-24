//! `Container`, `Padding`, `Align`, and `Center` — Flutter's box wrappers.

use gpui::prelude::*;
use gpui::{div, px, AnyElement, App, IntoElement, Window};

use super::{apply_alignment, apply_margin, apply_padding, Alignment, EdgeInsets};
use crate::theme::Color;

/// A configurable box: size, padding, margin, color, radius, border, and child
/// alignment. Flutter's `Container`.
#[derive(IntoElement)]
pub struct Container {
    child: Option<AnyElement>,
    width: Option<f32>,
    height: Option<f32>,
    padding: EdgeInsets,
    margin: EdgeInsets,
    color: Option<Color>,
    radius: f32,
    border: Option<(f32, Color)>,
    alignment: Option<Alignment>,
}

impl Container {
    pub fn new() -> Self {
        Container {
            child: None,
            width: None,
            height: None,
            padding: EdgeInsets::default(),
            margin: EdgeInsets::default(),
            color: None,
            radius: 0.0,
            border: None,
            alignment: None,
        }
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.child = Some(child.into_any_element());
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    pub fn padding(mut self, padding: EdgeInsets) -> Self {
        self.padding = padding;
        self
    }

    pub fn margin(mut self, margin: EdgeInsets) -> Self {
        self.margin = margin;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    pub fn border(mut self, width: f32, color: Color) -> Self {
        self.border = Some((width, color));
        self
    }

    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = Some(alignment);
        self
    }
}

impl Default for Container {
    fn default() -> Self {
        Container::new()
    }
}

impl RenderOnce for Container {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut el = div();
        if let Some(w) = self.width {
            el = el.w(px(w));
        }
        if let Some(h) = self.height {
            el = el.h(px(h));
        }
        el = apply_padding(el, self.padding);
        el = apply_margin(el, self.margin);
        if let Some(color) = self.color {
            el = el.bg(color.hsla());
        }
        if self.radius > 0.0 {
            el = el.rounded(px(self.radius));
        }
        if let Some((width, color)) = self.border {
            el = el.border(px(width)).border_color(color.hsla());
        }
        if let Some(alignment) = self.alignment {
            el = apply_alignment(el.flex(), alignment);
        }
        if let Some(child) = self.child {
            el = el.child(child);
        }
        el
    }
}

/// Insets a single child. Flutter's `Padding`.
#[derive(IntoElement)]
pub struct Padding {
    insets: EdgeInsets,
    child: Option<AnyElement>,
}

impl Padding {
    pub fn all(value: f32) -> Self {
        Padding {
            insets: EdgeInsets::all(value),
            child: None,
        }
    }

    pub fn symmetric(horizontal: f32, vertical: f32) -> Self {
        Padding {
            insets: EdgeInsets::symmetric(horizontal, vertical),
            child: None,
        }
    }

    pub fn only(insets: EdgeInsets) -> Self {
        Padding {
            insets,
            child: None,
        }
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.child = Some(child.into_any_element());
        self
    }
}

impl RenderOnce for Padding {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut el = apply_padding(div(), self.insets);
        if let Some(child) = self.child {
            el = el.child(child);
        }
        el
    }
}

/// Aligns a single child within the available space. Flutter's `Align`.
#[derive(IntoElement)]
pub struct Align {
    alignment: Alignment,
    child: Option<AnyElement>,
}

impl Align {
    pub fn new(alignment: Alignment) -> Self {
        Align {
            alignment,
            child: None,
        }
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.child = Some(child.into_any_element());
        self
    }
}

impl RenderOnce for Align {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut el = apply_alignment(div().flex().size_full(), self.alignment);
        if let Some(child) = self.child {
            el = el.child(child);
        }
        el
    }
}

/// Centers a single child. Flutter's `Center`.
#[derive(IntoElement)]
pub struct Center {
    child: Option<AnyElement>,
}

impl Center {
    pub fn new() -> Self {
        Center { child: None }
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.child = Some(child.into_any_element());
        self
    }
}

impl Default for Center {
    fn default() -> Self {
        Center::new()
    }
}

impl RenderOnce for Center {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut el = div().flex().size_full().items_center().justify_center();
        if let Some(child) = self.child {
            el = el.child(child);
        }
        el
    }
}
