//! The individual showcase sections rendered by the gallery.

use gpui::prelude::*;
use gpui::{div, px, App, IntoElement};

use guise::prelude::*;
use guise::ColorName;

pub fn header() -> impl IntoElement {
    Stack::new()
        .gap(Size::Xs)
        .child(
            Group::new()
                .justify(Justify::Between)
                .child(Title::new("guise").order(1))
                .child(
                    Button::new("toggle", "Toggle theme")
                        .variant(Variant::Default)
                        .left_section(Text::new("◐"))
                        .on_click(|_, window, cx| crate::toggle_theme(window, cx)),
                ),
        )
        .child(
            Text::new("A Mantine-inspired component library for gpui.")
                .size(Size::Lg)
                .dimmed(),
        )
}

pub fn buttons() -> impl IntoElement {
    // All children are `Button`, so a homogeneous `.children([...])` works
    // (mixed-type children would each need `.into_any_element()`).
    let variants = Group::new().children([
        Button::new("v-filled", "Filled").variant(Variant::Filled),
        Button::new("v-light", "Light").variant(Variant::Light),
        Button::new("v-outline", "Outline").variant(Variant::Outline),
        Button::new("v-subtle", "Subtle").variant(Variant::Subtle),
        Button::new("v-default", "Default").variant(Variant::Default),
        Button::new("v-transp", "Transparent").variant(Variant::Transparent),
    ]);

    let colors = Group::new().children([
        Button::new("c-blue", "Blue").color(ColorName::Blue),
        Button::new("c-teal", "Teal").color(ColorName::Teal),
        Button::new("c-grape", "Grape").color(ColorName::Grape),
        Button::new("c-red", "Red").color(ColorName::Red),
        Button::new("c-orange", "Orange").color(ColorName::Orange),
        Button::new("c-lime", "Lime").color(ColorName::Lime),
    ]);

    let sizes = Group::new().align(Align::Center).children([
        Button::new("s-xs", "xs").size(Size::Xs),
        Button::new("s-sm", "sm").size(Size::Sm),
        Button::new("s-md", "md").size(Size::Md),
        Button::new("s-lg", "lg").size(Size::Lg),
        Button::new("s-xl", "xl").size(Size::Xl),
    ]);

    Stack::new()
        .gap(Size::Md)
        .child(variants)
        .child(colors)
        .child(sizes)
}

pub fn badges() -> impl IntoElement {
    let colors = Group::new().children([
        Badge::new("Blue").color(ColorName::Blue),
        Badge::new("Teal").color(ColorName::Teal),
        Badge::new("Grape").color(ColorName::Grape),
        Badge::new("Red").color(ColorName::Red),
        Badge::new("Orange").color(ColorName::Orange),
    ]);

    let variants = Group::new().children([
        Badge::new("Filled").variant(Variant::Filled),
        Badge::new("Light").variant(Variant::Light),
        Badge::new("Outline").variant(Variant::Outline),
    ]);

    Stack::new().gap(Size::Md).child(colors).child(variants)
}

pub fn cards() -> impl IntoElement {
    let card = |id: &'static str, title: &'static str, color: ColorName| {
        Card::new().child(
            Stack::new()
                .gap(Size::Sm)
                .child(
                    Group::new()
                        .justify(Justify::Between)
                        .child(Text::new(title).bold().size(Size::Lg))
                        .child(Badge::new("New").color(color)),
                )
                .child(
                    Text::new("Cards compose Paper, Stack, Group, Text and Button.")
                        .size(Size::Sm)
                        .dimmed(),
                )
                .child(
                    Button::new(id, "Open")
                        .variant(Variant::Light)
                        .color(color)
                        .full_width(true),
                ),
        )
    };

    Group::new().grow(true).align(Align::Stretch).children([
        card("card-a", "Overview", ColorName::Blue),
        card("card-b", "Activity", ColorName::Teal),
        card("card-c", "Billing", ColorName::Grape),
    ])
}

pub fn typography() -> impl IntoElement {
    Stack::new()
        .gap(Size::Sm)
        .child(Title::new("Heading order 1").order(1))
        .child(Title::new("Heading order 2").order(2))
        .child(Title::new("Heading order 3").order(3))
        .child(Text::new("Body text at the default size.").size(Size::Md))
        .child(Text::new("Bold body text.").bold())
        .child(Text::new("Dimmed secondary text.").dimmed())
        .child(Divider::new().label("Section break"))
}

pub fn feedback() -> impl IntoElement {
    let alerts = Stack::new()
        .gap(Size::Sm)
        .child(
            Alert::new("Your changes have been saved.")
                .title("Success")
                .color(ColorName::Teal)
                .icon("✓"),
        )
        .child(
            Alert::new("Your session expires in 5 minutes.")
                .title("Heads up")
                .color(ColorName::Yellow)
                .variant(Variant::Outline)
                .icon("!"),
        )
        .child(
            Alert::new("Something went wrong while syncing.")
                .title("Error")
                .color(ColorName::Red)
                .variant(Variant::Filled)
                .icon("×"),
        );

    // Each loader is wrapped in an id'd div (so their animation ids stay
    // unique); all three are the same `Stateful<Div>` type → `.children`.
    let loaders = Group::new().gap(Size::Xl).align(Align::Center).children([
        div().id("loader-dots").child(Loader::new().color(ColorName::Blue)),
        div()
            .id("loader-bars")
            .child(Loader::new().variant(LoaderVariant::Bars).color(ColorName::Grape)),
        div()
            .id("loader-lg")
            .child(Loader::new().size(Size::Lg).color(ColorName::Teal)),
    ]);

    let bars = Stack::new().gap(Size::Sm).children([
        Progress::new(25.0).color(ColorName::Blue),
        Progress::new(60.0).color(ColorName::Teal),
        Progress::new(90.0).color(ColorName::Grape).size(Size::Lg),
    ]);

    Stack::new()
        .gap(Size::Md)
        .child(alerts)
        .child(loaders)
        .child(bars)
        .child(
            Notification::new("Deployment finished in 42s.")
                .title("Build complete")
                .color(ColorName::Teal)
                .icon("✓"),
        )
}

pub fn palette(cx: &App) -> impl IntoElement {
    let t = theme(cx);
    let mut rows = Stack::new().gap(Size::Xs);
    for name in ColorName::ALL {
        let swatches = Group::new()
            .gap(Size::Xs)
            .wrap(false)
            .child(
                div()
                    .w(px(56.0))
                    .text_size(px(12.0))
                    .text_color(t.dimmed().hsla())
                    .child(name.label()),
            )
            .children((0..10usize).map(|shade| {
                div()
                    .w(px(34.0))
                    .h(px(26.0))
                    .rounded(px(4.0))
                    .bg(t.color(name, shade).hsla())
            }));
        rows = rows.child(swatches);
    }
    rows
}
