//! Tailor's preferences, built on guise's own settings chrome.
//!
//! `SettingsView` supplies the shell — the page list, the search field, the
//! footer — and `SettingsSection` / `SettingsRow` the rows inside it. Using the
//! library's own settings components here is not just tidiness: it is the
//! largest single screen Tailor draws that is not the canvas, and building it
//! out of anything else would mean guise had a settings screen its own builder
//! did not trust.
//!
//! Every control reaches the workbench through a weak handle. `SettingsView`
//! rebuilds its content each frame from a `'static` closure, so there is
//! nothing to borrow and the rows always show live values.

use std::rc::Rc;

use gpui::prelude::*;
use gpui::{div, px, AnyElement, App, ClickEvent, Context, ElementId, SharedString, WeakEntity};
use guise::prelude::*;
use tailor_model::{Flavor, Scheme};
use tailor_store::{CanvasMode, Panel, Settings};

use crate::editor::Workbench;
use crate::theme;

/// The pages, in the order the sidebar lists them.
const GENERAL: &str = "general";
const CANVAS: &str = "canvas";
const PANELS: &str = "panels";
const ABOUT: &str = "about";

impl Workbench {
  /// Open the preferences sheet, or close it if it is already up.
  pub fn toggle_settings(&mut self, cx: &mut Context<Self>) {
    if self.settings_sheet.take().is_some() {
      cx.notify();
      return;
    }
    let weak = cx.entity().downgrade();
    let view = cx.new(|cx| {
      SettingsView::new(cx)
        .page_icon(GENERAL, "General", IconName::Settings)
        .page_icon(CANVAS, "Canvas", IconName::LayoutDashboard)
        .page_icon(PANELS, "Panels", IconName::PanelLeft)
        .page_icon(ABOUT, "About", IconName::Info)
        .searchable(true)
        .sidebar_width(170.0)
        .content({
          let weak = weak.clone();
          move |page, query, _window, cx| page_content(&weak, page, query, cx)
        })
        .footer(move |_window, cx| footer(&weak, cx))
    });
    self.settings_sheet = Some(view);
    cx.notify();
  }

  /// The sheet itself, floated over the workbench.
  pub(crate) fn render_settings_sheet(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
    let view = self.settings_sheet.clone()?;
    let chrome = theme::colors(cx);
    Some(
      div()
        .id("settings-scrim")
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .bg(gpui::black().opacity(0.45))
        .occlude()
        // Clicking the scrim closes, the way every sheet does.
        .on_click(cx.listener(|this, _, _window, cx| {
          this.settings_sheet = None;
          cx.notify();
        }))
        .child(
          div()
            .id("settings-sheet")
            .w(px(780.))
            .h(px(520.))
            .rounded(px(10.))
            .overflow_hidden()
            .border(px(1.))
            .border_color(chrome.border)
            .bg(chrome.body)
            .shadow_xl()
            // The sheet is not the scrim; a click inside it stays in.
            .on_click(|_, _window, cx| cx.stop_propagation())
            .child(view),
        )
        .into_any_element(),
    )
  }
}

/// Read the workbench's settings, or the defaults if it has gone away.
fn settings_of(weak: &WeakEntity<Workbench>, cx: &App) -> Settings {
  weak
    .upgrade()
    .map(|wb| wb.read(cx).settings.clone())
    .unwrap_or_default()
}

/// Change one setting and persist it.
fn edit(weak: &WeakEntity<Workbench>, cx: &mut App, change: impl FnOnce(&mut Settings)) {
  weak
    .update(cx, |workbench, cx| {
      change(&mut workbench.settings);
      workbench.save_settings();
      cx.notify();
    })
    .ok();
}

fn page_content(weak: &WeakEntity<Workbench>, page: &str, query: &str, cx: &mut App) -> AnyElement {
  let settings = settings_of(weak, cx);
  let defaults = Settings::default();
  let rows: Vec<Row> = match page {
    CANVAS => canvas_rows(weak, &settings, &defaults),
    PANELS => panel_rows(weak, &settings, &defaults),
    ABOUT => return about(cx),
    _ => general_rows(weak, &settings, &defaults),
  };

  // The shell hands the query down rather than filtering itself, so that a
  // page can decide what "matches" means. Here it is the label and the note.
  let needle = query.trim().to_lowercase();
  let matching: Vec<&Row> = rows
    .iter()
    .filter(|row| {
      needle.is_empty()
        || row.label.to_lowercase().contains(&needle)
        || row.note.to_lowercase().contains(&needle)
    })
    .collect();

  if matching.is_empty() {
    let dimmed = theme(cx).dimmed().hsla();
    return div()
      .pt(px(12.))
      .text_size(px(12.))
      .text_color(dimmed)
      .child(SharedString::from(format!(
        "Nothing here matches \"{query}\"."
      )))
      .into_any_element();
  }

  let last = matching.len() - 1;
  let mut section = SettingsSection::new(title_of(page)).description(blurb_of(page));
  for (index, row) in matching.into_iter().enumerate() {
    section = section.child(row.build(index == last, cx));
  }
  div().pt(px(4.)).child(section).into_any_element()
}

fn title_of(page: &str) -> &'static str {
  match page {
    CANVAS => "Canvas",
    PANELS => "Panels",
    ABOUT => "About",
    _ => "General",
  }
}

fn blurb_of(page: &str) -> &'static str {
  match page {
    CANVAS => "How the artboard behaves while you are laying something out.",
    PANELS => "The shape of the window. Panels also resize by dragging their dividers.",
    ABOUT => "",
    _ => "How Tailor behaves, and what a new project starts as.",
  }
}

/// A row's control, rebuilt every frame so it shows the live value.
type Control = Box<dyn Fn(&mut App) -> AnyElement>;
/// What a row's reset button does.
type Reset = Rc<dyn Fn(&mut App)>;

/// One row, gathered before it is built so a page can be searched.
struct Row {
  id: &'static str,
  label: &'static str,
  note: &'static str,
  modified: bool,
  control: Control,
  reset: Option<Reset>,
}

impl Row {
  fn build(&self, last: bool, cx: &mut App) -> impl IntoElement {
    let mut row = SettingsRow::new(ElementId::Name(SharedString::from(self.id)), self.label)
      .description(self.note)
      .modified(self.modified)
      .divider(!last)
      .control((self.control)(cx));
    if let Some(reset) = self.reset.clone() {
      row = row.on_reset(move |_: &ClickEvent, _window, cx| reset(cx));
    }
    row
  }
}

/// The editor picker's options, from the table the store keeps. Both halves are
/// `&'static str` already, so this is a view of it rather than a copy.
const EDITOR_OPTIONS: &[(&str, &str)] = &[
  ("zed", "Zed"),
  ("vscode", "VS Code"),
  ("sublime", "Sublime"),
  ("intellij", "IntelliJ"),
  ("emacs", "Emacs"),
  ("vim", "Neovim"),
];

fn general_rows(weak: &WeakEntity<Workbench>, now: &Settings, base: &Settings) -> Vec<Row> {
  vec![
    toggle(
      "autosave",
      "Autosave",
      "Write the project to disk whenever it changes.",
      now.autosave,
      base.autosave,
      weak,
      |settings, value| settings.autosave = value,
    ),
    toggle(
      "live-devtools",
      "Inspect the live window",
      "Open the live window with guise's inspector already showing.",
      now.live_devtools,
      base.live_devtools,
      weak,
      |settings, value| settings.live_devtools = value,
    ),
    picker(
      "editor",
      "Jump to",
      "Which editor `Open in Editor` opens, and what `--reveal` pairs with.",
      EDITOR_OPTIONS,
      &now.editor,
      now.editor != base.editor,
      weak,
      |settings, value| settings.editor = value.to_string(),
      |settings| settings.editor = Settings::default().editor,
    ),
    picker(
      "flavour",
      "Generated code",
      "What new projects export as. A project keeps its own choice.",
      &[("plain", "plain"), ("macros", "macros")],
      now.flavor.label(),
      now.flavor != base.flavor,
      weak,
      |settings, value| {
        settings.flavor = if value == "macros" {
          Flavor::Macros
        } else {
          Flavor::Plain
        }
      },
      |settings| settings.flavor = Settings::default().flavor,
    ),
    picker(
      "scheme",
      "Start screen",
      "An open project uses its own theme; this is what you see before then.",
      &[("dark", "dark"), ("light", "light"), ("system", "system")],
      // `system` is a third choice over the same field pair, not a third
      // scheme: the scheme it falls back to is remembered while you follow.
      if now.follow_system {
        "system"
      } else {
        now.scheme.label()
      },
      now.scheme != base.scheme || now.follow_system != base.follow_system,
      weak,
      |settings, value| {
        settings.follow_system = value == "system";
        if value == "light" {
          settings.scheme = Scheme::Light;
        } else if value == "dark" {
          settings.scheme = Scheme::Dark;
        }
      },
      |settings| {
        let defaults = Settings::default();
        settings.scheme = defaults.scheme;
        settings.follow_system = defaults.follow_system;
      },
    ),
  ]
}

fn canvas_rows(weak: &WeakEntity<Workbench>, now: &Settings, base: &Settings) -> Vec<Row> {
  vec![
    picker(
      "mode",
      "Mode",
      "What the canvas shows. Also ⌘1 through ⌘4.",
      &[
        ("Design", "Design"),
        ("Blueprint", "Blueprint"),
        ("Split", "Split"),
        ("Preview", "Preview"),
      ],
      now.canvas_mode.label(),
      now.canvas_mode != base.canvas_mode,
      weak,
      |settings, value| {
        settings.canvas_mode = CanvasMode::ALL
          .iter()
          .copied()
          .find(|mode| mode.label() == value)
          .unwrap_or(CanvasMode::Design)
      },
      |settings| settings.canvas_mode = Settings::default().canvas_mode,
    ),
    toggle(
      "grid",
      "Show the grid",
      "Rules behind the artboard, at the spacing below.",
      now.show_grid,
      base.show_grid,
      weak,
      |settings, value| settings.show_grid = value,
    ),
    picker(
      "gridsize",
      "Grid",
      "What a drag snaps to, and what shift-arrow nudges by.",
      &[
        ("4", "4"),
        ("8", "8"),
        ("12", "12"),
        ("16", "16"),
        ("24", "24"),
      ],
      &format!("{:.0}", now.grid),
      now.grid != base.grid,
      weak,
      |settings, value| settings.grid = value.parse().unwrap_or(8.0),
      |settings| settings.grid = Settings::default().grid,
    ),
    toggle(
      "snap",
      "Snap to grid",
      "A free-form drag catches on the spacing above.",
      now.snap,
      base.snap,
      weak,
      |settings, value| settings.snap = value,
    ),
    toggle(
      "snapobjects",
      "Snap to objects",
      "Catch on siblings' edges and centres, and draw a guide where it caught.",
      now.snap_objects,
      base.snap_objects,
      weak,
      |settings, value| settings.snap_objects = value,
    ),
    picker(
      "nudge",
      "Nudge",
      "How far one arrow-key press moves. Shift-arrow uses the grid.",
      &[("1", "1"), ("2", "2"), ("4", "4"), ("8", "8")],
      &format!("{:.0}", now.nudge),
      now.nudge != base.nudge,
      weak,
      |settings, value| settings.nudge = value.parse().unwrap_or(1.0),
      |settings| settings.nudge = Settings::default().nudge,
    ),
    toggle(
      "freeform",
      "New frames are free form",
      "A new frame places its children at explicit x/y instead of in a flow. \
             Any frame can be switched either way in the Size inspector.",
      now.free_form,
      base.free_form,
      weak,
      |settings, value| settings.free_form = value,
    ),
    toggle(
      "bounds",
      "Show layout bounds",
      "Outline every node, not only the selected one. Also ⇧⌘B.",
      now.show_outlines,
      base.show_outlines,
      weak,
      |settings, value| settings.show_outlines = value,
    ),
  ]
}

fn panel_rows(weak: &WeakEntity<Workbench>, now: &Settings, base: &Settings) -> Vec<Row> {
  let sizes = Panel::ALL
    .iter()
    .map(|panel| format!("{} {:.0}", panel.label(), now.size(*panel)))
    .collect::<Vec<_>>()
    .join(" · ");
  let changed = Panel::ALL
    .iter()
    .any(|panel| now.size(*panel) != base.size(*panel));
  let for_reset = weak.clone();

  vec![
    Row {
      id: "sizes",
      label: "Sizes",
      note: "Drag the divider beside a panel to resize it.",
      modified: changed,
      control: {
        let sizes = sizes.clone();
        Box::new(move |cx: &mut App| {
          let dimmed = theme(cx).dimmed().hsla();
          div()
            .text_size(px(11.))
            .text_color(dimmed)
            .child(SharedString::from(sizes.clone()))
            .into_any_element()
        })
      },
      reset: None,
    },
    Row {
      id: "reset-panels",
      label: "Reset the layout",
      note: "Put every panel back to its opening size, and open them all.",
      modified: false,
      control: Box::new(move |_cx: &mut App| {
        let weak = for_reset.clone();
        Button::new("reset-panels", "Reset")
          .variant(Variant::Default)
          .size(Size::Sm)
          .on_click(move |_, _window, cx| {
            edit(&weak, cx, |settings| {
              let base = Settings::default();
              for panel in Panel::ALL {
                settings.set_size(*panel, base.size(*panel));
                // `Code` is Split mode wearing a panel's
                // clothes; resetting the layout should not
                // change what the canvas is showing.
                if *panel != Panel::Code {
                  settings.set_open(*panel, base.is_open(*panel));
                }
              }
            });
          })
          .into_any_element()
      }),
      reset: None,
    },
  ]
}

fn about(cx: &mut App) -> AnyElement {
  let dimmed = theme(cx).dimmed().hsla();
  let path = tailor_store::config_dir().join("settings.json");
  div()
    .pt(px(4.))
    .flex()
    .flex_col()
    .gap(px(10.))
    .child(Title::new("Tailor").order(3))
    .child(
      Text::new(format!("Version {}", env!("CARGO_PKG_VERSION")))
        .size(Size::Sm)
        .dimmed(),
    )
    .child(
      Text::new(
        "A visual interface builder for gpui and guise. It ships in the guise \
                       repository, and everything it draws is built from the same components \
                       it places.",
      )
      .size(Size::Sm),
    )
    .child(Divider::new())
    .child(
      div()
        .flex()
        .flex_col()
        .gap(px(3.))
        .child(Text::new("Settings file").size(Size::Xs).dimmed())
        .child(
          div()
            .text_size(px(11.))
            .text_color(dimmed)
            .font_family(theme::MONO)
            .child(SharedString::from(path.display().to_string())),
        ),
    )
    .child(
      div().pt(px(4.)).child(
        Button::new("open-docs", "Documentation")
          .variant(Variant::Default)
          .size(Size::Sm)
          .left_section(Icon::new(IconName::BookOpen).size(Size::Xs))
          .on_click(|_, _window, cx| {
            cx.open_url("https://github.com/wess/guise/blob/main/docs/tailor.md")
          }),
      ),
    )
    .into_any_element()
}

/// A row whose control is a switch.
fn toggle(
  id: &'static str,
  label: &'static str,
  note: &'static str,
  value: bool,
  default: bool,
  weak: &WeakEntity<Workbench>,
  apply: fn(&mut Settings, bool),
) -> Row {
  let for_control = weak.clone();
  let for_reset = weak.clone();
  Row {
    id,
    label,
    note,
    modified: value != default,
    control: Box::new(move |_cx: &mut App| {
      let weak = for_control.clone();
      Switch::new(ElementId::Name(SharedString::from(id)))
        .checked(value)
        .on_change(move |_, _window, cx| {
          edit(&weak, cx, |settings| apply(settings, !value));
        })
        .into_any_element()
    }),
    reset: Some(Rc::new(move |cx: &mut App| {
      edit(&for_reset, cx, |settings| apply(settings, default));
    })),
  }
}

/// A row whose control is a row of chips. Chips rather than a `Select`, because
/// a picker with four options should not cost a click to see them.
#[allow(clippy::too_many_arguments)]
fn picker(
  id: &'static str,
  label: &'static str,
  note: &'static str,
  options: &'static [(&'static str, &'static str)],
  selected: &str,
  modified: bool,
  weak: &WeakEntity<Workbench>,
  apply: fn(&mut Settings, &str),
  reset: fn(&mut Settings),
) -> Row {
  let selected = selected.to_string();
  let for_control = weak.clone();
  let for_reset = weak.clone();
  Row {
    id,
    label,
    note,
    modified,
    control: Box::new(move |cx: &mut App| {
      let chrome = theme::colors(cx);
      let selected = selected.clone();
      let weak = for_control.clone();
      div()
        .flex()
        .gap(px(3.))
        .children(options.iter().map(|(label, value)| {
          let active = *value == selected;
          let value = *value;
          let weak = weak.clone();
          div()
            .id(ElementId::Name(SharedString::from(format!("{id}-{value}"))))
            .px(px(8.))
            .py(px(3.))
            .rounded(px(5.))
            .text_size(px(11.))
            .when(active, |d| {
              d.bg(chrome.accent_soft).text_color(chrome.accent)
            })
            .when(!active, |d| d.bg(chrome.raised).text_color(chrome.dimmed))
            .child(SharedString::from(*label))
            .on_click(move |_, _window, cx| {
              edit(&weak, cx, |settings| apply(settings, value));
            })
        }))
        .into_any_element()
    }),
    reset: Some(Rc::new(move |cx: &mut App| {
      edit(&for_reset, cx, reset);
    })),
  }
}

/// A note about when changes land, and the way out.
fn footer(weak: &WeakEntity<Workbench>, cx: &mut App) -> impl IntoElement {
  let dimmed = theme(cx).dimmed().hsla();
  let weak = weak.clone();
  div()
    .flex()
    .items_center()
    .justify_between()
    .w_full()
    .child(
      div()
        .text_size(px(10.))
        .text_color(dimmed)
        .child("Changes apply as you make them."),
    )
    .child(
      Button::new("settings-done", "Done")
        .variant(Variant::Filled)
        .size(Size::Sm)
        .on_click(move |_, _window, cx| {
          weak
            .update(cx, |workbench, cx| {
              workbench.settings_sheet = None;
              cx.notify();
            })
            .ok();
        }),
    )
}
