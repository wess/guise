# Dates & times

The date suite is three components over two plain-Rust value types. `Date` and
`Time` carry all the calendar math (leap years, month grids, weekday math,
formatting/parsing) with no dependency beyond `std` — the components are thin
themed views over them.

- **`Calendar`** — a controlled month grid (`RenderOnce` builder). The parent
  owns the visible month and the selection.
- **`DatePicker`** — a stateful entity: trigger + dropdown calendar, single
  date or range, emits `DatePickerEvent`.
- **`TimePicker`** — a stateful entity: trigger + hour/minute (and AM/PM)
  columns, emits `TimePickerEvent`.

## `Date` and `Time`

```rust
let d = Date::new(2026, 7, 14).unwrap();       // validated; Feb 30 is None
d.weekday();                                    // Weekday::Tuesday
d.add_days(30);                                 // crosses month/year boundaries
d.add_months(1);                                // Jan 31 + 1 month = Feb 28/29
d.format("MMM D, YYYY");                        // "Jul 14, 2026"
Date::parse_iso("2026-07-14");                  // Option<Date>
Date::today();                                  // UTC-based

let t = Time::new(14, 5).unwrap();
t.format_12();                                  // "2:05 PM"
Time::parse("2:05 pm");                         // Option<Time>
```

Format tokens: `YYYY`, `MM`/`M`, `DD`/`D`, `MMM` (Jul), `MMMM` (July);
anything else passes through. `Date` is `Copy`, `Ord`, and hashes — use it as
a plain value. `month_grid(year, month, week_start)` returns the 42 cells a
month view shows (leading/trailing days included), which is what `Calendar`
draws.

> **Note** `Date::today()` uses UTC — `std` has no timezone database. It
> drives the "today" highlight, not civil timekeeping.

## Calendar

A controlled builder, like `Checkbox`: pass the month and selection, handle
the clicks. Re-render with new state and it follows.

```rust
// Inside an entity's render: capture a weak handle, write state back through it.
let this = cx.entity().downgrade();
let month = this.clone();
Calendar::new("cal")
    .month(self.year, self.month)
    .value(self.picked)
    .on_select(move |date, _window, cx| {
        this.update(cx, |view, cx| { view.picked = Some(date); cx.notify(); }).ok();
    })
    .on_month_change(move |year, m, _window, cx| {
        month.update(cx, |view, cx| { view.year = year; view.month = m; cx.notify(); }).ok();
    })
```

| Method | Default | Notes |
| --- | --- | --- |
| `new(id)` | today's month | |
| `month(year, month)` | today's | month is 1–12 |
| `value(Option<Date>)` | `None` | single selection highlight |
| `range(start, Option<end>)` | `None` | range highlight while/after picking |
| `min(date)` / `max(date)` | unbounded | outside days render disabled |
| `week_start(Weekday)` | `Sunday` | |
| `size(Size)` | `Sm` | cell size + typography |
| `on_select(fn(Date, &mut Window, &mut App))` | — | never fires on disabled days |
| `on_month_change(fn(i32, u32, ..))` | — | prev/next clicks; receives the new (year, month) |

Selected days fill with the theme primary; today gets a primary outline;
days outside the shown month render dimmed.

## DatePicker

A gpui entity that owns its open state, visible month, and selection.

```rust
let picker = cx.new(|cx| {
    DatePicker::new(cx)
        .label("Ship date")
        .min(Date::new(2026, 1, 1).unwrap())
        .format("YYYY-MM-DD")
});
cx.subscribe(&picker, |_, event: &DatePickerEvent, _| match event {
    DatePickerEvent::Selected(date) => { /* single mode */ }
    DatePickerEvent::Range(start, end) => { /* range mode */ }
}).detach();
```

Range mode: `DatePicker::new(cx).range_mode()`. The first click sets the
start, the second completes the range (clicks before the start swap the
endpoints), and the picker closes emitting `DatePickerEvent::Range`.

Two-way binding, signal as source of truth:

```rust
let ship_date: Signal<Option<Date>> = Signal::new(cx, None);
DatePicker::bind(&picker, &ship_date, cx);
```

| Method | Default | Notes |
| --- | --- | --- |
| `range_mode()` | single | pick a start + end instead |
| `value(date)` | none | also moves the visible month |
| `min` / `max` / `week_start` | — | forwarded to the calendar |
| `format(pattern)` | `"MMM D, YYYY"` | trigger display |
| `placeholder` / `label` / `size` / `disabled` | — | field chrome |
| `selected_date()` / `selected_range()` | — | current value accessors |

## TimePicker

```rust
let time = cx.new(|cx| TimePicker::new(cx).label("Standup"));
cx.subscribe(&time, |_, TimePickerEvent(t), _| { /* t: Time */ }).detach();
```

Twelve-hour by default (hour, minute, AM/PM columns); `twenty_four_hour()`
switches to a 24-hour clock with two columns. Picking an hour or meridiem
keeps the dropdown open; picking a minute closes it. Every change emits
`TimePickerEvent`.

| Method | Default | Notes |
| --- | --- | --- |
| `twenty_four_hour()` | 12-hour | |
| `minute_step(step)` | `5` | minute list granularity, clamped 1–30 |
| `value(time)` | none | |
| `placeholder` / `label` / `size` / `disabled` | — | field chrome |
| `time()` | — | current value accessor |
| `bind(&entity, &Signal<Option<Time>>, cx)` | — | two-way binding |
