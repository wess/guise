# App walkthrough: a project tracker

The [tutorial](tutorial.md) tours the component set one piece at a time. This
walkthrough goes the other way: one small app — a project tracker with a
form, a task queue, and window-level overlays — built the way a real guise
app fits together. It leans on the newer layers: the reactive `Form`,
`DatePicker`/`FileInput`, `OverlayHost` toasts and modals, drag-to-reorder,
`Collapse` motion, and theme presets.

Skim the [installation page](gettingstarted.md) first; this page assumes a
running window.

## The shape of the app

One root view owns everything: the form's entities, the task list, and the
overlay host. gpui entities are cheap handles — create them once in `new`,
clone them into the layout every render.

```rust
use gpui::prelude::*;
use gpui::{App, Context, Entity, IntoElement, SharedString, Window};
use guise::prelude::*;

struct Tracker {
    // Form: each field is a Signal<String> under the hood.
    form: Form,
    name_input: Entity<TextInput>,
    ship_date: Entity<DatePicker>,
    brief: Entity<FileInput>,
    // The queue the SortableList reorders.
    tasks: Vec<SharedString>,
    details_open: bool,
    // Window services: modal stack + toasts.
    overlays: Entity<OverlayHost>,
}
```

## Wiring the form

`Form` fields plug straight into inputs — no change handlers, no copying
values out on submit. Rules run on `validate`/`submit`, and a field that
failed re-validates live as it's edited.

```rust
impl Tracker {
    fn new(cx: &mut Context<Self>) -> Self {
        let form = Form::new(cx)
            .field(cx, "name", "")
            .rule("name", validators::required())
            .rule("name", validators::min_len(3))
            .field(cx, "owner", "")
            .rule("owner", validators::email());

        let name_input = cx.new(|cx| TextInput::new(cx).placeholder("Project name"));
        TextInput::bind(&name_input, &form.signal("name"), cx);

        let ship_date = cx.new(|cx| DatePicker::new(cx).label("Ship date"));
        let brief = cx.new(|cx| FileInput::new(cx).label("Brief").accept(["pdf", "md"]));

        let overlays = cx.new(OverlayHost::new);

        // Re-render whenever validation state changes.
        watch(cx, &form.errors());

        Tracker {
            form,
            name_input,
            ship_date,
            brief,
            tasks: ["Design review", "Implement", "Ship"]
                .map(SharedString::new_static)
                .to_vec(),
            details_open: false,
            overlays,
        }
    }
}
```

## Submitting, with feedback

`form.submit(cx)` validates and hands back the values on success. The
overlay host turns the outcome into a toast — from any handler, no flags
threaded through the view:

```rust
let form = self.form.clone();
let overlays = self.overlays.clone();
Button::new("save", "Save project").on_click(cx.listener(move |this, _ev, _window, cx| {
    match form.submit(cx) {
        Some(values) => {
            overlays.update(cx, |host, cx| {
                host.toast_titled("Saved", values["name"].clone(), ColorName::Green, cx);
            });
        }
        None => {
            overlays.update(cx, |host, cx| {
                host.toast_titled("Fix the form", "Some fields need attention", ColorName::Red, cx);
            });
        }
    }
}));
```

Render field errors inline — they clear live as the user types, because
errored fields re-validate on change:

```rust
let name_error = self.form.error(cx, "name"); // Option<String>
```

## A confirm dialog from anywhere

`OverlayHost::open_modal` stacks a modal above everything, restores focus on
close, and closes on Escape. The builder gets a `ModalCloser` to wire to the
dialog's own affordances:

```rust
self.overlays.update(cx, |host, cx| {
    host.open_modal(window, cx, |close, _window, _cx| {
        Modal::new()
            .title("Archive project?")
            .on_close(move |_ev, window, cx| close(window, cx))
            .child(Text::new("Tasks are kept; the board disappears.").dimmed())
            .into_any_element()
    });
});
```

## The reorderable queue

The parent owns the `Vec`; the list reports `(from, to)` and
`apply_reorder` does the splice:

```rust
let tasks = self.tasks.clone();
let labels = self.tasks.clone();
let host = cx.entity().downgrade();
SortableList::new("tasks", tasks.len(), move |i, _window, cx| {
    let t = theme(cx);
    div()
        .px(px(10.0)).py(px(7.0)).rounded(px(6.0))
        .bg(t.surface_hover().hsla())
        .child(Text::new(tasks[i].clone()).size(Size::Sm))
})
.label_of(move |i| labels[i].clone())
.on_reorder(move |from, to, _window, cx| {
    host.update(cx, |this, cx| {
        guise::dnd::apply_reorder(&mut this.tasks, from, to);
        cx.notify();
    }).ok();
})
```

## A details drawer that really collapses

Give `Collapse` the content height and both directions animate — pick any
[easing](transitions.md#easing), including springs:

```rust
Collapse::new("details")
    .open(self.details_open)
    .height(120.0)
    .easing(Easing::Spring(Spring::default()))
    .child(details_panel())
```

## Assembling the render

Overlays render **last** so they paint above the page; the host carries the
toast stack and any open modals with it:

```rust
impl Render for Tracker {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = theme(cx);
        let body = t.body().hsla();
        // Resolve theme values FIRST — listeners need cx mutably.
        let name_error = self.form.error(cx, "name");

        let form_column = Stack::new()
            .gap(Size::Sm)
            .child(self.name_input.clone())
            .child(match name_error {
                Some(message) => Text::new(message).size(Size::Xs).color(ColorName::Red),
                None => Text::new("Looks good").size(Size::Xs).dimmed(),
            })
            .child(self.ship_date.clone())
            .child(self.brief.clone());

        div()
            .size_full()
            .bg(body)
            .p(px(24.0))
            .child(Stack::new().gap(Size::Lg).child(form_column) /* + queue, details … */)
            .child(self.overlays.clone())
    }
}
```

## Theming the whole thing

One line restyles every component — a prebuilt palette, or a JSON file the
user can edit:

```rust
Theme::tokyonight().init(cx);
// or:
Theme::from_json(&std::fs::read_to_string("theme.json")?)?.init(cx);
```

Responsive touches come from [breakpoints](layout.md#breakpoints): resolve a
`Responsive` value against the window each render and grids re-column as the
window resizes.

## Where to go next

- [Reactive state](reactive.md) — signals, bindings, lenses, and the `Form`
  layer this app leans on.
- [Overlays](overlays.md) — the full `OverlayHost` API, plus menus,
  popovers, and the onboarding `Tour`.
- [Dates & times](dates.md), [File handling](files.md),
  [Drag & drop](dnd.md) — deeper dives on the pieces used here.
- [Panels](panels.md) — when the app grows into split panes with tabs,
  `PaneGroup` includes layout persistence.
