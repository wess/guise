//! `ColorInput` — a color picker field (gpui entity).
//!
//! A swatch plus an editable hex/CSS text field; clicking the swatch opens a
//! deferred dropdown with the full theme palette (14 colors x 10 shades).
//! Typing any [`css`](crate::theme::css)-parsable color — `#40c057`,
//! `rgb(64, 192, 87)`, `teal` — updates the swatch live. Emits
//! [`ColorInputEvent`] whenever the value changes.
//!
//! ```ignore
//! let brand = cx.new(|cx| ColorInput::new(cx).label("Brand color").value(rgb(34, 139, 230)));
//! cx.subscribe(&brand, |_this, _input, event: &ColorInputEvent, _cx| {
//!     let color: Hsla = event.0;
//! })
//! .detach();
//! ```

use gpui::prelude::*;
use gpui::{
    deferred, div, px, App, Context, Entity, EventEmitter, FocusHandle, Hsla, IntoElement,
    KeyDownEvent, MouseButton, SharedString, Window,
};

use super::{apply_key, control_metrics, edit::TextEdit, Field, KeyOutcome};
use crate::reactive::Signal;
use crate::theme::{css, theme, Color, ColorName, Size};

/// Emitted when the color changes (typed, picked, or bound). Carries the color.
#[derive(Debug, Clone, Copy)]
pub struct ColorInputEvent(pub Hsla);

/// A color field with a palette dropdown. Create with
/// `cx.new(|cx| ColorInput::new(cx))`.
pub struct ColorInput {
    edit: TextEdit,
    value: Hsla,
    open: bool,
    focus: FocusHandle,
    label: Option<SharedString>,
    description: Option<SharedString>,
    error: Option<SharedString>,
    size: Size,
    disabled: bool,
}

impl EventEmitter<ColorInputEvent> for ColorInput {}

/// `#rrggbb` for a color, dropping alpha (the buffer holds opaque hex).
fn to_hex(color: Hsla) -> String {
    let c = Color::from_hsla(color);
    format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)
}

/// This input's value is always opaque: the buffer renders `#rrggbb`, so a
/// value carrying alpha would desync the text from the swatch and the emitted
/// color. Every value entering the field passes through here.
fn opaque(color: Hsla) -> Hsla {
    Hsla { a: 1.0, ..color }
}

impl ColorInput {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let value = gpui::black();
        ColorInput {
            edit: TextEdit::new(&to_hex(value)),
            value,
            open: false,
            focus: cx.focus_handle(),
            label: None,
            description: None,
            error: None,
            size: Size::Sm,
            disabled: false,
        }
    }

    /// The initial color (alpha is dropped). Also rewrites the text buffer
    /// as hex.
    pub fn value(mut self, color: impl Into<Hsla>) -> Self {
        let color = opaque(color.into());
        self.value = color;
        self.edit = TextEdit::new(&to_hex(color));
        self
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn error(mut self, error: impl Into<SharedString>) -> Self {
        self.error = Some(error.into());
        self
    }

    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// The current color.
    pub fn color_value(&self) -> Hsla {
        self.value
    }

    /// Two-way bind this input's color to a `Signal<Hsla>`. The signal is the
    /// source of truth: the input adopts its value now, picks and valid typed
    /// colors write back through [`Signal::set_if_changed`], and signal writes
    /// update the swatch and buffer without emitting [`ColorInputEvent`].
    /// Equality guards on both directions prevent update loops.
    pub fn bind(entity: &Entity<ColorInput>, signal: &Signal<Hsla>, cx: &mut App) {
        let initial = signal.get(cx);
        entity.update(cx, |this, cx| this.sync_value(initial, cx));
        let sink = signal.clone();
        cx.subscribe(entity, move |_input, event: &ColorInputEvent, cx| {
            sink.set_if_changed(cx, event.0);
        })
        .detach();
        let input = entity.downgrade();
        cx.observe(signal.entity(), move |observed, cx| {
            let value = *observed.read(cx);
            input.update(cx, |this, cx| this.sync_value(value, cx)).ok();
        })
        .detach();
    }

    /// Programmatic set: update swatch + buffer without emitting an event.
    fn sync_value(&mut self, color: Hsla, cx: &mut Context<Self>) {
        let color = opaque(color);
        if self.value != color {
            self.value = color;
            self.edit = TextEdit::new(&to_hex(color));
            cx.notify();
        }
    }

    /// A palette pick: set, normalize the buffer to hex, close, emit.
    fn choose(&mut self, color: Hsla, cx: &mut Context<Self>) {
        self.open = false;
        self.edit = TextEdit::new(&to_hex(color));
        if self.value != color {
            self.value = color;
            cx.emit(ColorInputEvent(color));
        }
        cx.notify();
    }

    /// Re-parse the buffer after an edit; a valid color updates the swatch.
    fn adopt_buffer(&mut self, cx: &mut Context<Self>) {
        if let Ok(color) = css(&self.edit.text()).map(opaque) {
            if self.value != color {
                self.value = color;
                cx.emit(ColorInputEvent(color));
            }
        }
    }

    fn on_key(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        match apply_key(&mut self.edit, &event.keystroke) {
            KeyOutcome::Submit => {
                self.adopt_buffer(cx);
                // Normalize whatever parsed (or the last valid color) to hex.
                self.edit = TextEdit::new(&to_hex(self.value));
                self.open = false;
                cx.notify();
                cx.stop_propagation();
            }
            KeyOutcome::Edited => {
                self.adopt_buffer(cx);
                cx.notify();
                cx.stop_propagation();
            }
            KeyOutcome::Cancel => {
                // Escape closes the dropdown; bubbles when already closed.
                if self.open {
                    self.open = false;
                    cx.notify();
                    cx.stop_propagation();
                }
            }
            KeyOutcome::Pass => {}
        }
    }
}

impl Render for ColorInput {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        let (height, pad_x, font) = control_metrics(self.size);
        let radius = t.radius(t.default_radius);
        let focused = self.focus.is_focused(window) && !self.disabled;

        let border = if self.error.is_some() {
            t.color(ColorName::Red, 6)
        } else if focused {
            t.primary()
        } else {
            t.border()
        }
        .hsla();
        let plain_border = t.border().hsla();
        let text_color = t.text().hsla();
        let surface = t.surface().hsla();
        let caret = t.primary().hsla();
        let swatch_px = height - 16.0;

        let swatch = div()
            .id("guise-colorinput-swatch")
            .flex_none()
            .w(px(swatch_px))
            .h(px(swatch_px))
            .rounded(px(4.0))
            .border_1()
            .border_color(plain_border)
            .bg(self.value)
            .cursor_pointer()
            .on_click(cx.listener(|this, _ev, window, cx| {
                if !this.disabled {
                    this.open = !this.open;
                    window.focus(&this.focus);
                    cx.notify();
                }
            }));

        let interior = if focused {
            let (before, after) = self.edit.split();
            div()
                .flex()
                .items_center()
                .text_color(text_color)
                .child(SharedString::from(before))
                .child(div().w(px(1.0)).h(px(font * 1.15)).bg(caret))
                .child(SharedString::from(after))
        } else {
            div()
                .text_color(text_color)
                .child(SharedString::from(self.edit.text()))
        };

        let field = div()
            .id("guise-colorinput")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _ev, window, cx| {
                    window.focus(&this.focus);
                    cx.notify();
                }),
            )
            .flex()
            .items_center()
            .gap(px(8.0))
            .h(px(height))
            .px(px(pad_x))
            .rounded(px(radius))
            .border_1()
            .border_color(border)
            .bg(surface)
            .text_size(px(font))
            .child(swatch)
            .child(interior);

        let mut wrap = div().relative().child(field);

        if self.open && !self.disabled {
            let current = Color::from_hsla(self.value);
            let mut grid = div()
                .occlude()
                .absolute()
                .top(px(height + 6.0))
                .left(px(0.0))
                .flex()
                .flex_col()
                .gap(px(2.0))
                .p(px(6.0))
                .rounded(px(radius))
                .border_1()
                .border_color(plain_border)
                .bg(surface)
                .shadow_md();

            for (row, name) in ColorName::ALL.into_iter().enumerate() {
                let mut cells = div().flex().flex_row().gap(px(2.0));
                for shade in 0..10 {
                    let cell_color = t.color(name, shade);
                    let cell_hsla = cell_color.hsla();
                    let mut cell = div()
                        .id(("guise-colorinput-cell", row * 10 + shade))
                        .w(px(14.0))
                        .h(px(14.0))
                        .rounded(px(3.0))
                        .bg(cell_hsla)
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, _ev, _window, cx| {
                            this.choose(cell_hsla, cx);
                        }));
                    if cell_color == current {
                        cell = cell
                            .border_2()
                            .border_color(cell_color.contrasting().hsla());
                    }
                    cells = cells.child(cell);
                }
                grid = grid.child(cells);
            }

            wrap = wrap.child(deferred(grid));
        }

        let mut chrome = Field::new().child(if self.disabled {
            wrap.opacity(0.6)
        } else {
            wrap
        });
        if let Some(label) = self.label.clone() {
            chrome = chrome.label(label);
        }
        if let Some(error) = self.error.clone() {
            chrome = chrome.error(error);
        } else if let Some(description) = self.description.clone() {
            chrome = chrome.description(description);
        }
        chrome
    }
}

#[cfg(test)]
mod tests {
    use super::to_hex;
    use crate::theme::css;

    #[test]
    fn hex_round_trips_through_css_parsing() {
        for hex in [
            "#ff0000", "#00ff00", "#0000ff", "#ffffff", "#000000", "#808080",
        ] {
            assert_eq!(to_hex(css(hex).unwrap()), hex);
        }
    }

    #[test]
    fn alpha_is_dropped() {
        let translucent = css("rgba(255, 0, 0, 0.5)").unwrap();
        assert_eq!(to_hex(translucent), "#ff0000");
    }

    #[test]
    fn adopted_colors_are_opaque() {
        // The value the field adopts must match the hex it displays: an
        // rgba() input and its opaque rgb() twin resolve to the same color.
        let adopted = super::opaque(css("rgba(255, 0, 0, 0.5)").unwrap());
        assert_eq!(adopted, css("rgb(255, 0, 0)").unwrap());
        assert_eq!(adopted.a, 1.0);
    }
}
