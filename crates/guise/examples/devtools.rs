//! The inspector, docked beside an app to inspect:
//! `cargo run -p guise-ui --example devtools`
//!
//! The left half is an ordinary `guise` UI. The right half is [`DevTools`],
//! reading that UI's live component tree and the records this example reports
//! as you click around.
//!
//! Pass a tab name to open on it: `--example devtools -- network`.

use std::time::Duration;

use gpui::prelude::*;
use gpui::{
  div, px, size, App, Bounds, Context, Entity, IntoElement, SharedString, Window, WindowBounds,
  WindowOptions,
};
use guise::prelude::*;
use guise::theme::{ColorName, Size, Theme};

struct Demo {
  devtools: Entity<DevTools>,
  checked: bool,
  plan: Signal<usize>,
  requests: u32,
  name: Entity<TextInput>,
}

impl Demo {
  fn new(cx: &mut Context<Self>) -> Self {
    let devtools = cx.new(|cx| DevTools::new(cx).tab(starting_tab()));
    let name = cx.new(|cx| {
      TextInput::new(cx)
        .label("Project")
        .placeholder("guise")
        .size(Size::Sm)
    });

    // The console prompt has nothing to evaluate against in a compiled
    // binary, so the host answers it — which is the point of the event.
    cx.subscribe(
      &devtools,
      |_this: &mut Demo, _devtools, event: &DevToolsEvent, cx| {
        // The one thing the inspector cannot do alone: open a file in
        // whatever the host calls an editor.
        if let DevToolsEvent::RevealSource(source) = event {
          guise::devtools::log(
            cx,
            LogLevel::Info,
            format!("Host asked to open {}", source.short()),
          );
        }
      },
    )
    .detach();

    // Publish a storage domain up front so the Storage panel opens on
    // something rather than on an empty state.
    guise::devtools::storage_set(
      cx,
      StorageDomain::new("prefs", "example.preferences")
        .kind(StorageKind::Local)
        .entry(StorageEntry::new("theme", "dark"))
        .entry(StorageEntry::new("plan", "0"))
        .entry(StorageEntry::new("window", "1280×820")),
    );
    guise::devtools::log(
      cx,
      LogLevel::Info,
      "Inspector attached. Click around on the left; the tree updates every frame.",
    );
    guise::devtools::log_record(
      cx,
      LogRecord::new(LogLevel::Warning, "Theme has no explicit `info` override")
        .detail("scheme", "Dark")
        .detail("falling back to", "Cyan/8"),
    );

    // A little seeded history so every panel opens on something. A real app
    // would have reported these as the work actually happened.
    let seeded: [(&str, &str, ResourceKind, u16, u64, u64, u64); 5] = [
      (
        "GET",
        "https://api.example.com/v1/session",
        ResourceKind::Document,
        200,
        1_204,
        3_180,
        46,
      ),
      (
        "GET",
        "https://cdn.example.com/assets/app.css",
        ResourceKind::Stylesheet,
        200,
        8_940,
        41_220,
        88,
      ),
      (
        "GET",
        "https://cdn.example.com/assets/Inter.woff2",
        ResourceKind::Font,
        200,
        31_002,
        31_002,
        132,
      ),
      (
        "POST",
        "https://api.example.com/v1/events",
        ResourceKind::Fetch,
        202,
        612,
        612,
        24,
      ),
      (
        "GET",
        "https://api.example.com/v1/items?page=1",
        ResourceKind::Fetch,
        503,
        344,
        344,
        214,
      ),
    ];
    for (index, (method, url, kind, status, transfer, resource, ms)) in
      seeded.into_iter().enumerate()
    {
      let mut record = NetworkRecord::new(method, url)
        .kind(kind)
        .status(
          status,
          if status < 400 {
            "OK"
          } else {
            "Service Unavailable"
          },
        )
        .sizes(transfer, resource)
        .timings(Timings {
          stalled: Duration::from_millis(1 + index as u64),
          dns: Duration::from_millis(4),
          connect: Duration::from_millis(9),
          tls: Duration::from_millis(16),
          request: Duration::from_millis(2),
          response: Duration::from_millis(ms),
        })
        .request_header("Accept", "*/*")
        .request_header("Cookie", "session=8f2c1a; theme=dark")
        .response_header("Content-Type", "application/json")
        .response_header("Set-Cookie", "session=8f2c1a; Path=/; HttpOnly")
        .response_body("{\n  \"items\": [],\n  \"page\": 1\n}");
      record.protocol = SharedString::new_static("h2");
      record.priority = SharedString::new_static("High");
      record.remote_address = SharedString::new_static("93.184.216.34:443");
      record.start = Duration::from_millis(index as u64 * 60);
      record.state = if status >= 400 {
        RequestState::Failed
      } else {
        RequestState::Finished
      };
      guise::devtools::network_begin(cx, record);
    }

    for (label, kind, ms) in [
      ("Theme::init", TimelineKind::Script, 3u64),
      ("layout pass", TimelineKind::Layout, 12),
      ("paint", TimelineKind::Paint, 7),
      ("GET /v1/session", TimelineKind::Network, 46),
      ("reindex()", TimelineKind::Script, 28),
    ] {
      guise::devtools::timeline_event(
        cx,
        TimelineEvent::new(kind, label, Duration::from_millis(ms)),
      );
    }

    let plan = use_state(cx, 0usize);

    Demo {
      devtools,
      checked: true,
      plan,
      requests: 0,
      name,
    }
  }

  /// Report a request the way a host with a real HTTP client would: open the
  /// record when it starts, settle it by id when it finishes.
  fn fetch(&mut self, cx: &mut Context<Self>) {
    self.requests += 1;
    let n = self.requests;

    let id = guise::devtools::network_begin(
      cx,
      NetworkRecord::new("GET", format!("https://api.example.com/v1/items?page={n}"))
        .kind(ResourceKind::Fetch)
        .request_header("Accept", "application/json")
        .request_header("Cookie", "session=8f2c1a; theme=dark"),
    );

    let Some(id) = id else { return };
    let failed = n.is_multiple_of(4);
    guise::devtools::network_update(cx, id, move |record| {
      record.timings = Timings {
        stalled: Duration::from_millis(2),
        dns: Duration::from_millis(4),
        connect: Duration::from_millis(11),
        tls: Duration::from_millis(18),
        request: Duration::from_millis(3),
        response: Duration::from_millis(21 + u64::from(n % 6) * 14),
      };
      record.transfer_size = 4_200 + u64::from(n) * 137;
      record.resource_size = 11_800 + u64::from(n) * 402;
      record.protocol = SharedString::new_static("h2");
      record.remote_address = SharedString::new_static("93.184.216.34:443");
      record.priority = SharedString::new_static("High");
      record
        .response_headers
        .push(("Content-Type".into(), "application/json".into()));
      record.response_headers.push((
        "Set-Cookie".into(),
        "session=8f2c1a; Path=/; HttpOnly".into(),
      ));
      record.response_body = Some("{\n  \"items\": [],\n  \"page\": 1\n}".into());
      if failed {
        record.state = RequestState::Failed;
        record.status = Some(503);
        record.status_text = SharedString::new_static("Service Unavailable");
        record.error = Some(SharedString::new_static("upstream timed out"));
      } else {
        record.state = RequestState::Finished;
        record.status = Some(200);
        record.status_text = SharedString::new_static("OK");
      }
    });

    if failed {
      guise::devtools::log(
        cx,
        LogLevel::Error,
        format!("Request {n} failed: upstream timed out"),
      );
    }
  }
}

impl Render for Demo {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let t = cx.global::<Theme>();
    let body = t.body().hsla();
    let text = t.text().hsla();
    let border = t.border().hsla();
    let font = t.font_family.clone();

    let app = Stack::new()
      .gap(Size::Lg)
      .child(Title::new("A small app").order(2))
      .child(
        Text::new(
          "Everything here reports itself to the inspector on the right. \
                     Open Elements and select a node to read its real style.",
        )
        .size(Size::Sm)
        .dimmed(),
      )
      .child(self.name.clone())
      .child(
        Group::new()
          .gap(Size::Sm)
          .child(
            Button::new("fetch", "Fetch items")
              .variant(Variant::Filled)
              .on_click(cx.listener(|this: &mut Demo, _, _, cx| {
                this.fetch(cx);
                cx.notify();
              })),
          )
          .child(
            Button::new("work", "Slow work")
              .variant(Variant::Default)
              .on_click(cx.listener(|_this, _, _, cx| {
                guise::devtools::measure(cx, "reindex()", || {
                  std::thread::sleep(Duration::from_millis(28));
                });
                cx.notify();
              })),
          )
          .child(
            Button::new("warn", "Warn")
              .variant(Variant::Light)
              .color(ColorName::Yellow)
              .on_click(cx.listener(|_this, _, _, cx| {
                guise::devtools::log_record(
                  cx,
                  LogRecord::new(LogLevel::Warning, "Layout pass exceeded the frame budget")
                    .detail("budget", "16.7ms")
                    .detail("actual", "22.4ms"),
                );
                cx.notify();
              })),
          ),
      )
      .child(
        Card::new().child(
          Stack::new()
            .gap(Size::Sm)
            .child(Title::new("Settings").order(4))
            .child(
              Checkbox::new("notify")
                .label("Email notifications")
                .checked(self.checked)
                .on_change(cx.listener(|this: &mut Demo, _, _, cx| {
                  this.checked = !this.checked;
                  guise::devtools::log(
                    cx,
                    LogLevel::Log,
                    format!("notifications = {}", this.checked),
                  );
                  cx.notify();
                })),
            )
            .child(
              RadioGroup::new()
                .label("Plan")
                .options(["Free", "Pro", "Team"])
                .value(self.plan.get(cx))
                .bind(self.plan.binding()),
            )
            .child(
              Group::new()
                .gap(Size::Xs)
                .child(Badge::new("stable").color(ColorName::Green))
                .child(Badge::new("v0.13").variant(Variant::Light)),
            ),
        ),
      )
      .child(
        Alert::new("Nothing here opens a socket — the host reports, guise displays.")
          .variant(Variant::Light),
      );

    div()
      .size_full()
      .flex()
      .bg(body)
      .text_color(text)
      .font_family(font)
      .child(
        div()
          .flex_1()
          .min_w(px(0.0))
          .h_full()
          .p(px(28.0))
          .overflow_hidden()
          .child(app),
      )
      .child(div().w(px(1.0)).h_full().bg(border))
      .child(
        div()
          .flex_none()
          .w(px(620.0))
          .h_full()
          .child(self.devtools.clone()),
      )
  }
}

/// The tab named on the command line, so a screenshot or a demo can open
/// straight onto the panel it is about.
fn starting_tab() -> DevToolsTab {
  let arg = std::env::args().nth(1).unwrap_or_default().to_lowercase();
  DevToolsTab::ALL
    .into_iter()
    .find(|tab| tab.label().to_lowercase() == arg)
    .unwrap_or(DevToolsTab::Elements)
}

fn main() {
  gpui_platform::application().run(|cx: &mut App| {
    Theme::dark().init(cx);
    DevToolsState::new().init(cx);

    let bounds = Bounds::centered(None, size(px(1360.0), px(840.0)), cx);
    cx.open_window(
      WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        ..Default::default()
      },
      |_window, cx| cx.new(Demo::new),
    )
    .expect("open window");
    cx.activate(true);
  });
}
