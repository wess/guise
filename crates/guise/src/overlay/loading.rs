//! `LoadingOverlay` — a dimming busy layer over a container.
//!
//! Stateless: render it as the **last child** of a `.relative()` container and
//! flip [`LoadingOverlay::visible`]. While visible it fills the parent with
//! the body color at 60% opacity, centers a [`Loader`], and occludes the mouse
//! so the content underneath can't be interacted with.
//!
//! ```ignore
//! div()
//!     .relative() // required: the overlay is absolutely positioned
//!     .child(form)
//!     .child(LoadingOverlay::new().visible(self.saving))
//! ```

use gpui::prelude::*;
use gpui::{div, px, App, IntoElement, Window};

use crate::feedback::Loader;
use crate::theme::theme;

/// A busy overlay for one container. The Mantine `LoadingOverlay`.
#[derive(IntoElement)]
pub struct LoadingOverlay {
    visible: bool,
    loader: Option<Loader>,
}

impl LoadingOverlay {
    pub fn new() -> Self {
        LoadingOverlay {
            visible: false,
            loader: None,
        }
    }

    /// Show or hide the overlay. Hidden renders nothing at all.
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Replace the default centered [`Loader`] (e.g. to change variant/color).
    pub fn loader(mut self, loader: Loader) -> Self {
        self.loader = Some(loader);
        self
    }
}

impl Default for LoadingOverlay {
    fn default() -> Self {
        LoadingOverlay::new()
    }
}

impl RenderOnce for LoadingOverlay {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        if !self.visible {
            return div().into_any_element();
        }

        let t = theme(cx);
        let scrim = t.body().alpha(0.6);

        div()
            .id("guise-loading-overlay")
            .occlude()
            .absolute()
            .top(px(0.0))
            .left(px(0.0))
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(scrim)
            .child(self.loader.unwrap_or_default())
            .into_any_element()
    }
}
