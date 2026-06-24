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
}
