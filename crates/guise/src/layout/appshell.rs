//! `AppShell` — the application frame: header, navbar, aside, and footer
//! regions around a scrollable main area.
//!
//! Regions take a fixed px size plus a content closure that is re-invoked
//! every render, so they show live data. The main area is the shell's
//! children (`ParentElement`), laid out as a scrollable column. The shell
//! fills its parent, so place it at the window root (or inside a sized box
//! for a framed demo).
//!
//! ```ignore
//! use guise::prelude::*;
//!
//! AppShell::new()
//!     .header(48.0, |_window, _cx| Text::new("guise"))
//!     .navbar(220.0, |_window, _cx| Text::new("nav links"))
//!     .footer(28.0, |_window, _cx| Text::new("status").size(Size::Xs))
//!     .child(Title::new("Main content").order(2))
//! ```

use gpui::prelude::*;
use gpui::{div, px, AnyElement, App, IntoElement, Window};

use crate::theme::theme;

/// A region-content builder, re-invoked every render (mirrors
/// `data::Content`, kept private to `layout`).
type Content = Box<dyn Fn(&mut Window, &mut App) -> AnyElement + 'static>;

/// The application frame. The Mantine `AppShell`.
///
/// Layout: header across the top, footer across the bottom, and a middle
/// row of navbar | main | aside. Every region gets the theme surface
/// background and a hairline border on its inner edge.
#[derive(IntoElement)]
pub struct AppShell {
    header: Option<(f32, Content)>,
    navbar: Option<(f32, Content)>,
    aside: Option<(f32, Content)>,
    footer: Option<(f32, Content)>,
    children: Vec<AnyElement>,
}

impl AppShell {
    pub fn new() -> Self {
        AppShell {
            header: None,
            navbar: None,
            aside: None,
            footer: None,
            children: Vec::new(),
        }
    }

    /// Top region: `height` px tall, spanning the full width.
    pub fn header<E>(
        mut self,
        height: f32,
        content: impl Fn(&mut Window, &mut App) -> E + 'static,
    ) -> Self
    where
        E: IntoElement,
    {
        self.header = Some((
            height,
            Box::new(move |window, cx| content(window, cx).into_any_element()),
        ));
        self
    }

    /// Left region: `width` px wide, between header and footer.
    pub fn navbar<E>(
        mut self,
        width: f32,
        content: impl Fn(&mut Window, &mut App) -> E + 'static,
    ) -> Self
    where
        E: IntoElement,
    {
        self.navbar = Some((
            width,
            Box::new(move |window, cx| content(window, cx).into_any_element()),
        ));
        self
    }

    /// Right region: `width` px wide, between header and footer.
    pub fn aside<E>(
        mut self,
        width: f32,
        content: impl Fn(&mut Window, &mut App) -> E + 'static,
    ) -> Self
    where
        E: IntoElement,
    {
        self.aside = Some((
            width,
            Box::new(move |window, cx| content(window, cx).into_any_element()),
        ));
        self
    }

    /// Bottom region: `height` px tall, spanning the full width.
    pub fn footer<E>(
        mut self,
        height: f32,
        content: impl Fn(&mut Window, &mut App) -> E + 'static,
    ) -> Self
    where
        E: IntoElement,
    {
        self.footer = Some((
            height,
            Box::new(move |window, cx| content(window, cx).into_any_element()),
        ));
        self
    }
}

impl Default for AppShell {
    fn default() -> Self {
        AppShell::new()
    }
}

impl ParentElement for AppShell {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for AppShell {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let body = t.body().hsla();
        let surface = t.surface().hsla();
        let border = t.border().hsla();

        let mut root = div().size_full().flex().flex_col().bg(body);

        if let Some((height, content)) = self.header {
            root = root.child(
                div()
                    .flex_none()
                    .w_full()
                    .h(px(height))
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .bg(surface)
                    .border_b_1()
                    .border_color(border)
                    .child(content(window, cx)),
            );
        }

        let mut middle = div().flex_1().min_h(px(0.0)).w_full().flex();

        if let Some((width, content)) = self.navbar {
            middle = middle.child(
                div()
                    .flex_none()
                    .w(px(width))
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .bg(surface)
                    .border_r_1()
                    .border_color(border)
                    .child(content(window, cx)),
            );
        }

        middle = middle.child(
            div()
                .id("guise-appshell-main")
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .overflow_y_scroll()
                .children(self.children),
        );

        if let Some((width, content)) = self.aside {
            middle = middle.child(
                div()
                    .flex_none()
                    .w(px(width))
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .bg(surface)
                    .border_l_1()
                    .border_color(border)
                    .child(content(window, cx)),
            );
        }

        root = root.child(middle);

        if let Some((height, content)) = self.footer {
            root = root.child(
                div()
                    .flex_none()
                    .w_full()
                    .h(px(height))
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .bg(surface)
                    .border_t_1()
                    .border_color(border)
                    .child(content(window, cx)),
            );
        }

        root
    }
}
