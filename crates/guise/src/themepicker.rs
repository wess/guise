//! `ThemePicker` — the list of installed themes, and the click that wears one.
//!
//! Reads [`ThemeManager`] out of the global and writes the choice straight back
//! to it, so there is no state to hold and nothing to wire: drop it in a
//! settings page and it is done. Without a manager installed it draws nothing
//! rather than guessing at a theme list.
//!
//! Each row previews the theme it offers — the swatch is painted from *that*
//! theme's body, surface, border and primary, which is the only honest way to
//! show a theme you are not currently wearing.

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{div, px, App, ElementId, FontWeight, Hsla, IntoElement, SharedString, Window};

use crate::devtools::Probed;
use crate::icon::{Icon, IconName};
use crate::theme::{manager, theme, ColorScheme, Size, ThemeChoice, ThemeManager};

/// How the themes are laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemePickerLayout {
  /// One row per theme: swatch, name, tick. Fits a settings pane.
  #[default]
  List,
  /// Wrapping cards with a large swatch. Fits a theme gallery.
  Grid,
}

impl ThemePickerLayout {
  pub fn label(self) -> &'static str {
    match self {
      ThemePickerLayout::List => "list",
      ThemePickerLayout::Grid => "grid",
    }
  }
}

type ChoiceHandler = Rc<dyn Fn(&ThemeChoice, &mut Window, &mut App)>;

/// A picker over the installed [`ThemeManager`]'s registry.
///
/// ```ignore
/// ThemePicker::new().layout(ThemePickerLayout::Grid)
/// ```
#[derive(IntoElement)]
pub struct ThemePicker {
  layout: ThemePickerLayout,
  system_option: bool,
  on_change: Option<ChoiceHandler>,
}

impl Default for ThemePicker {
  fn default() -> Self {
    ThemePicker::new()
  }
}

impl ThemePicker {
  pub fn new() -> Self {
    ThemePicker {
      layout: ThemePickerLayout::default(),
      system_option: true,
      on_change: None,
    }
  }

  pub fn layout(mut self, layout: ThemePickerLayout) -> Self {
    self.layout = layout;
    self
  }

  /// Offer "System" as the first entry (default true). Turn it off in an app
  /// that has no business following the OS.
  pub fn system_option(mut self, system_option: bool) -> Self {
    self.system_option = system_option;
    self
  }

  /// Called after the manager has applied the new choice — for persisting it.
  pub fn on_change(
    mut self,
    handler: impl Fn(&ThemeChoice, &mut Window, &mut App) + 'static,
  ) -> Self {
    self.on_change = Some(Rc::new(handler));
    self
  }
}

/// The four colors a swatch is painted from, lifted out of a theme that isn't
/// the active one.
#[derive(Clone, Copy)]
struct Swatch {
  body: Hsla,
  surface: Hsla,
  border: Hsla,
  primary: Hsla,
}

impl Swatch {
  fn of(theme: &crate::theme::Theme) -> Self {
    Swatch {
      body: theme.body().hsla(),
      surface: theme.surface().hsla(),
      border: theme.border().hsla(),
      primary: theme.primary().hsla(),
    }
  }
}

/// One offer in the picker, resolved out of the manager before any listener
/// needs `cx` mutably.
struct Row {
  choice: ThemeChoice,
  name: SharedString,
  selected: bool,
  /// One swatch, or the light/dark pair for the System row.
  swatches: Vec<Swatch>,
}

/// A theme preview: the window in miniature — body ground, a surface bar, an
/// accent bar.
fn swatch(colors: Swatch, width: Option<f32>, height: f32, radius: f32) -> impl IntoElement {
  let mut el = div()
    .flex()
    .flex_col()
    .justify_end()
    .gap(px(3.0))
    .h(px(height))
    .p(px(4.0))
    .rounded(px(radius))
    .bg(colors.body)
    .child(
      div()
        .h(px(4.0))
        .w_full()
        .rounded(px(2.0))
        .bg(colors.surface),
    )
    .child(
      div()
        .h(px(4.0))
        .w(px(height * 0.6))
        .rounded(px(2.0))
        .bg(colors.primary),
    );
  el = match width {
    Some(width) => el.w(px(width)),
    None => el.w_full(),
  };
  el.border_1().border_color(colors.border)
}

/// The System row's swatch: the light and dark halves side by side, which reads
/// as "whichever the OS says" without a word of explanation.
fn split_swatch(
  colors: &[Swatch],
  width: Option<f32>,
  height: f32,
  radius: f32,
) -> impl IntoElement {
  let mut el = div().flex().gap(px(3.0)).h(px(height));
  el = match width {
    Some(width) => el.w(px(width)),
    None => el.w_full(),
  };
  for half in colors {
    el = el.child(div().flex_1().child(swatch(*half, None, height, radius)));
  }
  el
}

impl RenderOnce for ThemePicker {
  fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
    let t = theme(cx);
    let radius = t.radius(t.default_radius);
    let text = t.text().hsla();
    let dimmed = t.dimmed().hsla();
    let accent = t.primary().hsla();
    let surface = t.surface().hsla();
    let surface_hover = t.surface_hover().hsla();
    let border = t.border().hsla();
    let selected_bg = crate::style::ColorValue::Named(t.primary_color).soft(t);
    let gap = t.spacing(Size::Xs);

    let mut rows: Vec<Row> = Vec::new();
    if let Some(manager) = manager(cx) {
      let choice = manager.selection().clone();
      if self.system_option {
        let light = manager
          .entries()
          .iter()
          .find(|e| e.scheme() == ColorScheme::Light);
        let dark = manager
          .entries()
          .iter()
          .find(|e| e.scheme() == ColorScheme::Dark);
        rows.push(Row {
          selected: choice == ThemeChoice::System,
          choice: ThemeChoice::System,
          name: SharedString::new_static("System"),
          swatches: [light, dark]
            .into_iter()
            .flatten()
            .map(|e| Swatch::of(&e.theme))
            .collect(),
        });
      }
      for entry in manager.entries() {
        rows.push(Row {
          selected: choice == ThemeChoice::Fixed(entry.id.clone()),
          choice: ThemeChoice::Fixed(entry.id.clone()),
          name: entry.name.clone(),
          swatches: vec![Swatch::of(&entry.theme)],
        });
      }
    }

    let grid = self.layout == ThemePickerLayout::Grid;
    let mut list = div().flex().gap(px(gap));
    list = if grid {
      list.flex_wrap()
    } else {
      list.flex_col()
    };

    for row in rows {
      let handler = self.on_change.clone();
      let choice = row.choice.clone();
      let id: ElementId = SharedString::from(format!("theme-picker-{choice}")).into();
      let preview = if row.swatches.len() > 1 {
        split_swatch(
          &row.swatches,
          (!grid).then_some(46.0),
          if grid { 54.0 } else { 28.0 },
          radius,
        )
        .into_any_element()
      } else {
        let colors = row.swatches.first().copied().unwrap_or(Swatch {
          body: surface,
          surface,
          border,
          primary: accent,
        });
        swatch(
          colors,
          (!grid).then_some(46.0),
          if grid { 54.0 } else { 28.0 },
          radius,
        )
        .into_any_element()
      };

      let mut item = div()
        .id(id)
        .cursor_pointer()
        .rounded(px(radius))
        .text_color(text)
        .on_click(move |_, window, cx| {
          ThemeManager::set_choice(cx, choice.clone());
          if let Some(handler) = &handler {
            handler(&choice, window, cx);
          }
        });

      item = if grid {
        item
          .flex()
          .flex_col()
          .gap(px(6.0))
          .w(px(132.0))
          .p(px(8.0))
          .border_1()
          .border_color(if row.selected { accent } else { border })
          .bg(if row.selected { selected_bg } else { surface })
          .child(preview)
          .child(
            div()
              .text_size(px(12.0))
              .font_weight(if row.selected {
                FontWeight::SEMIBOLD
              } else {
                FontWeight::NORMAL
              })
              .child(row.name),
          )
      } else {
        item
          .flex()
          .items_center()
          .gap(px(10.0))
          .h(px(40.0))
          .px(px(8.0))
          .when(row.selected, |el| el.bg(selected_bg))
          .when(!row.selected, |el| el.hover(move |s| s.bg(surface_hover)))
          .child(preview)
          .child(div().flex_1().text_size(px(14.0)).child(row.name))
          .child(
            div()
              .text_color(if row.selected { accent } else { dimmed })
              .when(row.selected, |el| {
                el.child(Icon::new(IconName::Check).size(Size::Sm))
              }),
          )
      };
      list = list.child(item);
    }

    list
      .probe("ThemePicker")
      .attr("layout", self.layout.label())
      .attr_if("system", self.system_option)
  }
}
