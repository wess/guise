//! `Paper` — a raised surface: themed background, radius, padding, optional
//! border and shadow. The base container most other surfaces build on.

use gpui::prelude::*;
use gpui::{div, px, AnyElement, App, Div, IntoElement, Window};

use crate::theme::{theme, Size};

/// A surface container. The Mantine `Paper`.
#[derive(IntoElement)]
pub struct Paper {
    children: Vec<AnyElement>,
    padding: Size,
    radius: Option<Size>,
    with_border: bool,
    shadow: Option<Size>,
}

impl Paper {
    pub fn new() -> Self {
        Paper {
            children: Vec::new(),
            padding: Size::Md,
            radius: None,
            with_border: false,
            shadow: None,
        }
    }

    pub fn padding(mut self, padding: Size) -> Self {
        self.padding = padding;
        self
    }

    pub fn radius(mut self, radius: Size) -> Self {
        self.radius = Some(radius);
        self
    }

    pub fn with_border(mut self, with_border: bool) -> Self {
        self.with_border = with_border;
        self
    }

    pub fn shadow(mut self, shadow: Size) -> Self {
        self.shadow = Some(shadow);
        self
    }
}

impl Default for Paper {
    fn default() -> Self {
        Paper::new()
    }
}

impl ParentElement for Paper {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Paper {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let radius = t.radius(self.radius.unwrap_or(t.default_radius));
        let mut el = div()
            .bg(t.surface().hsla())
            .rounded(px(radius))
            .p(px(t.spacing(self.padding)));
        if self.with_border {
            el = el.border_1().border_color(t.border().hsla());
        }
        el = apply_shadow(el, self.shadow);
        el.children(self.children)
    }
}

pub(crate) fn apply_shadow(el: Div, shadow: Option<Size>) -> Div {
    match shadow {
        Some(Size::Xs) => el.shadow_xs(),
        Some(Size::Sm) => el.shadow_sm(),
        Some(Size::Md) => el.shadow_md(),
        Some(Size::Lg) => el.shadow_lg(),
        Some(Size::Xl) => el.shadow_xl(),
        None => el,
    }
}
