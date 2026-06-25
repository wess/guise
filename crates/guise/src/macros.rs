//! Declarative layout macros: terse builders for container components.
//!
//! ```ignore
//! use guise::prelude::*;
//!
//! col![
//!     row![avatar, name, Spacer::new(), actions],
//!     divider,
//!     body,
//! ]
//! ```
//!
//! Each macro forwards its comma-separated arguments as children and returns
//! the underlying builder, so you can keep chaining configuration:
//!
//! ```ignore
//! card![title, body].with_border(true).shadow(Size::Md)
//! ```
//!
//! A trailing comma is allowed. The macros bring `.child()` into scope
//! themselves, so no extra imports are needed.
//!
//! There is intentionally **no macro per component**: macros only help
//! containers that take a variadic list of children. A leaf builder like
//! `Button::new(id, label).variant(..)` is already the better API — a macro
//! there would just lose the fluent setters.

/// A Flutter-style horizontal [`Row`](crate::flex::Row).
#[macro_export]
macro_rules! row {
    ($($child:expr),* $(,)?) => {{
        #[allow(unused_imports)]
        use $crate::__ParentElement as _;
        $crate::flex::Row::new() $(.child($child))*
    }};
}

/// A Flutter-style vertical [`Column`](crate::flex::Column).
///
/// Named `col!` (not `column!`) to avoid clashing with the std `column!` macro.
#[macro_export]
macro_rules! col {
    ($($child:expr),* $(,)?) => {{
        #[allow(unused_imports)]
        use $crate::__ParentElement as _;
        $crate::flex::Column::new() $(.child($child))*
    }};
}

/// A layered [`Stack`](crate::flex::Stack) (z-axis overlap).
#[macro_export]
macro_rules! zstack {
    ($($child:expr),* $(,)?) => {{
        #[allow(unused_imports)]
        use $crate::__ParentElement as _;
        $crate::flex::Stack::new() $(.child($child))*
    }};
}

/// A wrapping row, [`Wrap`](crate::flex::Wrap).
#[macro_export]
macro_rules! wrap {
    ($($child:expr),* $(,)?) => {{
        #[allow(unused_imports)]
        use $crate::__ParentElement as _;
        $crate::flex::Wrap::new() $(.child($child))*
    }};
}

/// A themed vertical [`Stack`](crate::layout::Stack) (token-based spacing).
#[macro_export]
macro_rules! vstack {
    ($($child:expr),* $(,)?) => {{
        #[allow(unused_imports)]
        use $crate::__ParentElement as _;
        $crate::layout::Stack::new() $(.child($child))*
    }};
}

/// A themed horizontal [`Group`](crate::layout::Group) (token-based spacing).
#[macro_export]
macro_rules! hstack {
    ($($child:expr),* $(,)?) => {{
        #[allow(unused_imports)]
        use $crate::__ParentElement as _;
        $crate::layout::Group::new() $(.child($child))*
    }};
}

/// A [`Center`](crate::layout::Center) wrapping the given children.
#[macro_export]
macro_rules! center {
    ($($child:expr),* $(,)?) => {{
        #[allow(unused_imports)]
        use $crate::__ParentElement as _;
        $crate::layout::Center::new() $(.child($child))*
    }};
}

/// A [`Paper`](crate::Paper) surface around the given children.
#[macro_export]
macro_rules! paper {
    ($($child:expr),* $(,)?) => {{
        #[allow(unused_imports)]
        use $crate::__ParentElement as _;
        $crate::Paper::new() $(.child($child))*
    }};
}

/// A [`Card`](crate::Card) around the given children.
#[macro_export]
macro_rules! card {
    ($($child:expr),* $(,)?) => {{
        #[allow(unused_imports)]
        use $crate::__ParentElement as _;
        $crate::Card::new() $(.child($child))*
    }};
}

/// A [`Modal`](crate::Modal) around the given children. Chain `.title(..)` and
/// `.on_close(..)` after.
#[macro_export]
macro_rules! modal {
    ($($child:expr),* $(,)?) => {{
        #[allow(unused_imports)]
        use $crate::__ParentElement as _;
        $crate::Modal::new() $(.child($child))*
    }};
}

// --- Component shorthands --------------------------------------------------
//
// Content components also accept `format!` args; e.g. `text!("Hi {}", name)`.
// All of these return the underlying builder, so `.variant(..)`, `.color(..)`,
// etc. still chain.

/// [`Text`](crate::Text), with optional `format!` arguments.
#[macro_export]
macro_rules! text {
    ($fmt:literal, $($arg:tt)*) => { $crate::Text::new(format!($fmt, $($arg)*)) };
    ($content:expr) => { $crate::Text::new($content) };
}

/// [`Title`](crate::Title), with optional `format!` arguments.
#[macro_export]
macro_rules! title {
    ($fmt:literal, $($arg:tt)*) => { $crate::Title::new(format!($fmt, $($arg)*)) };
    ($content:expr) => { $crate::Title::new($content) };
}

/// Inline [`Code`](crate::Code), with optional `format!` arguments.
#[macro_export]
macro_rules! code {
    ($fmt:literal, $($arg:tt)*) => { $crate::Code::new(format!($fmt, $($arg)*)) };
    ($content:expr) => { $crate::Code::new($content) };
}

/// A [`Kbd`](crate::Kbd) key, with optional `format!` arguments.
#[macro_export]
macro_rules! kbd {
    ($fmt:literal, $($arg:tt)*) => { $crate::Kbd::new(format!($fmt, $($arg)*)) };
    ($content:expr) => { $crate::Kbd::new($content) };
}

/// A [`Button`](crate::Button): `button!(id, label)`. Chain setters after.
#[macro_export]
macro_rules! button {
    ($($arg:expr),* $(,)?) => { $crate::Button::new($($arg),*) };
}

/// A [`Badge`](crate::Badge): `badge!(label)`. Chain setters after.
#[macro_export]
macro_rules! badge {
    ($($arg:expr),* $(,)?) => { $crate::Badge::new($($arg),*) };
}

/// A CSS-style color literal, producing a gpui `Hsla`.
///
/// ```ignore
/// color!(rgb(34, 139, 230))
/// color!(rgba(34, 139, 230, 0.5))
/// color!(hsl(210, 80, 52))          // s/l are percentages (no `%` token)
/// color!(hsla(210, 80, 52, 0.5))
/// color!(teal)                      // a CSS named color
/// color!("#228be6")                 // any CSS string — hex (incl. #rgba/#rrggbbaa),
/// color!("hsl(210, 80%, 52%)")      // functional with `%`, or a named color
/// ```
///
/// Bare hex (`#228be6`) can't be a macro token in Rust, so pass hex as a string.
/// The result is an `Hsla`, usable directly in `.bg(..)` / `.text_color(..)` and
/// anywhere a `ColorValue` is accepted (component `.color(..)`, `Theme::with_*`).
#[macro_export]
macro_rules! color {
    (rgb($r:expr, $g:expr, $b:expr $(,)?)) => {
        $crate::theme::rgb($r, $g, $b)
    };
    (rgba($r:expr, $g:expr, $b:expr, $a:expr $(,)?)) => {
        $crate::theme::rgba($r, $g, $b, $a)
    };
    (hsl($h:expr, $s:expr, $l:expr $(,)?)) => {
        $crate::theme::hsl($h as f32, $s as f32, $l as f32)
    };
    (hsla($h:expr, $s:expr, $l:expr, $a:expr $(,)?)) => {
        $crate::theme::hsla($h as f32, $s as f32, $l as f32, $a)
    };
    ($css:literal) => {
        $crate::theme::css($css).expect("guise: invalid CSS color literal")
    };
    ($name:ident) => {
        $crate::theme::css(stringify!($name)).expect("guise: unknown CSS color name")
    };
}

/// A CSS-like style block. Expands to an element transform applied with
/// [`StyleExt::apply`](crate::style::StyleExt):
///
/// ```ignore
/// use guise::prelude::*;
///
/// gpui::div().apply(style! {
///     display: flex;
///     direction: column;
///     align: center;
///     gap: 8;
///     padding: 16;
///     width: full;
///     radius: 12;
///     background: "#11151c";              // string → css() shorthand
///     color: color!(rgb(230, 230, 230));  // or any color! / Hsla expr
///     border: color!("#2a2f3a");          // 1px border of this color
///     weight: semibold;
///     opacity: 0.95;
/// })
/// ```
///
/// Numbers are pixels; colors are a string literal (parsed via `css`) or any
/// `Into<Hsla>` expression (e.g. `color!(..)`). Theme tokens (`Size::Md`) aren't
/// available here — `style!` is pure and has no `cx`; use raw px or the builder
/// methods for token-based values. Every declaration ends with `;`.
///
/// Supported: `background`, `color`, `border`; `display: flex`;
/// `direction: row|column|col`; `align: start|center|end|stretch`;
/// `justify: start|center|end|between|around|evenly`;
/// `position: absolute|relative`; `weight: bold|semibold|medium|normal`;
/// `width`/`height` (`full` or px), `size`, `min_width`, `min_height`,
/// `padding`/`px`/`py`/`pt`/`pr`/`pb`/`pl`, `margin`/`mx`/`my`/`mt`/`mr`/`mb`/`ml`,
/// `radius`, `gap`, `font_size`, `opacity`.
#[macro_export]
macro_rules! style {
    ( $($decls:tt)* ) => {
        |__guise_el| {
            #[allow(unused_imports)]
            use $crate::gpui::Styled as _;
            $crate::__style!(@m __guise_el ; $($decls)*)
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __style {
    (@m $el:expr ;) => { $el };

    // --- color-valued (string literal → css; else any Into<Hsla> expr) ---
    (@m $el:expr ; background : $v:literal ; $($r:tt)*) => {
        $crate::__style!(@m $el.bg($crate::theme::css($v).expect("style!: invalid color")) ; $($r)*)
    };
    (@m $el:expr ; background : $v:expr ; $($r:tt)*) => {
        $crate::__style!(@m $el.bg($v) ; $($r)*)
    };
    (@m $el:expr ; color : $v:literal ; $($r:tt)*) => {
        $crate::__style!(@m $el.text_color($crate::theme::css($v).expect("style!: invalid color")) ; $($r)*)
    };
    (@m $el:expr ; color : $v:expr ; $($r:tt)*) => {
        $crate::__style!(@m $el.text_color($v) ; $($r)*)
    };
    (@m $el:expr ; border : $v:literal ; $($r:tt)*) => {
        $crate::__style!(@m $el.border_1().border_color($crate::theme::css($v).expect("style!: invalid color")) ; $($r)*)
    };
    (@m $el:expr ; border : $v:expr ; $($r:tt)*) => {
        $crate::__style!(@m $el.border_1().border_color($v) ; $($r)*)
    };

    // --- keyword-valued (must precede the numeric arms for the same name) ---
    (@m $el:expr ; display : flex ; $($r:tt)*) => { $crate::__style!(@m $el.flex() ; $($r)*) };

    (@m $el:expr ; direction : row ; $($r:tt)*) => { $crate::__style!(@m $el.flex_row() ; $($r)*) };
    (@m $el:expr ; direction : column ; $($r:tt)*) => { $crate::__style!(@m $el.flex_col() ; $($r)*) };
    (@m $el:expr ; direction : col ; $($r:tt)*) => { $crate::__style!(@m $el.flex_col() ; $($r)*) };

    (@m $el:expr ; align : start ; $($r:tt)*) => { $crate::__style!(@m $el.items_start() ; $($r)*) };
    (@m $el:expr ; align : center ; $($r:tt)*) => { $crate::__style!(@m $el.items_center() ; $($r)*) };
    (@m $el:expr ; align : end ; $($r:tt)*) => { $crate::__style!(@m $el.items_end() ; $($r)*) };
    (@m $el:expr ; align : stretch ; $($r:tt)*) => { $crate::__style!(@m $el.items_stretch() ; $($r)*) };

    (@m $el:expr ; justify : start ; $($r:tt)*) => { $crate::__style!(@m $el.justify_start() ; $($r)*) };
    (@m $el:expr ; justify : center ; $($r:tt)*) => { $crate::__style!(@m $el.justify_center() ; $($r)*) };
    (@m $el:expr ; justify : end ; $($r:tt)*) => { $crate::__style!(@m $el.justify_end() ; $($r)*) };
    (@m $el:expr ; justify : between ; $($r:tt)*) => { $crate::__style!(@m $el.justify_between() ; $($r)*) };
    (@m $el:expr ; justify : around ; $($r:tt)*) => { $crate::__style!(@m $el.justify_around() ; $($r)*) };
    (@m $el:expr ; justify : evenly ; $($r:tt)*) => { $crate::__style!(@m $el.justify_evenly() ; $($r)*) };

    (@m $el:expr ; position : absolute ; $($r:tt)*) => { $crate::__style!(@m $el.absolute() ; $($r)*) };
    (@m $el:expr ; position : relative ; $($r:tt)*) => { $crate::__style!(@m $el.relative() ; $($r)*) };

    (@m $el:expr ; weight : bold ; $($r:tt)*) => { $crate::__style!(@m $el.font_weight($crate::gpui::FontWeight::BOLD) ; $($r)*) };
    (@m $el:expr ; weight : semibold ; $($r:tt)*) => { $crate::__style!(@m $el.font_weight($crate::gpui::FontWeight::SEMIBOLD) ; $($r)*) };
    (@m $el:expr ; weight : medium ; $($r:tt)*) => { $crate::__style!(@m $el.font_weight($crate::gpui::FontWeight::MEDIUM) ; $($r)*) };
    (@m $el:expr ; weight : normal ; $($r:tt)*) => { $crate::__style!(@m $el.font_weight($crate::gpui::FontWeight::NORMAL) ; $($r)*) };

    (@m $el:expr ; width : full ; $($r:tt)*) => { $crate::__style!(@m $el.w_full() ; $($r)*) };
    (@m $el:expr ; height : full ; $($r:tt)*) => { $crate::__style!(@m $el.h_full() ; $($r)*) };

    // --- numeric (px) ---
    (@m $el:expr ; width : $v:expr ; $($r:tt)*) => { $crate::__style!(@m $el.w($crate::gpui::px($v as f32)) ; $($r)*) };
    (@m $el:expr ; height : $v:expr ; $($r:tt)*) => { $crate::__style!(@m $el.h($crate::gpui::px($v as f32)) ; $($r)*) };
    (@m $el:expr ; size : $v:expr ; $($r:tt)*) => { $crate::__style!(@m $el.w($crate::gpui::px($v as f32)).h($crate::gpui::px($v as f32)) ; $($r)*) };
    (@m $el:expr ; min_width : $v:expr ; $($r:tt)*) => { $crate::__style!(@m $el.min_w($crate::gpui::px($v as f32)) ; $($r)*) };
    (@m $el:expr ; min_height : $v:expr ; $($r:tt)*) => { $crate::__style!(@m $el.min_h($crate::gpui::px($v as f32)) ; $($r)*) };

    (@m $el:expr ; padding : $v:expr ; $($r:tt)*) => { $crate::__style!(@m $el.p($crate::gpui::px($v as f32)) ; $($r)*) };
    (@m $el:expr ; px : $v:expr ; $($r:tt)*) => { $crate::__style!(@m $el.px($crate::gpui::px($v as f32)) ; $($r)*) };
    (@m $el:expr ; py : $v:expr ; $($r:tt)*) => { $crate::__style!(@m $el.py($crate::gpui::px($v as f32)) ; $($r)*) };
    (@m $el:expr ; pt : $v:expr ; $($r:tt)*) => { $crate::__style!(@m $el.pt($crate::gpui::px($v as f32)) ; $($r)*) };
    (@m $el:expr ; pr : $v:expr ; $($r:tt)*) => { $crate::__style!(@m $el.pr($crate::gpui::px($v as f32)) ; $($r)*) };
    (@m $el:expr ; pb : $v:expr ; $($r:tt)*) => { $crate::__style!(@m $el.pb($crate::gpui::px($v as f32)) ; $($r)*) };
    (@m $el:expr ; pl : $v:expr ; $($r:tt)*) => { $crate::__style!(@m $el.pl($crate::gpui::px($v as f32)) ; $($r)*) };

    (@m $el:expr ; margin : $v:expr ; $($r:tt)*) => { $crate::__style!(@m $el.m($crate::gpui::px($v as f32)) ; $($r)*) };
    (@m $el:expr ; mx : $v:expr ; $($r:tt)*) => { $crate::__style!(@m $el.mx($crate::gpui::px($v as f32)) ; $($r)*) };
    (@m $el:expr ; my : $v:expr ; $($r:tt)*) => { $crate::__style!(@m $el.my($crate::gpui::px($v as f32)) ; $($r)*) };
    (@m $el:expr ; mt : $v:expr ; $($r:tt)*) => { $crate::__style!(@m $el.mt($crate::gpui::px($v as f32)) ; $($r)*) };
    (@m $el:expr ; mr : $v:expr ; $($r:tt)*) => { $crate::__style!(@m $el.mr($crate::gpui::px($v as f32)) ; $($r)*) };
    (@m $el:expr ; mb : $v:expr ; $($r:tt)*) => { $crate::__style!(@m $el.mb($crate::gpui::px($v as f32)) ; $($r)*) };
    (@m $el:expr ; ml : $v:expr ; $($r:tt)*) => { $crate::__style!(@m $el.ml($crate::gpui::px($v as f32)) ; $($r)*) };

    (@m $el:expr ; radius : $v:expr ; $($r:tt)*) => { $crate::__style!(@m $el.rounded($crate::gpui::px($v as f32)) ; $($r)*) };
    (@m $el:expr ; gap : $v:expr ; $($r:tt)*) => { $crate::__style!(@m $el.gap($crate::gpui::px($v as f32)) ; $($r)*) };
    (@m $el:expr ; font_size : $v:expr ; $($r:tt)*) => { $crate::__style!(@m $el.text_size($crate::gpui::px($v as f32)) ; $($r)*) };
    (@m $el:expr ; opacity : $v:expr ; $($r:tt)*) => { $crate::__style!(@m $el.opacity($v as f32) ; $($r)*) };
}

#[cfg(test)]
mod tests {
    use crate::{Button, ColorName, Text, Variant};

    /// Every macro must expand to a valid builder. Constructed (not rendered),
    /// since `#[macro_export]` macros are only type-checked when invoked.
    #[test]
    fn macros_expand() {
        let _ = row![Text::new("a"), Button::new("b", "B")];
        let _ = col![Text::new("a")];
        let _ = zstack![Text::new("a")];
        let _ = wrap![Text::new("a")];
        let _ = vstack![Text::new("a")];
        let _ = hstack![Text::new("a")];
        let _ = center![Text::new("a")];
        let _ = paper![Text::new("a")];
        let _ = card![Text::new("a")];
        let _ = modal![Text::new("a")];
        // Trailing comma and chaining still work.
        let _ = card![Text::new("a"), Text::new("b"),].with_border(true);
    }

    #[test]
    fn component_macros_expand() {
        // Content macros: plain, and `format!`-style.
        let name = "Ada";
        let _ = text!("hello");
        let _ = text!("hello {}", name);
        let _ = title!("Heading").order(2);
        let _ = code!("guise::Button");
        let _ = kbd!("{}", "K");
        // Forwarding macros, still chainable.
        let _ = button!("save", "Save").variant(Variant::Filled);
        let _ = badge!("New").color(ColorName::Blue);
        let _ = Button::new("x", "y"); // sanity: still works directly
    }

    #[test]
    fn color_macro_matches_constructors() {
        use crate::theme::{css, hsl, rgb, rgba};
        assert_eq!(color!(rgb(34, 139, 230)), rgb(34, 139, 230));
        assert_eq!(color!(rgba(34, 139, 230, 0.5)), rgba(34, 139, 230, 0.5));
        assert_eq!(color!(hsl(210, 80, 52)), hsl(210.0, 80.0, 52.0));
        assert_eq!(color!(teal), css("teal").unwrap());
        assert_eq!(color!("#228be6"), css("#228be6").unwrap());
        // Usable anywhere a ColorValue is accepted.
        let _ = Button::new("x", "y").color(color!(rgba(112, 72, 232, 1.0)));
    }

    #[test]
    fn style_macro_builds() {
        use crate::StyleExt;
        let _ = gpui::div().apply(style! {
            display: flex;
            direction: column;
            align: center;
            justify: between;
            gap: 8;
            padding: 12;
            width: full;
            height: 200;
            radius: 8;
            background: "#11151c";
            color: color!(rgb(230, 230, 230));
            border: color!("#2a2f3a");
            opacity: 0.95;
            weight: semibold;
            position: relative;
        });
    }
}
