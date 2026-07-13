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
                        .left_section(Icon::new(IconName::SunMoon).size(Size::Sm))
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

pub fn icons() -> impl IntoElement {
    // A taste of the bundled Lucide set — every icon on lucide.dev is an
    // `IconName` variant, drawn from the embedded icon font.
    let sample = [
        IconName::House,
        IconName::Search,
        IconName::Settings,
        IconName::User,
        IconName::Bell,
        IconName::Calendar,
        IconName::Camera,
        IconName::Check,
        IconName::CircleAlert,
        IconName::Cloud,
        IconName::Code,
        IconName::Copy,
        IconName::Download,
        IconName::File,
        IconName::Folder,
        IconName::Globe,
        IconName::Heart,
        IconName::Mail,
        IconName::MapPin,
        IconName::Moon,
        IconName::Palette,
        IconName::Pencil,
        IconName::Rocket,
        IconName::Star,
        IconName::Sun,
        IconName::Terminal,
        IconName::Trash2,
        IconName::Wifi,
    ];

    let tiles = sample.map(|name| {
        div()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(6.0))
            .w(px(96.0))
            .py(px(8.0))
            .child(Icon::new(name).size(Size::Lg))
            .child(Text::new(name.name()).size(Size::Xs).dimmed())
    });

    let sizes = Group::new().align(Align::Center).children([
        Icon::new(IconName::Rocket).size(Size::Xs),
        Icon::new(IconName::Rocket).size(Size::Sm),
        Icon::new(IconName::Rocket).size(Size::Md),
        Icon::new(IconName::Rocket).size(Size::Lg),
        Icon::new(IconName::Rocket).size(Size::Xl),
    ]);

    let colors = Group::new().align(Align::Center).children([
        Icon::new(IconName::Heart).color(ColorName::Red),
        Icon::new(IconName::Star).color(ColorName::Yellow),
        Icon::new(IconName::Leaf).color(ColorName::Green),
        Icon::new(IconName::Droplet).color(ColorName::Blue),
        Icon::new(IconName::Zap).color(ColorName::Orange),
    ]);

    Stack::new()
        .gap(Size::Md)
        .child(div().flex().flex_wrap().children(tiles))
        .child(Group::new().gap(Size::Xl).child(sizes).child(colors))
        .child(
            Text::new(format!(
                "{} icons bundled from Lucide v{} — see IconName::all()",
                IconName::all().len(),
                guise::LUCIDE_VERSION,
            ))
            .size(Size::Sm)
            .dimmed(),
        )
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
        div()
            .id("loader-dots")
            .child(Loader::new().color(ColorName::Blue)),
        div().id("loader-bars").child(
            Loader::new()
                .variant(LoaderVariant::Bars)
                .color(ColorName::Grape),
        ),
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

pub fn media() -> impl IntoElement {
    Group::new()
        .gap(Size::Md)
        .align(Align::Start)
        .child(
            Image::new("https://picsum.photos/seed/guise/240/160")
                .width(240.0)
                .height(160.0)
                .radius(Size::Md)
                .fit(ObjectFit::Cover)
                .fallback(|| Text::new("image loading…").dimmed().size(Size::Xs)),
        )
        .child(
            Image::new("https://picsum.photos/seed/guise-avatar/96")
                .width(96.0)
                .height(96.0)
                .circle()
                .fallback(|| Text::new("…").dimmed().size(Size::Xs)),
        )
}

pub fn charts() -> impl IntoElement {
    let trend = [12.0, 18.0, 9.0, 24.0, 20.0, 31.0, 26.0];

    let sparklines = Group::new()
        .align(Align::Center)
        .child(Sparkline::new(trend).fill())
        .child(Sparkline::new([5.0, 3.0, 8.0, 2.0, 7.0, 4.0]).color(ColorName::Teal))
        .child(
            Sparkline::new([1.0, 4.0, 2.0, 8.0, 5.0, 9.0])
                .color(ColorName::Red)
                .stroke(1.0),
        );

    let bars = BarChart::entries([
        ("Mon", 12.0),
        ("Tue", 9.0),
        ("Wed", 15.0),
        ("Thu", 7.0),
        ("Fri", 18.0),
    ])
    .height(120.0);

    let line = LineChart::new(trend).fill().height(120.0);

    let pies = Group::new().gap(Size::Xl).children([
        PieChart::entries([("Rust", 62.0), ("TOML", 25.0), ("Other", 13.0)]).size(120.0),
        PieChart::new([40.0, 30.0, 20.0, 10.0])
            .donut(0.6)
            .size(120.0),
    ]);

    Stack::new()
        .gap(Size::Lg)
        .child(sparklines)
        .child(bars)
        .child(line)
        .child(pies)
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
