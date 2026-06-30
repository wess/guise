//! Gallery — a live showcase of `guise` components, in the spirit of the
//! Mantine docs. Run with `cargo run -p gallery`.

use std::collections::{HashMap, HashSet};

use gpui::prelude::*;
use gpui::{
    div, px, size, App, Application, Bounds, Context, Entity, IntoElement, SharedString,
    TitlebarOptions, Window, WindowBounds, WindowOptions,
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
    ("overlays", code::OVERLAYS),
    ("feedback", code::FEEDBACK),
    ("data", code::DATA),
    ("navigation", code::NAVIGATION),
    ("polish", code::POLISH),
    ("layout", code::LAYOUT),
    ("reactive", code::REACTIVE),
    ("cards", code::CARDS),
    ("typography", code::TYPOGRAPHY),
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
        .unwrap_or(code::Snippet { plain: "", macros: "" })
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
        cx.subscribe(&code_style, |this, _seg, event: &SegmentedControlEvent, cx| {
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
        })
        .detach();

        // A native web view loading an inline page (no network needed). Its
        // title updates flow back through `WebViewEvent` into the status text.
        let webview = cx.new(|cx| {
            WebView::new(cx)
                .html(WEBVIEW_DEMO_HTML)
                .height(320.0)
        });
        cx.subscribe(&webview, |this, _view, event: &WebViewEvent, cx| {
            if let WebViewEvent::TitleChanged(title) = event {
                this.webview_title = title.clone();
                cx.notify();
            }
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
            .variant(if open { Variant::Light } else { Variant::Subtle })
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
                    div()
                        .relative()
                        .child(code_block(cx, &source))
                        .child(
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
            .child(ActionIcon::new("ai-edit", "✎").variant(Variant::Light).color(ColorName::Blue))
            .child(ActionIcon::new("ai-del", "🗑").variant(Variant::Light).color(ColorName::Red))
            .child(ThemeIcon::new("★").color(ColorName::Yellow))
            .child(ThemeIcon::new("✓").color(ColorName::Teal).variant(Variant::Light))
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
            .child(AvatarGroup::new().avatars(["AL", "GH", "LT", "MK", "PR"]).limit(3));

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
                        Button::new("count-dec", "−").variant(Variant::Default).on_click(
                            cx.listener(|this, _, _, cx| this.count.update(cx, |n| *n -= 1)),
                        ),
                    )
                    .child(
                        Button::new("count-inc", "+").on_click(cx.listener(|this, _, _, cx| {
                            this.count.update(cx, |n| *n += 1)
                        })),
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
                                this.webview.update(cx, |wv, cx| {
                                    wv.load_url("https://example.com", cx)
                                });
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
                        Text::new(SharedString::from(format!(
                            "title: {}",
                            self.webview_title
                        )))
                        .size(Size::Sm)
                        .dimmed(),
                    ),
            )
            .child(view)
    }

    fn navigation(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let links = ["Dashboard", "Components", "Settings"];
        let sidebar = links.iter().enumerate().fold(
            Stack::new().gap(Size::Xs),
            |stack, (i, label)| {
                stack.child(
                    NavLink::new(("nav", i), *label)
                        .icon("•")
                        .active(self.nav_active == i)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.nav_active = i;
                            cx.notify();
                        })),
                )
            },
        );

        let stepper = Stepper::new()
            .step_desc("Account", "Create account")
            .step_desc("Profile", "Add details")
            .step_desc("Review", "Confirm & finish")
            .active(1)
            .color(ColorName::Teal);

        Stack::new()
            .gap(Size::Md)
            .child(Breadcrumbs::new().items(["Home", "Projects", "guise"]))
            .child(Paper::new().with_border(true).padding(Size::Sm).child(sidebar))
            .child(stepper)
            .child(self.pagination.clone())
    }

    fn inputs(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let plans = ["Free", "Pro", "Enterprise"];
        let radios = plans.iter().enumerate().fold(
            Group::new().gap(Size::Lg),
            |group, (i, label)| {
                group.child(
                    Radio::new(("plan", i))
                        .label(*label)
                        .checked(self.plan == i)
                        .on_change(cx.listener(move |this, _, _, cx| {
                            this.plan = i;
                            cx.notify();
                        })),
                )
            },
        );

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
                Button::new("open-modal", "Open modal").on_click(cx.listener(
                    |this, _, _, cx| {
                        this.modal_open = true;
                        cx.notify();
                    },
                )),
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
            .child(Avatar::new("LT").color(ColorName::Grape).variant(Variant::Filled))
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
        let overlays_body = self.overlays(cx);
        let overlays = self.section(cx, "overlays", "Overlays", overlays_body);
        let feedback = self.section(cx, "feedback", "Feedback", sections::feedback());
        let data_body = self.data_display();
        let data = self.section(cx, "data", "Data display", data_body);
        let nav_body = self.navigation(cx);
        let navigation = self.section(cx, "navigation", "Navigation", nav_body);
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
        let cards = self.section(cx, "cards", "Cards", sections::cards());
        let typography =
            self.section(cx, "typography", "Typography", sections::typography());
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
                    .child(overlays)
                    .child(feedback)
                    .child(data)
                    .child(navigation)
                    .child(polish)
                    .child(layout)
                    .child(reactive)
                    .child(cards)
                    .child(typography)
                    .child(palette),
            );

        let status = StatusBar::new()
            .left(Text::new("guise gallery").size(Size::Xs))
            .left(
                Badge::new(if is_dark { "Dark" } else { "Light" })
                    .size(Size::Sm)
                    .color(if is_dark { ColorName::Grape } else { ColorName::Yellow }),
            )
            .center(Text::new("Ready").size(Size::Xs).dimmed())
            .right(Text::new("v0.2.1").size(Size::Xs).dimmed());

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
