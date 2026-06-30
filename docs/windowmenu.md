# Window menu

The native application menu (the macOS menu bar / OS menu) is a gpui feature,
not a `guise` component — `guise` doesn't theme native chrome. This page shows
the idiomatic way to wire it, as the gallery does.

> For a **themed, in-window** menu bar — drawn by `guise`, useful when you
> render your own titlebar or run on a platform with no native menu bar — see
> [`MenuBar`](overlays.md#menubar-entity) instead.

## Define actions

Menu items dispatch `gpui::Action` types. Derive them:

```rust
#[derive(Clone, PartialEq, Default, Debug, gpui::Action)]
#[action(namespace = myapp, no_json)]
struct ToggleThemeAction;

#[derive(Clone, PartialEq, Default, Debug, gpui::Action)]
#[action(namespace = myapp, no_json)]
struct QuitAction;
```

## Build the menus

Call `cx.set_menus(...)` in your `run` closure. Use the fully-qualified
`gpui::Menu` / `gpui::MenuItem` so they don't clash with guise's overlay
[`Menu`](overlays.md#menu-entity).

```rust
cx.set_menus(vec![
    gpui::Menu {
        name: SharedString::new_static("My App"),
        disabled: false,
        items: vec![
            gpui::MenuItem::action("Toggle Theme", ToggleThemeAction),
            gpui::MenuItem::separator(),
            gpui::MenuItem::action("Quit", QuitAction),
        ],
    },
    gpui::Menu {
        name: SharedString::new_static("View"),
        disabled: false,
        items: vec![gpui::MenuItem::action("Toggle Theme", ToggleThemeAction)],
    },
]);
```

`gpui::Menu` has a required `disabled` field. `MenuItem` also offers
`MenuItem::submenu(menu)` and `MenuItem::os_action(...)` for standard OS roles.

## Handle the actions

Register global handlers with `cx.on_action`:

```rust
cx.on_action::<QuitAction>(|_, cx| cx.quit());

cx.on_action::<ToggleThemeAction>(|_, cx| {
    let dark = cx.global::<Theme>().scheme.is_dark();
    cx.global_mut::<Theme>().scheme = if dark { ColorScheme::Light } else { ColorScheme::Dark };
    cx.refresh_windows();
});
```

Global handlers fire regardless of which view is focused — convenient for a
single-window app. To scope an action to a view instead, register it on the
view's root element with `.on_action(cx.listener(Self::handler))`.

## Full wiring

```rust
gpui_platform::application().run(|cx: &mut App| {
    Theme::dark().init(cx);

    cx.set_menus(/* … as above … */);
    cx.on_action::<QuitAction>(|_, cx| cx.quit());
    cx.on_action::<ToggleThemeAction>(|_, cx| { /* … */ });

    // open_window(...);
    cx.activate(true);
});
```
