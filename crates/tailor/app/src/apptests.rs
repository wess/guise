//! Workbench tests on gpui's test harness.
//!
//! The document rules are unit-tested in `tailor-model` and the output in
//! `tailor-codegen`; what needs a live app is the layer between them — that a
//! command selects what it created, that undo puts the tree back, that
//! renaming a component rewrites every screen that places it.

use gpui::{px, TestAppContext, VisualTestContext};
use tailor_model::node::DEFAULT_SLOT;
use tailor_model::props::PropValue;
use tailor_model::style::Dimension;
use tailor_model::tokens::{EaseToken, EnterToken};
use tailor_model::{DocKind, Document, NodeId, Project, Scheme};
use tailor_render::{DropSpot, Handle};

use crate::editor::{Inspector, Workbench};
use crate::templates;
use crate::theme;
use crate::toasts::Toasts;

fn workbench(
  project: Project,
  cx: &mut TestAppContext,
) -> (gpui::Entity<Workbench>, &mut VisualTestContext) {
  cx.update(|cx| theme::chrome(Scheme::Dark).init(cx));
  let (workbench, cx) = cx.add_window_view(|_window, cx| {
    let toasts = Toasts::new(cx);
    Workbench::new(project, None, tailor_store::Settings::default(), toasts, cx)
  });
  workbench.update(cx, |this, cx| {
    // Never write the running user's settings from a test.
    this.persist_settings = false;
    // The debounce is a smoothness measure, not behaviour; tests drive the
    // executor directly and should not wait on a clock. The refresh
    // replaces the one the constructor already queued at the real delay —
    // dropping that task is what cancels it.
    this.analysis_delay = std::time::Duration::ZERO;
    this.autosave_delay = std::time::Duration::ZERO;
    this.refresh(cx);
  });
  (workbench, cx)
}

/// Let the background regenerate-and-lint land. The app never waits on it —
/// that is the point of moving it off the main thread — so a test that asserts
/// on generated code or on the problem list has to.
fn settle(cx: &mut VisualTestContext) {
  cx.run_until_parked();
}

/// The root of the open document.
fn root(workbench: &gpui::Entity<Workbench>, cx: &mut VisualTestContext) -> NodeId {
  workbench.update(cx, |this, _| this.doc().unwrap().root)
}

fn children(workbench: &gpui::Entity<Workbench>, cx: &mut VisualTestContext) -> Vec<NodeId> {
  workbench.update(cx, |this, _| {
    let doc = this.doc().unwrap();
    doc.children_of(doc.root).to_vec()
  })
}

#[gpui::test]
fn placing_a_component_selects_it_and_marks_the_project_edited(cx: &mut TestAppContext) {
  let (workbench, cx) = workbench(Project::new("T"), cx);
  let root = root(&workbench, cx);

  workbench.update(cx, |this, cx| {
    assert!(!this.dirty);
    this.insert_kind("button", DropSpot::at(root, DEFAULT_SLOT, 0), cx);
  });

  workbench.update(cx, |this, _| {
    assert!(this.dirty);
    assert_eq!(this.selection.len(), 1);
    let id = this.selection[0];
    assert_eq!(this.doc().unwrap().node(id).unwrap().kind, "button");
  });
}

#[gpui::test]
fn undo_puts_a_deleted_subtree_back(cx: &mut TestAppContext) {
  let (workbench, cx) = workbench(Project::new("T"), cx);
  let root = root(&workbench, cx);

  workbench.update(cx, |this, cx| {
    this.insert_kind("card", DropSpot::at(root, DEFAULT_SLOT, 0), cx);
  });
  let card = children(&workbench, cx)[0];
  workbench.update(cx, |this, cx| {
    this.insert_kind("text", DropSpot::at(card, DEFAULT_SLOT, 0), cx);
  });

  workbench.update_in(cx, |this, window, cx| {
    this.select_only(card, cx);
    this.delete_selection(window, cx);
  });
  assert!(children(&workbench, cx).is_empty());

  workbench.update_in(cx, |this, window, cx| this.undo(window, cx));
  let restored = children(&workbench, cx);
  assert_eq!(restored, [card]);
  workbench.update(cx, |this, _| {
    assert_eq!(this.doc().unwrap().children_of(card).len(), 1);
  });
}

#[gpui::test]
fn embed_wraps_the_selection_and_unwrap_lifts_it_back(cx: &mut TestAppContext) {
  let (workbench, cx) = workbench(Project::new("T"), cx);
  let root = root(&workbench, cx);

  workbench.update(cx, |this, cx| {
    this.insert_kind("text", DropSpot::at(root, DEFAULT_SLOT, 0), cx);
    this.insert_kind("badge", DropSpot::at(root, DEFAULT_SLOT, 1), cx);
  });
  let original = children(&workbench, cx);
  assert_eq!(original.len(), 2);

  workbench.update(cx, |this, cx| {
    this.selection = original.clone();
    this.embed("card", cx);
  });

  let after = children(&workbench, cx);
  assert_eq!(after.len(), 1, "both nodes moved inside the card");
  let card = after[0];
  workbench.update(cx, |this, _| {
    assert_eq!(this.doc().unwrap().node(card).unwrap().kind, "card");
    assert_eq!(this.doc().unwrap().children_of(card), original);
  });

  workbench.update_in(cx, |this, window, cx| {
    this.select_only(card, cx);
    this.unwrap_selection(window, cx);
  });
  assert_eq!(children(&workbench, cx), original);
}

#[gpui::test]
fn a_node_cannot_be_dropped_into_its_own_child(cx: &mut TestAppContext) {
  let (workbench, cx) = workbench(Project::new("T"), cx);
  let root = root(&workbench, cx);

  workbench.update(cx, |this, cx| {
    this.insert_kind("card", DropSpot::at(root, DEFAULT_SLOT, 0), cx);
  });
  let outer = children(&workbench, cx)[0];
  workbench.update(cx, |this, cx| {
    this.insert_kind("card", DropSpot::at(outer, DEFAULT_SLOT, 0), cx);
  });
  let inner = workbench.update(cx, |this, _| this.doc().unwrap().children_of(outer)[0]);

  workbench.update(cx, |this, cx| {
    this.move_to(outer, DropSpot::at(inner, DEFAULT_SLOT, 0), cx);
  });
  // Still where it was, and the refusal left no "Move" step behind.
  assert_eq!(children(&workbench, cx), [outer]);
  workbench.update(cx, |this, _| {
    assert_ne!(this.history.undo_label(), Some("Move"));
    assert!(!this.history.can_redo());
  });
}

#[gpui::test]
fn renaming_a_component_rewrites_every_screen_that_places_it(cx: &mut TestAppContext) {
  let mut project = Project::new("T");
  project
    .docs
    .push(Document::new("card", "StatCard", DocKind::Component));
  let (workbench, cx) = workbench(project, cx);
  let root = root(&workbench, cx);

  workbench.update(cx, |this, cx| {
    this.insert_kind("@StatCard", DropSpot::at(root, DEFAULT_SLOT, 0), cx);
    this.open_document("card", cx);
    this.rename_document("MetricCard", cx);
  });

  workbench.update(cx, |this, _| {
    let screen = this.project.doc("main").unwrap();
    let placed = screen.children_of(screen.root)[0];
    assert_eq!(screen.node(placed).unwrap().kind, "@MetricCard");
    assert!(tailor_model::lint::check(&this.project)
      .iter()
      .all(|problem| problem.severity != tailor_model::Severity::Error));
  });
}

#[gpui::test]
fn a_component_that_would_contain_itself_is_refused(cx: &mut TestAppContext) {
  let mut project = Project::new("T");
  project
    .docs
    .push(Document::new("card", "StatCard", DocKind::Component));
  let (workbench, cx) = workbench(project, cx);

  workbench.update(cx, |this, cx| {
    this.open_document("card", cx);
    let root = this.doc().unwrap().root;
    this.insert_kind("@StatCard", DropSpot::at(root, DEFAULT_SLOT, 0), cx);
  });

  workbench.update(cx, |this, _| {
    let card = this.project.doc("card").unwrap();
    assert!(
      card.children_of(card.root).is_empty(),
      "the drop was refused"
    );
  });
}

#[gpui::test]
fn editing_a_prop_regenerates_the_code(cx: &mut TestAppContext) {
  let (workbench, cx) = workbench(Project::new("T"), cx);
  let root = root(&workbench, cx);

  workbench.update(cx, |this, cx| {
    this.insert_kind("button", DropSpot::at(root, DEFAULT_SLOT, 0), cx);
  });
  let button = children(&workbench, cx)[0];

  workbench.update(cx, |this, cx| {
    this.set_prop(button, "label", PropValue::Text("Ship it".into()), cx);
  });
  settle(cx);
  workbench.update(cx, |this, _| {
    assert!(this.generated.contains("\"Ship it\""));
    assert!(this.generated.contains("impl Render for MainScreen"));
  });
}

#[gpui::test]
fn the_lint_pass_follows_a_deleted_action(cx: &mut TestAppContext) {
  let (workbench, cx) = workbench(Project::new("T"), cx);
  let root = root(&workbench, cx);

  workbench.update(cx, |this, cx| {
    this.insert_kind("button", DropSpot::at(root, DEFAULT_SLOT, 0), cx);
    this.add_action(cx);
  });
  let button = children(&workbench, cx)[0];

  workbench.update(cx, |this, cx| {
    let action = this.doc().unwrap().actions[0].name.clone();
    this.edit_node(button, "Connect", cx, move |node| {
      node.events.insert("click".into(), action);
    });
  });
  settle(cx);
  workbench.update(cx, |this, _| {
    assert!(this.generated.contains("cx.listener("));
    assert!(this
      .problems
      .iter()
      .all(|p| p.severity != tailor_model::Severity::Error));
  });

  workbench.update(cx, |this, cx| {
    this.edit_doc("Remove action", cx, |doc| {
      doc.actions.clear();
    });
  });
  settle(cx);
  workbench.update(cx, |this, _| {
    assert!(this
      .problems
      .iter()
      .any(|p| p.severity == tailor_model::Severity::Error));
  });
}

#[gpui::test]
fn right_click_opens_a_menu_and_the_next_edit_dismisses_it(cx: &mut TestAppContext) {
  let (workbench, cx) = workbench(Project::new("T"), cx);
  let root = root(&workbench, cx);
  workbench.update(cx, |this, cx| {
    this.insert_kind("button", DropSpot::at(root, DEFAULT_SLOT, 0), cx);
  });
  let button = children(&workbench, cx)[0];

  workbench.update_in(cx, |this, window, cx| {
    this.open_context_menu(Some(button), gpui::point(px(120.), px(80.)), window, cx);
  });
  workbench.update(cx, |this, cx| {
    let menu = this.menu.clone().expect("a menu opened");
    assert!(menu.read(cx).is_open());
  });

  // Any command that changes the tree closes it: a menu still open over a
  // node that has moved is pointing at the wrong thing.
  workbench.update(cx, |this, cx| {
    this.set_prop(button, "label", PropValue::Text("Go".into()), cx);
  });
  workbench.update(cx, |this, _| assert!(this.menu.is_none()));
}

#[gpui::test]
fn the_canvas_menu_opens_with_nothing_selected(cx: &mut TestAppContext) {
  let (workbench, cx) = workbench(Project::new("T"), cx);
  workbench.update_in(cx, |this, window, cx| {
    this.open_context_menu(None, gpui::point(px(10.), px(10.)), window, cx);
  });
  workbench.update(cx, |this, cx| {
    assert!(this
      .menu
      .as_ref()
      .map(|menu| menu.read(cx).is_open())
      .unwrap_or(false));
  });
}

#[gpui::test]
fn extracting_a_selection_leaves_a_component_reference_behind(cx: &mut TestAppContext) {
  let (workbench, cx) = workbench(Project::new("T"), cx);
  let root = root(&workbench, cx);
  workbench.update(cx, |this, cx| {
    this.insert_kind("card", DropSpot::at(root, DEFAULT_SLOT, 0), cx);
  });
  let card = children(&workbench, cx)[0];
  workbench.update(cx, |this, cx| {
    this.insert_kind("text", DropSpot::at(card, DEFAULT_SLOT, 0), cx);
    this.select_only(card, cx);
    this.extract_component(cx);
  });

  settle(cx);
  workbench.update(cx, |this, _| {
    assert_eq!(this.project.docs.len(), 2, "a component document was added");
    let component = &this.project.docs[1];
    assert_eq!(component.kind, tailor_model::DocKind::Component);
    // The card and its text came across.
    assert_eq!(component.children_of(component.root).len(), 1);

    let screen = this.project.doc("main").unwrap();
    let placed = screen.children_of(screen.root)[0];
    assert_eq!(
      screen.node(placed).unwrap().component_ref(),
      Some(component.name.as_str())
    );
    assert!(this
      .generated
      .contains(&format!("{}::new()", component.name)));
  });
}

/// The inspector is the one thing in Tailor that costs every component in the
/// document something per frame, so closing it has to actually drop it — a
/// hidden-but-alive `DevTools` would leave the recorder running for the rest of
/// the session.
#[gpui::test]
fn the_inspector_opens_on_the_live_window_and_closing_it_stops_the_recorder(
  cx: &mut TestAppContext,
) {
  let (workbench, cx) = workbench(Project::new("Demo"), cx);
  assert!(!guise::devtools::is_recording());

  workbench.update_in(cx, |this, window, cx| this.toggle_devtools(window, cx));
  settle(cx);
  assert!(
    guise::devtools::is_recording(),
    "opening the inspector should start the recorder"
  );

  workbench.update_in(cx, |this, window, cx| this.toggle_devtools(window, cx));
  settle(cx);
  assert!(
    !guise::devtools::is_recording(),
    "closing the inspector should drop it, not just hide it"
  );
}

/// A duplicate has to take a new name, not just a new id: a component's *name*
/// is what a screen's `@Reference` carries, so a copy sharing one would silently
/// steal every reference to the original.
#[gpui::test]
fn duplicating_a_document_gives_the_copy_its_own_name(cx: &mut TestAppContext) {
  let mut project = Project::new("Demo");
  project.docs.push(tailor_model::Document::new(
    "card",
    "StatCard",
    tailor_model::DocKind::Component,
  ));
  let (workbench, cx) = workbench(project, cx);

  workbench.update(cx, |this, cx| this.duplicate_document("card", cx));
  settle(cx);

  let (names, open) = workbench.update(cx, |this, _| {
    (
      this
        .project
        .docs
        .iter()
        .map(|doc| doc.name.clone())
        .collect::<Vec<_>>(),
      this.doc_id.clone(),
    )
  });
  assert_eq!(names.iter().filter(|name| *name == "StatCard").count(), 1);
  assert_eq!(names.len(), 3, "{names:?}");
  assert_ne!(open, "card", "the copy should be the one now open");

  // And it is one undo step, like every other edit.
  workbench.update_in(cx, |this, window, cx| this.undo(window, cx));
  settle(cx);
  workbench.update(cx, |this, _| assert_eq!(this.project.docs.len(), 2));
}

/// The tab menu's Rename opens the document and asks for its name field. The
/// field belongs to the inspector, so the ask has to survive until the panel
/// that owns it has rendered.
#[gpui::test]
fn renaming_from_the_tab_menu_focuses_the_document_name_field(cx: &mut TestAppContext) {
  let mut project = Project::new("Demo");
  project.docs.push(tailor_model::Document::new(
    "card",
    "StatCard",
    tailor_model::DocKind::Component,
  ));
  let (workbench, cx) = workbench(project, cx);

  workbench.update(cx, |this, cx| this.begin_rename_document("card", cx));
  settle(cx);

  workbench.update(cx, |this, cx| {
    assert_eq!(this.doc_id, "card");
    assert!(this.selection.is_empty(), "the document inspector needs it");
    assert!(this.settings.is_open(tailor_store::Panel::Inspector));
    assert!(
      this.focus_field.is_none(),
      "the field should have been found and focused, not left pending"
    );
    assert!(
      this.fields.contains_key("doc/card/name"),
      "{:?}",
      cx.entity()
    );
  });
}

#[gpui::test]
fn panels_toggle_and_keep_their_size(cx: &mut TestAppContext) {
  let (workbench, cx) = workbench(Project::new("T"), cx);
  workbench.update(cx, |this, cx| {
    assert!(this.settings.is_open(tailor_store::Panel::Inspector));
    this.toggle_panel(tailor_store::Panel::Inspector, cx);
    assert!(!this.settings.is_open(tailor_store::Panel::Inspector));

    // A splitter cannot drag a panel past its range.
    this
      .settings
      .set_size(tailor_store::Panel::Palette, 10_000.0);
    assert_eq!(
      this.settings.size(tailor_store::Panel::Palette),
      tailor_store::Panel::Palette.range().1
    );
  });
}

#[gpui::test]
fn dragging_a_corner_knob_resizes_the_node(cx: &mut TestAppContext) {
  let (workbench, cx) = workbench(Project::new("T"), cx);
  let root = root(&workbench, cx);
  workbench.update(cx, |this, cx| {
    this.insert_kind("card", DropSpot::at(root, DEFAULT_SLOT, 0), cx);
  });
  let card = children(&workbench, cx)[0];
  workbench.update(cx, |this, cx| {
    this.edit_style(card, "Size", cx, |style| {
      style.width = Dimension::Px(200.0);
      style.height = Dimension::Px(100.0);
    });
    this.settings.snap = false;
  });

  workbench.update(cx, |this, cx| {
    this.begin_grab(
      card,
      Some(Handle::SouthEast),
      gpui::point(px(0.), px(0.)),
      cx,
    );
    this.apply_grab(gpui::point(px(40.), px(25.)), cx);
    this.end_grab(cx);
  });

  workbench.update(cx, |this, _| {
    let style = &this.doc().unwrap().node(card).unwrap().style;
    assert_eq!(style.width, Dimension::Px(240.0));
    assert_eq!(style.height, Dimension::Px(125.0));
    // The whole drag is one undo step, not one per frame.
    assert_eq!(this.history.undo_label(), Some("Resize"));
  });

  workbench.update_in(cx, |this, window, cx| this.undo(window, cx));
  workbench.update(cx, |this, _| {
    assert_eq!(
      this.doc().unwrap().node(card).unwrap().style.width,
      Dimension::Px(200.0)
    );
  });
}

#[gpui::test]
fn a_leading_knob_moves_the_origin_of_an_absolute_child(cx: &mut TestAppContext) {
  let (workbench, cx) = workbench(Project::new("T"), cx);
  let root = root(&workbench, cx);
  workbench.update(cx, |this, cx| {
    this.edit_style(root, "Layout", cx, |style| {
      style.layout = tailor_model::LayoutMode::Absolute;
    });
    this.insert_kind("badge", DropSpot::at(root, DEFAULT_SLOT, 0), cx);
  });
  let badge = children(&workbench, cx)[0];
  workbench.update(cx, |this, cx| {
    this.edit_style(badge, "Size", cx, |style| {
      style.x = 100.0;
      style.y = 100.0;
      style.width = Dimension::Px(80.0);
      style.height = Dimension::Px(40.0);
    });
    this.settings.snap = false;
  });

  // Dragging the north-west knob right and down shrinks the node and pulls
  // its origin with it.
  workbench.update(cx, |this, cx| {
    this.begin_grab(
      badge,
      Some(Handle::NorthWest),
      gpui::point(px(0.), px(0.)),
      cx,
    );
    this.apply_grab(gpui::point(px(20.), px(10.)), cx);
    this.end_grab(cx);
  });

  workbench.update(cx, |this, _| {
    let style = &this.doc().unwrap().node(badge).unwrap().style;
    assert_eq!(style.width, Dimension::Px(60.0));
    assert_eq!(style.height, Dimension::Px(30.0));
    assert_eq!(style.x, 120.0);
    assert_eq!(style.y, 110.0);
  });
}

#[gpui::test]
fn dragging_the_body_moves_an_absolute_child_and_snaps(cx: &mut TestAppContext) {
  let (workbench, cx) = workbench(Project::new("T"), cx);
  let root = root(&workbench, cx);
  workbench.update(cx, |this, cx| {
    this.edit_style(root, "Layout", cx, |style| {
      style.layout = tailor_model::LayoutMode::Absolute;
    });
    this.insert_kind("badge", DropSpot::at(root, DEFAULT_SLOT, 0), cx);
  });
  let badge = children(&workbench, cx)[0];
  workbench.update(cx, |this, cx| {
    this.settings.snap = true;
    this.settings.grid = 8.0;
    this.begin_grab(badge, None, gpui::point(px(0.), px(0.)), cx);
    this.apply_grab(gpui::point(px(21.), px(13.)), cx);
    this.end_grab(cx);
  });

  workbench.update(cx, |this, _| {
    let style = &this.doc().unwrap().node(badge).unwrap().style;
    // 21 and 13 land on the nearest eight.
    assert_eq!(style.x, 24.0);
    assert_eq!(style.y, 16.0);
  });
}

#[gpui::test]
fn a_node_cannot_be_dragged_smaller_than_the_minimum(cx: &mut TestAppContext) {
  let (workbench, cx) = workbench(Project::new("T"), cx);
  let root = root(&workbench, cx);
  workbench.update(cx, |this, cx| {
    this.insert_kind("card", DropSpot::at(root, DEFAULT_SLOT, 0), cx);
  });
  let card = children(&workbench, cx)[0];
  workbench.update(cx, |this, cx| {
    this.edit_style(card, "Size", cx, |style| {
      style.width = Dimension::Px(100.0);
      style.height = Dimension::Px(100.0);
    });
    this.settings.snap = false;
    this.begin_grab(
      card,
      Some(Handle::SouthEast),
      gpui::point(px(0.), px(0.)),
      cx,
    );
    this.apply_grab(gpui::point(px(-500.), px(-500.)), cx);
    this.end_grab(cx);
  });

  workbench.update(cx, |this, _| {
    let style = &this.doc().unwrap().node(card).unwrap().style;
    assert_eq!(style.width, Dimension::Px(8.0));
    assert_eq!(style.height, Dimension::Px(8.0));
  });
}

#[gpui::test]
fn resizing_an_image_sets_its_own_size_props(cx: &mut TestAppContext) {
  let (workbench, cx) = workbench(Project::new("T"), cx);
  let root = root(&workbench, cx);
  workbench.update(cx, |this, cx| {
    this.insert_kind("image", DropSpot::at(root, DEFAULT_SLOT, 0), cx);
    this.settings.snap = false;
  });
  let image = children(&workbench, cx)[0];

  workbench.update(cx, |this, cx| {
    this.begin_grab(
      image,
      Some(Handle::SouthEast),
      gpui::point(px(0.), px(0.)),
      cx,
    );
    this.apply_grab(gpui::point(px(40.), px(30.)), cx);
    this.end_grab(cx);
  });

  settle(cx);
  workbench.update(cx, |this, _| {
    let node = this.doc().unwrap().node(image).unwrap();
    // The catalog's defaults are 160 x 120.
    assert_eq!(node.prop("width").unwrap().as_f64(), Some(200.0));
    assert_eq!(node.prop("height").unwrap().as_f64(), Some(150.0));
    // The box around it is left alone.
    assert!(node.style.width.is_auto());
    assert!(this.generated.contains(".width(200.)"));
  });
}

#[gpui::test]
fn a_knob_press_without_a_drag_leaves_no_undo_step(cx: &mut TestAppContext) {
  let (workbench, cx) = workbench(Project::new("T"), cx);
  let root = root(&workbench, cx);
  workbench.update(cx, |this, cx| {
    this.insert_kind("card", DropSpot::at(root, DEFAULT_SLOT, 0), cx);
  });
  let card = children(&workbench, cx)[0];

  workbench.update(cx, |this, cx| {
    this.begin_grab(card, Some(Handle::East), gpui::point(px(0.), px(0.)), cx);
    this.end_grab(cx);
    assert_ne!(this.history.undo_label(), Some("Resize"));
  });
}

#[gpui::test]
fn free_form_makes_a_new_frame_place_its_children(cx: &mut TestAppContext) {
  let (workbench, cx) = workbench(Project::new("T"), cx);
  let root = root(&workbench, cx);

  workbench.update(cx, |this, cx| {
    this.insert_kind("frame", DropSpot::at(root, DEFAULT_SLOT, 0), cx);
  });
  let flow = children(&workbench, cx)[0];
  workbench.update(cx, |this, _| {
    assert_eq!(
      this.doc().unwrap().node(flow).unwrap().style.layout,
      tailor_model::LayoutMode::Flow
    );
  });

  workbench.update_in(cx, |this, window, cx| {
    this.toggle_free_form(window, cx);
    this.insert_kind("frame", DropSpot::at(root, DEFAULT_SLOT, 1), cx);
  });
  let free = children(&workbench, cx)[1];
  workbench.update(cx, |this, _| {
    assert_eq!(
      this.doc().unwrap().node(free).unwrap().style.layout,
      tailor_model::LayoutMode::Absolute
    );
    // A card is not a frame: the preference is about frames.
    assert!(this.settings.free_form);
  });
}

#[gpui::test]
fn the_selected_container_flips_between_flow_and_free_form(cx: &mut TestAppContext) {
  let (workbench, cx) = workbench(Project::new("T"), cx);
  let root = root(&workbench, cx);
  workbench.update(cx, |this, cx| {
    this.insert_kind("frame", DropSpot::at(root, DEFAULT_SLOT, 0), cx);
  });
  let frame = children(&workbench, cx)[0];

  workbench.update_in(cx, |this, window, cx| {
    this.select_only(frame, cx);
    this.toggle_selection_layout(window, cx);
  });
  workbench.update(cx, |this, _| {
    assert_eq!(
      this.doc().unwrap().node(frame).unwrap().style.layout,
      tailor_model::LayoutMode::Absolute
    );
  });
  workbench.update_in(cx, |this, window, cx| {
    this.toggle_selection_layout(window, cx)
  });
  workbench.update(cx, |this, _| {
    assert_eq!(
      this.doc().unwrap().node(frame).unwrap().style.layout,
      tailor_model::LayoutMode::Flow
    );
  });
}

#[gpui::test]
fn snapping_to_the_grid_can_be_turned_off(cx: &mut TestAppContext) {
  let (workbench, cx) = workbench(Project::new("T"), cx);
  let root = root(&workbench, cx);
  workbench.update(cx, |this, cx| {
    this.edit_style(root, "Layout", cx, |style| {
      style.layout = tailor_model::LayoutMode::Absolute;
    });
    this.insert_kind("badge", DropSpot::at(root, DEFAULT_SLOT, 0), cx);
  });
  let badge = children(&workbench, cx)[0];

  workbench.update(cx, |this, cx| {
    this.settings.snap = false;
    this.settings.snap_objects = false;
    this.begin_grab(badge, None, gpui::point(px(0.), px(0.)), cx);
    this.apply_grab(gpui::point(px(21.), px(13.)), cx);
    this.end_grab(cx);
  });
  workbench.update(cx, |this, _| {
    let style = &this.doc().unwrap().node(badge).unwrap().style;
    // Left exactly where it was dropped, not rounded to eight.
    assert_eq!(style.x, 21.0);
    assert_eq!(style.y, 13.0);
  });
}

#[gpui::test]
fn the_nudge_step_follows_the_setting(cx: &mut TestAppContext) {
  let (workbench, cx) = workbench(Project::new("T"), cx);
  let root = root(&workbench, cx);
  workbench.update(cx, |this, cx| {
    this.edit_style(root, "Layout", cx, |style| {
      style.layout = tailor_model::LayoutMode::Absolute;
    });
    this.insert_kind("badge", DropSpot::at(root, DEFAULT_SLOT, 0), cx);
    this.settings.nudge = 4.0;
  });
  let badge = children(&workbench, cx)[0];

  workbench.update_in(cx, |this, window, cx| {
    this.select_only(badge, cx);
    this.nudge_right(window, cx);
  });
  workbench.update(cx, |this, _| {
    assert_eq!(this.doc().unwrap().node(badge).unwrap().style.x, 4.0);
  });
}

#[gpui::test]
fn arrow_keys_nudge_absolutely_and_reorder_in_a_flow(cx: &mut TestAppContext) {
  let (workbench, cx) = workbench(Project::new("T"), cx);
  let root = root(&workbench, cx);
  workbench.update(cx, |this, cx| {
    this.insert_kind("badge", DropSpot::at(root, DEFAULT_SLOT, 0), cx);
    this.insert_kind("text", DropSpot::at(root, DEFAULT_SLOT, 1), cx);
  });
  let flow = children(&workbench, cx);

  // Flow: up and down reorder, because there is no x to move.
  workbench.update_in(cx, |this, window, cx| {
    this.select_only(flow[1], cx);
    this.nudge_up(window, cx);
  });
  assert_eq!(children(&workbench, cx), [flow[1], flow[0]]);

  // Absolute: the arrows move it.
  workbench.update_in(cx, |this, window, cx| {
    this.edit_style(root, "Layout", cx, |style| {
      style.layout = tailor_model::LayoutMode::Absolute;
    });
    this.select_only(flow[1], cx);
    this.nudge_right(window, cx);
    this.nudge_down(window, cx);
  });
  workbench.update(cx, |this, _| {
    let style = &this.doc().unwrap().node(flow[1]).unwrap().style;
    assert_eq!(style.x, 1.0);
    assert_eq!(style.y, 1.0);
  });
}

/// Every `let x = x.clone();` a closure prologue emits must refer to a local
/// that already exists. Checking it here rather than in the generator because
/// the templates are the only place with the nesting that provokes it.
fn captures_resolve(source: &str) -> Result<(), String> {
  let mut declared: Vec<&str> = Vec::new();
  for line in source.lines() {
    let line = line.trim();
    if let Some(rest) = line.strip_prefix("let ") {
      let Some((name, value)) = rest.split_once(" = ") else {
        continue;
      };
      if value.starts_with(&format!("{name}.clone()")) {
        if !declared.contains(&name) {
          return Err(format!("`{name}` is cloned before it is built"));
        }
      } else {
        declared.push(name);
      }
    }
  }
  Ok(())
}

#[gpui::test]
fn every_template_generates_code_whose_captures_resolve(cx: &mut TestAppContext) {
  for template in templates::TEMPLATES {
    let project = (template.build)();
    let (workbench, cx) = workbench(project, cx);
    workbench.update(cx, |this, _| {
      for doc in &this.project.docs {
        let source = tailor_codegen::preview(&this.project, doc).source;
        if let Err(problem) = captures_resolve(&source) {
          panic!("{} / {}: {problem}\n{source}", template.name, doc.name);
        }
      }
    });
  }
}

#[gpui::test]
fn a_stale_background_result_is_discarded(cx: &mut TestAppContext) {
  let (workbench, cx) = workbench(Project::new("T"), cx);
  let root = root(&workbench, cx);
  workbench.update(cx, |this, cx| {
    this.insert_kind("button", DropSpot::at(root, DEFAULT_SLOT, 0), cx);
  });
  settle(cx);

  let current = workbench.update(cx, |this, _| this.generated.clone());
  assert!(current.contains("Button::new"));

  // A result computed before the last edit must not overwrite what is there:
  // regenerating is asynchronous, and two edits in quick succession can
  // finish out of order.
  workbench.update(cx, |this, cx| {
    let stale = this.revision.wrapping_sub(1);
    this.apply_analysis(stale, ("// nonsense".into(), Vec::new()), cx);
  });
  workbench.update(cx, |this, _| {
    assert_eq!(this.generated, current, "a stale result was applied");
  });

  // The current revision does apply.
  workbench.update(cx, |this, cx| {
    let now = this.revision;
    this.apply_analysis(now, ("// fresh".into(), Vec::new()), cx);
  });
  workbench.update(cx, |this, _| assert_eq!(this.generated, "// fresh"));
}

#[gpui::test]
fn every_template_opens_and_generates(cx: &mut TestAppContext) {
  for template in templates::TEMPLATES {
    let project = (template.build)();
    let (workbench, cx) = workbench(project, cx);
    settle(cx);
    workbench.update(cx, |this, _| {
      // Opening a project is not an edit. A file that comes up dirty
      // means something mutated it on the way in, and the next autosave
      // or close prompt would be lying.
      assert!(!this.dirty, "{} opened dirty", template.name);
      assert!(
        this.selection.is_empty(),
        "{} opened with a selection",
        template.name
      );
      assert!(
        !this.generated.is_empty(),
        "{} generated nothing",
        template.name
      );
      let errors: Vec<String> = tailor_model::lint::check(&this.project)
        .into_iter()
        .filter(|problem| problem.severity == tailor_model::Severity::Error)
        .map(|problem| problem.message)
        .collect();
      assert!(
        errors.is_empty(),
        "{} has errors: {errors:?}",
        template.name
      );
    });
  }
}

#[gpui::test]
fn setting_an_entrance_regenerates_the_code_and_replays_the_canvas(cx: &mut TestAppContext) {
  let (workbench, cx) = workbench(Project::new("T"), cx);
  let root = root(&workbench, cx);
  workbench.update(cx, |this, cx| {
    this.insert_kind("button", DropSpot::at(root, DEFAULT_SLOT, 0), cx);
  });
  let button = children(&workbench, cx)[0];
  settle(cx);

  let epoch = workbench.update(cx, |this, _| this.motion_epoch);
  workbench.update(cx, |this, cx| {
    this.edit_motion(button, "Entrance", cx, |motion| {
      motion.enter = Some(EnterToken::SlideUp);
      motion.ease = EaseToken::OutBack;
      motion.duration = 350.0;
    });
  });
  settle(cx);

  workbench.update(cx, |this, _| {
    // Bumping the epoch is what makes a mounted one-shot play again.
    assert!(this.motion_epoch > epoch);
    assert!(this.generated.contains(".animate("), "{}", this.generated);
    assert!(this
      .generated
      .contains("Motion::enter_from(TransitionKind::SlideUp"));
    assert!(this.generated.contains(".ease(Easing::Out(Curve::Back))"));
    assert!(this.generated.contains(".duration(350.)"));
  });

  // The Motion tab has to survive a draw with a live selection — every
  // control in it is built from the node under the cursor.
  workbench.update(cx, |this, cx| this.set_inspector(Inspector::Motion, cx));
  cx.run_until_parked();

  // And it is one undo step, not four.
  workbench.update_in(cx, |this, window, cx| this.undo(window, cx));
  settle(cx);
  workbench.update(cx, |this, _| {
    assert!(this.doc().unwrap().node(button).unwrap().motion.is_off());
    assert!(!this.generated.contains(".animate("));
  });
}

#[gpui::test]
fn a_staggered_container_animates_its_children_instead_of_itself(cx: &mut TestAppContext) {
  let (workbench, cx) = workbench(Project::new("T"), cx);
  let root = root(&workbench, cx);
  workbench.update(cx, |this, cx| {
    this.insert_kind("text", DropSpot::at(root, DEFAULT_SLOT, 0), cx);
    this.insert_kind("text", DropSpot::at(root, DEFAULT_SLOT, 1), cx);
    this.edit_motion(root, "Stagger", cx, |motion| {
      motion.enter = Some(EnterToken::Fade);
      motion.stagger = 80.0;
    });
  });
  settle(cx);

  workbench.update(cx, |this, _| {
    let doc = this.doc().unwrap();
    let kids = doc.children_of(doc.root).to_vec();
    assert_eq!(doc.motion_of(doc.root), None, "the container stays put");
    assert_eq!(doc.motion_of(kids[0]).unwrap().delay, 0.0);
    assert_eq!(doc.motion_of(kids[1]).unwrap().delay, 80.0);
    assert!(this.generated.contains(".delay(80.)"), "{}", this.generated);
  });
}

#[gpui::test]
fn the_manager_owns_the_theme_global(cx: &mut TestAppContext) {
  use guise::theme::{theme, ColorScheme, ThemeChoice, ThemeManager};
  use tailor_store::Settings;

  let mut settings = Settings::default();
  cx.update(|cx| {
    theme::install_manager(&settings, cx);
    // Dark by default, and the chrome ramp is installed rather than guise's.
    assert!(theme(cx).scheme.is_dark());
    assert_eq!(
      cx.global::<ThemeManager>().selection(),
      &ThemeChoice::Fixed("dark".into())
    );

    // Opening a project registers its theme and wears it.
    let project = Project::new("Demo");
    theme::install(&project.theme, cx);
    assert_eq!(
      cx.global::<ThemeManager>().resolved_id().as_ref(),
      "project"
    );

    // Closing it goes back to the chrome the settings name — the settings as
    // they are now, not as they were at launch.
    settings.scheme = Scheme::Light;
    theme::wear_chrome(&settings, cx);
    assert!(!theme(cx).scheme.is_dark());
  });

  cx.update(|cx| {
    // `follow_system` hands the choice to the OS, both ways.
    settings.follow_system = true;
    theme::wear_chrome(&settings, cx);
    assert_eq!(
      cx.global::<ThemeManager>().selection(),
      &ThemeChoice::System
    );

    ThemeManager::set_system_scheme(cx, ColorScheme::Dark);
    assert!(theme(cx).scheme.is_dark());
    ThemeManager::set_system_scheme(cx, ColorScheme::Light);
    assert!(!theme(cx).scheme.is_dark());
  });
}
