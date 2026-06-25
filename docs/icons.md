# Icons

`Icon` is a themed glyph — a single home for the symbols guise components draw
(chevrons, checks, close, …) so they stay visually consistent. Icons are Unicode
glyphs rather than SVG assets: no asset pipeline, and they inherit the
surrounding text color by default.

```rust
Icon::new(IconName::Check)
Icon::new(IconName::Search).size(Size::Lg).color(ColorName::Blue)
```

Because `Icon` is just an `IntoElement`, it slots into any slot that takes one —
e.g. a button section:

```rust
Button::new("save", "Save").left_section(Icon::new(IconName::Check))
```

Methods: `new(name)`, `size(Size)` (default `Md`; 14…32px), `color(ColorName)`
(defaults to inheriting the parent text color).

## IconName

```rust
pub enum IconName {
    Check, Close, Minus, Plus,
    ChevronDown, ChevronUp, ChevronLeft, ChevronRight,
    Search, Dot, Info, Warning, Star, Copy, Menu, Ellipsis, ArrowRight, ArrowLeft,
}
```

`IconName::glyph()` returns the underlying `&'static str` if you want to render it
yourself.
