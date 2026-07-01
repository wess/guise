# Charts

`Sparkline`, `BarChart`, `LineChart`, `PieChart` are stateless builders,
painted through gpui's `canvas` element (`paint_path` / `paint_quad`) —
minimal, axis-free visuals over plain `f32` series; bars and pies also take
`(label, value)` pairs. Everything on the canvas is geometry: no axes, ticks,
or value labels. The only text is `BarChart`'s category row and `PieChart`'s
legend, rendered as regular elements outside the canvas.

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

A sparkline grown up: the same min/max-normalized polyline plus four light
horizontal gridlines (the border color at 0.5 alpha). Fewer than two values
paint only the gridlines.

```rust
LineChart::new([12.0, 18.0, 9.0, 24.0, 20.0, 31.0]).fill().height(180.0)
```

| Method | Default | Notes |
| --- | --- | --- |
| `new(values)` | — | any `IntoIterator<Item = f32>` |
| `color(color)` | theme primary | `ColorName` or explicit color |
| `stroke(px)` | `2.0` | line width (min 0.5) |
| `fill()` | off | area to the baseline, line color at 0.15 alpha |
| `width(px)` | parent width | fixed px override |
| `height(px)` | `140.0` | |

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
