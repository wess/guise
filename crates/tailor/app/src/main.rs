//! Tailor — a visual interface builder for gpui and guise.
//!
//! `main` installs the chrome theme, wires the menu bar, and opens the root
//! window. Everything else lives in `editor/`; the document model, the
//! generator, and the file layer are their own crates, none of which know that
//! gpui exists except the renderer.

#[cfg(test)]
mod apptests;

mod editor;
mod root;
mod settings;
mod start;
mod templates;
mod theme;
mod toasts;

use gpui::prelude::*;
use gpui::{
  px, size, App, Application, Bounds, KeyBinding, Menu, MenuItem, OsAction, SharedString,
  TitlebarOptions, WindowBounds, WindowOptions,
};
use guise::prelude::*;

/// Declare a batch of no-payload actions. Every menu item and shortcut in
/// Tailor is one of these, dispatched at the workbench.
macro_rules! actions {
    ($($name:ident),* $(,)?) => {
        $(
            #[derive(Clone, PartialEq, Default, Debug, gpui::Action)]
            #[action(namespace = tailor, no_json)]
            pub struct $name;
        )*
    };
}

actions!(
  // Application
  Quit,
  Hide,
  HideOthers,
  ShowAll,
  ShowDocs,
  OpenSettings,
  // File
  NewProject,
  OpenProject,
  Save,
  SaveAs,
  NewScreen,
  NewComponent,
  ExportCode,
  CloseProject,
  // Edit
  Undo,
  Redo,
  Cut,
  Copy,
  Paste,
  Duplicate,
  Delete,
  SelectAll,
  SelectParent,
  Rename,
  // Editor
  EmbedFrame,
  EmbedStack,
  EmbedCard,
  EmbedScroll,
  Unwrap,
  MoveUp,
  MoveDown,
  AlignLeft,
  AlignCenterH,
  AlignRight,
  AlignTop,
  AlignMiddle,
  AlignBottom,
  DistributeH,
  DistributeV,
  // View
  ModeDesign,
  ModeBlueprint,
  ModeSplit,
  ModePreview,
  TogglePalette,
  ToggleOutline,
  ToggleInspector,
  ToggleProblems,
  ToggleGrid,
  ToggleSnap,
  ToggleSnapObjects,
  ToggleFreeForm,
  ToggleSelectionLayout,
  ToggleOutlines,
  // Arrow keys. Eight actions rather than one with a payload: `no_json`
  // actions carry nothing, and eight names read fine in a menu.
  NudgeLeft,
  NudgeRight,
  NudgeUp,
  NudgeDown,
  NudgeLeftBig,
  NudgeRightBig,
  NudgeUpBig,
  NudgeDownBig,
  ToggleOrientation,
  OpenLiveWindow,
  ToggleDevTools,
  OpenInEditor,
  InstallEditorTask,
);

fn menu(name: &'static str, items: Vec<MenuItem>) -> Menu {
  Menu {
    name: SharedString::new_static(name),
    items,
  }
}

/// The menu bar, grouped the way an interface builder's is: what you have open,
/// what you are editing, what the canvas is doing.
fn menus() -> Vec<Menu> {
  vec![
    menu(
      "Tailor",
      vec![
        MenuItem::action("Settings…", OpenSettings),
        MenuItem::separator(),
        MenuItem::action("Hide Tailor", Hide),
        MenuItem::action("Hide Others", HideOthers),
        MenuItem::action("Show All", ShowAll),
        MenuItem::separator(),
        MenuItem::action("Quit Tailor", Quit),
      ],
    ),
    menu(
      "File",
      vec![
        MenuItem::action("New Project", NewProject),
        MenuItem::action("Open…", OpenProject),
        MenuItem::separator(),
        MenuItem::action("New Screen", NewScreen),
        MenuItem::action("New Component", NewComponent),
        MenuItem::separator(),
        MenuItem::action("Save", Save),
        MenuItem::action("Save As…", SaveAs),
        MenuItem::separator(),
        MenuItem::action("Export Code…", ExportCode),
        MenuItem::separator(),
        MenuItem::action("Close Project", CloseProject),
      ],
    ),
    menu(
      "Edit",
      vec![
        MenuItem::action("Undo", Undo),
        MenuItem::action("Redo", Redo),
        MenuItem::separator(),
        MenuItem::os_action("Cut", Cut, OsAction::Cut),
        MenuItem::os_action("Copy", Copy, OsAction::Copy),
        MenuItem::os_action("Paste", Paste, OsAction::Paste),
        MenuItem::separator(),
        MenuItem::action("Duplicate", Duplicate),
        MenuItem::action("Delete", Delete),
        MenuItem::separator(),
        MenuItem::os_action("Select All", SelectAll, OsAction::SelectAll),
        MenuItem::action("Select Parent", SelectParent),
        MenuItem::action("Rename…", Rename),
      ],
    ),
    menu(
      "Arrange",
      vec![
        MenuItem::action("Embed in Frame", EmbedFrame),
        MenuItem::action("Embed in Stack", EmbedStack),
        MenuItem::action("Embed in Card", EmbedCard),
        MenuItem::action("Embed in Scroll Area", EmbedScroll),
        MenuItem::action("Unwrap", Unwrap),
        MenuItem::separator(),
        MenuItem::action("Move Up", MoveUp),
        MenuItem::action("Move Down", MoveDown),
        MenuItem::separator(),
        MenuItem::action("Align Left", AlignLeft),
        MenuItem::action("Align Center", AlignCenterH),
        MenuItem::action("Align Right", AlignRight),
        MenuItem::action("Align Top", AlignTop),
        MenuItem::action("Align Middle", AlignMiddle),
        MenuItem::action("Align Bottom", AlignBottom),
        MenuItem::separator(),
        MenuItem::action("Distribute Horizontally", DistributeH),
        MenuItem::action("Distribute Vertically", DistributeV),
        MenuItem::separator(),
        MenuItem::action("Flow / Free Form", ToggleSelectionLayout),
        MenuItem::action("Snap to Grid", ToggleSnap),
        MenuItem::action("Snap to Objects", ToggleSnapObjects),
        MenuItem::action("New Frames Are Free Form", ToggleFreeForm),
      ],
    ),
    menu(
      "View",
      vec![
        MenuItem::action("Design", ModeDesign),
        MenuItem::action("Blueprint", ModeBlueprint),
        MenuItem::action("Split", ModeSplit),
        MenuItem::action("Preview", ModePreview),
        MenuItem::separator(),
        MenuItem::action("Open Live Window", OpenLiveWindow),
        MenuItem::action("Developer Tools", ToggleDevTools),
        MenuItem::separator(),
        MenuItem::action("Open in Editor", OpenInEditor),
        MenuItem::action("Set Up Editor Jump…", InstallEditorTask),
        MenuItem::separator(),
        MenuItem::action("Library", TogglePalette),
        MenuItem::action("Outline", ToggleOutline),
        MenuItem::action("Inspector", ToggleInspector),
        MenuItem::action("Problems", ToggleProblems),
        MenuItem::separator(),
        MenuItem::action("Show Grid", ToggleGrid),
        MenuItem::action("Show Layout Bounds", ToggleOutlines),
        MenuItem::action("Rotate Device", ToggleOrientation),
      ],
    ),
    menu("Help", vec![MenuItem::action("Documentation", ShowDocs)]),
  ]
}

fn keys() -> Vec<KeyBinding> {
  vec![
    KeyBinding::new("cmd-n", NewProject, None),
    KeyBinding::new("cmd-o", OpenProject, None),
    KeyBinding::new("cmd-s", Save, None),
    KeyBinding::new("cmd-shift-s", SaveAs, None),
    KeyBinding::new("cmd-e", ExportCode, None),
    KeyBinding::new("cmd-q", Quit, None),
    KeyBinding::new("cmd-h", Hide, None),
    KeyBinding::new("alt-cmd-h", HideOthers, None),
    KeyBinding::new("cmd-,", OpenSettings, None),
    KeyBinding::new("cmd-z", Undo, None),
    KeyBinding::new("cmd-shift-z", Redo, None),
    KeyBinding::new("cmd-d", Duplicate, None),
    KeyBinding::new("backspace", Delete, Some("canvas")),
    KeyBinding::new("delete", Delete, Some("canvas")),
    KeyBinding::new("cmd-a", SelectAll, Some("canvas")),
    KeyBinding::new("escape", SelectParent, Some("canvas")),
    KeyBinding::new("enter", Rename, Some("canvas")),
    KeyBinding::new("cmd-shift-e", EmbedFrame, None),
    KeyBinding::new("cmd-shift-u", Unwrap, None),
    KeyBinding::new("cmd-1", ModeDesign, None),
    KeyBinding::new("cmd-2", ModeBlueprint, None),
    KeyBinding::new("cmd-3", ModeSplit, None),
    KeyBinding::new("cmd-4", ModePreview, None),
    KeyBinding::new("cmd-shift-l", OpenLiveWindow, None),
    KeyBinding::new("alt-cmd-i", ToggleDevTools, None),
    KeyBinding::new("alt-cmd-o", OpenInEditor, None),
    KeyBinding::new("cmd-alt-1", TogglePalette, None),
    KeyBinding::new("cmd-alt-2", ToggleOutline, None),
    KeyBinding::new("cmd-alt-3", ToggleInspector, None),
    KeyBinding::new("cmd-alt-4", ToggleProblems, None),
    KeyBinding::new("cmd-'", ToggleGrid, None),
    KeyBinding::new("cmd-shift-'", ToggleSnap, None),
    KeyBinding::new("cmd-shift-b", ToggleOutlines, None),
    KeyBinding::new("cmd-shift-g", ToggleSelectionLayout, None),
    KeyBinding::new("left", NudgeLeft, Some("canvas")),
    KeyBinding::new("right", NudgeRight, Some("canvas")),
    KeyBinding::new("up", NudgeUp, Some("canvas")),
    KeyBinding::new("down", NudgeDown, Some("canvas")),
    KeyBinding::new("shift-left", NudgeLeftBig, Some("canvas")),
    KeyBinding::new("shift-right", NudgeRightBig, Some("canvas")),
    KeyBinding::new("shift-up", NudgeUpBig, Some("canvas")),
    KeyBinding::new("shift-down", NudgeDownBig, Some("canvas")),
  ]
}

/// Split `path`, `path:line` or `path:line:column` — the shape every editor's
/// CLI uses — into a path and a row. Peels at most two trailing numeric
/// segments, so a directory called `12` in the path is safe.
fn split_target(target: &str) -> (String, usize) {
  let mut file = target;
  let mut line = 1usize;
  for _ in 0..2 {
    let Some((head, tail)) = file.rsplit_once(':') else {
      break;
    };
    let Ok(number) = tail.parse::<usize>() else {
      break;
    };
    // With both a row and a column, the row is peeled second and wins.
    line = number;
    file = head;
  }
  (file.to_string(), line)
}

/// Resolve `<file>:<line>` to a node and leave a request an open window picks
/// up. Editors hand a path and a row; everything else is looked up here.
fn reveal(target: &str) -> Result<String, String> {
  let (file, line) = split_target(target);

  let path = std::fs::canonicalize(&file).unwrap_or_else(|_| std::path::PathBuf::from(&file));
  let index = tailor_store::ExportIndex::load();
  let project_path = index.project_for(&path).ok_or_else(|| {
    format!(
      "no Tailor project has exported {}. Export one first — that is what \
             records where its files go.",
      path.display()
    )
  })?;
  let project = tailor_store::open(&project_path)
    .map_err(|err| format!("{}: {err}", project_path.display()))?;

  // Which document is this file? The exporter names files after the document,
  // so the stem is the answer.
  let stem = path
    .file_stem()
    .map(|stem| stem.to_string_lossy().to_string())
    .unwrap_or_default();
  let doc = project
    .docs
    .iter()
    .find(|doc| tailor_model::snake_case(&doc.name) == stem)
    .ok_or_else(|| format!("no document in {} generates {stem}.rs", project.name))?;

  // The node whose expression starts at or above the cursor — the innermost
  // thing that line belongs to.
  let generated = tailor_codegen::document(&project, doc);
  let node = generated
    .lines
    .iter()
    .filter(|(_, at)| **at <= line)
    .max_by_key(|(_, at)| **at)
    .map(|(id, _)| *id)
    .ok_or_else(|| format!("nothing on or above line {line} came from a component"))?;

  tailor_store::Focus::write(&project_path, &doc.id, node.0)?;
  Ok(format!(
    "{} · {} · node {} — selecting it in Tailor",
    project.name, doc.name, node.0
  ))
}

fn main() {
  let args: Vec<String> = std::env::args().skip(1).collect();

  // `tailordev --reveal src/ui/people.rs:106` goes the other way from *Open in
  // Zed*: it asks an open window to select whatever made that line. This is
  // what an editor binds a key to, so the pair behaves like a designer docked
  // to an editor rather than two apps sharing a folder.
  if args.first().map(|arg| arg == "--reveal").unwrap_or(false) {
    match args.get(1) {
      Some(target) => match reveal(target) {
        Ok(message) => println!("{message}"),
        Err(err) => {
          eprintln!("{err}");
          std::process::exit(1);
        }
      },
      None => eprintln!("usage: tailordev --reveal <file.rs>[:<line>]"),
    }
    return;
  }

  // `tailor --template dashboard out.tailor` writes a project and exits, so a
  // script can scaffold one without opening a window.
  if args.first().map(|arg| arg == "--template").unwrap_or(false) {
    if let (Some(name), Some(path)) = (args.get(1), args.get(2)) {
      match templates::TEMPLATES
        .iter()
        .find(|t| t.name.eq_ignore_ascii_case(name))
      {
        Some(template) => {
          let path = tailor_store::with_extension(std::path::PathBuf::from(path));
          match tailor_store::save(&path, &(template.build)()) {
            Ok(()) => println!("wrote {}", path.display()),
            Err(err) => eprintln!("could not write {}: {err}", path.display()),
          }
        }
        None => {
          let names: Vec<&str> = templates::TEMPLATES.iter().map(|t| t.name).collect();
          eprintln!("no template called {name}; try one of {}", names.join(", "));
        }
      }
    } else {
      eprintln!("usage: tailor --template <name> <path>");
    }
    return;
  }

  // `tailor --export project.tailor out/` generates without opening a window,
  // which is what a build script or a CI check wants.
  if args.first().map(|arg| arg == "--export").unwrap_or(false) {
    match (args.get(1), args.get(2)) {
      (Some(project), Some(out)) => match tailor_store::open(std::path::Path::new(project)) {
        Ok(project) => {
          let report = tailor_store::export(std::path::Path::new(out), &project);
          for (path, err) in &report.failed {
            eprintln!("{}: {err}", path.display());
          }
          for note in &report.notes {
            eprintln!("note: {note}");
          }
          println!("{}", report.summary());
          if !report.ok() {
            std::process::exit(1);
          }
        }
        Err(err) => {
          eprintln!("could not open {project}: {err}");
          std::process::exit(1);
        }
      },
      _ => eprintln!("usage: tailor --export <project.tailor> <directory>"),
    }
    return;
  }

  // `tailor path/to/project.tailor` opens that project instead of the start
  // screen — what a double-click in Finder and a shell alias both need.
  let opened = args.into_iter().next().map(std::path::PathBuf::from);

  Application::new().run(move |cx: &mut App| {
    let settings = tailor_store::Settings::load().sanitized();
    theme::install_manager(&settings, cx);

    cx.bind_keys(keys());
    cx.set_menus(menus());
    cx.on_action::<Quit>(|_, cx| cx.quit());
    cx.on_action::<Hide>(|_, cx| cx.hide());
    cx.on_action::<HideOthers>(|_, cx| cx.hide_other_apps());
    cx.on_action::<ShowAll>(|_, cx| cx.unhide_other_apps());
    cx.on_action::<ShowDocs>(|_, cx| cx.open_url("https://github.com/wess/guise"));

    let bounds = Bounds::centered(None, size(px(1440.0), px(900.0)), cx);
    cx.open_window(
      WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        window_min_size: Some(size(px(1080.0), px(680.0))),
        titlebar: Some(TitlebarOptions {
          title: Some(format!("Tailor {}", env!("CARGO_PKG_VERSION")).into()),
          ..Default::default()
        }),
        ..Default::default()
      },
      |window, cx| {
        // So `follow_system` tracks the OS rather than sampling it once.
        theme::watch(window, cx);
        cx.new(|cx| root::Root::new(settings, opened, cx))
      },
    )
    .unwrap();
    cx.activate(true);
  });
}

#[cfg(test)]
mod reveal_tests {
  use super::split_target;

  #[test]
  fn a_target_may_carry_a_row_a_column_or_neither() {
    assert_eq!(
      split_target("src/ui/people.rs:106:1"),
      ("src/ui/people.rs".to_string(), 106)
    );
    assert_eq!(
      split_target("src/ui/people.rs:106"),
      ("src/ui/people.rs".to_string(), 106)
    );
    assert_eq!(
      split_target("src/ui/people.rs"),
      ("src/ui/people.rs".to_string(), 1)
    );
    // Only trailing *numbers* are peeled, so a path is never eaten.
    assert_eq!(
      split_target("/work/12/people.rs"),
      ("/work/12/people.rs".to_string(), 1)
    );
  }
}
