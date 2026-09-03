//! Gallery — a live showcase of `guise` components, in the spirit of the
//! component docs. Run with `cargo run -p gallery`.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
  div, list, px, size, AnyElement, App, Bounds, Context, Entity, IntoElement, ListAlignment,
  ListState, MouseButton, MouseDownEvent, SharedString, TitlebarOptions, Window, WindowBounds,
  WindowOptions,
};

use guise::flex::{Container, EdgeInsets, Expanded, Row, SizedBox, Spacer};
use guise::prelude::*;
use std::time::Duration;

mod code;
mod sections;

/// A "view source" panel: the example's code in a monospace block.
fn code_block(cx: &App, source: &str) -> impl IntoElement {
  let t = cx.global::<Theme>();
  let bg = t.color(ColorName::Dark, 9).hsla();
  let fg = t.color(ColorName::Gray, 5).hsla();
  let radius = t.radius(Size::Md);
  let lines = source.lines().map(move |line| {
    div().child(SharedString::from(if line.is_empty() {
      " ".to_string()
    } else {
      line.to_string()
    }))
  });
  div()
    .flex()
    .flex_col()
    .w_full()
    .p(px(14.0))
    .rounded(px(radius))
    .bg(bg)
    .font_family("Menlo")
    .text_size(px(12.5))
    .text_color(fg)
    .children(lines)
}

/// Rust highlighting for the editor showcase: the tree-sitter backend when
/// built with `--features treesitter`, the built-in line tokenizer otherwise.
#[cfg(feature = "treesitter")]
fn rust_highlighting(editor: Editor) -> Editor {
  match guise::TreeSitterHighlighter::new(
    tree_sitter_rust::LANGUAGE.into(),
    tree_sitter_rust::HIGHLIGHTS_QUERY,
  ) {
    Ok(hl) => editor.highlighter(hl),
    Err(_) => editor.language(Language::Rust),
  }
}

#[cfg(not(feature = "treesitter"))]
fn rust_highlighting(editor: Editor) -> Editor {
  editor.language(Language::Rust)
}

/// A child view that shares the gallery's counter purely through context —
/// it never receives the `Signal` directly, it reads it via `use_context`.
struct Counter {
  count: Signal<i32>,
}

impl Counter {
  fn new(cx: &mut Context<Self>) -> Self {
    let count = use_context::<Signal<i32>>(cx).expect("counter provided");
    watch(cx, &count);
    Counter { count }
  }
}

impl Render for Counter {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    Text::new(format!("Shared count: {}", self.count.get(cx)))
      .bold()
      .size(Size::Lg)
  }
}

/// Row type for the TableView demo.
#[derive(Clone)]
struct Employee {
  name: &'static str,
  role: &'static str,
  tenure: u32,
}

/// Initial buffer for the Editor demo.
const EDITOR_DEMO_SOURCE: &str = "/// Greets a person by name.\nfn greet(name: &str) -> String {\n    // Press Cmd+Enter to \"run\" the buffer.\n    let excitement = 3;\n    format!(\"hello {name}{}\", \"!\".repeat(excitement))\n}";

struct Gallery {
  agree: bool,
  notifications: bool,
  plan: usize,
  chip_on: bool,
  modal_open: bool,
  name: Entity<TextInput>,
  framework: Entity<Select>,
  menu: Entity<Menu>,
  menubar: Entity<MenuBar>,
  tabs: Entity<Tabs>,
  accordion: Entity<Accordion>,
  pagination: Entity<Pagination>,
  segmented: Entity<SegmentedControl>,
  counter: Entity<Counter>,
  webview: Entity<WebView>,
  webview_title: SharedString,
  count: Signal<i32>,
  nav_active: usize,
  // TableView
  employees: Entity<TableView<Employee>>,
  tableview_status: SharedString,
  // DataView bindings
  dataview_items: Signal<Vec<String>>,
  dataview_query: Signal<String>,
  dataview_filter: Entity<TextInput>,
  dataview: Entity<DataView<String>>,
  dataview_selected: Option<usize>,
  // TreeView
  tree: Entity<TreeView>,
  tree_status: SharedString,
  // Editor
  editor: Entity<Editor>,
  editor_status: SharedString,
  // Panel + SplitPanel
  panel_collapsed: bool,
  split: Entity<SplitPanel>,
  // More inputs
  password: Entity<PasswordInput>,
  brand_color: Entity<ColorInput>,
  topics: Entity<TagsInput>,
  range: Entity<RangeSlider>,
  pin: Entity<PinInput>,
  pin_value: SharedString,
  stars: f32,
  // Floating overlays
  context_menu: Entity<ContextMenu>,
  hover_card: Entity<HoverCard>,
  loading_visible: bool,
  confirm_open: bool,
  // App structure
  tabbar: Entity<TabBar>,
  // Settings
  settings: Entity<SettingsView>,
  // DevTools
  devtools: Entity<DevTools>,
  devtools_requests: u32,
  // Typography extras
  spoiler_open: bool,
  // Dates & files
  datepicker: Entity<DatePicker>,
  timepicker: Entity<TimePicker>,
  fileinput: Entity<FileInput>,
  picked_date: SharedString,
  drop_note: SharedString,
  // Drag & drop
  queue: Vec<SharedString>,
  // Motion
  collapse_open: bool,
  /// Bumped to hand the one-shot demos fresh element ids, which is what
  /// replays them — a mounted animation has already run.
  motion_epoch: usize,
  motion_player: Entity<Animator>,
  // Software update: the prompt and the "nothing to install" notice, rendered
  // inline rather than in their own windows so both are visible at once.
  update_prompt: Entity<UpdatePrompt>,
  update_notice: Entity<UpdateNotice>,
  update_status: SharedString,
  // Misc components
  carousel: Entity<Carousel>,
  navmenu: Entity<NavigationMenu>,
  autocomplete: Entity<Autocomplete>,
  transfer: Entity<Transfer>,
  tour: Entity<Tour>,
  // AI: a transcript with a canned conversation, the prompt box, and the
  // controls around a request. No transport — the demo replies to itself.
  chat: Entity<AIChatView>,
  composer: Entity<AIComposer>,
  model: Entity<AIModelPicker>,
  ai_settings: Entity<AISettings>,
  ai_usage: AIUsage,
  /// Section keys whose "view source" panel is currently expanded.
  code_open: HashSet<&'static str>,
  /// One copy button per section, keyed by section key.
  copy_buttons: HashMap<&'static str, Entity<CopyButton>>,
  /// The "Builder | Macros" control + the chosen style.
  code_style: Entity<SegmentedControl>,
  use_macros: bool,
  sections: ListState,
}

/// Every section's `(key, snippet)` — drives the copy buttons and code panels.
/// Each snippet carries both a plain and a macro variant.
const SECTION_SOURCES: &[(&str, code::Snippet)] = &[
  ("buttons", code::BUTTONS),
  ("icons", code::ICONS),
  ("webview", code::WEBVIEW),
  ("badges", code::BADGES),
  ("inputs", code::INPUTS),
  ("inputs2", code::INPUTS2),
  ("overlays", code::OVERLAYS),
  ("overlays2", code::OVERLAYS2),
  ("feedback", code::FEEDBACK),
  ("data", code::DATA),
  ("tableview", code::TABLEVIEW),
  ("tree", code::TREE),
  ("charts", code::CHARTS),
  ("gpuview", code::GPU_VIEW),
  ("dates", code::DATES),
  ("dnd", code::DND),
  ("motion", code::MOTION),
  ("misc", code::MISC),
  ("update", code::UPDATE),
  ("editor", code::EDITOR),
  ("ai", code::AI),
  ("devtools", code::DEVTOOLS),
  ("settings", code::SETTINGS),
  ("navigation", code::NAVIGATION),
  ("shell", code::SHELL),
  ("panels", code::PANELS),
  ("polish", code::POLISH),
  ("layout", code::LAYOUT),
  ("reactive", code::REACTIVE),
  ("dataview", code::DATAVIEW),
  ("cards", code::CARDS),
  ("typography", code::TYPOGRAPHY),
  ("typeextras", code::TYPEEXTRAS),
  ("media", code::MEDIA),
  ("palette", code::PALETTE),
  ("themes", code::THEMES),
];

/// Inline page rendered by the WebView demo — keeps the showcase offline.
const WEBVIEW_DEMO_HTML: &str = r#"<!doctype html>
<html><head><meta charset="utf-8"><title>guise · WebView</title>
<style>
  html,body{margin:0;height:100%;font-family:-apple-system,system-ui,sans-serif}
  body{display:flex;align-items:center;justify-content:center;
       background:linear-gradient(135deg,#4c6ef5,#9775fa);color:#fff}
  .card{text-align:center}
  h1{font-size:34px;margin:0 0 8px}
  p{opacity:.85;margin:0 0 18px}
  a{color:#fff;text-decoration:underline}
</style></head>
<body><div class="card">
  <h1>Native WebView</h1>
  <p>Real WKWebView / WebView2 / WebKitGTK, embedded by <code>wry</code>.</p>
  <a href="https://example.com">Navigate to example.com →</a>
</div></body></html>"#;

/// The snippet pair for a section key.
fn snippet(key: &str) -> code::Snippet {
  SECTION_SOURCES
    .iter()
    .find(|(k, _)| *k == key)
    .map(|(_, s)| *s)
    .unwrap_or(code::Snippet {
      plain: "",
      macros: "",
    })
}

/// Native window-menu actions.
#[derive(Clone, PartialEq, Default, Debug, gpui::Action)]
#[action(namespace = gallery, no_json)]
struct ToggleThemeAction;

#[derive(Clone, PartialEq, Default, Debug, gpui::Action)]
#[action(namespace = gallery, no_json)]
struct QuitAction;

/// The Appearance page: a switch bound to a signal, and a row of choice buttons.
fn appearance_page(
  dark: &Signal<bool>,
  size: &Signal<String>,
  query: &str,
  cx: &mut App,
) -> AnyElement {
  let current = size.get(cx);
  let mut sizes = Group::new().gap(Size::Xs);
  for value in ["12", "14", "16", "18"] {
    let target = size.clone();
    sizes = sizes.child(
      Button::new(SharedString::from(format!("fs-{value}")), value)
        .size(Size::Xs)
        .variant(if current == value {
          Variant::Filled
        } else {
          Variant::Default
        })
        .on_click(move |_, _, cx| target.set(cx, value.to_string())),
    );
  }

  let mut section = SettingsSection::new("Theme").description("How the gallery looks.");
  if query.is_empty() || "dark mode".contains(&query.to_lowercase()) {
    section = section.child(
      SettingsRow::new("dark", "Dark mode")
        .description("Pin the scheme instead of following the system.")
        .modified(dark.get(cx))
        .control(Switch::new("dark-switch").bind(dark.binding())),
    );
  }
  section = section.child(
    SettingsRow::new("font-size", "Editor font size")
      .description("Named sizes beat a slider that invites values nobody wants.")
      .modified(current != "14")
      .divider(false)
      .control(sizes),
  );
  section.into_any_element()
}

/// The Editor page.
fn editor_page(autosave: &Signal<bool>, cx: &mut App) -> AnyElement {
  SettingsSection::new("Editing")
    .description("How notes behave while you write them.")
    .child(
      SettingsRow::new("autosave", "Autosave")
        .description("Write to disk as you type.")
        .modified(autosave.get(cx))
        .control(Switch::new("autosave-switch").bind(autosave.binding())),
    )
    .child(
      SettingsRow::new("wrap", "Soft wrap")
        .description("Wrap long lines to the viewport.")
        .divider(false)
        .control(Switch::new("wrap-switch").checked(true)),
    )
    .into_any_element()
}

/// The Security page, showing the reset affordance.
fn security_page() -> AnyElement {
  SettingsSection::new("Keys")
    .description("The settings worth reading twice.")
    .child(
      SettingsRow::new("lock", "Lock after")
        .description("A reset arrow appears on a row the user has pinned.")
        .modified(true)
        .on_reset(|_, _, _| {})
        .divider(false)
        .control(Badge::new("15 minutes").size(Size::Sm)),
    )
    .into_any_element()
}

impl Gallery {
  fn new(cx: &mut Context<Self>) -> Self {
    let name = cx.new(|cx| {
      TextInput::new(cx)
        .label("Name")
        .placeholder("Ada Lovelace")
        .description("Click to focus, then type.")
    });
    let framework = cx.new(|cx| {
      Select::new(cx)
        .label("Framework")
        .placeholder("Choose one…")
        .data(["gpui", "SwiftUI", "Flutter", "Jetpack Compose", "AppKit"])
    });
    let menu = cx.new(|cx| {
      Menu::new(cx, "Actions")
        .section("Edit")
        .item("Copy", |_, _| {})
        .item("Rename", |_, _| {})
        .divider()
        .danger_item("Delete", |_, _| {})
    });
    let menubar = cx.new(|cx| {
      MenuBar::new(cx)
        .menu("File", |m| {
          m.item_shortcut("New Tab", "⌘T", |_, _| {})
            .item_shortcut("New Window", "⌘N", |_, _| {})
            .divider()
            .item_shortcut("Close Tab", "⌘W", |_, _| {})
            .divider()
            .danger_item("Quit", |_, _| {})
        })
        .menu("Edit", |m| {
          m.item_shortcut("Undo", "⌘Z", |_, _| {})
            .disabled_item("Redo")
            .divider()
            .section("Clipboard")
            .item_shortcut("Copy", "⌘C", |_, _| {})
            .item_shortcut("Paste", "⌘V", |_, _| {})
        })
        .menu("View", |m| {
          m.item("Toggle Sidebar", |_, _| {})
            .item("Zoom In", |_, _| {})
            .item("Zoom Out", |_, _| {})
        })
    });
    let tabs = cx.new(|cx| {
      Tabs::new(cx)
        .tab("Overview", |_, _| {
          Text::new("Usage, install steps, and a quick tour.")
            .dimmed()
            .size(Size::Sm)
        })
        .tab("Members", |_, _| {
          Text::new("Three people have access to this project.")
            .dimmed()
            .size(Size::Sm)
        })
        .tab("Settings", |_, _| {
          Text::new("Danger zone and project configuration.")
            .dimmed()
            .size(Size::Sm)
        })
    });
    let accordion = cx.new(|cx| {
      Accordion::new(cx)
        .item("What is guise?", |_, _| {
          Text::new("A component library for gpui.").size(Size::Sm)
        })
        .item("Is it themeable?", |_, _| {
          Text::new("Yes — light/dark plus the full open-color palette.").size(Size::Sm)
        })
        .default_open(0)
    });
    let pagination = cx.new(|cx| Pagination::new(cx, 10).active(1));
    let segmented = cx.new(|cx| {
      SegmentedControl::new(cx)
        .data(["Day", "Week", "Month"])
        .selected(1)
    });

    // Reactive: create shared state, provide it as context, then build a
    // child view that reads it back via `use_context`.
    let count = use_state(cx, 0i32);
    provide(cx, count.clone());
    let counter = cx.new(Counter::new);

    let copy_buttons: HashMap<&'static str, Entity<CopyButton>> = SECTION_SOURCES
      .iter()
      .map(|(key, snip)| (*key, cx.new(|_| CopyButton::new(snip.plain))))
      .collect();

    // The "Builder | Macros" toggle. When it changes we mirror the choice
    // into `use_macros` and re-point every copy button at that variant.
    let code_style = cx.new(|cx| {
      SegmentedControl::new(cx)
        .data(["Builder", "Macros"])
        .selected(0)
    });
    cx.subscribe(
      &code_style,
      |this, _seg, event: &SegmentedControlEvent, cx| {
        let macros = event.0 == 1;
        this.use_macros = macros;
        let keys: Vec<&'static str> = this.copy_buttons.keys().copied().collect();
        for key in keys {
          let src = snippet(key).pick(macros);
          if let Some(btn) = this.copy_buttons.get(key) {
            btn.update(cx, |b, _| b.set_text(src));
          }
        }
        cx.notify();
      },
    )
    .detach();

    // A native web view loading an inline page (no network needed). Its
    // title updates flow back through `WebViewEvent` into the status text.
    let webview = cx.new(|cx| WebView::new(cx).html(WEBVIEW_DEMO_HTML).height(320.0));
    cx.subscribe(&webview, |this, _view, event: &WebViewEvent, cx| {
      if let WebViewEvent::TitleChanged(title) = event {
        this.webview_title = title.clone();
        cx.notify();
      }
    })
    .detach();

    let sections = ListState::new(SECTION_SOURCES.len() + 1, ListAlignment::Top, px(240.0));
    let native_webview = webview.downgrade();
    sections.set_scroll_handler(move |event, _window, cx| {
      native_webview
        .update(cx, |webview, _| {
          webview.set_visible(event.visible_range.contains(&3));
        })
        .ok();
    });

    // TableView over typed rows: sortable columns, multi-selection, and a
    // virtualized body (fixed height). Events carry source-row indices.
    let employees = cx.new(|cx| {
      TableView::new(cx)
        .columns(vec![
          Column::new("Name")
            .text(|e: &Employee| e.name.into())
            .sortable_by(|a: &Employee, b: &Employee| a.name.cmp(b.name)),
          Column::new("Role").text(|e: &Employee| e.role.into()),
          Column::new("Tenure")
            .width(110.0)
            .align(Align::End)
            .text(|e: &Employee| format!("{} yrs", e.tenure).into())
            .sortable_by(|a: &Employee, b: &Employee| a.tenure.cmp(&b.tenure)),
          Column::new("Status")
            .width(120.0)
            .cell(|e: &Employee, _, _| {
              if e.tenure >= 5 {
                Badge::new("Senior").color(ColorName::Teal)
              } else {
                Badge::new("Growing").color(ColorName::Orange)
              }
            }),
        ])
        .rows(vec![
          Employee {
            name: "Ada Lovelace",
            role: "Engineering",
            tenure: 7,
          },
          Employee {
            name: "Grace Hopper",
            role: "Compilers",
            tenure: 9,
          },
          Employee {
            name: "Linus Torvalds",
            role: "Kernel",
            tenure: 3,
          },
          Employee {
            name: "Margaret Hamilton",
            role: "Flight Software",
            tenure: 5,
          },
          Employee {
            name: "Katherine Johnson",
            role: "Trajectories",
            tenure: 2,
          },
          Employee {
            name: "Alan Turing",
            role: "Research",
            tenure: 4,
          },
        ])
        .selection_mode(SelectionMode::Multi)
        .striped(true)
        .highlight_on_hover(true)
        .with_border(true)
        .height(240.0)
    });
    cx.subscribe(&employees, |this, _table, event: &TableViewEvent, cx| {
      this.tableview_status = match event {
        TableViewEvent::SelectionChanged(rows) if rows.is_empty() => {
          SharedString::from("Nothing selected")
        }
        TableViewEvent::SelectionChanged(rows) => {
          SharedString::from(format!("Selected source rows: {rows:?}"))
        }
        TableViewEvent::Activated(row) => SharedString::from(format!("Activated row {row}")),
        TableViewEvent::Sorted(Some((col, dir))) => {
          SharedString::from(format!("Sorted by column {col} ({dir:?})"))
        }
        TableViewEvent::Sorted(None) => SharedString::from("Sort cleared"),
      };
      cx.notify();
    })
    .detach();

    // DataView bindings: one Signal<Vec<String>> is the source of truth.
    // The DataView renders it live, a TextInput bound to a query signal
    // drives the filter projection, and the buttons write to the signal.
    let dataview_items = use_state(
      cx,
      vec![
        "SwiftUI".to_string(),
        "gpui".to_string(),
        "SwiftUI".to_string(),
        "Flutter".to_string(),
      ],
    );
    watch(cx, &dataview_items); // keep the item-count text live

    let dataview_query = use_state(cx, String::new());
    let dataview_filter = cx.new(|cx| TextInput::new(cx).placeholder("Filter frameworks…"));
    TextInput::bind(&dataview_filter, &dataview_query, cx);

    // The filter closure gets no `cx`, so the query flows through a shared
    // cell: the observer below copies each signal write into it.
    let query_cache: Rc<RefCell<String>> = Rc::new(RefCell::new(String::new()));
    let filter_cache = query_cache.clone();
    let dataview = cx.new(|cx| {
      DataView::new(cx, &dataview_items)
        .item(|name: &String, _ix, _window, _cx| Text::new(name.clone()).size(Size::Sm))
        .filter(move |name: &String| {
          let query = filter_cache.borrow();
          query.is_empty() || name.to_lowercase().contains(&query.to_lowercase())
        })
        .sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()))
        .selectable()
    });
    cx.subscribe(&dataview, |this, _view, event: &DataViewEvent, cx| {
      let DataViewEvent::Selected(ix) = event;
      this.dataview_selected = Some(*ix);
      cx.notify();
    })
    .detach();
    let filtered_view = dataview.clone();
    cx.observe(dataview_query.entity(), move |_this, query, cx| {
      *query_cache.borrow_mut() = query.read(cx).clone();
      filtered_view.update(cx, |_, cx| cx.notify());
      cx.notify(); // the gallery's status line shows the query too
    })
    .detach();

    // TreeView: folder/file sample with keyboard navigation.
    let tree = cx.new(|cx| {
      TreeView::new(cx)
        .nodes(vec![
          TreeNode::new("src", "src")
            .child(
              TreeNode::new("data", "data")
                .child(TreeNode::new("tree", "tree.rs"))
                .child(TreeNode::new("tabs", "tabs.rs")),
            )
            .child(TreeNode::new("lib", "lib.rs"))
            .child(TreeNode::new("main", "main.rs")),
          TreeNode::new("docs", "docs")
            .child(TreeNode::new("readme", "readme.md"))
            .child(TreeNode::new("architecture", "architecture.md")),
          TreeNode::new("cargo", "Cargo.toml").icon(IconName::Star),
        ])
        .expand("src")
    });
    cx.subscribe(&tree, |this, _tree, event: &TreeViewEvent, cx| {
      this.tree_status = match event {
        TreeViewEvent::Selected(id) => SharedString::from(format!("selected: {id}")),
        TreeViewEvent::Toggled(id, open) => SharedString::from(format!("{id} expanded: {open}")),
        TreeViewEvent::Activated(id) => SharedString::from(format!("activated: {id}")),
        TreeViewEvent::ContextMenu(id, at) => {
          SharedString::from(format!("context menu: {id} at {}, {}", at.x, at.y))
        }
      };
      cx.notify();
    })
    .detach();

    // Editor: syntax-highlighted Rust buffer; Cmd+Enter emits Run.
    let editor = cx.new(|cx| {
      rust_highlighting(Editor::new(cx))
        .rows(8)
        .placeholder("Type some Rust…")
        .value(EDITOR_DEMO_SOURCE)
    });
    cx.subscribe(&editor, |this, _editor, event: &EditorEvent, cx| {
      match event {
        EditorEvent::Run(source) => {
          this.editor_status =
            SharedString::from(format!("Run requested ({} chars)", source.len()));
        }
        EditorEvent::Change(text) => {
          this.editor_status = SharedString::from(format!("{} lines", text.lines().count().max(1)));
        }
      }
      cx.notify();
    })
    .detach();

    // SplitPanel: a vertical split nested inside a horizontal one. The
    // inner entity is captured by the outer pane's builder closure.
    let split_inner = cx.new(|cx| {
      SplitPanel::new(cx)
        .direction(SplitDirection::Vertical)
        .ratio(0.55)
        .min_first(60.0)
        .min_second(60.0)
        .first(|_, _| {
          div()
            .p(px(12.0))
            .child(Text::new("Editor pane").size(Size::Sm).dimmed())
        })
        .second(|_, _| {
          div().p(px(12.0)).child(
            Text::new("Terminal pane — a nested vertical split.")
              .size(Size::Sm)
              .dimmed(),
          )
        })
    });
    let split = cx.new(|cx| {
      SplitPanel::new(cx)
        .direction(SplitDirection::Horizontal)
        .ratio(0.35)
        .min_first(140.0)
        .min_second(200.0)
        .first(|_, _| {
          div().p(px(12.0)).child(
            Stack::new()
              .gap(Size::Xs)
              .child(Text::new("Sidebar").bold().size(Size::Sm))
              .child(Text::new("Drag the dividers.").size(Size::Sm).dimmed()),
          )
        })
        .second(move |_, _| split_inner.clone())
    });
    cx.subscribe(&split, |_this, _split, _event: &SplitPanelEvent, cx| {
      cx.notify()
    })
    .detach();

    // More inputs: entities own their state; Rating is controlled.
    let password = cx.new(|cx| {
      PasswordInput::new(cx)
        .label("Password")
        .placeholder("At least 8 characters")
        .description("The eye toggles visibility.")
    });
    let brand_color = cx.new(|cx| {
      ColorInput::new(cx)
        .label("Brand color")
        .value(rgb(34, 139, 230))
    });
    let topics = cx.new(|cx| {
      TagsInput::new(cx)
        .label("Topics")
        .placeholder("Type and press Enter…")
        .tags(["rust", "gpui"])
        .max_tags(6)
    });
    let range = cx.new(|cx| {
      RangeSlider::new(cx)
        .min(0.0)
        .max(100.0)
        .min_gap(10.0)
        .value((20.0, 80.0))
        .color(ColorName::Teal)
    });
    let pin = cx.new(|cx| PinInput::new(cx).length(4));
    cx.subscribe(&pin, |this, _pin, event: &PinInputEvent, cx| {
      match event {
        PinInputEvent::Change(code) => this.pin_value = SharedString::from(code.clone()),
        PinInputEvent::Complete(code) => {
          this.pin_value = SharedString::from(format!("{code} — complete!"))
        }
      }
      cx.notify();
    })
    .detach();

    // Floating overlays: a pointer-positioned context menu and a hover card.
    let context_menu = cx.new(|cx| {
      ContextMenu::new(cx)
        .section("File")
        .item_icon(IconName::Copy, "Copy path", |_, _| {})
        .item("Rename", |_, _| {})
        .divider()
        .danger_item("Delete", |_, _| {})
    });
    let hover_card = cx.new(|cx| {
      HoverCard::new(
        cx,
        |_, _| Badge::new("@ada").into_any_element(),
        |_, _| {
          Stack::new()
            .gap(Size::Xs)
            .child(Text::new("Ada Lovelace").bold())
            .child(
              Text::new("Wrote the first published program.")
                .dimmed()
                .size(Size::Sm),
            )
            .into_any_element()
        },
      )
      .width(260.0)
    });

    // TabBar: closing is a request — the parent decides and removes.
    let tabbar = cx.new(|cx| {
      TabBar::new(cx)
        .tabs(["main.rs", "lib.rs", "theme.rs"])
        .active(0)
    });
    cx.subscribe(&tabbar, |_this, bar, event: &TabBarEvent, cx| match event {
      TabBarEvent::Close(i) => {
        let i = *i;
        bar.update(cx, |b, cx| b.remove_tab(i, cx));
      }
      TabBarEvent::Add => {
        bar.update(cx, |b, cx| {
          let n = b.len() + 1;
          b.add_tab(format!("untitled {n}"), cx);
        });
      }
      TabBarEvent::Select(_) => {}
    })
    .detach();

    // Dates & files: pickers emit events; the drop/file status lines live
    // on the gallery itself.
    let datepicker = cx.new(|cx| DatePicker::new(cx).label("Ship date"));
    cx.subscribe(&datepicker, |this, _picker, event: &DatePickerEvent, cx| {
      if let DatePickerEvent::Selected(date) = event {
        this.picked_date = SharedString::from(format!("picked {}", date.format("MMM D, YYYY")));
        cx.notify();
      }
    })
    .detach();
    let timepicker = cx.new(|cx| TimePicker::new(cx).label("Standup"));
    let fileinput = cx.new(|cx| FileInput::new(cx).label("Attachment"));

    // Misc components.
    let carousel = cx.new(|cx| {
      let slide = |title: &'static str| {
        move |_: &mut Window, _: &mut App| {
          div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(Title::new(title).order(2))
        }
      };
      Carousel::new(cx)
        .slide(slide("Slide one"))
        .slide(slide("Slide two"))
        .slide(slide("Slide three"))
        .height(150.0)
    });
    let navmenu = cx.new(|cx| {
      NavigationMenu::new(cx)
        .item("home", "Home")
        .menu(
          "docs",
          "Docs",
          [("tutorial", "Tutorial"), ("api", "API reference")],
        )
        .item("about", "About")
        .active("home")
    });
    let autocomplete = cx.new(|cx| {
      Autocomplete::new(cx).label("Language").suggestions([
        "Rust",
        "Ruby",
        "Python",
        "TypeScript",
        "Go",
        "Swift",
      ])
    });
    let transfer = cx.new(|cx| {
      Transfer::new(cx)
        .data(["Ada", "Grace", "Linus", "Margaret", "Alan"])
        .chosen([0])
        .titles("Bench", "Team")
        .height(150.0)
    });
    let tour = cx.new(|cx| {
      Tour::new(cx)
        .step("Welcome", "This gallery shows every guise component.")
        .step("View source", "The </> toggle reveals each section's code.")
        .step("Theming", "Toggle light/dark from the header button.")
    });

    // --- motion -----------------------------------------------------
    // A looping clip: slide out, drop and round off, then come back. The
    // tracks are geometry only, so the same animator is correct in both
    // themes — the colour half of the story is a one-shot built during
    // render, where the theme is live.
    let orbit = sequence![
        motion! { duration: 620; ease: out cubic; x: 0 => 190; },
        motion! {
            duration: 520;
            ease: out elastic;
            y: 0 => 26;
            radius: 10 => 28;
        },
        rel(140) => motion! {
            duration: 720;
            ease: in_out sine;
            x: 190 => 0;
            y: 26 => 0;
            radius: 28 => 10;
        },
    ]
    .repeat_forever();
    // Deliberately not autoplaying: an endless clip asks for a frame
    // forever, and a showcase that pins a core doing it off-screen is a
    // bad neighbour. Press Play.
    let motion_player = cx.new(|cx| Animator::new(orbit, cx));

    // The gallery runs from `target/`, which is neither an installed .app nor
    // an AppImage — so `detect()` reports `Unknown` and the prompt honestly
    // offers the download page instead of an install it can't perform. The
    // stages below are driven by hand; nothing here touches the network.
    let updater = Updater::github("Gallery", env!("CARGO_PKG_VERSION"), "wess/guise");
    let offered = Release {
      version: "9.9.9".to_string(),
      url: "https://github.com/wess/guise/releases".to_string(),
      assets: Vec::new(),
    };
    let update_prompt = cx.new(|cx| UpdatePrompt::new(updater.clone(), offered, cx));
    cx.subscribe(
      &update_prompt,
      |this, _prompt, event: &UpdatePromptEvent, cx| {
        this.update_status = SharedString::from(match event {
          UpdatePromptEvent::Started => "install started".to_string(),
          UpdatePromptEvent::Stage(stage) => stage.label().to_string(),
          UpdatePromptEvent::Installed(_) => "installed — restarting".to_string(),
          UpdatePromptEvent::Failed(why) => format!("failed: {why}"),
          UpdatePromptEvent::Dismissed => "dismissed".to_string(),
        });
        cx.notify();
      },
    )
    .detach();
    let update_notice =
      cx.new(|cx| UpdateNotice::new(updater, UpdateOutcome::Pending("9.9.9".into()), cx));

    // --- AI ---------------------------------------------------------
    // A canned exchange, so the section shows what a real transcript looks
    // like: a reply with reasoning, a tool call, and cited sources.
    let chat = cx.new(|cx| {
            AIChatView::new(cx)
                .max_width(680.0)
                .empty_message("Ask something to start.")
                .turns([
                    AITurn::user("What changed in the update module?"),
                    AITurn::assistant(
                        "Two things:\n\n                         1. Downloads are now checked against a **published SHA-256** \
                         when the release ships one.\n                         2. `require_checksum(true)` makes a *missing* digest a refusal \
                         rather than a silent pass — which is what you want on Linux, \
                         where there is no `codesign` to fall back on.",
                    )
                    .name("claude-opus-5")
                    .meta("1.8s · 412 tokens")
                    .reasoning(
                        "The macOS path already gates on a pinned codesign requirement. \
                         The AppImage path had nothing equivalent, so a byte count was \
                         the only check standing between a swapped asset and code that \
                         runs on the next launch.",
                    )
                    .tools([AITurnTool::new("read_file")
                        .status(AIToolStatus::Ok)
                        .arguments("{\"path\": \"crates/guise/src/update/appimage.rs\"}")
                        .result("166 lines")])
                    .sources([
                        AISource::new("update.md", "docs/update.md")
                            .excerpt("A byte count is not an integrity check."),
                        AISource::new("checksum.rs", "crates/guise/src/update/checksum.rs"),
                    ]),
                ])
        });
    let composer = cx.new(|cx| {
      AIComposer::new(cx)
        .attachments(true)
        .hint("Enter sends · Shift+Enter for a new line")
    });
    // The gallery has no transport, so the demo answers itself — enough to
    // show the streaming caret, the stop button, and stick-to-bottom.
    cx.subscribe(&composer, |this, _composer, event: &AIComposerEvent, cx| {
      if let AIComposerEvent::Submit(text) = event {
        let echo = format!(
          "The gallery has no model behind it, so this is a canned reply to \
                     _{}_ — but the streaming caret, the stop button, and stick-to-bottom \
                     scrolling are the real ones.",
          text.trim()
        );
        this.chat.update(cx, |chat, cx| {
          chat.push(AITurn::user(text.clone()), cx);
          chat.begin_reply(cx);
          chat.push_delta(&echo, cx);
          chat.end_reply(cx);
        });
        this.ai_usage = this.ai_usage + AIUsage::new(text.len() as u64, echo.len() as u64);
        cx.notify();
      }
    })
    .detach();
    let model = cx.new(|cx| {
      AIModelPicker::new(cx)
        .label("Model")
        .models([
          AIModel::new("claude-opus-5", "Opus 5")
            .description("Deepest reasoning")
            .context(200_000)
            .pricing(AIPricing::new(15.0, 75.0)),
          AIModel::new("claude-sonnet-5", "Sonnet 5")
            .description("Balanced speed and depth")
            .context(200_000)
            .pricing(AIPricing::new(3.0, 15.0)),
          AIModel::new("claude-haiku-4-5", "Haiku 4.5")
            .description("Fastest, cheapest")
            .context(200_000)
            .pricing(AIPricing::new(1.0, 5.0)),
        ])
        .selected_id("claude-sonnet-5")
    });
    let ai_settings = cx.new(AISettings::new);

    // A settings screen driven by signals, so the demo rows are live: the
    // content closure re-runs every frame and reads the current values.
    let dark_ui = use_state(cx, true);
    let autosave = use_state(cx, false);
    let font_size = use_state(cx, String::from("14"));
    let settings = cx.new(|cx| {
      let (dark, save, size) = (dark_ui.clone(), autosave.clone(), font_size.clone());
      SettingsView::new(cx)
        .page_icon("appearance", "Appearance", IconName::Palette)
        .page_icon("editor", "Editor", IconName::FileCode)
        .page_icon("security", "Security", IconName::ShieldCheck)
        .searchable(true)
        .content(move |page, query, _window, cx| match page {
          "appearance" => appearance_page(&dark, &size, query, cx),
          "editor" => editor_page(&save, cx),
          _ => security_page(),
        })
        .footer(|_window, _cx| {
          Group::new()
            .align(Align::Center)
            .child(
              Text::new("Settings live in a JSON file you can edit directly.")
                .size(Size::Xs)
                .dimmed(),
            )
            .child(div().flex_1())
            .child(Button::new("settings-done", "Done").size(Size::Xs))
        })
    });

    // The inspector. Records are seeded here so every panel has something
    // to show; a real app feeds them from where the work happens.
    let devtools = cx.new(DevTools::new);
    cx.subscribe(
      &devtools,
      |_this: &mut Gallery, devtools, event: &DevToolsEvent, cx| match event {
        // Nothing to close into here, so send it back to Elements.
        DevToolsEvent::Close => {
          devtools.update(cx, |devtools, cx| {
            devtools.set_tab(DevToolsTab::Elements, cx)
          });
        }
        DevToolsEvent::RevealSource(source) => {
          guise::devtools::log(
            cx,
            LogLevel::Info,
            format!("Host asked to open {}", source.short()),
          );
        }
        _ => {}
      },
    )
    .detach();

    Gallery {
      agree: false,
      settings,
      devtools,
      devtools_requests: 0,
      notifications: true,
      plan: 0,
      chip_on: true,
      modal_open: false,
      name,
      framework,
      menu,
      menubar,
      tabs,
      accordion,
      pagination,
      segmented,
      counter,
      webview,
      webview_title: SharedString::from("(loading…)"),
      count,
      nav_active: 1,
      employees,
      tableview_status: SharedString::from("Nothing selected"),
      dataview_items,
      dataview_query,
      dataview_filter,
      dataview,
      dataview_selected: None,
      tree,
      tree_status: SharedString::from("click a row, then use the arrow keys"),
      editor,
      editor_status: SharedString::from("6 lines"),
      panel_collapsed: false,
      split,
      password,
      brand_color,
      topics,
      range,
      pin,
      pin_value: SharedString::from("(empty)"),
      stars: 3.0,
      context_menu,
      hover_card,
      loading_visible: false,
      confirm_open: false,
      tabbar,
      spoiler_open: false,
      datepicker,
      timepicker,
      fileinput,
      picked_date: SharedString::from("no date picked yet"),
      drop_note: SharedString::from("nothing dropped yet"),
      queue: vec![
        SharedString::new_static("Design review"),
        SharedString::new_static("Fix flaky test"),
        SharedString::new_static("Ship 0.8"),
        SharedString::new_static("Write changelog"),
      ],
      collapse_open: true,
      motion_epoch: 0,
      motion_player,
      update_prompt,
      update_notice,
      update_status: SharedString::new_static("idle"),
      carousel,
      navmenu,
      autocomplete,
      transfer,
      tour,
      chat,
      composer,
      model,
      ai_settings,
      ai_usage: AIUsage::new(18_400, 2_100).cache_read(96_000),
      code_open: HashSet::new(),
      copy_buttons,
      code_style,
      use_macros: false,
      sections,
    }
  }

  /// Software update: the prompt in each of its states, beside the notice a
  /// manual check answers with when there's nothing to install. The buttons
  /// drive the states by hand — the prompt's own action button runs the real
  /// installer, which here means opening the release page (see `Gallery::new`).
  fn update_demo(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let controls = Group::new()
      .align(Align::Center)
      .child(
        Button::new("upd-idle", "Idle")
          .variant(Variant::Subtle)
          .on_click(cx.listener(|this, _, _, cx| {
            this.update_prompt.update(cx, |prompt, cx| prompt.reset(cx));
          })),
      )
      .child(
        Button::new("upd-dl", "Downloading")
          .variant(Variant::Light)
          .on_click(cx.listener(|this, _, _, cx| {
            this.update_prompt.update(cx, |prompt, cx| {
              prompt.set_stage(
                UpdateStage::Downloading {
                  done: 8_400_000,
                  total: 20_314_688,
                },
                cx,
              )
            });
          })),
      )
      .child(
        Button::new("upd-verify", "Verifying")
          .variant(Variant::Light)
          .on_click(cx.listener(|this, _, _, cx| {
            this.update_prompt.update(cx, |prompt, cx| {
              prompt.set_stage(UpdateStage::Verifying, cx)
            });
          })),
      )
      .child(
        Button::new("upd-fail", "Failed")
          .variant(Variant::Light)
          .color(ColorName::Red)
          .on_click(cx.listener(|this, _, _, cx| {
            this.update_prompt.update(cx, |prompt, cx| {
              prompt.set_failed("the update isn't signed by Gallery", cx)
            });
          })),
      )
      .child(
        Text::new(self.update_status.clone())
          .size(Size::Sm)
          .dimmed(),
      );

    let panel = |width: f32, height: f32, body: gpui::AnyElement| {
      Paper::new()
        .with_border(true)
        .padding(Size::Xs)
        .child(div().w(px(width)).h(px(height)).child(body))
    };

    Stack::new().gap(Size::Sm).child(controls).child(
      Group::new()
        .gap(Size::Md)
        .child(panel(
          400.0,
          220.0,
          self.update_prompt.clone().into_any_element(),
        ))
        .child(panel(
          320.0,
          140.0,
          self.update_notice.clone().into_any_element(),
        )),
    )
  }

  /// Wrap a section body with its title and a "view source" toggle (`</>`).
  /// Clicking the toggle reveals the example's code (with a copy button)
  /// beneath the demo.
  /// The inspector, live, with controls that give its panels something to
  /// show. Everything below the buttons is the host reporting its own work —
  /// `guise` never sees the request, only the record of it.
  fn devtools_demo(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let controls = Group::new()
      .gap(Size::Sm)
      .child(
        Button::new("dt-log", "Log")
          .variant(Variant::Default)
          .size(Size::Xs)
          .on_click(cx.listener(|_this, _, _, cx| {
            guise::devtools::log(cx, LogLevel::Log, "Gallery: hello from the host");
            cx.notify();
          })),
      )
      .child(
        Button::new("dt-warn", "Warn")
          .variant(Variant::Light)
          .color(ColorName::Yellow)
          .size(Size::Xs)
          .on_click(cx.listener(|_this, _, _, cx| {
            guise::devtools::log_record(
              cx,
              LogRecord::new(LogLevel::Warning, "Layout pass took 22ms")
                .detail("budget", "16.7ms")
                .detail("frame", "1042"),
            );
            cx.notify();
          })),
      )
      .child(
        Button::new("dt-error", "Error")
          .variant(Variant::Light)
          .color(ColorName::Red)
          .size(Size::Xs)
          .on_click(cx.listener(|_this, _, _, cx| {
            guise::devtools::log(cx, LogLevel::Error, "Failed to decode avatar.png");
            cx.notify();
          })),
      )
      .child(
        Button::new("dt-request", "Request")
          .variant(Variant::Default)
          .size(Size::Xs)
          .on_click(cx.listener(|this: &mut Gallery, _, _, cx| {
            this.devtools_requests += 1;
            let n = this.devtools_requests;
            let record =
              NetworkRecord::new("GET", format!("https://api.example.com/v1/items?page={n}"))
                .kind(ResourceKind::Fetch)
                .status(200, "OK")
                .sizes(4_200 + u64::from(n) * 130, 11_800 + u64::from(n) * 400)
                .timings(Timings {
                  stalled: Duration::from_millis(2),
                  dns: Duration::from_millis(4),
                  connect: Duration::from_millis(11),
                  tls: Duration::from_millis(19),
                  request: Duration::from_millis(3),
                  response: Duration::from_millis(24 + u64::from(n % 7) * 9),
                })
                .request_header("Accept", "application/json")
                .request_header("Cookie", "session=8f2c1a; theme=dark")
                .response_header("Content-Type", "application/json")
                .response_header("Set-Cookie", "session=8f2c1a; Path=/; HttpOnly")
                .response_body("{\n  \"items\": [],\n  \"page\": 1\n}")
                .finished();
            guise::devtools::network_begin(cx, record);
            cx.notify();
          })),
      )
      .child(
        Button::new("dt-work", "Slow work")
          .variant(Variant::Default)
          .size(Size::Xs)
          .on_click(cx.listener(|_this, _, _, cx| {
            guise::devtools::measure(cx, "reindex()", || {
              std::thread::sleep(Duration::from_millis(24));
            });
            cx.notify();
          })),
      )
      .child(
        Button::new("dt-storage", "Publish storage")
          .variant(Variant::Default)
          .size(Size::Xs)
          .on_click(cx.listener(|_this, _, _, cx| {
            let scheme = cx.global::<Theme>().scheme;
            guise::devtools::storage_set(
              cx,
              StorageDomain::new("prefs", "gallery.preferences")
                .kind(StorageKind::Local)
                .entry(StorageEntry::new("theme", format!("{scheme:?}")))
                .entry(StorageEntry::new("code.style", "builder"))
                .entry(StorageEntry::new("window.size", "960×880")),
            );
            cx.notify();
          })),
      );

    Stack::new()
      .gap(Size::Sm)
      .child(
        Text::new(
          "A Safari-shaped inspector for the app it is running inside. Elements, \
                     Layers and Styles read the live component tree; Console, Network, \
                     Storage and Timelines are fed by the host.",
        )
        .size(Size::Sm)
        .dimmed(),
      )
      .child(controls)
      .child(
        div()
          .h(px(520.0))
          .w_full()
          .rounded(px(6.0))
          .overflow_hidden()
          .border_1()
          .border_color(cx.global::<Theme>().border().hsla())
          .child(self.devtools.clone()),
      )
  }

  /// The settings shell, plus the two pieces of window chrome ported with it.
  fn settings_demo(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let _ = cx;
    Stack::new()
      .gap(Size::Sm)
      .child(
        Text::new(
          "The chrome only: a page list, groups, and rows. Each app keeps its own \
                     schema and write path — that is the part worth owning.",
        )
        .size(Size::Sm)
        .dimmed(),
      )
      .child(
        div()
          .h(px(420.0))
          .w_full()
          .rounded(px(6.0))
          .overflow_hidden()
          .border_1()
          .border_color(cx.global::<Theme>().border().hsla())
          .child(self.settings.clone()),
      )
      .child(
        Group::new()
          .gap(Size::Md)
          .child(
            Paper::new().child(
              About::new("guise gallery")
                .version(env!("CARGO_PKG_VERSION"))
                .tagline("A component library for gpui.")
                .build(BuildKind::Development, "2026-08-18")
                .icon(Icon::new(IconName::Boxes).size(Size::Xl))
                .link(Anchor::new("about-repo", "github.com/wess/guise"))
                .credits("MIT licensed"),
            ),
          )
          .child(
            Paper::new().child(
              Stack::new()
                .gap(Size::Xs)
                .child(Text::new("Window controls").size(Size::Sm))
                .child(
                  Text::new("Drawn by the app where the OS doesn't — Linux CSD.")
                    .size(Size::Xs)
                    .dimmed(),
                )
                .child(WindowControls::new()),
            ),
          ),
      )
  }

  fn section(
    &self,
    cx: &mut Context<Self>,
    key: &'static str,
    title: &'static str,
    body: impl IntoElement,
  ) -> impl IntoElement {
    let open = self.code_open.contains(key);
    let toggle = ActionIcon::new(SharedString::from(format!("code-{key}")), IconName::CodeXml)
      .label(if open { "Hide code" } else { "Show code" })
      .variant(if open {
        Variant::Light
      } else {
        Variant::Subtle
      })
      .color(ColorName::Blue)
      .on_click(cx.listener(move |this, _, _, cx| {
        if !this.code_open.remove(key) {
          this.code_open.insert(key);
        }
        cx.notify();
      }));

    let mut stack = Stack::new()
      .gap(Size::Sm)
      .child(
        Group::new()
          .justify(Justify::Between)
          .child(Title::new(title).order(3))
          .child(toggle),
      )
      .child(Divider::new())
      .child(body);

    if open {
      if let Some(copy) = self.copy_buttons.get(key) {
        let source = copy.read(cx).text();
        stack = stack.child(
          div().relative().child(code_block(cx, &source)).child(
            div()
              .absolute()
              .top(px(8.0))
              .right(px(8.0))
              .child(copy.clone()),
          ),
        );
      }
    }
    stack
  }

  fn gallery_item(&mut self, index: usize, cx: &mut Context<Self>) -> AnyElement {
    let gap = cx.global::<Theme>().spacing(Size::Xl);
    let content = match index {
      0 => Stack::new()
        .gap(Size::Xl)
        .child(sections::header())
        .child(
          Group::new()
            .align(Align::Center)
            .child(Text::new("Code examples:").size(Size::Sm).dimmed())
            .child(self.code_style.clone()),
        )
        .into_any_element(),
      1 => self
        .section(cx, "buttons", "Buttons", sections::buttons())
        .into_any_element(),
      2 => self
        .section(cx, "icons", "Icons", sections::icons())
        .into_any_element(),
      3 => {
        let body = self.webview_demo(cx);
        self
          .section(cx, "webview", "WebView (native)", body)
          .into_any_element()
      }
      4 => self
        .section(cx, "badges", "Badges", sections::badges())
        .into_any_element(),
      5 => {
        let body = self.inputs(cx);
        self
          .section(cx, "inputs", "Inputs", body)
          .into_any_element()
      }
      6 => {
        let body = self.inputs2(cx);
        self
          .section(cx, "inputs2", "More inputs", body)
          .into_any_element()
      }
      7 => {
        let body = self.dates(cx);
        self
          .section(cx, "dates", "Dates & files", body)
          .into_any_element()
      }
      8 => {
        let body = self.overlays(cx);
        self
          .section(cx, "overlays", "Overlays", body)
          .into_any_element()
      }
      9 => {
        let body = self.floating_overlays(cx);
        self
          .section(cx, "overlays2", "Context menu, hover card & confirm", body)
          .into_any_element()
      }
      10 => self
        .section(cx, "feedback", "Feedback", sections::feedback())
        .into_any_element(),
      11 => {
        let body = self.data_display();
        self
          .section(cx, "data", "Data display", body)
          .into_any_element()
      }
      12 => {
        let body = self.tableview_demo();
        self
          .section(cx, "tableview", "TableView", body)
          .into_any_element()
      }
      13 => {
        let body = self.tree_demo();
        self
          .section(cx, "tree", "TreeView", body)
          .into_any_element()
      }
      14 => self
        .section(cx, "charts", "Charts", sections::charts())
        .into_any_element(),
      15 => {
        let body = sections::gpu_view(cx);
        self
          .section(cx, "gpuview", "GPU View", body)
          .into_any_element()
      }
      16 => {
        let body = self.dnd(cx);
        self
          .section(cx, "dnd", "Drag & drop", body)
          .into_any_element()
      }
      17 => {
        let body = self.motion(cx);
        self
          .section(cx, "motion", "Motion", body)
          .into_any_element()
      }
      18 => {
        let body = self.editor_demo();
        self
          .section(cx, "editor", "Editor", body)
          .into_any_element()
      }
      19 => {
        let body = self.ai_demo(cx);
        self.section(cx, "ai", "AI", body).into_any_element()
      }
      20 => {
        let body = self.settings_demo(cx);
        self
          .section(cx, "settings", "Settings screen", body)
          .into_any_element()
      }
      21 => {
        let body = self.devtools_demo(cx);
        self
          .section(cx, "devtools", "DevTools (inspector)", body)
          .into_any_element()
      }
      22 => {
        let body = self.navigation(cx);
        self
          .section(cx, "navigation", "Navigation", body)
          .into_any_element()
      }
      23 => {
        let body = self.misc(cx);
        self
          .section(cx, "misc", "Carousel & more", body)
          .into_any_element()
      }
      24 => {
        let body = self.update_demo(cx);
        self
          .section(cx, "update", "Software update", body)
          .into_any_element()
      }
      25 => {
        let body = self.shell_demo(cx);
        self
          .section(cx, "shell", "App structure", body)
          .into_any_element()
      }
      26 => {
        let body = self.panels_demo(cx);
        self
          .section(cx, "panels", "Panels & SplitPanel", body)
          .into_any_element()
      }
      27 => {
        let body = self.polish(cx);
        self
          .section(cx, "polish", "Polish", body)
          .into_any_element()
      }
      28 => {
        let body = self.layout_demo(cx);
        self
          .section(cx, "layout", "Flex layout & macros", body)
          .into_any_element()
      }
      29 => {
        let body = self.reactive_demo(cx);
        self
          .section(cx, "reactive", "Reactive state (Context / Signal)", body)
          .into_any_element()
      }
      30 => {
        let body = self.dataview_demo(cx);
        self
          .section(cx, "dataview", "DataView (collection bindings)", body)
          .into_any_element()
      }
      31 => self
        .section(cx, "cards", "Cards", sections::cards())
        .into_any_element(),
      32 => self
        .section(cx, "typography", "Typography", sections::typography())
        .into_any_element(),
      33 => {
        let body = self.typeextras(cx);
        self
          .section(cx, "typeextras", "Typography extras", body)
          .into_any_element()
      }
      34 => self
        .section(cx, "media", "Image", sections::media())
        .into_any_element(),
      35 => {
        let body = sections::palette(cx);
        self
          .section(cx, "palette", "Palette", body)
          .into_any_element()
      }
      36 => {
        let body = sections::themes(cx);
        self
          .section(cx, "themes", "Theme manager", body)
          .into_any_element()
      }
      _ => unreachable!("gallery list requested an unknown item"),
    };

    div()
      .id(("gallery-item", index))
      .w_full()
      .pb(px(gap))
      .child(content)
      .into_any_element()
  }

  fn polish(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let icons = Group::new()
      .child(
        ActionIcon::new("ai-edit", IconName::Pencil)
          .label("Edit")
          .variant(Variant::Light)
          .color(ColorName::Blue),
      )
      .child(
        ActionIcon::new("ai-del", IconName::Trash2)
          .label("Delete")
          .variant(Variant::Light)
          .color(ColorName::Red),
      )
      .child(ThemeIcon::new(IconName::Star).color(ColorName::Yellow))
      .child(
        ThemeIcon::new(IconName::Check)
          .color(ColorName::Teal)
          .variant(Variant::Light),
      )
      .child(Indicator::new(ThemeIcon::new(IconName::Mail).color(ColorName::Grape)).label("3"))
      .child(CloseButton::new("close-demo"));

    let typography = Group::new()
      .align(Align::Center)
      .child(Anchor::new("anchor-demo", "A text link").color(ColorName::Blue))
      .child(Text::new("press"))
      .child(Kbd::new("⌘"))
      .child(Kbd::new("K"))
      .child(Text::new("to run"))
      .child(Code::new("guise::Button").color(ColorName::Grape));

    let chips = Group::new()
      .child(
        Chip::new("chip-demo", "Notifications")
          .checked(self.chip_on)
          .on_change(cx.listener(|this, _, _, cx| {
            this.chip_on = !this.chip_on;
            cx.notify();
          })),
      )
      .child(self.segmented.clone())
      .child(
        AvatarGroup::new()
          .avatars(["AL", "GH", "LT", "MK", "PR"])
          .limit(3),
      );

    let skeletons = Stack::new()
      .gap(Size::Xs)
      .child(Skeleton::new().height(14.0).width(220.0))
      .child(Skeleton::new().height(14.0).width(160.0))
      .child(
        Group::new()
          .child(Skeleton::new().circle(40.0))
          .child(Skeleton::new().height(40.0).width(180.0)),
      );

    Stack::new()
      .gap(Size::Md)
      .child(icons)
      .child(typography)
      .child(chips)
      .child(skeletons)
  }

  fn layout_demo(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let t = cx.global::<Theme>();
    let shade = if t.scheme.is_dark() { 8 } else { 1 };
    let col = |name| t.color(name, shade);
    let tile = |label: &str, color: Color| {
      Container::new()
        .padding(EdgeInsets::all(12.0))
        .radius(8.0)
        .color(color)
        .child(Text::new(label.to_string()).bold())
    };

    // Flutter-style Row with a filling tile + Spacer.
    let bar = Row::new()
      .gap(8.0)
      .child(tile("tile", col(ColorName::Blue)))
      .child(Spacer::new())
      .child(Badge::new("right"));

    // The same idea via the `row!` macro.
    let macro_row = row![
      Badge::new("row!").color(ColorName::Teal),
      SizedBox::width(12.0),
      Badge::new("macro").color(ColorName::Grape),
      Spacer::new(),
      Badge::new("end").color(ColorName::Orange),
    ];

    let columns = Row::new()
      .gap(12.0)
      .child(Expanded::new(tile("flex 2", col(ColorName::Indigo))).flex(2.0))
      .child(Expanded::new(tile("flex 1", col(ColorName::Cyan))).flex(1.0));

    col![
      bar,
      SizedBox::height(8.0),
      macro_row,
      SizedBox::height(8.0),
      columns,
    ]
  }

  fn reactive_demo(&self, cx: &mut Context<Self>) -> impl IntoElement {
    Stack::new()
      .gap(Size::Sm)
      .child(
        Text::new("Two independent views share one Signal via context:")
          .dimmed()
          .size(Size::Sm),
      )
      .child(self.counter.clone())
      .child(
        Group::new()
          .child(
            Button::new("count-dec", "−")
              .variant(Variant::Default)
              .on_click(cx.listener(|this, _, _, cx| this.count.update(cx, |n| *n -= 1))),
          )
          .child(
            Button::new("count-inc", "+")
              .on_click(cx.listener(|this, _, _, cx| this.count.update(cx, |n| *n += 1))),
          )
          .child(
            Button::new("count-reset", "Reset")
              .variant(Variant::Subtle)
              .on_click(cx.listener(|this, _, _, cx| this.count.set(cx, 0))),
          ),
      )
  }

  fn webview_demo(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let view = self.webview.clone();
    let reset = WEBVIEW_DEMO_HTML;
    Stack::new()
      .gap(Size::Sm)
      .child(
        Group::new()
          .align(Align::Center)
          .child(
            Button::new("wv-go", "Load example.com")
              .variant(Variant::Light)
              .on_click(cx.listener(|this, _, _, cx| {
                this
                  .webview
                  .update(cx, |wv, cx| wv.load_url("https://example.com", cx));
              })),
          )
          .child(
            Button::new("wv-home", "Reset")
              .variant(Variant::Subtle)
              .on_click(cx.listener(move |this, _, _, cx| {
                this.webview.update(cx, |wv, cx| wv.load_html(reset, cx));
              })),
          )
          .child(
            Text::new(SharedString::from(format!("title: {}", self.webview_title)))
              .size(Size::Sm)
              .dimmed(),
          ),
      )
      .child(view)
  }

  fn tableview_demo(&self) -> impl IntoElement {
    Stack::new()
      .gap(Size::Sm)
      .child(
        Text::new(
          "Click selects, ⌘-click toggles, ⇧-click ranges. Headers sort; \
                     drag a header's right edge to resize. Double-click or Enter activates.",
        )
        .size(Size::Sm)
        .dimmed(),
      )
      .child(self.employees.clone())
      .child(
        Text::new(self.tableview_status.clone())
          .size(Size::Xs)
          .dimmed(),
      )
  }

  fn dataview_demo(&self, cx: &mut Context<Self>) -> impl IntoElement {
    // Resolve everything read from `cx` before the listeners below.
    let (total, selected) = {
      let items = self.dataview_items.read(cx);
      let selected = self
        .dataview_selected
        .and_then(|i| items.get(i).cloned())
        .unwrap_or_else(|| "none".to_string());
      (items.len(), selected)
    };
    let query = self.dataview_query.read(cx).clone();
    let status = if query.is_empty() {
      format!("{total} items · selected: {selected}")
    } else {
      format!("{total} items · filter: \"{query}\" · selected: {selected}")
    };

    Stack::new()
      .gap(Size::Sm)
      .child(
        Text::new(
          "One Signal<Vec<String>> is the source of truth: the DataView renders \
                     it live, the input filters it through TextInput::bind, and the \
                     buttons write straight to the signal.",
        )
        .dimmed()
        .size(Size::Sm),
      )
      .child(self.dataview_filter.clone())
      .child(self.dataview.clone())
      .child(
        Group::new()
          .gap(Size::Sm)
          .align(Align::Center)
          .child(
            Button::new("dv-add", "Add item")
              .variant(Variant::Light)
              .on_click(cx.listener(|this, _, _, cx| {
                let n = this.dataview_items.read(cx).len() + 1;
                this
                  .dataview_items
                  .update(cx, move |items| items.push(format!("Item {n}")));
              })),
          )
          .child(
            Button::new("dv-pop", "Remove last")
              .variant(Variant::Subtle)
              .on_click(cx.listener(|this, _, _, cx| {
                this.dataview_items.update(cx, |items| {
                  items.pop();
                });
              })),
          )
          .child(Text::new(status).size(Size::Sm).dimmed()),
      )
  }

  fn tree_demo(&self) -> impl IntoElement {
    Stack::new()
      .gap(Size::Sm)
      .child(div().max_w(px(420.0)).child(self.tree.clone()))
      .child(Text::new(self.tree_status.clone()).dimmed().size(Size::Sm))
  }

  fn editor_demo(&self) -> impl IntoElement {
    Stack::new().gap(Size::Sm).child(self.editor.clone()).child(
      Text::new(format!("Status: {}", self.editor_status))
        .dimmed()
        .size(Size::Sm),
    )
  }

  /// The AI section: a transcript with a live composer on the left, and the
  /// controls and meters that surround a request on the right.
  fn ai_demo(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let border = cx.global::<Theme>().border().hsla();
    let pricing = self
      .model
      .read(cx)
      .selection()
      .and_then(|model| model.pricing)
      .unwrap_or(AIPricing::new(3.0, 15.0));
    let context = self
      .model
      .read(cx)
      .selection()
      .map(|model| model.context)
      .unwrap_or(200_000);

    let conversation = Stack::new()
      .gap(Size::Sm)
      .child(
        div()
          .h(px(360.0))
          .rounded(px(8.0))
          .border_1()
          .border_color(border)
          .child(self.chat.clone()),
      )
      .child(self.composer.clone());

    let controls = Stack::new()
      .gap(Size::Md)
      .child(self.model.clone())
      .child(AITokenMeter::new(self.ai_usage.total(), context).label("Context"))
      .child(
        AICost::new(self.ai_usage, pricing)
          .label("Session")
          .breakdown(true),
      )
      .child(Divider::new())
      .child(self.ai_settings.clone())
      .child(Divider::new())
      .child(AIThinking::new().label("Searching the web"))
      .child(
        AIToolCall::new("gallery-tool", "run_tests")
          .status(AIToolStatus::Error)
          .meta("2.4s")
          .result("3 failed, 415 passed")
          .open(true),
      );

    div()
      .flex()
      .flex_row()
      .gap(px(24.0))
      .w_full()
      .child(div().flex_1().min_w(px(0.0)).child(conversation))
      .child(div().w(px(300.0)).flex_none().child(controls))
  }

  fn panels_demo(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let ratio = self.split.read(cx).current_ratio();
    let border = cx.global::<Theme>().border().hsla();

    let panel = Panel::new()
      .id("demo-panel")
      .title("Project status")
      .description("Weekly summary")
      .icon(ThemeIcon::new(IconName::LayoutGrid).color(ColorName::Blue))
      .action(
        ActionIcon::new("panel-more", IconName::Ellipsis)
          .label("More")
          .size(Size::Sm),
      )
      .collapsible()
      .collapsed(self.panel_collapsed)
      .on_toggle(cx.listener(|this, _, _, cx| {
        this.panel_collapsed = !this.panel_collapsed;
        cx.notify();
      }))
      .footer(Text::new("Updated 5 minutes ago").size(Size::Xs).dimmed())
      .child(Text::new("Panels frame content with header, body and footer chrome.").size(Size::Sm));

    let split_frame = div()
      .h(px(240.0))
      .w_full()
      .border_1()
      .border_color(border)
      .rounded(px(8.0))
      .overflow_hidden()
      .child(self.split.clone());

    Stack::new()
      .gap(Size::Md)
      .child(panel)
      .child(
        Text::new(format!("Split ratio: {:.0}%", ratio * 100.0))
          .size(Size::Sm)
          .dimmed(),
      )
      .child(split_frame)
  }

  fn inputs2(&self, cx: &mut Context<Self>) -> impl IntoElement {
    Stack::new()
      .gap(Size::Md)
      .child(
        Group::new()
          .align(Align::Start)
          .gap(Size::Lg)
          .child(div().flex_1().child(self.password.clone()))
          .child(div().flex_1().child(self.brand_color.clone())),
      )
      .child(self.topics.clone())
      .child(
        Stack::new()
          .gap(Size::Xs)
          .child(Text::new("Price range").size(Size::Sm))
          .child(self.range.clone()),
      )
      .child(
        Group::new()
          .align(Align::Center)
          .gap(Size::Sm)
          .child(
            Rating::new("gallery-stars")
              .value(self.stars)
              .on_change(cx.listener(|this, value: &f32, _, cx| {
                this.stars = *value;
                cx.notify();
              })),
          )
          .child(
            Text::new(format!("{:.0} of 5", self.stars))
              .dimmed()
              .size(Size::Sm),
          ),
      )
      .child(
        Group::new()
          .align(Align::Center)
          .gap(Size::Md)
          .child(self.pin.clone())
          .child(
            Text::new(format!("code: {}", self.pin_value))
              .size(Size::Sm)
              .dimmed(),
          ),
      )
  }

  fn floating_overlays(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let t = cx.global::<Theme>();
    let border = t.border().hsla();
    let text = t.text().hsla();
    let dimmed = t.dimmed().hsla();

    let context_target = div()
      .id("ctx-target")
      .px(px(14.0))
      .py(px(10.0))
      .rounded(px(6.0))
      .border_1()
      .border_color(border)
      .text_color(text)
      .child("Right-click me")
      .on_mouse_down(
        MouseButton::Right,
        cx.listener(|this, ev: &MouseDownEvent, window, cx| {
          let position = ev.position;
          this
            .context_menu
            .update(cx, |menu, cx| menu.show(position, window, cx));
        }),
      );

    let loading_panel = div()
      .relative()
      .w(px(280.0))
      .h(px(120.0))
      .rounded(px(6.0))
      .border_1()
      .border_color(border)
      .p(px(12.0))
      .text_color(dimmed)
      .child("Panel content")
      .child(LoadingOverlay::new().visible(self.loading_visible));

    Group::new()
      .gap(Size::Md)
      .align(Align::Start)
      .child(context_target)
      .child(self.context_menu.clone())
      .child(self.hover_card.clone())
      .child(
        Stack::new().gap(Size::Sm).child(loading_panel).child(
          Switch::new("toggle-loading")
            .label("Loading")
            .checked(self.loading_visible)
            .on_change(cx.listener(|this, _, _, cx| {
              this.loading_visible = !this.loading_visible;
              cx.notify();
            })),
        ),
      )
      .child(
        Button::new("open-confirm", "Delete file…")
          .color(ColorName::Red)
          .variant(Variant::Light)
          .on_click(cx.listener(|this, _, _, cx| {
            this.confirm_open = true;
            cx.notify();
          })),
      )
  }

  fn shell_demo(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let border = cx.global::<Theme>().border().hsla();

    let shell = AppShell::new()
      .header(40.0, |_, _| {
        div()
          .flex()
          .items_center()
          .h_full()
          .px(px(14.0))
          .child(Text::new("Header").size(Size::Sm).dimmed())
      })
      .navbar(140.0, |_, _| {
        div()
          .p(px(10.0))
          .child(Text::new("Navbar").size(Size::Sm).dimmed())
      })
      .aside(120.0, |_, _| {
        div()
          .p(px(10.0))
          .child(Text::new("Aside").size(Size::Sm).dimmed())
      })
      .footer(28.0, |_, _| {
        div()
          .flex()
          .items_center()
          .h_full()
          .px(px(14.0))
          .child(Text::new("Footer").size(Size::Xs).dimmed())
      })
      .child(
        guise::layout::Container::new()
          .size(Size::Xs)
          .padding(Size::Md)
          .child(Space::y(Size::Md))
          .child(Title::new("Main").order(4))
          .child(Space::y(Size::Sm))
          .child(
            Text::new(
              "A Container caps the content width and centers it; \
                             Space adds fixed theme-scale gaps.",
            )
            .size(Size::Sm)
            .dimmed(),
          ),
      );

    Stack::new().gap(Size::Md).child(self.tabbar.clone()).child(
      div()
        .w_full()
        .h(px(280.0))
        .border_1()
        .border_color(border)
        .rounded(px(8.0))
        .overflow_hidden()
        .child(shell),
    )
  }

  fn typeextras(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let spoiler_text = "guise is a component library for gpui. It ships \
            themed buttons, inputs, overlays, data views, and a reactive binding layer. Every \
            visual value resolves from the active theme, so light and dark schemes come free. \
            This paragraph exists purely to be tall enough to clip.";

    let marked = Group::new()
      .gap(Size::Xs)
      .align(Align::Center)
      .child(Text::new("Highlight the"))
      .child(Mark::new("important part"))
      .child(Text::new("of a sentence, or go"))
      .child(Mark::new("teal instead").color(ColorName::Teal))
      .child(Text::new("."));

    let quote = Blockquote::new()
      .icon(IconName::Info)
      .color(ColorName::Indigo)
      .text("Life is like an npm install – you never know what you are going to get.")
      .cite("– Forrest Gump");

    let spoiler = Spoiler::new("gallery-spoiler")
      .max_height(44.0)
      .expanded(self.spoiler_open)
      .on_toggle(cx.listener(|this, _, _, cx| {
        this.spoiler_open = !this.spoiler_open;
        cx.notify();
      }))
      .child(Text::new(spoiler_text).size(Size::Sm));

    Stack::new()
      .gap(Size::Md)
      .child(marked)
      .child(quote)
      .child(spoiler)
  }

  fn navigation(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let links = ["Dashboard", "Components", "Settings"];
    let sidebar = links
      .iter()
      .enumerate()
      .fold(Stack::new().gap(Size::Xs), |stack, (i, label)| {
        stack.child(
          NavLink::new(("nav", i), *label)
            .icon("•")
            .active(self.nav_active == i)
            .on_click(cx.listener(move |this, _, _, cx| {
              this.nav_active = i;
              cx.notify();
            })),
        )
      });

    let stepper = Stepper::new()
      .step_desc("Account", "Create account")
      .step_desc("Profile", "Add details")
      .step_desc("Review", "Confirm & finish")
      .active(1)
      .color(ColorName::Teal);

    Stack::new()
      .gap(Size::Md)
      .child(Breadcrumbs::new().items(["Home", "Projects", "guise"]))
      .child(
        Paper::new()
          .with_border(true)
          .padding(Size::Sm)
          .child(sidebar),
      )
      .child(stepper)
      .child(self.pagination.clone())
  }

  fn inputs(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let plans = ["Free", "Pro", "Enterprise"];
    let radios = plans
      .iter()
      .enumerate()
      .fold(Group::new().gap(Size::Lg), |group, (i, label)| {
        group.child(
          Radio::new(("plan", i))
            .label(*label)
            .checked(self.plan == i)
            .on_change(cx.listener(move |this, _, _, cx| {
              this.plan = i;
              cx.notify();
            })),
        )
      });

    Stack::new()
      .gap(Size::Md)
      .child(self.name.clone())
      .child(self.framework.clone())
      .child(
        Checkbox::new("agree")
          .label("I agree to the terms")
          .checked(self.agree)
          .on_change(cx.listener(|this, _, _, cx| {
            this.agree = !this.agree;
            cx.notify();
          })),
      )
      .child(
        Switch::new("notifications")
          .label("Enable notifications")
          .checked(self.notifications)
          .on_change(cx.listener(|this, _, _, cx| {
            this.notifications = !this.notifications;
            cx.notify();
          })),
      )
      .child(radios)
  }

  fn overlays(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let t = cx.global::<Theme>();
    let border = t.border().hsla();
    let text = t.text().hsla();

    Group::new()
      .gap(Size::Md)
      .child(
        Button::new("open-modal", "Open modal").on_click(cx.listener(|this, _, _, cx| {
          this.modal_open = true;
          cx.notify();
        })),
      )
      .child(self.menu.clone())
      .child(self.menubar.clone())
      .child(
        div()
          .id("tooltip-target")
          .px(px(14.0))
          .py(px(8.0))
          .rounded(px(6.0))
          .border_1()
          .border_color(border)
          .text_color(text)
          .child("Hover for tooltip")
          .tooltip(guise::tooltip("Tooltips float above, fully themed.")),
      )
  }

  fn data_display(&self) -> impl IntoElement {
    let avatars = Group::new()
      .gap(Size::Sm)
      .child(Avatar::new("AL").color(ColorName::Blue))
      .child(Avatar::new("GH").color(ColorName::Teal))
      .child(
        Avatar::new("LT")
          .color(ColorName::Grape)
          .variant(Variant::Filled),
      )
      .child(Avatar::new("+5").color(ColorName::Gray));

    let list = List::new().items([
      "Install guise",
      "Install a Theme global at startup",
      "Compose components with the builder API",
      "Ship it",
    ]);

    let table = Table::new()
      .with_border(true)
      .striped(true)
      .highlight_on_hover(true)
      .head(["Name", "Role", "Status"])
      .row(["Ada", "Admin", "Active"])
      .row(["Grace", "Editor", "Active"])
      .row(["Linus", "Viewer", "Invited"]);

    Stack::new()
      .gap(Size::Md)
      .child(avatars)
      .child(self.tabs.clone())
      .child(self.accordion.clone())
      .child(list)
      .child(table)
  }

  fn dates(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let host = cx.entity().downgrade();
    Stack::new()
      .gap(Size::Md)
      .child(
        Group::new()
          .grow(true)
          .align(Align::Start)
          .child(div().flex_1().child(self.datepicker.clone()))
          .child(div().flex_1().child(self.timepicker.clone())),
      )
      .child(Text::new(self.picked_date.clone()).size(Size::Sm).dimmed())
      .child(self.fileinput.clone())
      .child(
        Dropzone::new("gallery-dropzone")
          .label("Drop files here")
          .hint("any type — the count lands below")
          .height(110.0)
          .on_files(move |paths, cx| {
            host
              .update(cx, |this, cx| {
                this.drop_note = SharedString::from(format!("dropped {} file(s)", paths.len()));
                cx.notify();
              })
              .ok();
          }),
      )
      .child(Text::new(self.drop_note.clone()).size(Size::Sm).dimmed())
  }

  fn dnd(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let queue = self.queue.clone();
    let labels = self.queue.clone();
    let host = cx.entity().downgrade();
    let list = SortableList::new("gallery-queue", queue.len(), move |i, _window, cx| {
      let t = theme(cx);
      div()
        .px(px(10.0))
        .py(px(7.0))
        .rounded(px(6.0))
        .bg(t.surface_hover().hsla())
        .child(Text::new(queue[i].clone()).size(Size::Sm))
    })
    .label_of(move |i| labels[i].clone())
    .on_reorder(move |from, to, _window, cx| {
      host
        .update(cx, |this, cx| {
          guise::dnd::apply_reorder(&mut this.queue, from, to);
          cx.notify();
        })
        .ok();
    });

    Stack::new()
      .gap(Size::Sm)
      .child(Text::new("Drag rows to reorder").size(Size::Sm).dimmed())
      .child(list)
  }

  fn motion(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let t = theme(cx);
    let card_bg = t.surface_hover().hsla();
    let accent = t.color(ColorName::Blue, 5).hsla();
    let accent_soft = t.color(ColorName::Grape, 5).hsla();
    let dimmed = t.dimmed().hsla();
    let epoch = self.motion_epoch;
    let player = self.motion_player.read(cx);
    let playing = player.is_playing();
    let progress = player.progress();

    let toggle = Button::new(
      "motion-toggle",
      if self.collapse_open {
        "Collapse"
      } else {
        "Expand"
      },
    )
    .variant(Variant::Light)
    .on_click(cx.listener(|this, _ev, _window, cx| {
      this.collapse_open = !this.collapse_open;
      cx.notify();
    }));

    // --- staggered entrance ------------------------------------------
    // One clip per element, offset by index. Replaying is a matter of
    // handing them new ids: a mounted animation has already played.
    let words = ["Keyframes", "Springs", "Stagger", "Timelines"];
    let rise = Stagger::new(70.0).from(StaggerFrom::First);
    let stagger_row = Group::new().gap(Size::Xs).children(
      words
        .iter()
        .enumerate()
        .map(|(index, word)| {
          Animated::new(("motion-stagger", epoch * words.len() + index))
            .motion(motion! {
                enter: slide_up 14;
                duration: 460;
                ease: out back;
                delay: rise.at(index, words.len());
            })
            .child(Badge::new(*word).color(ColorName::Blue))
        })
        .collect::<Vec<_>>(),
    );

    // --- a keyframed one-shot, colours read live from the theme -------
    let swatch = Animated::new(("motion-keyframes", epoch))
      .motion(
        Motion::new()
          .duration(900.0)
          .ease(Easing::InOut(Curve::Quad))
          .keyframes(
            Prop::Background,
            accent_soft,
            [
              Keyframe::to(accent),
              Keyframe::to(accent_soft).ease(Easing::Out(Curve::Expo)),
            ],
          )
          .keyframes(Prop::Radius, 6.0, [Keyframe::to(26.0), Keyframe::to(6.0)]),
      )
      .child(div().w(px(44.0)).h(px(44.0)));

    let replay = Button::new("motion-replay", "Replay")
      .variant(Variant::Light)
      .on_click(cx.listener(|this, _ev, _window, cx| {
        this.motion_epoch += 1;
        cx.notify();
      }));

    // --- the controllable player --------------------------------------
    let play_pause = Button::new("motion-play", if playing { "Pause" } else { "Play" })
      .variant(Variant::Light)
      .on_click(cx.listener(|this, _ev, _window, cx| {
        this
          .motion_player
          .update(cx, |player, cx| player.toggle(cx));
      }));
    let reverse = Button::new("motion-reverse", "Reverse")
      .variant(Variant::Light)
      .on_click(cx.listener(|this, _ev, _window, cx| {
        this
          .motion_player
          .update(cx, |player, cx| player.reverse(cx));
      }));
    let restart = Button::new("motion-restart", "Restart")
      .variant(Variant::Light)
      .on_click(cx.listener(|this, _ev, _window, cx| {
        this
          .motion_player
          .update(cx, |player, cx| player.restart(cx));
      }));
    let slower = Button::new("motion-slow", "0.5x")
      .variant(Variant::Subtle)
      .on_click(cx.listener(|this, _ev, _window, cx| {
        this
          .motion_player
          .update(cx, |player, cx| player.set_speed(0.5, cx));
      }));
    let normal = Button::new("motion-normal", "1x")
      .variant(Variant::Subtle)
      .on_click(cx.listener(|this, _ev, _window, cx| {
        this
          .motion_player
          .update(cx, |player, cx| player.set_speed(1.0, cx));
      }));

    let stage = div()
      .w_full()
      .h(px(112.0))
      .p(px(12.0))
      .rounded(px(10.0))
      .bg(card_bg)
      .overflow_hidden()
      .child(
        Animated::new("motion-player")
          .animator(&self.motion_player)
          .child(div().w(px(52.0)).h(px(52.0)).bg(accent)),
      );

    Stack::new()
      .gap(Size::Sm)
      .child(Group::new().child(toggle))
      .child(
        Collapse::new("motion-collapse")
          .open(self.collapse_open)
          .height(88.0)
          .easing(Easing::EaseInOutCubic)
          .child(Card::new().child(
            Text::new("A real height collapse — opening and closing both animate.").size(Size::Sm),
          )),
      )
      .child(
        Transition::new("motion-enter")
          .kind(TransitionKind::SlideUp)
          .easing(Easing::Spring(Spring::default()))
          .child(Text::new("Spring-eased entrance").size(Size::Sm).dimmed()),
      )
      .child(Divider::new())
      .child(
        Text::new("Motion: keyframes, stagger, and a playhead you own")
          .size(Size::Sm)
          .dimmed(),
      )
      .child(
        Group::new()
          .gap(Size::Sm)
          .align(Align::Center)
          .child(swatch)
          .child(stagger_row)
          .child(replay),
      )
      .child(stage)
      .child(
        Group::new()
          .gap(Size::Xs)
          .align(Align::Center)
          .child(play_pause)
          .child(reverse)
          .child(restart)
          .child(slower)
          .child(normal)
          .child(
            div()
              .text_color(dimmed)
              .child(SharedString::from(format!("{:.0}%", progress * 100.0))),
          ),
      )
  }

  fn misc(&self, cx: &mut Context<Self>) -> impl IntoElement {
    let start_tour = Button::new("misc-tour", "Start tour")
      .variant(Variant::Default)
      .on_click(cx.listener(|this, _ev, _window, cx| {
        this.tour.update(cx, |tour, cx| tour.start(cx));
      }));

    Stack::new()
      .gap(Size::Md)
      .child(self.navmenu.clone())
      .child(self.carousel.clone())
      .child(
        Group::new()
          .grow(true)
          .align(Align::End)
          .child(div().flex_1().child(self.autocomplete.clone()))
          .child(start_tour),
      )
      .child(self.transfer.clone())
      .child(self.tour.clone())
  }

  fn close_modal(&mut self, _: &gpui::ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
    self.modal_open = false;
    cx.notify();
  }

  fn modal(&self, cx: &mut Context<Self>) -> impl IntoElement {
    Modal::new()
      .title("Delete project?")
      .on_close(cx.listener(Self::close_modal))
      .child(
        Text::new("This action is permanent and cannot be undone.")
          .dimmed()
          .size(Size::Sm),
      )
      .child(
        Group::new()
          .justify(Justify::End)
          .gap(Size::Sm)
          .child(
            Button::new("modal-cancel", "Cancel")
              .variant(Variant::Default)
              .on_click(cx.listener(Self::close_modal)),
          )
          .child(
            Button::new("modal-confirm", "Delete")
              .color(ColorName::Red)
              .on_click(cx.listener(Self::close_modal)),
          ),
      )
  }
}

impl Render for Gallery {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let t = cx.global::<Theme>();
    let body = t.body().hsla();
    let text = t.text().hsla();
    let font = t.font_family.clone();
    let is_dark = t.scheme.is_dark();

    let gallery = cx.weak_entity();
    let main = list(self.sections.clone(), move |index, _window, cx| {
      gallery
        .update(cx, |gallery, cx| gallery.gallery_item(index, cx))
        .unwrap_or_else(|_| div().into_any_element())
    })
    .flex_1()
    .min_h(px(0.0))
    .px(px(48.0))
    .pt(px(40.0));

    let status = StatusBar::new()
      .left(Text::new("guise gallery").size(Size::Xs))
      .left(
        Badge::new(if is_dark { "Dark" } else { "Light" })
          .size(Size::Sm)
          .color(if is_dark {
            ColorName::Grape
          } else {
            ColorName::Yellow
          }),
      )
      .center(Text::new("Ready").size(Size::Xs).dimmed())
      .right(
        Text::new(concat!("v", env!("CARGO_PKG_VERSION")))
          .size(Size::Xs)
          .dimmed(),
      );

    let mut root = div()
      .relative()
      .size_full()
      .flex()
      .flex_col()
      .bg(body)
      .text_color(text)
      .font_family(font)
      .child(main)
      .child(status);

    if self.modal_open {
      root = root.child(self.modal(cx));
    }
    if self.confirm_open {
      root = root.child(
        ConfirmModal::new()
          .title("Delete file?")
          .message("del.rs will be moved to the Trash.")
          .confirm_label("Delete")
          .danger()
          .on_confirm(cx.listener(|this, _, _, cx| {
            this.confirm_open = false;
            cx.notify();
          }))
          .on_cancel(cx.listener(|this, _, _, cx| {
            this.confirm_open = false;
            cx.notify();
          })),
      );
    }
    root
  }
}

/// The accent "Toggle theme" pill in the header flips the global color scheme.
/// It goes through the manager rather than writing `Theme` directly, so the
/// theme picker further down the page stays in step with it.
pub fn toggle_theme(window: &mut Window, cx: &mut App) {
  ThemeManager::toggle(cx);
  window.refresh();
}

fn main() {
  gpui::Application::new().run(|cx: &mut App| {
    // The registry behind the "Theme manager" section — and the source of
    // truth for the header's toggle and the View menu.
    ThemeManager::new()
      .with_presets()
      .choice(ThemeChoice::Fixed("dark".into()))
      .install(cx);
    // The inspector's record store. Installing it is what makes the
    // `devtools::*` reporting calls anything other than a no-op.
    DevToolsState::new().init(cx);

    // The native window menu. Actions dispatch to the global handlers below.
    cx.set_menus(vec![
      gpui::Menu {
        name: SharedString::new_static("guise gallery"),
        items: vec![
          gpui::MenuItem::action("Toggle Theme", ToggleThemeAction),
          gpui::MenuItem::separator(),
          gpui::MenuItem::action("Quit", QuitAction),
        ],
      },
      gpui::Menu {
        name: SharedString::new_static("View"),
        items: vec![gpui::MenuItem::action("Toggle Theme", ToggleThemeAction)],
      },
    ]);
    cx.on_action::<QuitAction>(|_, cx| cx.quit());
    cx.on_action::<ToggleThemeAction>(|_, cx| ThemeManager::toggle(cx));

    let bounds = Bounds::centered(None, size(px(960.0), px(880.0)), cx);
    cx.open_window(
      WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: Some(TitlebarOptions {
          title: Some(SharedString::new_static("guise gallery")),
          ..Default::default()
        }),
        ..Default::default()
      },
      |window, cx| {
        // Keeps `ThemeChoice::System` honest when macOS flips at sundown.
        ThemeManager::watch(window, cx);
        cx.new(Gallery::new)
      },
    )
    .expect("open window");
    cx.activate(true);
  });
}
