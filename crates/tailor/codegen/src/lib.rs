//! Tailor's Rust generator.
//!
//! A `.tailor` document in, a guise component out — and the output is the point.
//! It is not a runtime format the app has to keep loading: it is a file you can
//! read, paste into a crate, and then own, with no dependency on Tailor left in
//! it. That is the difference between a mockup tool and a builder.
//!
//! The generator is deliberately pure. Every decision it makes comes from the
//! catalog in `tailor-model`, so the same table drives the palette, the canvas,
//! and this — and a component cannot render one way and generate another.

pub mod app;
pub mod expr;
pub mod file;
pub mod node;
pub mod rust;
pub mod style;

pub use file::{document, module, Generated};

use tailor_model::{Document, Project};

/// Every file an export writes, relative to the chosen directory.
pub fn project_files(project: &Project) -> Vec<Generated> {
  let module_dir = tailor_model::snake_case(&project.gen.module);
  let mut out = Vec::new();

  for doc in &project.docs {
    let mut file = file::document(project, doc);
    file.path = format!("src/{module_dir}/{}", file.path);
    out.push(file);
  }
  let mut module_file = file::module(project);
  module_file.path = format!("src/{module_dir}/mod.rs");
  out.push(module_file);

  if project.gen.emit_app {
    let mut main = app::main_rs(project);
    main.path = "src/main.rs".into();
    out.push(main);

    let mut theme = app::theme_rs(project);
    theme.path = "src/theme.rs".into();
    out.push(theme);

    // Beside `theme.rs`, because that is where its `include_str!` looks.
    if let Some(mut json) = app::theme_json(project) {
      json.path = "src/theme.json".into();
      out.push(json);
    }

    out.push(app::cargo_toml(project));
  }
  out
}

/// The single file for one document — what the code panel shows.
pub fn preview(project: &Project, doc: &Document) -> Generated {
  file::document(project, doc)
}

#[cfg(test)]
mod tests {
  use super::*;
  use tailor_model::motion::MotionProps;
  use tailor_model::node::DEFAULT_SLOT;
  use tailor_model::props::PropValue;
  use tailor_model::style::{Dimension, Edges, LayoutMode};
  use tailor_model::tokens::{EaseToken, EnterToken};
  use tailor_model::{ColorSpec, ColorToken, DocKind, Flavor};

  /// One of every kind the catalog offers, generated. This does not compile
  /// the output — nothing here can — but it does walk every arm in the
  /// emitter, which is where a new catalog entry with no generator support
  /// shows up as a panic or an empty expression.
  #[test]
  fn every_catalog_kind_generates_something() {
    for spec in tailor_model::catalog::all() {
      let mut project = Project::new("Demo");
      let doc = &mut project.docs[0];
      let root = doc.root;
      let node = doc.create(spec.kind);
      doc.insert(root, DEFAULT_SLOT, 0, node);

      let file = preview(&project, &project.docs[0]);
      assert!(
        !file.source.is_empty(),
        "{} generated nothing at all",
        spec.kind
      );
      // Anything built the ordinary way should name its own type. The
      // `Special` kinds are the ones the emitter writes by hand — a tooltip
      // is a `.tooltip(..)` call on a div, a frame is a bare div — so they
      // are covered by the assertions below instead.
      if spec.ctor != tailor_model::catalog::Ctor::Special && !spec.rust.is_empty() {
        assert!(
          file.source.contains(spec.rust),
          "{} generated no reference to {}",
          spec.kind,
          spec.rust
        );
      }
    }
  }

  #[test]
  fn the_hand_written_kinds_still_name_their_component() {
    for (kind, expected) in [
      ("virtuallist", "VirtualList::new("),
      ("aicost", "AICost::new(AIUsage::new("),
      ("aisources", "AISources::new(["),
    ] {
      let mut project = Project::new("Demo");
      let doc = &mut project.docs[0];
      let root = doc.root;
      let node = doc.create(kind);
      doc.insert(root, DEFAULT_SLOT, 0, node);
      let source = preview(&project, &project.docs[0]).source;
      assert!(source.contains(expected), "{kind} did not emit {expected}");
    }
  }

  #[test]
  fn the_settings_screen_declares_its_pages_and_fills_them_from_one_closure() {
    let mut project = Project::new("Demo");
    let doc = &mut project.docs[0];
    let root = doc.root;
    let mut view = doc.create("settingsview");
    view.set_prop(
      "pages",
      PropValue::Items(vec!["Appearance".into(), "Editor".into()]),
    );
    let view = doc.insert(root, DEFAULT_SLOT, 0, view);
    let mut title = doc.create("title");
    title.set_prop("content", PropValue::Text("Colours".into()));
    doc.insert(view, "page:0", 0, title);

    let source = preview(&project, &project.docs[0]).source;
    assert!(source.contains(".page(\"appearance\", \"Appearance\")"));
    assert!(source.contains(".page(\"editor\", \"Editor\")"));
    assert!(source.contains(".content("));
    assert!(source.contains("match page {"));
    assert!(source.contains("\"appearance\" =>"));
    // The last arm is the catch-all a `&str` match needs.
    assert!(source.contains("_ =>"));
    assert!(source.contains("Colours"));
  }

  #[test]
  fn a_menu_becomes_one_call_per_line() {
    let mut project = Project::new("Demo");
    let doc = &mut project.docs[0];
    let root = doc.root;
    let mut menu = doc.create("menu");
    menu.set_prop("trigger", PropValue::Text("File".into()));
    menu.set_prop(
      "items",
      PropValue::Items(vec![
        "# Recent".into(),
        "Open…".into(),
        "-".into(),
        "Quit".into(),
      ]),
    );
    doc.insert(root, DEFAULT_SLOT, 0, menu);

    let source = preview(&project, &project.docs[0]).source;
    assert!(source.contains("Menu::new(cx, \"File\")"));
    assert!(source.contains(".section(\"Recent\")"));
    assert!(source.contains(".item(\"Open…\", |_window, _cx| {})"));
    assert!(source.contains(".divider()"));
  }

  #[test]
  fn the_ai_components_carry_their_two_constructor_arguments() {
    let mut project = Project::new("Demo");
    let doc = &mut project.docs[0];
    let root = doc.root;
    let mut message = doc.create("aimessage");
    message.set_prop("body", PropValue::Text("Hello".into()));
    doc.insert(root, DEFAULT_SLOT, 0, message);
    let meter = doc.create("aitokenmeter");
    doc.insert(root, DEFAULT_SLOT, 1, meter);

    let source = preview(&project, &project.docs[0]).source;
    assert!(source.contains("AIMessage::new(AIRole::Assistant, \"Hello\")"));
    assert!(source.contains("AITokenMeter::new(24000, 200000)"));
  }

  /// A screen with a bit of everything: a styled frame, a stateless
  /// component, an entity field, a bound event, and a state variable.
  pub fn kitchen_sink() -> Project {
    let mut project = Project::new("Demo");
    let doc = &mut project.docs[0];
    doc.state.push(tailor_model::StateVar::new(
      "query",
      tailor_model::VarType::Text,
    ));
    doc.actions.push(tailor_model::ActionDef::new("submit"));
    let root = doc.root;
    doc.node_mut(root).unwrap().style.padding = Edges::all(24.0);
    doc.node_mut(root).unwrap().style.gap = Some(16.0);

    let mut title = doc.create("title");
    title.set_prop("content", PropValue::Text("Sign in".into()));
    doc.insert(root, DEFAULT_SLOT, 0, title);

    let mut field = doc.create("textinput");
    field.name = Some("Email".into());
    field.set_prop("placeholder", PropValue::Text("you@example.com".into()));
    doc.insert(root, DEFAULT_SLOT, 1, field);

    let mut button = doc.create("button");
    button.set_prop("label", PropValue::Text("Continue".into()));
    button.events.insert("click".into(), "submit".into());
    button.style.background = Some(ColorSpec::Named(ColorToken::Violet));
    button.style.width = Dimension::Full;
    doc.insert(root, DEFAULT_SLOT, 2, button);

    project
  }

  #[test]
  fn a_node_with_a_motion_animates_its_own_box() {
    let mut project = kitchen_sink();
    let doc = &mut project.docs[0];
    let title = doc.children_of(doc.root)[0];
    doc.node_mut(title).unwrap().motion = MotionProps {
      enter: Some(EnterToken::SlideUp),
      ease: EaseToken::OutBack,
      duration: 420.0,
      distance: 16.0,
      ..Default::default()
    };

    let source = preview(&project, &project.docs[0]).source;
    assert!(source.contains(".animate("), "{source}");
    assert!(source.contains(&format!("\"{}\"", title.element_id())));
    assert!(source.contains("Motion::enter_from(TransitionKind::SlideUp, 16.)"));
    assert!(source.contains(".duration(420.)"));
    assert!(source.contains(".ease(Easing::Out(Curve::Back))"));
    // No delay was set, so none is printed.
    assert!(!source.contains(".delay("));
  }

  #[test]
  fn a_stagger_prints_one_delay_per_child() {
    let mut project = kitchen_sink();
    let doc = &mut project.docs[0];
    let root = doc.root;
    doc.node_mut(root).unwrap().motion = MotionProps {
      enter: Some(EnterToken::Fade),
      stagger: 60.0,
      ..Default::default()
    };

    let source = preview(&project, &project.docs[0]).source;
    // Three children, one wave: 0 / 60 / 120. The container itself does
    // not animate.
    assert_eq!(
      source
        .matches("Motion::enter(TransitionKind::Fade)")
        .count(),
      3
    );
    assert!(source.contains(".delay(60.)"));
    assert!(source.contains(".delay(120.)"));
  }

  #[test]
  fn a_pinned_node_animates_its_margins_not_its_inset() {
    let mut project = kitchen_sink();
    let doc = &mut project.docs[0];
    let root = doc.root;
    // Free-form parent: its children are `absolute()` at an offset, and
    // an animated inset would drag one off that offset.
    doc.node_mut(root).unwrap().style.layout = LayoutMode::Absolute;
    let title = doc.children_of(root)[0];
    doc.node_mut(title).unwrap().motion = MotionProps {
      enter: Some(EnterToken::SlideUp),
      ..Default::default()
    };

    let source = preview(&project, &project.docs[0]).source;
    assert!(source.contains(".as_margins()"), "{source}");
  }

  #[test]
  fn the_macro_flavour_emits_a_motion_block() {
    let mut project = kitchen_sink();
    project.gen.flavor = Flavor::Macros;
    let doc = &mut project.docs[0];
    let title = doc.children_of(doc.root)[0];
    doc.node_mut(title).unwrap().motion = MotionProps {
      enter: Some(EnterToken::SlideUp),
      ease: EaseToken::InOutSine,
      duration: 400.0,
      delay: 60.0,
      distance: 16.0,
      repeat: tailor_model::tokens::LoopToken::Forever,
      alternate: true,
      ..Default::default()
    };

    let source = preview(&project, &project.docs[0]).source;
    assert!(source.contains("motion! {"), "{source}");
    assert!(source.contains("enter: slide_up 16.;"));
    assert!(source.contains("duration: 400.;"));
    assert!(source.contains("delay: 60.;"));
    assert!(source.contains("ease: in_out sine;"));
    assert!(source.contains("repeat: forever;"));
    assert!(source.contains("alternate;"));
    // The builder spelling is the other flavour's.
    assert!(!source.contains("Motion::enter_from"));
  }

  #[test]
  fn nothing_animates_unless_it_was_asked_to() {
    let project = kitchen_sink();
    let source = preview(&project, &project.docs[0]).source;
    assert!(!source.contains(".animate("));
    assert!(!source.contains("Motion::"));
  }

  #[test]
  fn an_icon_button_emits_its_label() {
    let mut project = kitchen_sink();
    let doc = &mut project.docs[0];
    let mut action = doc.create("actionicon");
    action.set_prop("icon", PropValue::Icon("pencil".into()));
    action.set_prop("label", PropValue::Text("Edit".into()));
    doc.insert(doc.root, DEFAULT_SLOT, 3, action);

    let source = preview(&project, &project.docs[0]).source;
    assert!(source.contains("ActionIcon::new("), "{source}");
    assert!(source.contains(".label(\"Edit\")"), "{source}");
  }

  #[test]
  fn the_kitchen_sink_generates_a_whole_component() {
    let project = kitchen_sink();
    let file = preview(&project, &project.docs[0]);
    let source = &file.source;

    assert!(source.contains("pub struct MainScreen {"));
    assert!(source.contains("email: Entity<TextInput>,"));
    assert!(source.contains("pub query: Signal<String>,"));
    assert!(source.contains("let email = cx.new(|cx| {"));
    assert!(source.contains("TextInput::new(cx)"));
    assert!(source.contains(".placeholder(\"you@example.com\")"));
    assert!(source.contains("pub fn submit(&mut self, cx: &mut Context<Self>)"));
    assert!(source.contains("cx.listener(|this, _event, _window, cx| this.submit(cx))"));
    assert!(source.contains(".child(self.email.clone())"));
    assert!(source.contains("let violet_6 = theme(cx).color(ColorName::Violet, 6).hsla();"));
    assert!(source.contains(".bg(violet_6)"));
    assert!(source.contains("Title::new(\"Sign in\")"));
  }

  #[test]
  fn a_field_never_collides_with_a_state_variable() {
    let mut project = Project::new("Demo");
    let doc = &mut project.docs[0];
    doc.state.push(tailor_model::StateVar::new(
      "email",
      tailor_model::VarType::Text,
    ));
    let root = doc.root;
    let mut field = doc.create("textinput");
    field.name = Some("Email".into());
    doc.insert(root, DEFAULT_SLOT, 0, field);

    let file = preview(&project, &project.docs[0]);
    assert!(file.source.contains("pub email_field: Entity<TextInput>,"));
    assert!(file.source.contains("pub email: Signal<String>,"));
    assert!(file.source.contains("let email_field = cx.new("));
    assert!(file.source.contains(".child(self.email_field.clone())"));
  }

  #[test]
  fn a_bound_prop_reads_the_signal() {
    let mut project = kitchen_sink();
    let doc = &mut project.docs[0];
    let text = doc.create("text");
    let id = doc.insert(doc.root, DEFAULT_SLOT, 0, text);
    doc
      .node_mut(id)
      .unwrap()
      .set_prop("content", PropValue::Binding("query".into()));

    let file = preview(&project, &project.docs[0]);
    assert!(file.source.contains("Text::new(self.query.get(cx))"));
  }

  /// A controlled builder binds in the chain. Reading the signal without
  /// writing back is not a binding — the switch would show the value and then
  /// refuse to change it.
  #[test]
  fn a_bound_controlled_builder_binds_both_ways() {
    let mut project = kitchen_sink();
    let doc = &mut project.docs[0];
    doc.state.push(tailor_model::StateVar::new(
      "ready",
      tailor_model::VarType::Bool,
    ));
    let switch = doc.create("switch");
    let id = doc.insert(doc.root, DEFAULT_SLOT, 0, switch);
    doc
      .node_mut(id)
      .unwrap()
      .set_prop("checked", PropValue::Binding("ready".into()));

    let file = preview(&project, &project.docs[0]);
    assert!(
      file.source.contains(".bind(self.ready.binding())"),
      "{}",
      file.source
    );
    assert!(
      !file.source.contains(".checked(self.ready"),
      "{}",
      file.source
    );
  }

  #[test]
  fn an_absolute_frame_pins_its_children() {
    let mut project = Project::new("Demo");
    let doc = &mut project.docs[0];
    let root = doc.root;
    doc.node_mut(root).unwrap().style.layout = LayoutMode::Absolute;
    let mut badge = doc.create("badge");
    badge.set_prop("label", PropValue::Text("New".into()));
    badge.style.x = 40.0;
    badge.style.y = 24.0;
    doc.insert(root, DEFAULT_SLOT, 0, badge);

    let file = preview(&project, &project.docs[0]);
    assert!(file.source.contains(".relative()"));
    assert!(file.source.contains(".absolute()"));
    assert!(file.source.contains(".left(px(40.))"));
    assert!(file.source.contains(".top(px(24.))"));
  }

  #[test]
  fn tabs_generate_a_closure_per_panel() {
    let mut project = Project::new("Demo");
    let doc = &mut project.docs[0];
    let root = doc.root;
    let tabs = doc.create("tabs");
    let tabs = doc.insert(root, DEFAULT_SLOT, 0, tabs);
    doc
      .node_mut(tabs)
      .unwrap()
      .set_prop("tabs", PropValue::Items(vec!["One".into(), "Two".into()]));
    let inner = doc.create("text");
    let inner_id = inner.id;
    doc.nodes.insert(inner_id, inner);
    doc.node_mut(tabs).unwrap().slot_mut("tab:0").push(inner_id);
    doc
      .node_mut(inner_id)
      .unwrap()
      .set_prop("content", PropValue::Text("Panel".into()));

    let file = preview(&project, &project.docs[0]);
    assert!(file.source.contains(".tab(\"One\", |_window, _cx| {"));
    assert!(file.source.contains(".tab(\"Two\", |_window, _cx| {"));
    assert!(file.source.contains("Text::new(\"Panel\")"));
  }

  #[test]
  fn a_field_a_closure_captures_is_built_before_the_thing_that_captures_it() {
    // Tabs is created before its panels' contents are, so a slider inside a
    // panel has to become a local first — otherwise the closure prologue
    // clones a name that does not exist yet.
    let mut project = Project::new("Demo");
    let doc = &mut project.docs[0];
    let root = doc.root;
    let tabs = doc.create("tabs");
    let tabs = doc.insert(root, DEFAULT_SLOT, 0, tabs);
    doc
      .node_mut(tabs)
      .unwrap()
      .set_prop("tabs", PropValue::Items(vec!["One".into()]));
    let slider = doc.create("slider");
    let slider_id = slider.id;
    doc.nodes.insert(slider_id, slider);
    doc.node_mut(slider_id).unwrap().name = Some("Font size".into());
    doc
      .node_mut(tabs)
      .unwrap()
      .slot_mut("tab:0")
      .push(slider_id);

    let file = preview(&project, &project.docs[0]);
    let source = &file.source;
    let slider_at = source
      .find("let font_size = cx.new(")
      .expect("the slider is built");
    let tabs_at = source.find("let tabs = ").expect("the tabs are built");
    assert!(
      slider_at < tabs_at,
      "the slider must exist before the tabs capture it:\n{source}"
    );
    assert!(source.contains("let font_size = font_size.clone();"));
  }

  #[test]
  fn a_field_inside_a_closure_is_cloned_in() {
    let mut project = Project::new("Demo");
    let doc = &mut project.docs[0];
    let root = doc.root;
    let shell = doc.create("appshell");
    let shell = doc.insert(root, DEFAULT_SLOT, 0, shell);
    let field = doc.create("textinput");
    let field_id = field.id;
    doc.nodes.insert(field_id, field);
    doc.node_mut(field_id).unwrap().name = Some("Search".into());
    doc
      .node_mut(shell)
      .unwrap()
      .slot_mut("navbar")
      .push(field_id);

    let file = preview(&project, &project.docs[0]);
    assert!(file.source.contains("let search = self.search.clone();"));
    assert!(file.source.contains("move |_window, _cx| {"));
    assert!(file.source.contains("search.clone()"));
  }

  #[test]
  fn the_macro_flavour_uses_a_style_block() {
    let mut project = kitchen_sink();
    project.gen.flavor = Flavor::Macros;
    let file = preview(&project, &project.docs[0]);
    assert!(file.source.contains(".apply(style! {"));
    assert!(file.source.contains("padding: 24.;"));
  }

  #[test]
  fn an_export_writes_a_runnable_crate() {
    let project = kitchen_sink();
    let files = project_files(&project);
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
    assert!(paths.contains(&"src/ui/main_screen.rs"));
    assert!(paths.contains(&"src/ui/mod.rs"));
    assert!(paths.contains(&"src/main.rs"));
    assert!(paths.contains(&"src/theme.rs"));
    assert!(paths.contains(&"Cargo.toml"));
  }

  #[test]
  fn a_placed_component_calls_its_constructor() {
    let mut project = Project::new("Demo");
    project.docs.push(tailor_model::Document::new(
      "card",
      "StatCard",
      DocKind::Component,
    ));
    let doc = &mut project.docs[0];
    let root = doc.root;
    let placed = doc.create("@StatCard");
    doc.insert(root, DEFAULT_SLOT, 0, placed);

    let file = preview(&project, &project.docs[0]);
    assert!(file.source.contains(".child(StatCard::new())"));
    assert!(file.source.contains("use super::StatCard;"));
  }

  #[test]
  fn a_labelled_bar_chart_uses_the_entries_constructor() {
    let mut project = Project::new("Demo");
    let doc = &mut project.docs[0];
    let root = doc.root;
    let chart = doc.create("barchart");
    let id = doc.insert(root, DEFAULT_SLOT, 0, chart);
    doc
      .node_mut(id)
      .unwrap()
      .set_prop("values", PropValue::Numbers(vec![1.0, 2.0]));
    doc
      .node_mut(id)
      .unwrap()
      .set_prop("labels", PropValue::Items(vec!["a".into(), "b".into()]));

    let file = preview(&project, &project.docs[0]);
    assert!(file
      .source
      .contains("BarChart::entries([(\"a\", 1.), (\"b\", 2.)])"));
    assert!(!file.source.contains(".entries("));
  }

  #[test]
  fn a_screen_with_nothing_to_build_does_not_name_its_context() {
    let project = Project::new("Demo");
    let file = preview(&project, &project.docs[0]);
    assert!(file
      .source
      .contains("pub fn new(_cx: &mut Context<Self>) -> Self"));
  }
}

/// Timings for the work that runs on every edit. Not a benchmark suite — a
/// measurement, so the decision about what to move off the main thread is made
/// from numbers rather than from a hunch. `TAILOR_BENCH=1 cargo test -p
/// tailor-codegen bench -- --nocapture`.
#[cfg(test)]
mod bench {
  use std::time::Instant;
  use tailor_model::node::DEFAULT_SLOT;
  use tailor_model::props::PropValue;
  use tailor_model::{DocKind, Document, Project};

  /// A project about as big as a real one gets: several screens, a few
  /// hundred nodes each.
  fn big(screens: usize, per_screen: usize) -> Project {
    let mut project = Project::new("Big");
    project.docs.clear();
    for s in 0..screens {
      let mut doc = Document::new(format!("s{s}"), format!("Screen{s}"), DocKind::Screen);
      let root = doc.root;
      let mut row = root;
      for n in 0..per_screen {
        if n % 6 == 0 {
          let frame = doc.create("frame");
          row = doc.insert(root, DEFAULT_SLOT, usize::MAX, frame);
        }
        let kind = ["button", "text", "badge", "switch", "card"][n % 5];
        let mut node = doc.create(kind);
        node.set_prop("label", PropValue::Text(format!("Item {n}")));
        node.set_prop("content", PropValue::Text(format!("Item {n}")));
        doc.insert(row, DEFAULT_SLOT, usize::MAX, node);
      }
      project.docs.push(doc);
    }
    project
  }

  fn time(label: &str, runs: u32, mut f: impl FnMut()) {
    let start = Instant::now();
    for _ in 0..runs {
      f();
    }
    let each = start.elapsed() / runs;
    println!("  {label:<22} {:>8.3} ms", each.as_secs_f64() * 1000.0);
  }

  /// What one edit costs on the main thread, before and after. "Before" is
  /// a deep copy for the undo snapshot plus a deep copy for the canvas every
  /// frame; "after" is one copy-on-write, shared with both.
  #[test]
  fn what_sharing_saves() {
    if std::env::var("TAILOR_BENCH").is_err() {
      return;
    }
    use std::sync::Arc;
    for (screens, per) in [(4, 150), (8, 400)] {
      let project = big(screens, per);
      let nodes: usize = project.docs.iter().map(|d| d.nodes.len()).sum();
      println!("\n{nodes} nodes — one edit, main thread:");

      time("owned: commit + frame", 50, || {
        let history_copy = project.clone(); // undo snapshot
        let frame_copy = project.clone(); // canvas snapshot, every frame
        std::hint::black_box((history_copy, frame_copy));
      });

      let shared = Arc::new(project.clone());
      time("shared: commit + frame", 50, || {
        let history_copy = Arc::clone(&shared); // undo snapshot
        let frame_copy = Arc::clone(&shared); // canvas snapshot
        let mut editing = Arc::clone(&shared);
        Arc::make_mut(&mut editing).name.push('x'); // the one real copy
        std::hint::black_box((history_copy, frame_copy, editing));
      });

      time("shared: idle frame", 50, || {
        std::hint::black_box(Arc::clone(&shared));
      });
    }
  }

  #[test]
  fn what_an_edit_costs() {
    if std::env::var("TAILOR_BENCH").is_err() {
      return;
    }
    for (screens, per) in [(1, 60), (4, 150), (8, 400)] {
      let project = big(screens, per);
      let nodes: usize = project.docs.iter().map(|d| d.nodes.len()).sum();
      println!(
        "
{screens} screens, {nodes} nodes:"
      );
      time("clone", 50, || {
        let _ = project.clone();
      });
      time("codegen (one doc)", 50, || {
        let _ = crate::preview(&project, &project.docs[0]);
      });
      time("codegen (all docs)", 20, || {
        let _ = crate::project_files(&project);
      });
      time("lint", 50, || {
        let _ = tailor_model::lint::check(&project);
      });
      time("to_json", 20, || {
        let _ = project.to_json();
      });
    }
  }
}

#[cfg(test)]
mod dump {
  #[test]
  fn write_sample() {
    if std::env::var("TAILOR_DUMP").is_err() {
      return;
    }
    let project = super::tests::kitchen_sink();
    let file = super::preview(&project, &project.docs[0]);
    std::fs::write(std::env::var("TAILOR_DUMP").unwrap(), file.source).unwrap();
  }
}
