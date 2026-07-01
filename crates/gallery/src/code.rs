//! Hand-written source snippets shown by each section's "view code" toggle.
//! Each section has two variants — plain builder API and the layout macros —
//! toggled by the gallery's "Builder | Macros" control. Kept representative of
//! (not byte-identical to) the rendered example.

/// A pair of equivalent snippets for one section.
#[derive(Clone, Copy)]
pub struct Snippet {
    pub plain: &'static str,
    pub macros: &'static str,
}

impl Snippet {
    pub fn pick(self, macros: bool) -> &'static str {
        if macros {
            self.macros
        } else {
            self.plain
        }
    }
}

pub const BUTTONS: Snippet = Snippet {
    plain: r#"Group::new().children([
    Button::new("filled", "Filled").variant(Variant::Filled),
    Button::new("light", "Light").variant(Variant::Light),
    Button::new("outline", "Outline").variant(Variant::Outline),
]);"#,
    macros: r#"hstack![
    button!("filled", "Filled").variant(Variant::Filled),
    button!("light", "Light").variant(Variant::Light),
    button!("outline", "Outline").variant(Variant::Outline),
];"#,
};

pub const WEBVIEW: Snippet = Snippet {
    plain: r#"// Stateful entity — owns a native OS web view via `wry`.
let view = cx.new(|cx| {
    WebView::new(cx)
        .url("https://example.com")
        .height(360.0)
});

// Subscribe for title / load events:
cx.subscribe(&view, |_this, _view, event: &WebViewEvent, _cx| {
    if let WebViewEvent::TitleChanged(title) = event {
        println!("title: {title}");
    }
}).detach();"#,
    macros: r#"let view = cx.new(|cx| {
    WebView::new(cx)
        .url("https://example.com")
        .height(360.0)
});"#,
};

pub const BADGES: Snippet = Snippet {
    plain: r#"Group::new().children([
    Badge::new("Blue").color(ColorName::Blue),
    Badge::new("Light").variant(Variant::Light),
    Badge::new("Outline").variant(Variant::Outline),
]);"#,
    macros: r#"hstack![
    badge!("Blue").color(ColorName::Blue),
    badge!("Light").variant(Variant::Light),
    badge!("Outline").variant(Variant::Outline),
];"#,
};

pub const INPUTS: Snippet = Snippet {
    plain: r#"// Stateful entities — create with cx.new, keep the handle.
let name = cx.new(|cx| TextInput::new(cx).label("Name"));

Stack::new()
    .gap(Size::Md)
    .child(name)
    .child(
        Checkbox::new("agree")
            .checked(self.agree)
            .on_change(cx.listener(|this, _, _, cx| {
                this.agree = !this.agree;
                cx.notify();
            })),
    );"#,
    macros: r#"// Stateful entities — create with cx.new, keep the handle.
let name = cx.new(|cx| TextInput::new(cx).label("Name"));

vstack![
    name,
    Checkbox::new("agree")
        .checked(self.agree)
        .on_change(cx.listener(|this, _, _, cx| {
            this.agree = !this.agree;
            cx.notify();
        })),
]
.gap(Size::Md);"#,
};

pub const OVERLAYS: Snippet = Snippet {
    // Mixed child types (Button, Menu entity, div) can't go in a homogeneous
    // `.children([...])` — chain `.child()` instead...
    plain: r#"Group::new()
    .child(Button::new("open-modal", "Open modal").on_click(
        cx.listener(|this, _, _, cx| { this.modal_open = true; cx.notify(); }),
    ))
    .child(menu.clone())
    .child(div().id("tip").child("Hover me")
        .tooltip(guise::tooltip("A themed tooltip")));"#,
    // ...or let the macro flatten them (it expands to `.child()` calls).
    macros: r#"hstack![
    Button::new("open-modal", "Open modal").on_click(
        cx.listener(|this, _, _, cx| { this.modal_open = true; cx.notify(); }),
    ),
    menu.clone(),
    div().id("tip").child("Hover me")
        .tooltip(guise::tooltip("A themed tooltip")),
];"#,
};

pub const FEEDBACK: Snippet = Snippet {
    plain: r#"Stack::new()
    .gap(Size::Md)
    .child(Alert::new("Saved.").title("Success").color(ColorName::Teal).icon("✓"))
    .child(Progress::new(60.0).color(ColorName::Teal));"#,
    macros: r#"vstack![
    Alert::new("Saved.").title("Success").color(ColorName::Teal).icon("✓"),
    Progress::new(60.0).color(ColorName::Teal),
]
.gap(Size::Md);"#,
};

pub const DATA: Snippet = Snippet {
    plain: r#"Stack::new()
    .gap(Size::Md)
    .child(AvatarGroup::new().avatars(["AL", "GH", "LT"]).limit(2))
    .child(tabs.clone())
    .child(
        Table::new()
            .head(["Name", "Role"])
            .row(["Ada", "Admin"]),
    );"#,
    macros: r#"vstack![
    AvatarGroup::new().avatars(["AL", "GH", "LT"]).limit(2),
    tabs.clone(),
    Table::new()
        .head(["Name", "Role"])
        .row(["Ada", "Admin"]),
]
.gap(Size::Md);"#,
};

pub const NAVIGATION: Snippet = Snippet {
    plain: r#"Stack::new()
    .gap(Size::Md)
    .child(Breadcrumbs::new().items(["Home", "Projects", "guise"]))
    .child(Stepper::new().step("Account").step("Profile").active(1))
    .child(pagination.clone());"#,
    macros: r#"vstack![
    Breadcrumbs::new().items(["Home", "Projects", "guise"]),
    Stepper::new().step("Account").step("Profile").active(1),
    pagination.clone(),
]
.gap(Size::Md);"#,
};

pub const POLISH: Snippet = Snippet {
    plain: r#"Group::new()
    .child(ActionIcon::new("edit", "✎").variant(Variant::Light))
    .child(ThemeIcon::new("★").color(ColorName::Yellow))
    .child(Chip::new("chip", "Notifications").checked(self.chip_on));"#,
    macros: r#"hstack![
    ActionIcon::new("edit", "✎").variant(Variant::Light),
    ThemeIcon::new("★").color(ColorName::Yellow),
    Chip::new("chip", "Notifications").checked(self.chip_on),
];"#,
};

pub const LAYOUT: Snippet = Snippet {
    plain: r#"use guise::flex::*;

Row::new()
    .gap(8.0)
    .child(tile)
    .child(Spacer::new())
    .child(Badge::new("right"));

Row::new()
    .child(Expanded::new(left).flex(2.0))
    .child(Expanded::new(right).flex(1.0));"#,
    macros: r#"use guise::prelude::*; // brings row!, col!, ...

row![tile, Spacer::new(), Badge::new("right")];

col![
    row![Badge::new("a"), SizedBox::width(12.0), Spacer::new(), Badge::new("b")],
    row![Expanded::new(left).flex(2.0), Expanded::new(right).flex(1.0)],
];"#,
};

// No containers here — the two variants are identical.
pub const REACTIVE: Snippet = Snippet {
    plain: r#"// Create shared state and provide it via context.
let count = use_state(cx, 0i32);
provide(cx, count.clone());

// A child view reads it back through context and subscribes.
let count = use_context::<Signal<i32>>(cx).unwrap();
watch(cx, &count);

// Mutate anywhere — every watcher re-renders.
count.update(cx, |n| *n += 1);"#,
    macros: r#"// Create shared state and provide it via context.
let count = use_state(cx, 0i32);
provide(cx, count.clone());

// A child view reads it back through context and subscribes.
let count = use_context::<Signal<i32>>(cx).unwrap();
watch(cx, &count);

// Mutate anywhere — every watcher re-renders.
count.update(cx, |n| *n += 1);"#,
};

pub const CARDS: Snippet = Snippet {
    plain: r#"Card::new().child(
    Stack::new()
        .gap(Size::Sm)
        .child(Text::new("Overview").bold())
        .child(Text::new("Cards compose Paper, Stack, Group, Text.").dimmed())
        .child(Button::new("open", "Open").variant(Variant::Light).full_width(true)),
);"#,
    macros: r#"card![vstack![
    text!("Overview").bold(),
    text!("Cards compose Paper, Stack, Group, Text.").dimmed(),
    button!("open", "Open").variant(Variant::Light).full_width(true),
]
.gap(Size::Sm)];"#,
};

pub const TYPOGRAPHY: Snippet = Snippet {
    plain: r#"Stack::new()
    .gap(Size::Sm)
    .child(Title::new("Heading order 1").order(1))
    .child(Text::new("Body text at the default size."))
    .child(Text::new("Dimmed secondary text.").dimmed());"#,
    macros: r#"vstack![
    title!("Heading order 1").order(1),
    text!("Body text at the default size."),
    text!("Dimmed secondary text.").dimmed(),
]
.gap(Size::Sm);"#,
};

// TableView is entity-only; no macro form.
pub const TABLEVIEW: Snippet = Snippet {
    plain: r#"#[derive(Clone)]
struct Employee { name: &'static str, role: &'static str, tenure: u32 }

let table = cx.new(|cx| {
    TableView::new(cx)
        .columns(vec![
            Column::new("Name")
                .text(|e: &Employee| e.name.into())
                .sortable_by(|a, b| a.name.cmp(b.name)),
            Column::new("Role").text(|e: &Employee| e.role.into()),
            Column::new("Tenure")
                .width(110.0)
                .align(Align::End)
                .text(|e: &Employee| format!("{} yrs", e.tenure).into())
                .sortable_by(|a, b| a.tenure.cmp(&b.tenure)),
        ])
        .rows(employees)                      // or .bind_rows(&signal, cx)
        .selection_mode(SelectionMode::Multi)
        .striped(true)
        .height(240.0)                        // fixed height => virtualized body
});

cx.subscribe(&table, |_, _, event: &TableViewEvent, _| match event {
    TableViewEvent::SelectionChanged(rows) => { /* source-row indices */ }
    TableViewEvent::Activated(row) => { /* double-click or Enter */ }
    TableViewEvent::Sorted(sort) => { /* Some((column, dir)) or None */ }
})
.detach();"#,
    macros: r#"#[derive(Clone)]
struct Employee { name: &'static str, role: &'static str, tenure: u32 }

let table = cx.new(|cx| {
    TableView::new(cx)
        .columns(vec![
            Column::new("Name")
                .text(|e: &Employee| e.name.into())
                .sortable_by(|a, b| a.name.cmp(b.name)),
            Column::new("Role").text(|e: &Employee| e.role.into()),
        ])
        .rows(employees)
        .selection_mode(SelectionMode::Multi)
        .height(240.0)
});"#,
};

// Signal plumbing is identical in both styles.
pub const DATAVIEW: Snippet = Snippet {
    plain: r#"// One Signal<Vec<T>> is the source of truth; everything binds to it.
let items = use_state(cx, vec!["Mantine".to_string(), "gpui".to_string()]);

// A TextInput two-way bound to a query Signal drives the filter. The
// `.filter(..)` closure gets no `cx`, so it reads the query from a shared
// cell that an observer keeps in sync with the signal.
let query = use_state(cx, String::new());
let input = cx.new(|cx| TextInput::new(cx).placeholder("Filter…"));
TextInput::bind(&input, &query, cx);
let query_cache: Rc<RefCell<String>> = Rc::new(RefCell::new(String::new()));

// A DataView renders the collection live; filter/sort are projections.
let filter_cache = query_cache.clone();
let view = cx.new(|cx| {
    DataView::new(cx, &items)
        .item(|name, _ix, _window, _cx| Text::new(name.clone()))
        .filter(move |name| name.to_lowercase().contains(&*filter_cache.borrow()))
        .sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()))
        .selectable()
});
cx.subscribe(&view, |_, _, event: &DataViewEvent, _| {
    let DataViewEvent::Selected(ix) = event; // ix = SOURCE index
})
.detach();

// Each query write updates the cell and repaints the view.
let filtered = view.clone();
cx.observe(query.entity(), move |_this, query, cx| {
    *query_cache.borrow_mut() = query.read(cx).to_lowercase();
    filtered.update(cx, |_, cx| cx.notify());
})
.detach();

// Any write repaints every bound view — no manual wiring:
items.update(cx, |list| list.push("SwiftUI".into()));"#,
    macros: r#"// One Signal<Vec<T>> is the source of truth; everything binds to it.
let items = use_state(cx, vec!["Mantine".to_string(), "gpui".to_string()]);

// A TextInput two-way bound to a query Signal drives the filter. The
// `.filter(..)` closure gets no `cx`, so it reads the query from a shared
// cell that an observer keeps in sync with the signal.
let query = use_state(cx, String::new());
let input = cx.new(|cx| TextInput::new(cx).placeholder("Filter…"));
TextInput::bind(&input, &query, cx);
let query_cache: Rc<RefCell<String>> = Rc::new(RefCell::new(String::new()));

// A DataView renders the collection live; filter/sort are projections.
let filter_cache = query_cache.clone();
let view = cx.new(|cx| {
    DataView::new(cx, &items)
        .item(|name, _ix, _window, _cx| Text::new(name.clone()))
        .filter(move |name| name.to_lowercase().contains(&*filter_cache.borrow()))
        .sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()))
        .selectable()
});

// Each query write updates the cell and repaints the view.
let filtered = view.clone();
cx.observe(query.entity(), move |_this, query, cx| {
    *query_cache.borrow_mut() = query.read(cx).to_lowercase();
    filtered.update(cx, |_, cx| cx.notify());
})
.detach();

// Any write repaints every bound view — no manual wiring:
items.update(cx, |list| list.push("SwiftUI".into()));"#,
};

// TreeView is entity-only; no macro form.
pub const TREE: Snippet = Snippet {
    plain: r#"let tree = cx.new(|cx| {
    TreeView::new(cx)
        .nodes(vec![
            TreeNode::new("src", "src")
                .child(TreeNode::new("main", "main.rs"))
                .child(TreeNode::new("lib", "lib.rs")),
            TreeNode::new("readme", "README.md"),
        ])
        .expand("src")
});

cx.subscribe(&tree, |_this, _tree, event: &TreeViewEvent, _cx| {
    match event {
        TreeViewEvent::Selected(id) => { /* row clicked */ }
        TreeViewEvent::Toggled(id, open) => { /* branch expanded */ }
        TreeViewEvent::Activated(id) => { /* Enter or double-click */ }
    }
})
.detach();"#,
    macros: r#"let tree = cx.new(|cx| {
    TreeView::new(cx)
        .nodes(vec![
            TreeNode::new("src", "src")
                .child(TreeNode::new("main", "main.rs"))
                .child(TreeNode::new("lib", "lib.rs")),
            TreeNode::new("readme", "README.md"),
        ])
        .expand("src")
});"#,
};

// Editor is entity-only; no macro form.
pub const EDITOR: Snippet = Snippet {
    plain: r#"// A syntax-highlighted code editor. Cmd+Enter emits EditorEvent::Run.
let editor = cx.new(|cx| {
    Editor::new(cx)
        .language(Language::Rust)      // None | Rust | Sql | Json
        .rows(8)                       // minimum visible lines
        .placeholder("Type some Rust…")
        .value("fn main() {\n    println!(\"hi\");\n}")
});

cx.subscribe(&editor, |_this, _editor, event: &EditorEvent, _cx| {
    match event {
        EditorEvent::Change(text) => { /* every edit, full text */ }
        EditorEvent::Run(source) => { /* Cmd+Enter */ }
    }
})
.detach();

// Two-way binding to a Signal<String>:
let source = use_state(cx, String::new());
Editor::bind(&editor, &source, cx);"#,
    macros: r#"let editor = cx.new(|cx| {
    Editor::new(cx)
        .language(Language::Rust)
        .rows(8)
        .value("fn main() {\n    println!(\"hi\");\n}")
});"#,
};

// Panel is a builder, SplitPanel an entity; neither has a macro form.
pub const PANELS: Snippet = Snippet {
    plain: r#"// Panel — Card chrome + header/footer, controlled collapse.
Panel::new()
    .id("status")
    .title("Project status")
    .description("Weekly summary")
    .collapsible()
    .collapsed(self.collapsed)
    .on_toggle(cx.listener(|this, _, _, cx| {
        this.collapsed = !this.collapsed;
        cx.notify();
    }))
    .footer(Text::new("Updated 5 minutes ago").dimmed())
    .child(Text::new("Everything on track."));

// SplitPanel — live panes + draggable divider; nest for complex layouts.
let inner = cx.new(|cx| {
    SplitPanel::new(cx)
        .direction(SplitDirection::Vertical)
        .first(|_, _| Text::new("Editor"))
        .second(|_, _| Text::new("Terminal"))
});
let split = cx.new(|cx| {
    SplitPanel::new(cx)
        .direction(SplitDirection::Horizontal)
        .ratio(0.35)
        .min_first(140.0)
        .first(|_, _| Text::new("Sidebar"))
        .second(move |_, _| inner.clone())
});
cx.subscribe(&split, |_this, _split, event: &SplitPanelEvent, _cx| {
    let SplitPanelEvent::Resized(ratio) = event;
})
.detach();"#,
    macros: r#"// Panel — Card chrome + header/footer, controlled collapse.
Panel::new()
    .id("status")
    .title("Project status")
    .collapsible()
    .collapsed(self.collapsed)
    .on_toggle(cx.listener(|this, _, _, cx| {
        this.collapsed = !this.collapsed;
        cx.notify();
    }))
    .child(Text::new("Everything on track."));

// SplitPanel — live panes + draggable divider; nest for complex layouts.
let split = cx.new(|cx| {
    SplitPanel::new(cx)
        .direction(SplitDirection::Horizontal)
        .ratio(0.35)
        .first(|_, _| Text::new("Sidebar"))
        .second(|_, _| Text::new("Main content"))
});"#,
};

pub const INPUTS2: Snippet = Snippet {
    plain: r#"// Entities own their state; create them once with cx.new.
let password = cx.new(|cx| PasswordInput::new(cx).label("Password"));
let brand = cx.new(|cx| ColorInput::new(cx).value(rgb(34, 139, 230)));
let topics = cx.new(|cx| TagsInput::new(cx).tags(["rust", "gpui"]).max_tags(6));
let range = cx.new(|cx| {
    RangeSlider::new(cx).min(0.0).max(100.0).min_gap(10.0).value((20.0, 80.0))
});
let pin = cx.new(|cx| PinInput::new(cx).length(4));

cx.subscribe(&pin, |_this, _pin, event: &PinInputEvent, _cx| {
    if let PinInputEvent::Complete(code) = event { /* verify */ }
})
.detach();

// Rating is controlled: the parent owns the value.
Rating::new("stars")
    .value(self.stars)
    .on_change(cx.listener(|this, value: &f32, _window, cx| {
        this.stars = *value;
        cx.notify();
    }));

// Or skip the handlers and two-way bind to signals:
let secret = use_state(cx, String::new());
PasswordInput::bind(&password, &secret, cx);"#,
    macros: r#"// Entities own their state; create them once with cx.new.
let password = cx.new(|cx| PasswordInput::new(cx).label("Password"));
let brand = cx.new(|cx| ColorInput::new(cx).value(rgb(34, 139, 230)));
let topics = cx.new(|cx| TagsInput::new(cx).tags(["rust", "gpui"]).max_tags(6));
let pin = cx.new(|cx| PinInput::new(cx).length(4));

vstack![
    password,
    brand,
    topics,
    pin,
    Rating::new("stars")
        .value(self.stars)
        .on_change(cx.listener(|this, value: &f32, _window, cx| {
            this.stars = *value;
            cx.notify();
        })),
]
.gap(Size::Md);"#,
};

pub const OVERLAYS2: Snippet = Snippet {
    plain: r#"// ContextMenu: show at the pointer from a right-click.
let menu = cx.new(|cx| {
    ContextMenu::new(cx)
        .item_icon(IconName::Copy, "Copy path", |_, _| {})
        .item("Rename", |_, _| {})
        .divider()
        .danger_item("Delete", |_, _| {})
});
div()
    .id("target")
    .child("Right-click me")
    .on_mouse_down(MouseButton::Right, cx.listener(|this, ev: &MouseDownEvent, window, cx| {
        let position = ev.position;
        this.menu.update(cx, |menu, cx| menu.show(position, window, cx));
    }))
    .child(menu.clone());

// HoverCard: opens after 300ms of hover, survives the trip onto the card.
let card = cx.new(|cx| {
    HoverCard::new(
        cx,
        |_, _| Badge::new("@ada").into_any_element(),
        |_, _| Text::new("Ada Lovelace — wrote the first program.").into_any_element(),
    )
    .width(260.0)
});

// LoadingOverlay: last child of a `.relative()` container.
div()
    .relative()
    .child(form)
    .child(LoadingOverlay::new().visible(self.saving));

// ConfirmModal: controlled, like Modal.
if self.confirm_open {
    root = root.child(
        ConfirmModal::new()
            .title("Delete file?")
            .message("del.rs will be moved to the Trash.")
            .confirm_label("Delete")
            .danger()
            .on_confirm(cx.listener(|this, _, _, cx| { this.confirm_open = false; cx.notify(); }))
            .on_cancel(cx.listener(|this, _, _, cx| { this.confirm_open = false; cx.notify(); })),
    );
}"#,
    macros: r#"// ContextMenu: show at the pointer from a right-click.
let menu = cx.new(|cx| {
    ContextMenu::new(cx)
        .item_icon(IconName::Copy, "Copy path", |_, _| {})
        .divider()
        .danger_item("Delete", |_, _| {})
});

// LoadingOverlay + ConfirmModal are controlled by parent flags.
zstack![
    form,
    LoadingOverlay::new().visible(self.saving),
];"#,
};

pub const SHELL: Snippet = Snippet {
    plain: r#"// TabBar (entity): a document tab strip that emits Select / Close / Add.
let bar = cx.new(|cx| TabBar::new(cx).tabs(["main.rs", "lib.rs", "theme.rs"]));
cx.subscribe(&bar, |_this, bar, event: &TabBarEvent, cx| match event {
    TabBarEvent::Close(i) => {
        let i = *i;
        bar.update(cx, |b, cx| b.remove_tab(i, cx));
    }
    TabBarEvent::Add => bar.update(cx, |b, cx| b.add_tab("untitled", cx)),
    TabBarEvent::Select(_) => {}
})
.detach();

// AppShell: fixed-size live regions around a scrollable main area.
AppShell::new()
    .header(44.0, |_, _| Text::new("Header"))
    .navbar(160.0, |_, _| Text::new("Navbar"))
    .footer(28.0, |_, _| Text::new("Footer").size(Size::Xs))
    .child(
        guise::layout::Container::new() // max-width column (flex::Container differs)
            .size(Size::Xs)             // 540 px cap
            .padding(Size::Md)
            .child(Title::new("Main").order(4))
            .child(Space::y(Size::Sm))  // fixed theme-scale gap
            .child(Text::new("Centered, capped, scrollable.")),
    );"#,
    macros: r#"// AppShell: fixed-size live regions around a scrollable main area.
AppShell::new()
    .header(44.0, |_, _| Text::new("Header"))
    .navbar(160.0, |_, _| Text::new("Navbar"))
    .footer(28.0, |_, _| Text::new("Footer").size(Size::Xs))
    .child(
        guise::layout::Container::new()
            .size(Size::Xs)
            .padding(Size::Md)
            .child(title!("Main").order(4))
            .child(Space::y(Size::Sm))
            .child(text!("Centered, capped, scrollable.")),
    );"#,
};

// Image is a plain builder; no macro form.
pub const MEDIA: Snippet = Snippet {
    plain: r#"// Image — wraps gpui's img(); URI, asset path, or file path.
Image::new("https://example.com/cat.png")
    .width(240.0)
    .height(160.0)
    .radius(Size::Md)
    .fit(ObjectFit::Cover)
    .fallback(|| Text::new("no image").dimmed());

// Circular avatar crop.
Image::new("https://example.com/me.png")
    .width(96.0)
    .height(96.0)
    .circle()
    .fallback(|| Text::new("…").dimmed());"#,
    macros: r#"// Image — wraps gpui's img(); URI, asset path, or file path.
Image::new("https://example.com/cat.png")
    .width(240.0)
    .height(160.0)
    .radius(Size::Md)
    .fit(ObjectFit::Cover)
    .fallback(|| Text::new("no image").dimmed());

// Circular avatar crop.
Image::new("https://example.com/me.png")
    .width(96.0)
    .height(96.0)
    .circle()
    .fallback(|| Text::new("…").dimmed());"#,
};

pub const TYPEEXTRAS: Snippet = Snippet {
    plain: r#"// Mark — inline highlighter pen.
Group::new()
    .gap(Size::Xs)
    .child(Text::new("Highlight the"))
    .child(Mark::new("important part"))            // Yellow by default
    .child(Text::new("of a sentence."));

// Blockquote — left accent border + optional icon and citation.
Blockquote::new()
    .icon(IconName::Info)
    .text("Life is like an npm install – you never know what you are going to get.")
    .cite("– Forrest Gump");

// Spoiler — controlled clip with a "Show more" link.
Spoiler::new("bio-spoiler")
    .max_height(60.0)
    .expanded(self.bio_open)
    .on_toggle(cx.listener(|this, _, _, cx| {
        this.bio_open = !this.bio_open;
        cx.notify();
    }))
    .child(Text::new(LONG_BIO).size(Size::Sm));"#,
    macros: r#"// Mark — inline highlighter pen.
hstack![
    text!("Highlight the"),
    Mark::new("important part"),
    text!("of a sentence."),
]
.gap(Size::Xs);

// Blockquote — left accent border + optional icon and citation.
Blockquote::new()
    .icon(IconName::Info)
    .text("Life is like an npm install – you never know what you are going to get.")
    .cite("– Forrest Gump");"#,
};

// Charts are plain builders; no macro forms.
pub const CHARTS: Snippet = Snippet {
    plain: r#"// Inline trend line, filled to the baseline.
Sparkline::new([3.0, 5.0, 2.0, 8.0, 6.0]).fill();

// Bars from (label, value) pairs; labels render under the bars.
BarChart::entries([("Mon", 12.0), ("Tue", 9.0), ("Wed", 15.0)]).gap(0.3);

// A sparkline grown up: gridlines + area fill.
LineChart::new([12.0, 18.0, 9.0, 24.0, 31.0]).fill().height(120.0);

// Proportional slices with a donut hole and a legend.
PieChart::entries([("Rust", 62.0), ("TOML", 25.0), ("Other", 13.0)])
    .donut(0.6);"#,
    macros: r#"// Inline trend line, filled to the baseline.
Sparkline::new([3.0, 5.0, 2.0, 8.0, 6.0]).fill();

// Bars from (label, value) pairs; labels render under the bars.
BarChart::entries([("Mon", 12.0), ("Tue", 9.0), ("Wed", 15.0)]).gap(0.3);

// A sparkline grown up: gridlines + area fill.
LineChart::new([12.0, 18.0, 9.0, 24.0, 31.0]).fill().height(120.0);

// Proportional slices with a donut hole and a legend.
PieChart::entries([("Rust", 62.0), ("TOML", 25.0), ("Other", 13.0)])
    .donut(0.6);"#,
};

// The swatch grid is a data-driven loop; no macro form.
pub const PALETTE: Snippet = Snippet {
    plain: r#"for name in ColorName::ALL {
    Group::new()
        .wrap(false)
        .children((0..10).map(|shade| {
            div()
                .w(px(34.0)).h(px(26.0)).rounded(px(4.0))
                .bg(theme(cx).color(name, shade).hsla())
        }));
}"#,
    macros: r#"for name in ColorName::ALL {
    Group::new()
        .wrap(false)
        .children((0..10).map(|shade| {
            div()
                .w(px(34.0)).h(px(26.0)).rounded(px(4.0))
                .bg(theme(cx).color(name, shade).hsla())
        }));
}"#,
};
