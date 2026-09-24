//! The record store behind every devtools panel.
//!
//! Safari's Web Inspector reads a live page; there is no equivalent firehose in
//! a native app, so `guise` inverts it: the host *reports* what it does — a log
//! line, a request, a storage domain — and this global keeps the rolling
//! history the panels render. Nothing here opens a socket or reads a file, the
//! same way [`crate::ai`] never issues the request it displays.
//!
//! Every store is a ring: capped, oldest-first eviction, so a long-running app
//! cannot grow the inspector without bound. `generation` ticks on every
//! mutation, which is what lets a panel skip work when nothing changed.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use gpui::{App, Global, SharedString};

/// Where a record came from in the source. Built from a gpui element's
/// `#[track_caller]` location, or supplied by the host for its own records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRef {
  pub file: SharedString,
  pub line: u32,
  pub column: u32,
}

impl SourceRef {
  pub fn new(file: impl Into<SharedString>, line: u32, column: u32) -> Self {
    SourceRef {
      file: file.into(),
      line,
      column,
    }
  }

  /// Just the file name, which is all the one-line displays have room for.
  pub fn basename(&self) -> &str {
    let file = self.file.as_ref();
    match file.rsplit_once('/') {
      Some((_, name)) => name,
      None => file,
    }
  }

  /// `foo.rs:12:5`, the form both Safari and rustc print.
  pub fn short(&self) -> String {
    format!("{}:{}:{}", self.basename(), self.line, self.column)
  }
}

impl From<&'static std::panic::Location<'static>> for SourceRef {
  fn from(loc: &'static std::panic::Location<'static>) -> Self {
    SourceRef::new(loc.file(), loc.line(), loc.column())
  }
}

/// Log severity. Mirrors the levels Safari's console filters by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LogLevel {
  /// `console.log` — no icon, no color.
  #[default]
  Log,
  /// `console.debug` — dimmed.
  Debug,
  /// `console.info` — blue dot.
  Info,
  /// `console.warn` — amber, counted in the toolbar.
  Warning,
  /// `console.error` — red, counted in the toolbar.
  Error,
}

impl LogLevel {
  pub fn label(self) -> &'static str {
    match self {
      LogLevel::Log => "Log",
      LogLevel::Debug => "Debug",
      LogLevel::Info => "Info",
      LogLevel::Warning => "Warning",
      LogLevel::Error => "Error",
    }
  }

  /// Whether the toolbar's warning/error badges count this level.
  pub fn is_issue(self) -> bool {
    matches!(self, LogLevel::Warning | LogLevel::Error)
  }
}

/// One log line. `count` is the repeat tally: Safari collapses identical
/// consecutive messages into a single row with a counter rather than scrolling
/// the useful history away, and so do we.
#[derive(Debug, Clone)]
pub struct LogRecord {
  pub id: u64,
  pub level: LogLevel,
  pub message: SharedString,
  /// Expandable key/value rows shown when the row is disclosed — the native
  /// stand-in for expanding a logged object.
  pub details: Vec<(SharedString, SharedString)>,
  pub source: Option<SourceRef>,
  pub at: Duration,
  pub count: usize,
}

impl LogRecord {
  pub fn new(level: LogLevel, message: impl Into<SharedString>) -> Self {
    LogRecord {
      id: 0,
      level,
      message: message.into(),
      details: Vec::new(),
      source: None,
      at: Duration::ZERO,
      count: 1,
    }
  }

  pub fn detail(mut self, key: impl Into<SharedString>, value: impl Into<SharedString>) -> Self {
    self.details.push((key.into(), value.into()));
    self
  }

  pub fn details(mut self, rows: impl IntoIterator<Item = (SharedString, SharedString)>) -> Self {
    self.details.extend(rows);
    self
  }

  pub fn source(mut self, source: SourceRef) -> Self {
    self.source = Some(source);
    self
  }

  /// Two records coalesce when they would render identically.
  fn same_as(&self, other: &LogRecord) -> bool {
    self.level == other.level && self.message == other.message && self.details == other.details
  }
}

/// What kind of resource a request fetched. Drives the Network panel's type
/// filter and the waterfall color, exactly as in Safari.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ResourceKind {
  Document,
  Stylesheet,
  Image,
  Font,
  Script,
  /// `XHR` and `fetch` in Safari; any app-level API call here.
  Fetch,
  WebSocket,
  Media,
  #[default]
  Other,
}

impl ResourceKind {
  pub fn label(self) -> &'static str {
    match self {
      ResourceKind::Document => "Document",
      ResourceKind::Stylesheet => "Stylesheet",
      ResourceKind::Image => "Image",
      ResourceKind::Font => "Font",
      ResourceKind::Script => "Script",
      ResourceKind::Fetch => "Fetch",
      ResourceKind::WebSocket => "Socket",
      ResourceKind::Media => "Media",
      ResourceKind::Other => "Other",
    }
  }

  /// The set the Network panel's type filter offers, in Safari's order.
  pub const ALL: [ResourceKind; 9] = [
    ResourceKind::Document,
    ResourceKind::Stylesheet,
    ResourceKind::Image,
    ResourceKind::Font,
    ResourceKind::Script,
    ResourceKind::Fetch,
    ResourceKind::WebSocket,
    ResourceKind::Media,
    ResourceKind::Other,
  ];
}

/// The phase breakdown behind the waterfall bar. Each field is the time spent
/// in that phase, not a timestamp, so a partially-complete request is just one
/// with later phases still zero.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Timings {
  pub stalled: Duration,
  pub dns: Duration,
  pub connect: Duration,
  pub tls: Duration,
  pub request: Duration,
  pub response: Duration,
}

impl Timings {
  pub fn total(&self) -> Duration {
    self.stalled + self.dns + self.connect + self.tls + self.request + self.response
  }

  /// The phases in waterfall order, skipping the ones that took no time.
  pub fn phases(&self) -> Vec<(&'static str, Duration)> {
    [
      ("Stalled", self.stalled),
      ("DNS", self.dns),
      ("Connect", self.connect),
      ("Secure", self.tls),
      ("Request", self.request),
      ("Response", self.response),
    ]
    .into_iter()
    .filter(|(_, d)| !d.is_zero())
    .collect()
  }
}

/// How far along a request is. A record starts `Pending` and the host settles
/// it later by id — the inspector shows the row the whole time, as Safari does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RequestState {
  #[default]
  Pending,
  Finished,
  Failed,
  Canceled,
}

/// One network request.
#[derive(Debug, Clone)]
pub struct NetworkRecord {
  pub id: u64,
  pub method: SharedString,
  pub url: SharedString,
  pub kind: ResourceKind,
  pub state: RequestState,
  pub status: Option<u16>,
  pub status_text: SharedString,
  pub protocol: SharedString,
  pub remote_address: SharedString,
  pub priority: SharedString,
  /// Bytes on the wire, after compression.
  pub transfer_size: u64,
  /// Bytes after decoding — what the app actually received.
  pub resource_size: u64,
  pub cached: bool,
  pub request_headers: Vec<(SharedString, SharedString)>,
  pub response_headers: Vec<(SharedString, SharedString)>,
  pub request_body: Option<SharedString>,
  pub response_body: Option<SharedString>,
  pub initiator: Option<SourceRef>,
  pub error: Option<SharedString>,
  /// When the request started, relative to the store's epoch. The waterfall
  /// lays rows out against this.
  pub start: Duration,
  pub timings: Timings,
}

impl NetworkRecord {
  pub fn new(method: impl Into<SharedString>, url: impl Into<SharedString>) -> Self {
    NetworkRecord {
      id: 0,
      method: method.into(),
      url: url.into(),
      kind: ResourceKind::default(),
      state: RequestState::Pending,
      status: None,
      status_text: SharedString::default(),
      protocol: SharedString::default(),
      remote_address: SharedString::default(),
      priority: SharedString::default(),
      transfer_size: 0,
      resource_size: 0,
      cached: false,
      request_headers: Vec::new(),
      response_headers: Vec::new(),
      request_body: None,
      response_body: None,
      initiator: None,
      error: None,
      start: Duration::ZERO,
      timings: Timings::default(),
    }
  }

  pub fn kind(mut self, kind: ResourceKind) -> Self {
    self.kind = kind;
    self
  }

  pub fn status(mut self, status: u16, text: impl Into<SharedString>) -> Self {
    self.status = Some(status);
    self.status_text = text.into();
    self
  }

  pub fn sizes(mut self, transfer: u64, resource: u64) -> Self {
    self.transfer_size = transfer;
    self.resource_size = resource;
    self
  }

  pub fn timings(mut self, timings: Timings) -> Self {
    self.timings = timings;
    self
  }

  pub fn request_header(
    mut self,
    name: impl Into<SharedString>,
    value: impl Into<SharedString>,
  ) -> Self {
    self.request_headers.push((name.into(), value.into()));
    self
  }

  pub fn response_header(
    mut self,
    name: impl Into<SharedString>,
    value: impl Into<SharedString>,
  ) -> Self {
    self.response_headers.push((name.into(), value.into()));
    self
  }

  pub fn request_body(mut self, body: impl Into<SharedString>) -> Self {
    self.request_body = Some(body.into());
    self
  }

  pub fn response_body(mut self, body: impl Into<SharedString>) -> Self {
    self.response_body = Some(body.into());
    self
  }

  pub fn initiator(mut self, source: SourceRef) -> Self {
    self.initiator = Some(source);
    self
  }

  pub fn finished(mut self) -> Self {
    self.state = RequestState::Finished;
    self
  }

  pub fn failed(mut self, error: impl Into<SharedString>) -> Self {
    self.state = RequestState::Failed;
    self.error = Some(error.into());
    self
  }

  /// The Name column: the last path segment, or the host for a bare origin.
  pub fn name(&self) -> &str {
    let url = self.url.as_ref();
    let path = url
      .split_once("://")
      .map(|(_, rest)| rest)
      .unwrap_or(url)
      .split(['?', '#'])
      .next()
      .unwrap_or("");
    match path.rsplit_once('/') {
      Some((_, last)) if !last.is_empty() => last,
      _ => path.split('/').next().unwrap_or(url),
    }
  }

  /// The Domain column.
  pub fn domain(&self) -> &str {
    let url = self.url.as_ref();
    let rest = url.split_once("://").map(|(_, r)| r).unwrap_or(url);
    rest.split(['/', '?', '#']).next().unwrap_or(rest)
  }

  /// The Scheme column.
  pub fn scheme(&self) -> &str {
    self
      .url
      .as_ref()
      .split_once("://")
      .map(|(s, _)| s)
      .unwrap_or("")
  }

  /// Whether the row renders in the error color: a transport failure, or any
  /// 4xx/5xx response.
  pub fn is_error(&self) -> bool {
    self.state == RequestState::Failed || self.status.is_some_and(|s| s >= 400)
  }

  pub fn duration(&self) -> Duration {
    self.timings.total()
  }
}

/// What a storage domain holds. Only affects the icon and the sidebar grouping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StorageKind {
  /// Persisted settings — the native analogue of Local Storage.
  #[default]
  Local,
  /// In-memory state that dies with the process.
  Session,
  Cookies,
  /// A structured store: a database, an index.
  Database,
  Cache,
}

impl StorageKind {
  pub fn label(self) -> &'static str {
    match self {
      StorageKind::Local => "Local Storage",
      StorageKind::Session => "Session Storage",
      StorageKind::Cookies => "Cookies",
      StorageKind::Database => "Databases",
      StorageKind::Cache => "Caches",
    }
  }
}

/// One row in a storage domain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageEntry {
  pub key: SharedString,
  pub value: SharedString,
  /// Extra columns — cookie domain/path/expiry, a record's type, and so on.
  pub extra: Vec<(SharedString, SharedString)>,
}

impl StorageEntry {
  pub fn new(key: impl Into<SharedString>, value: impl Into<SharedString>) -> Self {
    StorageEntry {
      key: key.into(),
      value: value.into(),
      extra: Vec::new(),
    }
  }

  pub fn extra(mut self, name: impl Into<SharedString>, value: impl Into<SharedString>) -> Self {
    self.extra.push((name.into(), value.into()));
    self
  }
}

/// A named collection of storage rows, listed in the Storage panel's sidebar.
#[derive(Debug, Clone)]
pub struct StorageDomain {
  pub id: SharedString,
  pub name: SharedString,
  pub kind: StorageKind,
  pub entries: Vec<StorageEntry>,
  /// Extra column headers beyond Key and Value.
  pub columns: Vec<SharedString>,
}

impl StorageDomain {
  pub fn new(id: impl Into<SharedString>, name: impl Into<SharedString>) -> Self {
    StorageDomain {
      id: id.into(),
      name: name.into(),
      kind: StorageKind::default(),
      entries: Vec::new(),
      columns: Vec::new(),
    }
  }

  pub fn kind(mut self, kind: StorageKind) -> Self {
    self.kind = kind;
    self
  }

  pub fn columns(mut self, columns: impl IntoIterator<Item = SharedString>) -> Self {
    self.columns = columns.into_iter().collect();
    self
  }

  pub fn entry(mut self, entry: StorageEntry) -> Self {
    self.entries.push(entry);
    self
  }

  pub fn entries(mut self, entries: impl IntoIterator<Item = StorageEntry>) -> Self {
    self.entries.extend(entries);
    self
  }

  /// Total bytes, as the Storage panel's footer reports.
  pub fn size(&self) -> u64 {
    self
      .entries
      .iter()
      .map(|e| (e.key.len() + e.value.len()) as u64)
      .sum()
  }
}

/// A band on the Timelines panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TimelineKind {
  /// A rendered frame — the band that reveals dropped frames.
  #[default]
  Frame,
  Layout,
  Paint,
  /// App work: a handler, a task, a computation.
  Script,
  Network,
}

impl TimelineKind {
  pub fn label(self) -> &'static str {
    match self {
      TimelineKind::Frame => "Frames",
      TimelineKind::Layout => "Layout",
      TimelineKind::Paint => "Rendering",
      TimelineKind::Script => "JavaScript & Events",
      TimelineKind::Network => "Network Requests",
    }
  }

  pub const ALL: [TimelineKind; 5] = [
    TimelineKind::Frame,
    TimelineKind::Layout,
    TimelineKind::Paint,
    TimelineKind::Script,
    TimelineKind::Network,
  ];
}

/// One span on a timeline band.
#[derive(Debug, Clone)]
pub struct TimelineEvent {
  pub id: u64,
  pub kind: TimelineKind,
  pub label: SharedString,
  pub start: Duration,
  pub duration: Duration,
  pub source: Option<SourceRef>,
}

impl TimelineEvent {
  pub fn new(kind: TimelineKind, label: impl Into<SharedString>, duration: Duration) -> Self {
    TimelineEvent {
      id: 0,
      kind,
      label: label.into(),
      start: Duration::ZERO,
      duration,
      source: None,
    }
  }

  pub fn end(&self) -> Duration {
    self.start + self.duration
  }
}

/// How many records each store keeps before evicting the oldest.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
  pub logs: usize,
  pub network: usize,
  pub timeline: usize,
}

impl Default for Limits {
  fn default() -> Self {
    Limits {
      logs: 1000,
      network: 1000,
      timeline: 4000,
    }
  }
}

/// Everything the inspector displays, and the only mutable state it owns.
///
/// Installed once with [`DevToolsState::init`]; feed it through the free
/// functions in [`crate::devtools`] (`console_log`, `network_begin`, …), which
/// are no-ops when it was never installed. That is deliberate: instrumentation
/// left in a release build costs a global lookup and nothing more.
pub struct DevToolsState {
  epoch: Instant,
  next_id: u64,
  generation: u64,
  limits: Limits,
  logs: VecDeque<LogRecord>,
  network: VecDeque<NetworkRecord>,
  timeline: VecDeque<TimelineEvent>,
  storage: Vec<StorageDomain>,
  /// Wall-clock frame durations, newest last — the Timelines FPS graph.
  frames: VecDeque<Duration>,
  last_frame: Option<Instant>,
}

impl Global for DevToolsState {}

impl Default for DevToolsState {
  fn default() -> Self {
    DevToolsState::new()
  }
}

impl DevToolsState {
  pub fn new() -> Self {
    DevToolsState {
      epoch: Instant::now(),
      next_id: 1,
      generation: 0,
      limits: Limits::default(),
      logs: VecDeque::new(),
      network: VecDeque::new(),
      timeline: VecDeque::new(),
      storage: Vec::new(),
      frames: VecDeque::new(),
      last_frame: None,
    }
  }

  pub fn limits(mut self, limits: Limits) -> Self {
    self.limits = limits;
    self
  }

  /// Install as the app global. Call once at startup, before opening
  /// [`crate::devtools::DevTools`].
  pub fn init(self, cx: &mut App) {
    cx.set_global(self);
  }

  /// Bumped on every mutation, so a panel can tell "nothing changed" cheaply.
  pub fn generation(&self) -> u64 {
    self.generation
  }

  /// Time since the store was created — the clock every record is stamped on.
  pub fn now(&self) -> Duration {
    self.epoch.elapsed()
  }

  fn tick(&mut self) -> u64 {
    self.generation += 1;
    let id = self.next_id;
    self.next_id += 1;
    id
  }

  // logs

  /// Append a line, coalescing it into the previous one when identical.
  pub fn push_log(&mut self, mut record: LogRecord) {
    if let Some(last) = self.logs.back_mut() {
      if last.same_as(&record) {
        last.count += 1;
        self.generation += 1;
        return;
      }
    }
    record.id = self.tick();
    record.at = self.now();
    self.logs.push_back(record);
    while self.logs.len() > self.limits.logs {
      self.logs.pop_front();
    }
  }

  pub fn logs(&self) -> &VecDeque<LogRecord> {
    &self.logs
  }

  pub fn clear_logs(&mut self) {
    self.logs.clear();
    self.generation += 1;
  }

  /// Warning and error tallies for the toolbar badges.
  pub fn log_issues(&self) -> (usize, usize) {
    let mut warnings = 0;
    let mut errors = 0;
    for record in &self.logs {
      match record.level {
        LogLevel::Warning => warnings += record.count,
        LogLevel::Error => errors += record.count,
        _ => {}
      }
    }
    (warnings, errors)
  }

  // network

  /// Record a request that has started. Returns its id, which the host keeps
  /// to settle the request later.
  pub fn push_network(&mut self, mut record: NetworkRecord) -> u64 {
    let id = self.tick();
    record.id = id;
    if record.start.is_zero() {
      record.start = self.now();
    }
    self.network.push_back(record);
    while self.network.len() > self.limits.network {
      self.network.pop_front();
    }
    id
  }

  /// Amend a request in flight — the response landed, the transfer grew, it
  /// failed. Silently does nothing if the record was already evicted.
  pub fn update_network(&mut self, id: u64, f: impl FnOnce(&mut NetworkRecord)) {
    if let Some(record) = self.network.iter_mut().find(|r| r.id == id) {
      f(record);
      self.generation += 1;
    }
  }

  pub fn network(&self) -> &VecDeque<NetworkRecord> {
    &self.network
  }

  pub fn clear_network(&mut self) {
    self.network.clear();
    self.generation += 1;
  }

  /// The window the waterfall is drawn against: earliest start to latest end.
  pub fn network_span(&self) -> (Duration, Duration) {
    let start = self
      .network
      .iter()
      .map(|r| r.start)
      .min()
      .unwrap_or(Duration::ZERO);
    let end = self
      .network
      .iter()
      .map(|r| r.start + r.duration())
      .max()
      .unwrap_or(Duration::ZERO);
    (start, end.max(start))
  }

  /// Row count, total transfer, and total resource bytes — the status bar.
  pub fn network_totals(&self) -> (usize, u64, u64) {
    let transfer = self.network.iter().map(|r| r.transfer_size).sum();
    let resource = self.network.iter().map(|r| r.resource_size).sum();
    (self.network.len(), transfer, resource)
  }

  // storage

  /// Register a domain, replacing any existing one with the same id. Hosts
  /// call this whenever their store changes; the panel always shows the
  /// latest snapshot.
  pub fn set_storage(&mut self, domain: StorageDomain) {
    self.generation += 1;
    match self.storage.iter_mut().find(|d| d.id == domain.id) {
      Some(existing) => *existing = domain,
      None => self.storage.push(domain),
    }
  }

  pub fn remove_storage(&mut self, id: &str) {
    self.storage.retain(|d| d.id.as_ref() != id);
    self.generation += 1;
  }

  pub fn storage(&self) -> &[StorageDomain] {
    &self.storage
  }

  // timelines

  pub fn push_timeline(&mut self, mut event: TimelineEvent) {
    event.id = self.tick();
    if event.start.is_zero() {
      event.start = self.now().saturating_sub(event.duration);
    }
    self.timeline.push_back(event);
    while self.timeline.len() > self.limits.timeline {
      self.timeline.pop_front();
    }
  }

  pub fn timeline(&self) -> &VecDeque<TimelineEvent> {
    &self.timeline
  }

  pub fn clear_timeline(&mut self) {
    self.timeline.clear();
    self.frames.clear();
    self.last_frame = None;
    self.generation += 1;
  }

  /// Called once per rendered frame by the inspector itself. Deriving the
  /// interval here — rather than asking the host to report it — is what makes
  /// the Frames band work with no wiring at all.
  pub fn record_frame(&mut self) {
    let now = Instant::now();
    if let Some(previous) = self.last_frame.replace(now) {
      let delta = now.duration_since(previous);
      // A frame gap longer than a second means the window was idle, not
      // slow; counting it would flatten the graph for minutes.
      if delta < Duration::from_secs(1) {
        self.frames.push_back(delta);
        while self.frames.len() > 240 {
          self.frames.pop_front();
        }
      }
    }
  }

  /// Forget where the last frame landed. Called when recording stops so the
  /// idle gap before it resumes is not measured as one enormous frame.
  pub fn stop_frames(&mut self) {
    self.last_frame = None;
  }

  pub fn frames(&self) -> &VecDeque<Duration> {
    &self.frames
  }

  /// Frames per second over the recorded window, or `None` before the second
  /// frame has been seen.
  pub fn fps(&self) -> Option<f32> {
    if self.frames.is_empty() {
      return None;
    }
    let total: Duration = self.frames.iter().sum();
    if total.is_zero() {
      return None;
    }
    Some(self.frames.len() as f32 / total.as_secs_f32())
  }

  /// Drop every record. The toolbar's clear button, and what a host calls
  /// when it wants a clean slate around a reproduction.
  pub fn clear_all(&mut self) {
    self.logs.clear();
    self.network.clear();
    self.timeline.clear();
    self.frames.clear();
    self.last_frame = None;
    self.generation += 1;
  }
}

/// Format a byte count the way Safari's size columns do.
pub fn format_bytes(bytes: u64) -> String {
  const KB: f64 = 1024.0;
  let bytes = bytes as f64;
  if bytes < KB {
    format!("{} B", bytes as u64)
  } else if bytes < KB * KB {
    format!("{:.1} KB", bytes / KB)
  } else if bytes < KB * KB * KB {
    format!("{:.2} MB", bytes / (KB * KB))
  } else {
    format!("{:.2} GB", bytes / (KB * KB * KB))
  }
}

/// Format a duration the way Safari's timing columns do: sub-millisecond work
/// still reads as a number, and anything past a second switches unit.
pub fn format_duration(duration: Duration) -> String {
  let ms = duration.as_secs_f64() * 1000.0;
  if ms < 1.0 {
    format!("{:.2} ms", ms)
  } else if ms < 1000.0 {
    format!("{:.0} ms", ms)
  } else {
    format!("{:.2} s", ms / 1000.0)
  }
}

/// Elapsed time as the log's timestamp column shows it.
pub fn format_timestamp(at: Duration) -> String {
  let total = at.as_secs();
  let minutes = total / 60;
  let seconds = total % 60;
  format!("{:02}:{:02}.{:03}", minutes, seconds, at.subsec_millis())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn identical_log_lines_coalesce() {
    let mut state = DevToolsState::new();
    state.push_log(LogRecord::new(LogLevel::Warning, "slow frame"));
    state.push_log(LogRecord::new(LogLevel::Warning, "slow frame"));
    state.push_log(LogRecord::new(LogLevel::Warning, "slow frame"));

    assert_eq!(state.logs().len(), 1);
    assert_eq!(state.logs()[0].count, 3);
    assert_eq!(state.log_issues(), (3, 0));
  }

  #[test]
  fn a_different_line_breaks_the_run() {
    let mut state = DevToolsState::new();
    state.push_log(LogRecord::new(LogLevel::Log, "a"));
    state.push_log(LogRecord::new(LogLevel::Log, "b"));
    state.push_log(LogRecord::new(LogLevel::Log, "a"));

    assert_eq!(state.logs().len(), 3);
    assert!(state.logs().iter().all(|r| r.count == 1));
  }

  #[test]
  fn logs_evict_oldest_past_the_limit() {
    let mut state = DevToolsState::new().limits(Limits {
      logs: 3,
      ..Limits::default()
    });
    for i in 0..6 {
      state.push_log(LogRecord::new(LogLevel::Log, format!("line {i}")));
    }

    assert_eq!(state.logs().len(), 3);
    assert_eq!(state.logs()[0].message.as_ref(), "line 3");
    assert_eq!(state.logs()[2].message.as_ref(), "line 5");
  }

  #[test]
  fn a_request_settles_by_id() {
    let mut state = DevToolsState::new();
    let id = state.push_network(NetworkRecord::new(
      "GET",
      "https://api.example.com/v1/users",
    ));
    assert_eq!(state.network()[0].state, RequestState::Pending);

    state.update_network(id, |record| {
      record.state = RequestState::Finished;
      record.status = Some(200);
    });

    assert_eq!(state.network()[0].state, RequestState::Finished);
    assert_eq!(state.network()[0].status, Some(200));
  }

  #[test]
  fn settling_an_evicted_request_is_a_no_op() {
    let mut state = DevToolsState::new().limits(Limits {
      network: 1,
      ..Limits::default()
    });
    let first = state.push_network(NetworkRecord::new("GET", "https://example.com/a"));
    state.push_network(NetworkRecord::new("GET", "https://example.com/b"));

    state.update_network(first, |record| record.status = Some(500));

    assert_eq!(state.network().len(), 1);
    assert_eq!(state.network()[0].status, None);
  }

  #[test]
  fn url_splits_into_name_domain_and_scheme() {
    let record = NetworkRecord::new("GET", "https://api.example.com/v1/users?page=2");
    assert_eq!(record.name(), "users");
    assert_eq!(record.domain(), "api.example.com");
    assert_eq!(record.scheme(), "https");

    let root = NetworkRecord::new("GET", "https://example.com/");
    assert_eq!(root.domain(), "example.com");

    let bare = NetworkRecord::new("GET", "https://example.com");
    assert_eq!(bare.name(), "example.com");
  }

  #[test]
  fn errors_are_status_or_transport() {
    assert!(NetworkRecord::new("GET", "/a")
      .status(404, "Not Found")
      .is_error());
    assert!(NetworkRecord::new("GET", "/a")
      .status(500, "Server Error")
      .is_error());
    assert!(!NetworkRecord::new("GET", "/a")
      .status(304, "Not Modified")
      .is_error());
    assert!(NetworkRecord::new("GET", "/a").failed("offline").is_error());
  }

  #[test]
  fn timings_sum_and_drop_empty_phases() {
    let timings = Timings {
      dns: Duration::from_millis(4),
      connect: Duration::from_millis(11),
      response: Duration::from_millis(35),
      ..Timings::default()
    };

    assert_eq!(timings.total(), Duration::from_millis(50));
    assert_eq!(
      timings.phases().iter().map(|(n, _)| *n).collect::<Vec<_>>(),
      vec!["DNS", "Connect", "Response"]
    );
  }

  #[test]
  fn registering_a_storage_domain_twice_replaces_it() {
    let mut state = DevToolsState::new();
    state
      .set_storage(StorageDomain::new("prefs", "Preferences").entry(StorageEntry::new("a", "1")));
    state.set_storage(
      StorageDomain::new("prefs", "Preferences")
        .entry(StorageEntry::new("a", "2"))
        .entry(StorageEntry::new("b", "3")),
    );

    assert_eq!(state.storage().len(), 1);
    assert_eq!(state.storage()[0].entries.len(), 2);
    assert_eq!(state.storage()[0].entries[0].value.as_ref(), "2");
  }

  #[test]
  fn network_span_covers_every_row() {
    let mut state = DevToolsState::new();
    let mut first = NetworkRecord::new("GET", "/a");
    first.start = Duration::from_millis(100);
    first.timings.response = Duration::from_millis(50);
    let mut second = NetworkRecord::new("GET", "/b");
    second.start = Duration::from_millis(20);
    second.timings.response = Duration::from_millis(10);
    state.push_network(first);
    state.push_network(second);

    let (start, end) = state.network_span();
    assert_eq!(start, Duration::from_millis(20));
    assert_eq!(end, Duration::from_millis(150));
  }

  #[test]
  fn byte_and_duration_formats_match_the_columns() {
    assert_eq!(format_bytes(512), "512 B");
    assert_eq!(format_bytes(2048), "2.0 KB");
    assert_eq!(format_bytes(5 * 1024 * 1024), "5.00 MB");

    assert_eq!(format_duration(Duration::from_micros(250)), "0.25 ms");
    assert_eq!(format_duration(Duration::from_millis(42)), "42 ms");
    assert_eq!(format_duration(Duration::from_millis(1500)), "1.50 s");

    assert_eq!(format_timestamp(Duration::from_millis(63_042)), "01:03.042");
  }

  #[test]
  fn source_refs_shorten_to_basename() {
    let source = SourceRef::new("crates/guise/src/button.rs", 42, 9);
    assert_eq!(source.basename(), "button.rs");
    assert_eq!(source.short(), "button.rs:42:9");
  }
}
