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
