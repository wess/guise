//! `Panel` — a titled surface: `Card` chrome plus a header row (icon, title,
//! description, trailing actions), an optional footer, and a controlled
//! collapse.
//!
//! ```ignore
//! Panel::new()
//!     .id("status")
//!     .title("Project status")
//!     .description("Weekly summary")
//!     .action(ActionIcon::new("status-more", "…"))
//!     .collapsible()
//!     .collapsed(self.collapsed)
//!     .on_toggle(cx.listener(|this, _ev, _window, cx| {
//!         this.collapsed = !this.collapsed;
//!         cx.notify();
//!     }))
//!     .footer(Text::new("Updated 5 minutes ago").dimmed())
//!     .child(Text::new("Everything on track."))
//! ```

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    div, px, AnyElement, App, ClickEvent, ElementId, FontWeight, IntoElement, SharedString, Window,
};

use crate::actionicon::ActionIcon;
use crate::icon::IconName;
use crate::paper::apply_shadow;
use crate::theme::{theme, Size};

type ToggleHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// A titled surface with header/footer chrome. Reads like [`Card`](crate::Card)
/// but framed: a header (chevron + icon + title/description on the left,
/// actions on the right), the body, and an optional footer. The header gets a
/// bottom divider whenever the body is visible.
///
/// Collapsing is controlled, like `Modal`: the parent owns the flag, passes it
/// through [`collapsed`](Panel::collapsed), and flips it in
/// [`on_toggle`](Panel::on_toggle) (wired to the header chevron).
#[derive(IntoElement)]
pub struct Panel {
    id: Option<ElementId>,
    title: Option<SharedString>,
    description: Option<SharedString>,
    icon: Option<AnyElement>,
    actions: Vec<AnyElement>,
    footer: Option<AnyElement>,
    children: Vec<AnyElement>,
    padding: Size,
    radius: Option<Size>,
    with_border: bool,
    shadow: Option<Size>,
    collapsible: bool,
    collapsed: bool,
    on_toggle: Option<ToggleHandler>,
}

impl Panel {
    pub fn new() -> Self {
        Panel {
            id: None,
            title: None,
            description: None,
            icon: None,
            actions: Vec::new(),
            footer: None,
            children: Vec::new(),
            padding: Size::Lg,
            radius: Some(Size::Md),
            with_border: true,
            shadow: Some(Size::Sm),
            collapsible: false,
            collapsed: false,
            on_toggle: None,
        }
    }

    /// Scope the panel's internal element ids (the collapse chevron). Set one
    /// when several collapsible panels are siblings.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// The header title.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Dimmed secondary line under the title.
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Leading header content (e.g. a `ThemeIcon`), shown before the title.
    pub fn icon(mut self, icon: impl IntoElement) -> Self {
        self.icon = Some(icon.into_any_element());
        self
    }

    /// Append one trailing header action (e.g. an `ActionIcon`).
    pub fn action(mut self, action: impl IntoElement) -> Self {
        self.actions.push(action.into_any_element());
        self
    }

    /// Replace the trailing header actions.
    pub fn actions(mut self, actions: Vec<AnyElement>) -> Self {
        self.actions = actions;
        self
    }

    /// Footer content, rendered under the body behind a top divider.
    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_any_element());
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

    pub fn with_border(mut self, with_border: bool) -> Self {
        self.with_border = with_border;
        self
    }

    pub fn shadow(mut self, shadow: Size) -> Self {
        self.shadow = Some(shadow);
        self
    }

    /// Show a collapse chevron in the header. Pair with
    /// [`collapsed`](Panel::collapsed) + [`on_toggle`](Panel::on_toggle).
    pub fn collapsible(mut self) -> Self {
        self.collapsible = true;
        self
    }

    /// Whether the body (and footer) are hidden. Controlled by the parent.
    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    /// Called when the collapse chevron is clicked. Wire it with
    /// `cx.listener(...)` to flip the parent's `collapsed` flag.
    pub fn on_toggle(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self
    }
}

impl Default for Panel {
    fn default() -> Self {
        Panel::new()
    }
}

impl ParentElement for Panel {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Panel {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = theme(cx);
        let radius = t.radius(self.radius.unwrap_or(Size::Md));
        let pad = t.spacing(self.padding);
        let border = t.border().hsla();
        let text = t.text().hsla();
        let dimmed = t.dimmed().hsla();
        let font_md = t.font_size(Size::Md);
        let font_sm = t.font_size(Size::Sm);
        let surface = t.surface().hsla();

        let body_visible = !(self.collapsible && self.collapsed);
        let has_header = self.title.is_some()
            || self.description.is_some()
            || self.icon.is_some()
            || !self.actions.is_empty()
            || self.collapsible;
        let has_body = !self.children.is_empty();

        let mut root = div().flex().flex_col().bg(surface).rounded(px(radius));
        if self.with_border {
            root = root.border_1().border_color(border);
        }
        root = apply_shadow(root, self.shadow);

        if has_header {
            let mut left = div().flex().items_center().gap(px(10.0));

            if self.collapsible {
                let chevron = if self.collapsed {
                    IconName::ChevronRight
                } else {
                    IconName::ChevronDown
                };
                let mut toggle = ActionIcon::new(
                    "guise-panel-toggle",
                    SharedString::new_static(chevron.glyph()),
                )
                .size(Size::Sm);
                if let Some(handler) = self.on_toggle.clone() {
                    toggle = toggle.on_click(move |ev, window, cx| handler(ev, window, cx));
                }
                left = left.child(toggle);
            }
            if let Some(icon) = self.icon {
                left = left.child(icon);
            }

            let mut heading = div().flex().flex_col().gap(px(2.0));
            if let Some(title) = self.title {
                heading = heading.child(
                    div()
                        .text_size(px(font_md))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(text)
                        .child(title),
                );
            }
            if let Some(description) = self.description {
                heading = heading.child(
                    div()
                        .text_size(px(font_sm))
                        .text_color(dimmed)
                        .child(description),
                );
            }
            left = left.child(heading);

            let mut header = div()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(pad))
                .px(px(pad))
                .py(px(pad * 0.75));
            if body_visible && (has_body || self.footer.is_some()) {
                header = header.border_b_1().border_color(border);
            }
            header = header.child(left);
            if !self.actions.is_empty() {
                header = header.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .children(self.actions),
                );
            }
            root = root.child(header);
        }

        if body_visible {
            if has_body {
                root = root.child(div().flex().flex_col().p(px(pad)).children(self.children));
            }
            if let Some(footer) = self.footer {
                let mut foot = div().px(px(pad)).py(px(pad * 0.75));
                if has_body {
                    foot = foot.border_t_1().border_color(border);
                }
                root = root.child(foot.child(footer));
            }
        }

        match self.id {
            Some(id) => root.id(id).into_any_element(),
            None => root.into_any_element(),
        }
    }
}
