//! `Rating` — a row of clickable stars (controlled).
//!
//! The parent owns the value (an `f32` rendered as whole stars) and passes a
//! change handler, or two-way binds it with [`Rating::bind`]. Clicking star
//! `i` sets the value to `i`; hovering an unfilled star previews it in the
//! accent color. `readonly` renders a static display.
//!
//! ```ignore
//! Rating::new("stars")
//!     .value(self.stars)
//!     .color(ColorName::Yellow)
//!     .on_change(cx.listener(|this, value: &f32, _w, cx| {
//!         this.stars = *value;
//!         cx.notify();
//!     }))
//! ```

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{div, px, App, ElementId, IntoElement, SharedString, Window};

use crate::reactive::Binding;
use crate::style::ColorValue;
use crate::theme::{theme, ColorName, Size};

type ChangeHandler = Rc<dyn Fn(&f32, &mut Window, &mut App) + 'static>;

/// A star rating. The Mantine `Rating`. Controlled: pass `value` and an
/// `on_change`, or two-way bind with [`Rating::bind`].
#[derive(IntoElement)]
pub struct Rating {
    id: ElementId,
    value: f32,
    count: usize,
    color: ColorValue,
    size: Size,
    readonly: bool,
    binding: Option<Binding<f32>>,
    on_change: Option<ChangeHandler>,
}

impl Rating {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Rating {
            id: id.into(),
            value: 0.0,
            count: 5,
            color: ColorValue::Named(ColorName::Yellow),
            size: Size::Md,
            readonly: false,
            binding: None,
            on_change: None,
        }
    }

    pub fn value(mut self, value: f32) -> Self {
        self.value = value;
        self
    }

    /// How many stars to draw (default 5).
    pub fn count(mut self, count: usize) -> Self {
        self.count = count.max(1);
        self
    }

    pub fn color(mut self, color: impl Into<ColorValue>) -> Self {
        self.color = color.into();
        self
    }

    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Display-only: no hover preview, no clicks.
    pub fn readonly(mut self, readonly: bool) -> Self {
        self.readonly = readonly;
        self
    }

    /// Two-way bind the value. Overrides `value`; clicks write the new rating
    /// back through the binding, then run any `on_change`.
    pub fn bind(mut self, binding: Binding<f32>) -> Self {
        self.binding = Some(binding);
        self
    }

    pub fn on_change(mut self, handler: impl Fn(&f32, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }

    fn glyph_px(&self) -> f32 {
        match self.size {
            Size::Xs => 14.0,
            Size::Sm => 18.0,
            Size::Md => 22.0,
            Size::Lg => 28.0,
            Size::Xl => 36.0,
        }
    }
}

/// How many leading stars read as filled for `value` (rounded, clamped).
fn filled_stars(value: f32, count: usize) -> usize {
    if value <= 0.0 {
        0
    } else {
        (value.round() as usize).min(count)
    }
}

impl RenderOnce for Rating {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let accent = self.color.accent(t);
        let empty = if t.scheme.is_dark() {
            t.color(ColorName::Dark, 3)
        } else {
            t.color(ColorName::Gray, 4)
        }
        .hsla();
        let glyph = self.glyph_px();

        let value = self.binding.as_ref().map_or(self.value, |b| b.get(cx));
        let filled = filled_stars(value, self.count);

        let mut row = div()
            .id(self.id)
            .flex()
            .flex_row()
            .items_center()
            .gap(px(2.0));

        for i in 1..=self.count {
            let is_filled = i <= filled;
            let mut star = div()
                .id(("guise-rating-star", i))
                .text_size(px(glyph))
                .text_color(if is_filled { accent } else { empty })
                .child(SharedString::new_static(if is_filled {
                    "\u{2605}"
                } else {
                    "\u{2606}"
                }));
            if !self.readonly {
                star = star.cursor_pointer().hover(move |s| s.text_color(accent));
                let binding = self.binding.clone();
                let handler = self.on_change.clone();
                let next = i as f32;
                star = star.on_click(move |_ev, window, cx| {
                    if let Some(binding) = &binding {
                        binding.set(cx, next);
                    }
                    if let Some(handler) = &handler {
                        handler(&next, window, cx);
                    }
                });
            }
            row = row.child(star);
        }
        row
    }
}

#[cfg(test)]
mod tests {
    use super::filled_stars;

    #[test]
    fn rounds_to_the_nearest_star() {
        assert_eq!(filled_stars(2.4, 5), 2);
        assert_eq!(filled_stars(2.5, 5), 3);
        assert_eq!(filled_stars(3.0, 5), 3);
    }

    #[test]
    fn clamps_into_the_star_range() {
        assert_eq!(filled_stars(0.0, 5), 0);
        assert_eq!(filled_stars(-1.0, 5), 0);
        assert_eq!(filled_stars(9.0, 5), 5);
    }
}
