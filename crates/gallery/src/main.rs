//! Gallery — a live showcase of `guise` components, in the spirit of the
//! Mantine docs. Run with `cargo run -p gallery`.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    div, px, size, App, Application, Bounds, Context, Entity, IntoElement, MouseButton,
    MouseDownEvent, SharedString, TitlebarOptions, Window, WindowBounds, WindowOptions,
};

use guise::flex::{Container, EdgeInsets, Expanded, Row, SizedBox, Spacer};
use guise::prelude::*;

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
    // Typography extras
    spoiler_open: bool,
    /// Section keys whose "view source" panel is currently expanded.
    code_open: HashSet<&'static str>,
    /// One copy button per section, keyed by section key.
    copy_buttons: HashMap<&'static str, Entity<CopyButton>>,
    /// The "Builder | Macros" control + the chosen style.
    code_style: Entity<SegmentedControl>,
    use_macros: bool,
}

/// Every section's `(key, snippet)` — drives the copy buttons and code panels.
/// Each snippet carries both a plain and a macro variant.
const SECTION_SOURCES: &[(&str, code::Snippet)] = &[
    ("buttons", code::BUTTONS),
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
    ("editor", code::EDITOR),
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
                .data(["gpui", "Mantine", "SwiftUI", "Flutter", "Jetpack Compose"])
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
                    Text::new("A Mantine-inspired component library for gpui.").size(Size::Sm)
                })
                .item("Is it themeable?", |_, _| {
                    Text::new("Yes — light/dark plus the full Mantine palette.").size(Size::Sm)
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
                TableViewEvent::Activated(row) => {
                    SharedString::from(format!("Activated row {row}"))
                }
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
                "Mantine".to_string(),
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
                TreeViewEvent::Toggled(id, open) => {
                    SharedString::from(format!("{id} expanded: {open}"))
                }
                TreeViewEvent::Activated(id) => SharedString::from(format!("activated: {id}")),
            };
            cx.notify();
        })
        .detach();

        // Editor: syntax-highlighted Rust buffer; Cmd+Enter emits Run.
        let editor = cx.new(|cx| {
            Editor::new(cx)
                .language(Language::Rust)
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
                    this.editor_status =
                        SharedString::from(format!("{} lines", text.lines().count().max(1)));
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

        Gallery {
            agree: false,
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
            code_open: HashSet::new(),
            copy_buttons,
            code_style,
            use_macros: false,
        }
    }

    /// Wrap a section body with its title and a "view source" toggle (`</>`).
    /// Clicking the toggle reveals the example's code (with a copy button)
    /// beneath the demo.
    fn section(
        &self,
        cx: &mut Context<Self>,
        key: &'static str,
        title: &'static str,
        body: impl IntoElement,
    ) -> impl IntoElement {
        let open = self.code_open.contains(key);
        let toggle = ActionIcon::new(SharedString::from(format!("code-{key}")), "</>")
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

    fn polish(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let icons = Group::new()
            .child(
                ActionIcon::new("ai-edit", "✎")
                    .variant(Variant::Light)
                    .color(ColorName::Blue),
            )
            .child(
                ActionIcon::new("ai-del", "🗑")
                    .variant(Variant::Light)
                    .color(ColorName::Red),
            )
            .child(ThemeIcon::new("★").color(ColorName::Yellow))
            .child(
                ThemeIcon::new("✓")
                    .color(ColorName::Teal)
                    .variant(Variant::Light),
            )
            .child(Indicator::new(ThemeIcon::new("✉").color(ColorName::Grape)).label("3"))
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
                            .on_click(
                                cx.listener(|this, _, _, cx| this.count.update(cx, |n| *n -= 1)),
                            ),
                    )
                    .child(
                        Button::new("count-inc", "+").on_click(
                            cx.listener(|this, _, _, cx| this.count.update(cx, |n| *n += 1)),
                        ),
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
                                this.webview
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
                                this.dataview_items
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

    fn panels_demo(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let ratio = self.split.read(cx).current_ratio();
        let border = cx.global::<Theme>().border().hsla();

        let panel = Panel::new()
            .id("demo-panel")
            .title("Project status")
            .description("Weekly summary")
            .icon(ThemeIcon::new("▦").color(ColorName::Blue))
            .action(ActionIcon::new("panel-more", "…").size(Size::Sm))
            .collapsible()
            .collapsed(self.panel_collapsed)
            .on_toggle(cx.listener(|this, _, _, cx| {
                this.panel_collapsed = !this.panel_collapsed;
                cx.notify();
            }))
            .footer(Text::new("Updated 5 minutes ago").size(Size::Xs).dimmed())
            .child(
                Text::new("Panels frame content with header, body and footer chrome.")
                    .size(Size::Sm),
            );

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
                    this.context_menu
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
        let spoiler_text = "guise is a Mantine-inspired component library for gpui. It ships \
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
        let sidebar =
            links
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
        let radios =
            plans
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

        // Build each section (body + "view source" toggle). Bodies that need
        // `cx` are bound first so `self.section(cx, …)` doesn't double-borrow it.
        let buttons = self.section(cx, "buttons", "Buttons", sections::buttons());
        let webview_body = self.webview_demo(cx);
        let webview = self.section(cx, "webview", "WebView (native)", webview_body);
        let badges = self.section(cx, "badges", "Badges", sections::badges());
        let inputs_body = self.inputs(cx);
        let inputs = self.section(cx, "inputs", "Inputs", inputs_body);
        let inputs2_body = self.inputs2(cx);
        let inputs2 = self.section(cx, "inputs2", "More inputs", inputs2_body);
        let overlays_body = self.overlays(cx);
        let overlays = self.section(cx, "overlays", "Overlays", overlays_body);
        let overlays2_body = self.floating_overlays(cx);
        let overlays2 = self.section(
            cx,
            "overlays2",
            "Context menu, hover card & confirm",
            overlays2_body,
        );
        let feedback = self.section(cx, "feedback", "Feedback", sections::feedback());
        let data_body = self.data_display();
        let data = self.section(cx, "data", "Data display", data_body);
        let tableview_body = self.tableview_demo();
        let tableview = self.section(cx, "tableview", "TableView", tableview_body);
        let tree_body = self.tree_demo();
        let tree = self.section(cx, "tree", "TreeView", tree_body);
        let charts = self.section(cx, "charts", "Charts", sections::charts());
        let editor_body = self.editor_demo();
        let editor = self.section(cx, "editor", "Editor", editor_body);
        let nav_body = self.navigation(cx);
        let navigation = self.section(cx, "navigation", "Navigation", nav_body);
        let shell_body = self.shell_demo(cx);
        let shell = self.section(cx, "shell", "App structure", shell_body);
        let panels_body = self.panels_demo(cx);
        let panels = self.section(cx, "panels", "Panels & SplitPanel", panels_body);
        let polish_body = self.polish(cx);
        let polish = self.section(cx, "polish", "Polish", polish_body);
        let layout_body = self.layout_demo(cx);
        let layout = self.section(cx, "layout", "Flex layout & macros", layout_body);
        let reactive_body = self.reactive_demo(cx);
        let reactive = self.section(
            cx,
            "reactive",
            "Reactive state (Context / Signal)",
            reactive_body,
        );
        let dataview_body = self.dataview_demo(cx);
        let dataview = self.section(
            cx,
            "dataview",
            "DataView (collection bindings)",
            dataview_body,
        );
        let cards = self.section(cx, "cards", "Cards", sections::cards());
        let typography = self.section(cx, "typography", "Typography", sections::typography());
        let typeextras_body = self.typeextras(cx);
        let typeextras = self.section(cx, "typeextras", "Typography extras", typeextras_body);
        let media = self.section(cx, "media", "Image", sections::media());
        let palette_body = sections::palette(cx);
        let palette = self.section(cx, "palette", "Palette", palette_body);

        let main = div()
            .id("scroll")
            .flex_1()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .px(px(48.0))
            .py(px(40.0))
            .child(
                Stack::new()
                    .gap(Size::Xl)
                    .child(sections::header())
                    .child(
                        Group::new()
                            .align(Align::Center)
                            .child(Text::new("Code examples:").size(Size::Sm).dimmed())
                            .child(self.code_style.clone()),
                    )
                    .child(buttons)
                    .child(webview)
                    .child(badges)
                    .child(inputs)
                    .child(inputs2)
                    .child(overlays)
                    .child(overlays2)
                    .child(feedback)
                    .child(data)
                    .child(tableview)
                    .child(tree)
                    .child(charts)
                    .child(editor)
                    .child(navigation)
                    .child(shell)
                    .child(panels)
                    .child(polish)
                    .child(layout)
                    .child(reactive)
                    .child(dataview)
                    .child(cards)
                    .child(typography)
                    .child(typeextras)
                    .child(media)
                    .child(palette),
            );

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
pub fn toggle_theme(window: &mut Window, cx: &mut App) {
    let next = cx.global::<Theme>().scheme.toggled();
    cx.global_mut::<Theme>().scheme = next;
    window.refresh();
}

fn main() {
    Application::new().run(|cx: &mut App| {
        Theme::dark().init(cx);

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
        cx.on_action::<ToggleThemeAction>(|_, cx| {
            let next = cx.global::<Theme>().scheme.toggled();
            cx.global_mut::<Theme>().scheme = next;
            cx.refresh_windows();
        });

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
            |_window, cx| cx.new(Gallery::new),
        )
        .expect("open window");
        cx.activate(true);
    });
}
