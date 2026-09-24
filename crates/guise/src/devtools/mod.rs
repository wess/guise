//! `DevTools` — Safari's Web Inspector, for a gpui app.
//!
//! The Web Inspector's layout is doing real work: a tab bar of *tools* rather
//! than a stack of panels, a tree beside a details sidebar, a console that
//! drops down over whatever else you were looking at. This module reproduces
//! that — Elements, Network, Sources, Timelines, Storage, Layers, Logs and
//! Audit — against a native app instead of a page.
//!
//! Where the data comes from differs by panel, and that difference is the whole
//! design:
//!
//! * **Elements**, **Layers** and the Styles sidebar are *real introspection*.
//!   Components tag their root with [`Probed::probe`], so the tree, the
//!   bounds, and the style declarations are read back out of what actually
//!   rendered this frame.
//! * **Logs**, **Network**, **Storage** and **Timelines** are *reported*.
//!   `guise` never opens a socket, so the host calls [`log`], [`network_begin`],
//!   [`storage_set`] and friends, exactly as [`crate::ai`] displays a
//!   conversation it does not conduct.
//!
//!   Safari's equivalent tab is the Console, and it is called Logs here for a
//!   reason: half of that tab is a JavaScript evaluator, and a compiled binary
//!   has nothing to evaluate. A prompt that cannot run anything is worse than
//!   no prompt, so the panel is named for the half that does transfer.
//! * **Audit** is computed here, from the tree, against the library's own
//!   conventions.
//!
//! ```ignore
//! // once at startup
//! DevToolsState::new().init(cx);
//!
//! // wherever the inspector should live — a pane, a drawer, its own window
//! let devtools = cx.new(|cx| DevTools::new(cx));
//!
//! // and from anywhere in the app
//! guise::devtools::log(cx, LogLevel::Warning, "cache miss");
//! ```
//!
//! Nothing here is compiled out of release builds, because dead-code
//! elimination already handles that: an app that never constructs [`DevTools`]
//! links none of it. What an app *does* pay for is the [`Probed::probe`] calls
//! left in components, and those are a single boolean check while the
//! inspector is closed.

use std::time::Duration;

mod audit;
mod elements;
mod logs;
mod network;
mod probe;
mod shell;
mod sources;
mod state;
mod storage;
mod styles;
mod timelines;

pub use elements::ElementsSidebar;
pub use probe::{
  begin_frame, set_enabled, tree, with_tree, Probe, ProbeNode, ProbeTree, Probed, ProbedAny,
};
pub use state::*;
pub use styles::{box_model, declarations, hex, BoxModel, Declaration};

use gpui::prelude::*;
use gpui::{
  div, px, App, Context, EventEmitter, FocusHandle, Focusable, SharedString, Task, Window,
};

use audit::AuditPanel;
use elements::ElementsPanel;
use logs::LogsPanel;
use network::NetworkPanel;
use probe::ProbeTree as Tree;
use shell::{glyph, hairline, tool_button, Ink, BAR_HEIGHT, LABEL_SIZE};
use sources::SourcesPanel;
use storage::StoragePanel;
use timelines::TimelinesPanel;

use crate::icon::IconName;

const RECORDER_IDLE_AFTER: Duration = Duration::from_millis(250);

/// The tools along the top, in Safari's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DevToolsTab {
  #[default]
  Elements,
  Network,
  Sources,
  Timelines,
  Storage,
  Layers,
  Logs,
  Audit,
}

impl DevToolsTab {
  pub fn label(self) -> &'static str {
    match self {
      DevToolsTab::Elements => "Elements",
      DevToolsTab::Network => "Network",
      DevToolsTab::Sources => "Sources",
      DevToolsTab::Timelines => "Timelines",
      DevToolsTab::Storage => "Storage",
      DevToolsTab::Layers => "Layers",
      DevToolsTab::Logs => "Logs",
      DevToolsTab::Audit => "Audit",
    }
  }

  fn icon(self) -> IconName {
    match self {
      DevToolsTab::Elements => IconName::Code,
      DevToolsTab::Network => IconName::Network,
      DevToolsTab::Sources => IconName::FileCode,
      DevToolsTab::Timelines => IconName::Activity,
      DevToolsTab::Storage => IconName::Database,
      DevToolsTab::Layers => IconName::Layers,
      DevToolsTab::Logs => IconName::ScrollText,
      DevToolsTab::Audit => IconName::ShieldCheck,
    }
  }

  pub const ALL: [DevToolsTab; 8] = [
    DevToolsTab::Elements,
    DevToolsTab::Network,
    DevToolsTab::Sources,
    DevToolsTab::Timelines,
    DevToolsTab::Storage,
    DevToolsTab::Layers,
    DevToolsTab::Logs,
    DevToolsTab::Audit,
  ];
}

/// Where the host has put the inspector. `guise` cannot move a panel it does
/// not own, so this only sets which dock button reads as pressed — the host
/// acts on [`DevToolsEvent::Dock`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Dock {
  #[default]
  Right,
  Bottom,
  /// Its own window.
  Detached,
}

/// What the inspector asks the host to do. Everything it can do alone, it does
/// alone; these are the things that need the host.
#[derive(Debug, Clone)]
pub enum DevToolsEvent {
  /// The close button was pressed.
  Close,
  /// A dock button was pressed.
  Dock(Dock),
  /// A source location was clicked. The host opens it in its editor; the
  /// Sources panel has already switched to it if it knows the file.
  RevealSource(SourceRef),
  /// The element picker was armed or disarmed.
  Picking(bool),
}

/// The inspector. Create with `cx.new(|cx| DevTools::new(cx))`.
pub struct DevTools {
  focus: FocusHandle,
  tab: DevToolsTab,
  dock: Dock,
  /// The tree recorded by the previous frame, refreshed at the top of every
  /// render.
  tree: Tree,
  /// The logs drawer, which Safari drops over any non-Logs tool.
  drawer_open: bool,
  picking: bool,
  recorder_active: bool,
  recorder_heartbeat: u64,
  recorder_watchdog: Option<Task<()>>,
  pub(crate) elements: ElementsPanel,
  pub(crate) logs: LogsPanel,
  pub(crate) network: NetworkPanel,
  pub(crate) storage: StoragePanel,
  pub(crate) timelines: TimelinesPanel,
  pub(crate) sources: SourcesPanel,
  pub(crate) audit: AuditPanel,
}

impl EventEmitter<DevToolsEvent> for DevTools {}

impl Focusable for DevTools {
  fn focus_handle(&self, _cx: &App) -> FocusHandle {
    self.focus.clone()
  }
}

impl DevTools {
  pub fn new(cx: &mut Context<Self>) -> Self {
    let mut inspector = DevTools {
      focus: cx.focus_handle(),
      tab: DevToolsTab::default(),
      dock: Dock::default(),
      tree: Tree::default(),
      drawer_open: false,
      picking: false,
      recorder_active: false,
      recorder_heartbeat: 0,
      recorder_watchdog: None,
      elements: ElementsPanel::default(),
      logs: LogsPanel::new(cx),
      network: NetworkPanel::new(cx),
      storage: StoragePanel::default(),
      timelines: TimelinesPanel::default(),
      sources: SourcesPanel::default(),
      audit: AuditPanel::default(),
    };
    inspector.activate_recorder(cx);
    inspector
  }

  fn activate_recorder(&mut self, cx: &mut Context<Self>) {
    self.recorder_heartbeat = self.recorder_heartbeat.wrapping_add(1);
    if self.recorder_active {
      return;
    }

    probe::retain();
    self.recorder_active = true;
    let mut last_heartbeat = self.recorder_heartbeat;
    self.recorder_watchdog = Some(cx.spawn(async move |this, cx| loop {
      cx.background_executor().timer(RECORDER_IDLE_AFTER).await;
      let stop = this
        .update(cx, |inspector, _cx| {
          if inspector.recorder_heartbeat != last_heartbeat {
            last_heartbeat = inspector.recorder_heartbeat;
            return false;
          }
          if inspector.recorder_active {
            probe::release();
            inspector.recorder_active = false;
          }
          true
        })
        .unwrap_or(true);
      if stop {
        break;
      }
    }));
  }

  /// Open on a particular tool.
  pub fn tab(mut self, tab: DevToolsTab) -> Self {
    self.tab = tab;
    self
  }

  /// Set which dock button reads as pressed.
  pub fn dock(mut self, dock: Dock) -> Self {
    self.dock = dock;
    self
  }

  /// Open the Elements sidebar on a particular pane.
  pub fn elements_sidebar(mut self, sidebar: ElementsSidebar) -> Self {
    self.elements.sidebar = sidebar;
    self
  }

  /// Start with the logs drawer down.
  pub fn drawer(mut self, open: bool) -> Self {
    self.drawer_open = open;
    self
  }

  pub fn active_tab(&self) -> DevToolsTab {
    self.tab
  }

  pub fn set_tab(&mut self, tab: DevToolsTab, cx: &mut Context<Self>) {
    self.tab = tab;
    cx.notify();
  }

  /// Whether the element picker is armed.
  pub fn is_picking(&self) -> bool {
    self.picking
  }

  /// Arm or disarm the picker. The host is told, because the actual hit
  /// testing happens in the window, not here.
  pub fn set_picking(&mut self, picking: bool, cx: &mut Context<Self>) {
    self.picking = picking;
    cx.emit(DevToolsEvent::Picking(picking));
    cx.notify();
  }

  /// Select the element at a window point — what a host wires its picker to.
  /// Returns whether anything was under it.
  pub fn pick_at(&mut self, point: gpui::Point<gpui::Pixels>, cx: &mut Context<Self>) -> bool {
    let Some(index) = self.tree.hit(point) else {
      return false;
    };
    let key = self.tree.nodes[index].key.clone();
    self.elements.reveal(&self.tree, &key);
    self.tab = DevToolsTab::Elements;
    self.picking = false;
    cx.emit(DevToolsEvent::Picking(false));
    cx.notify();
    true
  }

  /// The bounds of the selected element, for a host that paints a highlight
  /// over its own window.
  pub fn selected_bounds(&self) -> Option<gpui::Bounds<gpui::Pixels>> {
    self
      .elements
      .selected_node(&self.tree)
      .map(|node| node.bounds)
  }

  /// The tree as of the last rendered frame.
  pub fn tree(&self) -> &ProbeTree {
    &self.tree
  }

  /// Switch to Sources on `source`, and tell the host in case it would rather
  /// open the file in a real editor.
  pub fn reveal_source(&mut self, source: SourceRef, cx: &mut Context<Self>) {
    self.sources.reveal(source.clone());
    self.tab = DevToolsTab::Sources;
    cx.emit(DevToolsEvent::RevealSource(source));
    cx.notify();
  }

  /// Toggle the logs drawer.
  pub fn toggle_drawer(&mut self, cx: &mut Context<Self>) {
    self.drawer_open = !self.drawer_open;
    cx.notify();
  }

  fn toolbar(&self, ink: &Ink, cx: &mut Context<Self>) -> gpui::Div {
    let (warnings, errors) = cx
      .try_global::<DevToolsState>()
      .map(|state| state.log_issues())
      .unwrap_or((0, 0));
    let (requests, transfer, _) = cx
      .try_global::<DevToolsState>()
      .map(|state| state.network_totals())
      .unwrap_or((0, 0, 0));

    let badge = |count: usize, icon: IconName, color: gpui::Hsla, cx: &App| {
      div()
        .flex()
        .items_center()
        .gap(px(3.0))
        .child(glyph(icon, 11.0, color, cx))
        .child(
          div()
            .text_color(if count > 0 { ink.text } else { ink.dim })
            .child(SharedString::from(count.to_string())),
        )
    };

    div()
      .flex()
      .flex_none()
      .items_center()
      .gap(px(2.0))
      .h(px(BAR_HEIGHT))
      .w_full()
      .px(px(6.0))
      .bg(ink.chrome)
      .border_b_1()
      .border_color(ink.border)
      .child(
        tool_button(
          "devtools-dock-right",
          IconName::PanelRight,
          "Dock to right",
          self.dock == Dock::Right,
          ink,
          cx,
        )
        .on_click(cx.listener(|this, _event, _window, cx| {
          this.dock = Dock::Right;
          cx.emit(DevToolsEvent::Dock(Dock::Right));
          cx.notify();
        })),
      )
      .child(
        tool_button(
          "devtools-dock-bottom",
          IconName::PanelBottom,
          "Dock to bottom",
          self.dock == Dock::Bottom,
          ink,
          cx,
        )
        .on_click(cx.listener(|this, _event, _window, cx| {
          this.dock = Dock::Bottom;
          cx.emit(DevToolsEvent::Dock(Dock::Bottom));
          cx.notify();
        })),
      )
      .child(div().w(px(6.0)))
      .child(
        tool_button(
          "devtools-pick",
          IconName::Crosshair,
          "Select an element",
          self.picking,
          ink,
          cx,
        )
        .on_click(cx.listener(|this, _event, _window, cx| {
          let picking = !this.picking;
          this.set_picking(picking, cx);
        })),
      )
      .child(
        tool_button(
          "devtools-drawer",
          IconName::ScrollText,
          "Show logs drawer",
          self.drawer_open,
          ink,
          cx,
        )
        .on_click(cx.listener(|this, _event, _window, cx| this.toggle_drawer(cx))),
      )
      // The activity viewer: what the page cost, in Safari's words.
      .child(
        div()
          .flex()
          .flex_1()
          .items_center()
          .justify_center()
          .gap(px(10.0))
          .mx(px(8.0))
          .h(px(19.0))
          .rounded(px(4.0))
          .bg(ink.content)
          .border_1()
          .border_color(ink.border)
          .text_size(px(LABEL_SIZE))
          .text_color(ink.dim)
          .child(
            div()
              .flex()
              .items_center()
              .gap(px(3.0))
              .child(glyph(IconName::Boxes, 11.0, ink.dim, cx))
              .child(SharedString::from(format!("{} elements", self.tree.len()))),
          )
          .child(
            div()
              .flex()
              .items_center()
              .gap(px(3.0))
              .child(glyph(IconName::ArrowUpDown, 11.0, ink.dim, cx))
              .child(SharedString::from(format!(
                "{requests} requests · {}",
                format_bytes(transfer)
              ))),
          )
          .child(badge(warnings, IconName::TriangleAlert, ink.warning, cx))
          .child(badge(errors, IconName::CircleX, ink.danger, cx)),
      )
      .child(
        tool_button(
          "devtools-clear",
          IconName::Ban,
          "Clear all records",
          false,
          ink,
          cx,
        )
        .on_click(cx.listener(|_this, _event, _window, cx| {
          if cx.has_global::<DevToolsState>() {
            cx.update_global::<DevToolsState, _>(|state, _cx| state.clear_all());
          }
          cx.notify();
        })),
      )
      .child(
        tool_button("devtools-close", IconName::X, "Close", false, ink, cx)
          .on_click(cx.listener(|_this, _event, _window, cx| cx.emit(DevToolsEvent::Close))),
      )
  }

  fn tab_bar(&self, ink: &Ink, cx: &mut Context<Self>) -> gpui::Div {
    let mut bar = div()
      .flex()
      .flex_none()
      .items_center()
      .gap(px(1.0))
      .h(px(BAR_HEIGHT))
      .w_full()
      .px(px(4.0))
      .bg(ink.chrome)
      .border_b_1()
      .border_color(ink.border);

    for tab in DevToolsTab::ALL {
      let active = self.tab == tab;
      let fg = if active { ink.text } else { ink.dim };
      let hover_bg = ink.hover;
      bar = bar.child(
        div()
          .id(("devtools-tab", tab as usize))
          .flex()
          .flex_none()
          .items_center()
          .gap(px(4.0))
          .h(px(22.0))
          .px(px(8.0))
          .rounded(px(4.0))
          .text_size(px(LABEL_SIZE))
          .text_color(fg)
          .when(active, |el| el.bg(ink.chrome_active))
          .when(!active, |el| el.hover(move |st| st.bg(hover_bg)))
          .child(glyph(tab.icon(), 12.0, fg, cx))
          .child(SharedString::new_static(tab.label()))
          .on_click(cx.listener(move |this, _event, _window, cx| {
            this.tab = tab;
            cx.notify();
          })),
      );
    }

    bar
  }

  fn status_bar(&self, ink: &Ink, cx: &mut Context<Self>) -> gpui::Div {
    let fps = self
      .timelines
      .recording
      .then(|| {
        cx.try_global::<DevToolsState>()
          .and_then(|state| state.fps())
      })
      .flatten();

    div()
      .flex()
      .flex_none()
      .items_center()
      .gap(px(10.0))
      .h(px(20.0))
      .w_full()
      .px(px(8.0))
      .bg(ink.chrome)
      .border_t_1()
      .border_color(ink.border)
      .text_size(px(LABEL_SIZE))
      .text_color(ink.dim)
      .child(SharedString::new_static("guise devtools"))
      .child(div().flex_1())
      .when_some(fps, |el, fps| {
        el.child(SharedString::from(format!("{fps:.0} fps")))
      })
      .child(SharedString::from(self.tab.label()))
  }
}

impl Render for DevTools {
  fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let was_idle = !self.recorder_active;
    self.activate_recorder(cx);
    if was_idle {
      // The first frame turns probes back on; the follow-up captures the
      // complete tree so reopening an inspector never needs a second
      // user interaction.
      cx.notify();
    }

    // Rotate the recorder before anything prepaints this frame, then read
    // the tree the previous frame finished. Doing it here — rather than in
    // the Elements panel — keeps every panel on the same snapshot.
    probe::begin_frame(window);
    self.tree = probe::tree();

    // Open on something rather than on "no element selected", the way the
    // Web Inspector starts on `<body>`. Only ever fills an empty
    // selection, so it cannot fight the user.
    if self.elements.selected.is_none() {
      if let Some(root) = self.tree.roots.first() {
        let key = self.tree.nodes[*root].key.clone();
        self.elements.select(key);
      }
    }
    if self.timelines.recording && cx.has_global::<DevToolsState>() {
      cx.update_global::<DevToolsState, _>(|state, _cx| state.record_frame());
    }

    let ink = Ink::read(cx);
    let body = match self.tab {
      DevToolsTab::Elements => self.elements.render(&self.tree, window, cx),
      DevToolsTab::Network => self.network.render(window, cx),
      DevToolsTab::Sources => self.sources.render(&self.tree, window, cx),
      DevToolsTab::Timelines => self.timelines.render(window, cx),
      DevToolsTab::Storage => self.storage.render(window, cx),
      DevToolsTab::Layers => {
        elements::layers_view(&self.tree, self.elements.selected.as_ref(), &ink, cx)
      }
      DevToolsTab::Logs => self.logs.render(window, cx),
      DevToolsTab::Audit => self.audit.render(&self.tree, window, cx),
    };

    // The drawer is the log shown *under* another tool, so it never appears
    // while Logs is the tool.
    let drawer = (self.drawer_open && self.tab != DevToolsTab::Logs).then(|| {
      div()
        .flex()
        .flex_col()
        .flex_none()
        .h(px(200.0))
        .w_full()
        .child(hairline(&ink))
        .child(self.logs.render(window, cx))
    });

    div()
      .track_focus(&self.focus)
      .key_context("DevTools")
      .flex()
      .flex_col()
      .size_full()
      .min_h(px(0.0))
      .bg(ink.content)
      .text_color(ink.text)
      .child(self.toolbar(&ink, cx))
      .child(self.tab_bar(&ink, cx))
      .child(
        div()
          .flex()
          .flex_col()
          .flex_1()
          .min_h(px(0.0))
          .w_full()
          .child(body),
      )
      .children(drawer)
      .child(self.status_bar(&ink, cx))
  }
}

impl Drop for DevTools {
  fn drop(&mut self) {
    // The last inspector to close stops the recording, and with it the
    // per-frame cost every probe in the app would otherwise keep paying.
    if self.recorder_active {
      probe::release();
    }
  }
}

// the host-facing feed
//
// Every one of these is a no-op when `DevToolsState` was never installed, so
// instrumentation can be left in place unconditionally. They are
// `#[track_caller]` so the record knows where it came from without the caller
// passing a location or reaching for a macro.

/// Append a log line, stamped with the source location it was called from.
#[track_caller]
pub fn log(cx: &mut App, level: LogLevel, message: impl Into<SharedString>) {
  let source = SourceRef::from(std::panic::Location::caller());
  log_record(cx, LogRecord::new(level, message).source(source));
}

/// Append a fully-built log record — the form to use when the line has
/// expandable detail rows.
pub fn log_record(cx: &mut App, record: LogRecord) {
  if cx.has_global::<DevToolsState>() {
    cx.update_global::<DevToolsState, _>(|state, _cx| state.push_log(record));
  }
}

/// Record a request that has just started. The returned id settles it later;
/// `None` means the inspector is not installed and nothing was recorded.
pub fn network_begin(cx: &mut App, record: NetworkRecord) -> Option<u64> {
  if !cx.has_global::<DevToolsState>() {
    return None;
  }
  Some(cx.update_global::<DevToolsState, _>(|state, _cx| state.push_network(record)))
}

/// Amend a request in flight.
pub fn network_update(cx: &mut App, id: u64, f: impl FnOnce(&mut NetworkRecord)) {
  if cx.has_global::<DevToolsState>() {
    cx.update_global::<DevToolsState, _>(|state, _cx| state.update_network(id, f));
  }
}

/// Publish a storage domain, replacing any previous snapshot of it.
pub fn storage_set(cx: &mut App, domain: StorageDomain) {
  if cx.has_global::<DevToolsState>() {
    cx.update_global::<DevToolsState, _>(|state, _cx| state.set_storage(domain));
  }
}

/// Remove a storage domain.
pub fn storage_remove(cx: &mut App, id: &str) {
  if cx.has_global::<DevToolsState>() {
    cx.update_global::<DevToolsState, _>(|state, _cx| state.remove_storage(id));
  }
}

/// Record a span on a timeline band.
pub fn timeline_event(cx: &mut App, event: TimelineEvent) {
  if cx.has_global::<DevToolsState>() {
    cx.update_global::<DevToolsState, _>(|state, _cx| state.push_timeline(event));
  }
}

/// Time `f` and record it as a Script span. The return value is passed through,
/// so this wraps an existing call without restructuring it.
pub fn measure<R>(cx: &mut App, label: impl Into<SharedString>, f: impl FnOnce() -> R) -> R {
  let start = std::time::Instant::now();
  let result = f();
  timeline_event(
    cx,
    TimelineEvent::new(TimelineKind::Script, label, start.elapsed()),
  );
  result
}

/// Drop every record.
pub fn clear(cx: &mut App) {
  if cx.has_global::<DevToolsState>() {
    cx.update_global::<DevToolsState, _>(|state, _cx| state.clear_all());
  }
}

/// Whether the element recorder is running, for a host that wants to skip its
/// own instrumentation while the inspector is closed.
pub fn is_recording() -> bool {
  probe::is_enabled()
}

#[cfg(test)]
mod tests {
  use std::time::Duration;

  use gpui::{AppContext, TestAppContext};

  use super::{probe, DevTools, RECORDER_IDLE_AFTER};

  #[gpui::test]
  fn recorder_stops_after_the_inspector_stops_rendering(cx: &mut TestAppContext) {
    probe::set_enabled(false);
    let inspector = cx.update(|cx| cx.new(DevTools::new));
    assert!(probe::is_enabled());

    cx.executor()
      .advance_clock(RECORDER_IDLE_AFTER - Duration::from_millis(1));
    inspector.update(cx, |inspector, cx| inspector.activate_recorder(cx));
    cx.executor().advance_clock(Duration::from_millis(1));
    cx.run_until_parked();
    assert!(probe::is_enabled(), "a recent render should keep recording");

    cx.executor().advance_clock(RECORDER_IDLE_AFTER);
    cx.run_until_parked();
    assert!(
      !probe::is_enabled(),
      "an idle inspector should release its tree"
    );
  }
}
