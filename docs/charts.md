# Charts

Six stateless builders painted through gpui's `canvas` element: `Sparkline`,
`LineChart`, `AreaChart`, `BarChart`, `ScatterChart`, `PieChart`. Minimal by
default — axis-free trend visuals — but the cartesian charts opt into a real
frame: `.axis()` adds nice-number tick labels with aligned gridlines,
`.hover()` adds per-point value tooltips, `.labels(..)` a category row, and
named series get a color-dot legend. All text renders as regular elements
outside the canvas; the canvas stays pure geometry.

Axis ticks come from a Heckbert nice-numbers pass (`chart::nice_ticks`),
so gridlines always land on round values; labels compact large values
(`1500 → 1.5k`, `2000000 → 2M`, via `chart::tick_label`).

## Colors

Single-series charts (`Sparkline`, `LineChart`) default to the theme primary.
Multi-item charts (`BarChart`, `PieChart`) rotate through the twelve chromatic
palette hues, ordered so neighbors contrast — `Dark` and `Gray` are left out
because they read as chrome, not data. Named colors resolve at the chart
accent shade (4 in dark mode, 6 in light), so every chart adapts to the
scheme. Override with `.color(..)` (one color for everything) or `.colors(..)`
(per item, cycled when shorter than the series); both accept a `ColorName` or
an explicit `Hsla` via [`ColorValue`](theming.md#css-style-colors).

## Sparkline

A tiny inline trend line. Values are min/max normalized to the height (a flat
series draws a centerline); fewer than two values paint nothing. NaN and
infinite entries are left out of the min/max scale and draw at mid-height.

```rust
Sparkline::new([3.0, 5.0, 2.0, 8.0, 6.0]).fill()
Sparkline::new(history).color(ColorName::Teal).stroke(1.0).full_width()
```

| Method | Default | Notes |
| --- | --- | --- |
| `new(values)` | — | any `IntoIterator<Item = f32>` |
| `color(color)` | theme primary | `ColorName` or explicit color |
| `stroke(px)` | `2.0` | line width (min 0.5) |
| `fill()` | off | area to the baseline, line color at 0.15 alpha |
| `width(px)` | `120.0` | fixed width |
| `full_width()` | off | stretch to the parent instead |
| `height(px)` | `32.0` | |

## BarChart

Vertical bars scaled against the largest value, baseline at zero. With
`entries`, category labels render in a row of equal-width cells under the
bars — plain elements, lined up with the painted slots.

```rust
BarChart::new([12.0, 9.0, 15.0, 7.0])
BarChart::entries([("Mon", 12.0), ("Tue", 9.0), ("Wed", 15.0)]).gap(0.3)
```

| Method | Default | Notes |
| --- | --- | --- |
| `new(values)` / `entries(pairs)` | — | `f32`s, or `(label, value)` pairs |
| `color(color)` | palette rotation | one color for every bar |
| `colors(iter)` | palette rotation | per-bar colors, cycled |
| `gap(fraction)` | `0.2` | empty share of each bar slot, clamped `0.0..=0.9` |
| `width(px)` | parent width | fixed px override |
| `height(px)` | `140.0` | plot only, excluding the label row |

> **Note** Negative values clamp to zero — bars scale against the tallest
> value with the baseline pinned at zero, so there are no downward bars.

## LineChart

One or more line series. A bare `new(values)` is the old minimal chart
(min/max normalized, four light gridlines); named series, axis, labels, and
hover build it into a full chart.

```rust
LineChart::new([12.0, 18.0, 9.0, 24.0, 20.0, 31.0]).fill().height(180.0)

LineChart::series("Revenue", [12.0, 18.0, 24.0, 30.0])
    .add_series("Costs", [8.0, 11.0, 13.0, 16.0])
    .axis()
    .labels(["Q1", "Q2", "Q3", "Q4"])
    .hover()
```

| Method | Default | Notes |
| --- | --- | --- |
| `new(values)` | — | single anonymous series |
| `series(label, values)` | — | named first series (shows in the legend) |
| `add_series(label, values)` | — | more series; all share the y scale |
| `color(color)` / `colors(iter)` | primary / palette rotation | per-series when multi |
| `stroke(px)` | `2.0` | line width (min 0.5) |
| `fill()` | off | area to the baseline, line color at 0.15 alpha |
| `axis()` | off | y tick labels + aligned gridlines; lines scale to the tick range |
| `labels(iter)` | none | category labels under the plot, one per point |
| `hover()` | off | tooltip with every series' value at the hovered point |
| `width(px)` / `height(px)` | parent / `140.0` | |

## AreaChart

Filled series, **stacked** by default — each band shows its contribution and
the top edge is the total (junk/negative values count as zero in the stack).
`.overlaid()` draws the raw series over each other instead.

```rust
AreaChart::series("Free", [40.0, 42.0, 45.0, 48.0])
    .add_series("Pro", [12.0, 15.0, 21.0, 26.0])
    .axis()
    .labels(["Apr", "May", "Jun", "Jul"])
```

Methods: `new(values)`, `series(label, values)`, `add_series(label, values)`,
`overlaid()`, `colors(iter)`, `axis()`, `labels(iter)`, `width(px)`,
`height(px)` (default `140.0`). Named series get the legend row.

## ScatterChart

`(x, y)` points, one or more series. Axes are always on — a scatter without
a scale reads as noise — with nice ticks on both axes and x tick labels along
the bottom. `.hover()` puts each point's coordinates in a tooltip.

```rust
ScatterChart::series("Trial A", [(1.0, 3.2), (2.0, 4.1), (3.5, 2.8)])
    .add_series("Trial B", [(1.5, 2.0), (2.5, 5.5)])
    .hover()
```

Methods: `new(points)`, `series(label, points)`, `add_series(label, points)`,
`colors(iter)`, `hover()`, `width(px)`, `height(px)` (default `180.0`).
Non-finite points are skipped.

## PieChart

Proportional slices, starting at 12 o'clock and sweeping clockwise.
Non-positive and non-finite values contribute nothing. `donut` cuts a hole in
the middle; with `entries`, a wrapping color-dot legend renders below the
circle.

```rust
PieChart::new([40.0, 30.0, 20.0, 10.0]).donut(0.6)
PieChart::entries([("Rust", 62.0), ("TOML", 25.0), ("Other", 13.0)])
```

| Method | Default | Notes |
| --- | --- | --- |
| `new(values)` / `entries(pairs)` | — | `f32`s, or `(label, value)` pairs |
| `color(color)` | palette rotation | one color for every slice |
| `colors(iter)` | palette rotation | per-slice colors, cycled |
| `size(px)` | `160.0` | diameter — pies are square |
| `donut(fraction)` | off | the hole's share of the radius, clamped `0.05..=0.95` |

> **Tip** Charts have no intrinsic titles or captions — compose them with
> [`Text`](typography.md#text) and a [`Stack`](layout.md#stack)/`Group` like
> the gallery's Charts section does.
