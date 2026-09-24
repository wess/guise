//! Declarative macros for motion, in the shape of the layout ones.
//!
//! [`motion!`](crate::motion) is to an animation what
//! [`style!`](crate::style) is to a box: a block of declarations instead of a
//! chain of setters, so the timing and the tweens read as one thing.
//!
//! ```ignore
//! use guise::prelude::*;
//!
//! div().child(card).animate("card", motion! {
//!     duration: 420;
//!     ease: out back;
//!     opacity: 0 => 1;
//!     y: 12 => 0;
//! })
//! ```
//!
//! [`sequence!`](crate::sequence) is the variadic one — the same job `col!`
//! does for children, for motions on a clock:
//!
//! ```ignore
//! sequence![
//!     slide_out,
//!     rel(-140) => drop_down,      // overlapping the tail
//!     with(0) => tint,             // alongside the previous
//! ]
//! ```
//!
//! Both return the builder, so anything the block does not cover still
//! chains: `motion! { … }.repeat(3)`.

/// A [`Motion`](crate::anim::Motion) as a block of declarations.
///
/// Timing first, tweens after — though the order is yours; the only rule is
/// that `enter:` / `exit:` must come first, because they pick the
/// constructor rather than chain onto it.
///
/// ```ignore
/// motion! {
///     enter: slide_up 24;     // a preset, optionally with its distance
///     duration: 420;          // ms
///     delay: 80;
///     end_delay: 120;
///     ease: out back;         // direction + curve
///     repeat: forever;        // or `once`, or a count
///     alternate;              // bare flags
///     reversed;
///     margins;                // offsets as margins, for a pinned element
///
///     opacity: 0 => 1;        // prop: from => to
///     y: 0 => [-30, 0];       // or a list of legs
///     bg: color!("#111") => color!("#333");
/// }
/// ```
///
/// **Easings**: `linear`, `spring`, `steps(4)`, or a direction and a curve —
/// `in`/`out`/`in_out` over `quad`, `cubic`, `quart`, `quint`, `sine`,
/// `expo`, `circ`, `back`, `elastic`, `bounce`. Any [`Easing`](crate::Easing)
/// expression works too.
///
/// **Presets**: `fade`, `slide_up`, `slide_down`, `slide_left`,
/// `slide_right`.
///
/// **Props**: `opacity`, `x`, `y`, `w`/`width`, `h`/`height`, `mt`/`mr`/`mb`/
/// `ml`, `pt`/`pr`/`pb`/`pl`, `radius`, `border_width`, `gap`, `font_size`,
/// `bg`/`background`, `border_color`, `color`, `rotate`, `scale`, and
/// `custom("name")` for a number that is not a style at all. Numbers are px
/// (or degrees, or a multiplier); colours are any `Into<Hsla>`.
#[macro_export]
macro_rules! motion {
    ( enter : $kind:ident $distance:expr ; $($rest:tt)* ) => {
        $crate::__motion!(
            @m $crate::anim::Motion::enter_from($crate::__kind!($kind), $distance as f32)
            ; $($rest)*
        )
    };
    ( enter : $kind:ident ; $($rest:tt)* ) => {
        $crate::__motion!(@m $crate::anim::Motion::enter($crate::__kind!($kind)) ; $($rest)*)
    };
    ( exit : $kind:ident $distance:expr ; $($rest:tt)* ) => {
        $crate::__motion!(
            @m $crate::anim::Motion::exit_to($crate::__kind!($kind), $distance as f32)
            ; $($rest)*
        )
    };
    ( exit : $kind:ident ; $($rest:tt)* ) => {
        $crate::__motion!(@m $crate::anim::Motion::exit($crate::__kind!($kind)) ; $($rest)*)
    };
    ( $($decls:tt)* ) => {
        $crate::__motion!(@m $crate::anim::Motion::new() ; $($decls)*)
    };
}

/// A [`Sequence`](crate::anim::Sequence) of motions on one clock.
///
/// A bare motion lands after everything before it. To place one anywhere
/// else, put the position in front of it:
///
/// ```ignore
/// sequence![
///     fade_in,
///     rel(-120) => slide_up,              // 120ms before the end so far
///     with(0) => tint,                    // alongside the previous entry
///     abs(600) => flash,                  // from the sequence's own start
///     label("settled", 50) => ripple,     // 50ms after a placed label
/// ]
/// ```
///
/// The position goes first because a Rust macro cannot read anything but
/// `,`, `;` or `=>` after an expression — and `=>` reads like a timeline
/// anyway. Labels are placed with `Sequence::label`, so a sequence that uses
/// them starts from the builder.
#[macro_export]
macro_rules! sequence {
    ( $($items:tt)* ) => {
        // The trailing comma the muncher relies on. A list that already had
        // one ends in `,,`, which the terminal arm eats.
        $crate::__sequence!(@s $crate::anim::Sequence::new() ; $($items)* ,)
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __motion {
    (@m $m:expr ;) => { $m };

    // timing
    (@m $m:expr ; duration : $v:expr ; $($r:tt)*) => {
        $crate::__motion!(@m $m.duration($v as f32) ; $($r)*)
    };
    (@m $m:expr ; delay : $v:expr ; $($r:tt)*) => {
        $crate::__motion!(@m $m.delay($v as f32) ; $($r)*)
    };
    (@m $m:expr ; end_delay : $v:expr ; $($r:tt)*) => {
        $crate::__motion!(@m $m.end_delay($v as f32) ; $($r)*)
    };

    // --- easing: keyword forms before the expression fallback, which would
    //     otherwise try to parse `out back` and fail without backtracking ---
    (@m $m:expr ; ease : linear ; $($r:tt)*) => {
        $crate::__motion!(@m $m.ease($crate::Easing::Linear) ; $($r)*)
    };
    (@m $m:expr ; ease : spring ; $($r:tt)*) => {
        $crate::__motion!(@m $m.ease($crate::Easing::Spring($crate::Spring::default())) ; $($r)*)
    };
    (@m $m:expr ; ease : steps($n:expr) ; $($r:tt)*) => {
        $crate::__motion!(@m $m.ease($crate::Easing::Steps($n as u32)) ; $($r)*)
    };
    (@m $m:expr ; ease : in_out $c:ident ; $($r:tt)*) => {
        $crate::__motion!(@m $m.ease($crate::Easing::InOut($crate::__curve!($c))) ; $($r)*)
    };
    (@m $m:expr ; ease : in $c:ident ; $($r:tt)*) => {
        $crate::__motion!(@m $m.ease($crate::Easing::In($crate::__curve!($c))) ; $($r)*)
    };
    (@m $m:expr ; ease : out $c:ident ; $($r:tt)*) => {
        $crate::__motion!(@m $m.ease($crate::Easing::Out($crate::__curve!($c))) ; $($r)*)
    };
    (@m $m:expr ; ease : $e:expr ; $($r:tt)*) => {
        $crate::__motion!(@m $m.ease($e) ; $($r)*)
    };

    // repetition
    (@m $m:expr ; repeat : forever ; $($r:tt)*) => {
        $crate::__motion!(@m $m.repeat_forever() ; $($r)*)
    };
    (@m $m:expr ; repeat : once ; $($r:tt)*) => {
        $crate::__motion!(@m $m.repeat(1) ; $($r)*)
    };
    (@m $m:expr ; repeat : $n:expr ; $($r:tt)*) => {
        $crate::__motion!(@m $m.repeat($n as u32) ; $($r)*)
    };
    (@m $m:expr ; alternate ; $($r:tt)*) => {
        $crate::__motion!(@m $m.alternate(true) ; $($r)*)
    };
    (@m $m:expr ; reversed ; $($r:tt)*) => {
        $crate::__motion!(@m $m.reversed(true) ; $($r)*)
    };
    (@m $m:expr ; margins ; $($r:tt)*) => {
        $crate::__motion!(@m $m.as_margins() ; $($r)*)
    };

    // --- tracks: `custom(..)` first (it is not an ident), then the list form,
    //     since `[a, b]` is also a perfectly good expression ---
    (@m $m:expr ; custom($n:literal) : $from:expr => [ $($k:expr),* $(,)? ] ; $($r:tt)*) => {
        $crate::__motion!(@m $m.keyframes($crate::Prop::Custom($n), $from, [$($k),*]) ; $($r)*)
    };
    (@m $m:expr ; custom($n:literal) : $from:expr => $to:expr ; $($r:tt)*) => {
        $crate::__motion!(@m $m.tween($crate::Prop::Custom($n), $from, $to) ; $($r)*)
    };
    (@m $m:expr ; $p:ident : $from:expr => [ $($k:expr),* $(,)? ] ; $($r:tt)*) => {
        $crate::__motion!(@m $m.keyframes($crate::__prop!($p), $from, [$($k),*]) ; $($r)*)
    };
    (@m $m:expr ; $p:ident : $from:expr => $to:expr ; $($r:tt)*) => {
        $crate::__motion!(@m $m.tween($crate::__prop!($p), $from, $to) ; $($r)*)
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __sequence {
    (@s $s:expr ;) => { $s };
    (@s $s:expr ; ,) => { $s };

    (@s $s:expr ; rel($v:expr) => $m:expr , $($r:tt)*) => {
        $crate::__sequence!(@s $s.add_at($m, $crate::At::Rel($v as f32)) ; $($r)*)
    };
    (@s $s:expr ; abs($v:expr) => $m:expr , $($r:tt)*) => {
        $crate::__sequence!(@s $s.add_at($m, $crate::At::Abs($v as f32)) ; $($r)*)
    };
    (@s $s:expr ; with($v:expr) => $m:expr , $($r:tt)*) => {
        $crate::__sequence!(@s $s.add_at($m, $crate::At::With($v as f32)) ; $($r)*)
    };
    (@s $s:expr ; label($n:expr, $v:expr) => $m:expr , $($r:tt)*) => {
        $crate::__sequence!(@s $s.add_at($m, $crate::At::Label($n.into(), $v as f32)) ; $($r)*)
    };
    (@s $s:expr ; $m:expr , $($r:tt)*) => {
        $crate::__sequence!(@s $s.add($m) ; $($r)*)
    };
}

/// The [`TransitionKind`](crate::TransitionKind) a preset word names.
#[macro_export]
#[doc(hidden)]
macro_rules! __kind {
  (fade) => {
    $crate::TransitionKind::Fade
  };
  (slide_up) => {
    $crate::TransitionKind::SlideUp
  };
  (slide_down) => {
    $crate::TransitionKind::SlideDown
  };
  (slide_left) => {
    $crate::TransitionKind::SlideLeft
  };
  (slide_right) => {
    $crate::TransitionKind::SlideRight
  };
}

/// The [`Curve`](crate::Curve) a shape word names.
#[macro_export]
#[doc(hidden)]
macro_rules! __curve {
  (quad) => {
    $crate::Curve::Quad
  };
  (cubic) => {
    $crate::Curve::Cubic
  };
  (quart) => {
    $crate::Curve::Quart
  };
  (quint) => {
    $crate::Curve::Quint
  };
  (sine) => {
    $crate::Curve::Sine
  };
  (expo) => {
    $crate::Curve::Expo
  };
  (circ) => {
    $crate::Curve::Circ
  };
  (back) => {
    $crate::Curve::Back
  };
  (elastic) => {
    $crate::Curve::Elastic
  };
  (bounce) => {
    $crate::Curve::Bounce
  };
}

/// The [`Prop`](crate::Prop) a declaration name refers to.
#[macro_export]
#[doc(hidden)]
macro_rules! __prop {
  (opacity) => {
    $crate::Prop::Opacity
  };
  (x) => {
    $crate::Prop::X
  };
  (y) => {
    $crate::Prop::Y
  };
  (w) => {
    $crate::Prop::Width
  };
  (width) => {
    $crate::Prop::Width
  };
  (h) => {
    $crate::Prop::Height
  };
  (height) => {
    $crate::Prop::Height
  };
  (mt) => {
    $crate::Prop::MarginTop
  };
  (mr) => {
    $crate::Prop::MarginRight
  };
  (mb) => {
    $crate::Prop::MarginBottom
  };
  (ml) => {
    $crate::Prop::MarginLeft
  };
  (pt) => {
    $crate::Prop::PadTop
  };
  (pr) => {
    $crate::Prop::PadRight
  };
  (pb) => {
    $crate::Prop::PadBottom
  };
  (pl) => {
    $crate::Prop::PadLeft
  };
  (radius) => {
    $crate::Prop::Radius
  };
  (border_width) => {
    $crate::Prop::BorderWidth
  };
  (gap) => {
    $crate::Prop::Gap
  };
  (font_size) => {
    $crate::Prop::FontSize
  };
  (bg) => {
    $crate::Prop::Background
  };
  (background) => {
    $crate::Prop::Background
  };
  (border_color) => {
    $crate::Prop::BorderColor
  };
  (color) => {
    $crate::Prop::TextColor
  };
  (rotate) => {
    $crate::Prop::Rotate
  };
  (scale) => {
    $crate::Prop::Scale
  };
  (custom($n:literal)) => {
    $crate::Prop::Custom($n)
  };
}

#[cfg(test)]
mod tests {
  use crate::anim::{Loop, Motion, Prop, Sequence};
  use crate::Easing;

  /// A `#[macro_export]` macro is only type-checked where it is invoked, so
  /// every arm needs a call site somewhere.
  #[test]
  fn every_declaration_expands() {
    let m = motion! {
        duration: 420;
        delay: 80;
        end_delay: 120;
        ease: out back;
        repeat: 3;
        alternate;
        reversed;
        margins;
        opacity: 0 => 1;
        radius: 6 => 24;
    };
    assert_eq!(m.duration, 420.0);
    assert_eq!(m.delay, 80.0);
    assert_eq!(m.end_delay, 120.0);
    assert_eq!(m.ease, Easing::Out(crate::Curve::Back));
    assert_eq!(m.loops, Loop::Times(3));
    assert!(m.alternate && m.reversed);
  }

  #[test]
  fn every_easing_spelling_expands() {
    let curves = [
      motion! { ease: linear; },
      motion! { ease: spring; },
      motion! { ease: steps(4); },
      motion! { ease: in quad; },
      motion! { ease: out cubic; },
      motion! { ease: in_out sine; },
      motion! { ease: Easing::CubicBezier(0.25, 0.1, 0.25, 1.0); },
      motion! { ease: in quart; },
      motion! { ease: out quint; },
      motion! { ease: in expo; },
      motion! { ease: out circ; },
      motion! { ease: out elastic; },
      motion! { ease: out bounce; },
    ];
    assert_eq!(curves[0].ease, Easing::Linear);
    assert_eq!(curves[2].ease, Easing::Steps(4));
    assert_eq!(curves[4].ease, Easing::Out(crate::Curve::Cubic));
  }

  #[test]
  fn presets_pick_the_constructor() {
    let m = motion! { enter: slide_up; duration: 300; };
    assert_eq!(m.duration, 300.0);
    assert_eq!(m.sample(0.0).number(Prop::Opacity), Some(0.0));
    assert_eq!(m.sample(300.0).number(Prop::Y), Some(0.0));

    // With a distance, and the exit twin.
    let far = motion! { enter: slide_left 24; };
    assert_eq!(far.sample(0.0).number(Prop::X), Some(24.0));
    let out = motion! { exit: fade; };
    assert_eq!(out.sample(0.0).number(Prop::Opacity), Some(1.0));
    let _ = motion! { exit: slide_down 12; };
    let _ = motion! { enter: fade; };
    let _ = motion! { enter: slide_right; };
  }

  #[test]
  fn a_leg_list_takes_values_or_keyframes() {
    let plain = motion! {
        duration: 300;
        ease: linear;
        y: 0 => [-30, 0];
    };
    assert_eq!(plain.iteration_ms(), 300.0);
    assert_eq!(plain.sample(150.0).number(Prop::Y), Some(-30.0));

    // The same track, with a leg that sets its own time.
    let timed = motion! {
        duration: 300;
        ease: linear;
        y: 0 => [crate::Keyframe::to(-30.0).duration(100.0), crate::Keyframe::to(0.0)];
    };
    assert_eq!(timed.sample(100.0).number(Prop::Y), Some(-30.0));
  }

  #[test]
  fn colours_tween_through_the_macro() {
    let m = motion! {
        duration: 200;
        bg: color!("#111111") => color!("#333333");
        color: color!(teal) => color!(orchid);
        border_color: color!("#000000") => color!("#ffffff");
    };
    assert!(m.sample(0.0).color(Prop::Background).is_some());
    assert!(m.sample(200.0).color(Prop::TextColor).is_some());
  }

  #[test]
  fn every_prop_word_maps() {
    let m = motion! {
        opacity: 0 => 1;
        x: 0 => 1;
        y: 0 => 1;
        w: 0 => 1;
        width: 0 => 1;
        h: 0 => 1;
        height: 0 => 1;
        mt: 0 => 1;
        mr: 0 => 1;
        mb: 0 => 1;
        ml: 0 => 1;
        pt: 0 => 1;
        pr: 0 => 1;
        pb: 0 => 1;
        pl: 0 => 1;
        radius: 0 => 1;
        border_width: 0 => 1;
        gap: 0 => 1;
        font_size: 0 => 1;
        rotate: 0 => 1;
        scale: 0 => 1;
        custom("progress"): 0 => 1;
        custom("legs"): 0 => [2, 4];
    };
    // `w`/`width` and `h`/`height` are the same track written twice.
    assert_eq!(m.tracks.len(), 23);
    assert_eq!(m.sample(m.total_ms()).number(Prop::Rotate), Some(1.0));
    assert_eq!(
      m.sample(m.total_ms()).number(Prop::Custom("progress")),
      Some(1.0)
    );
    assert_eq!(
      m.sample(m.total_ms()).number(Prop::Custom("legs")),
      Some(4.0)
    );
  }

  /// Tailor's macro flavour prints floats as `16.`, so that exact spelling
  /// has to parse — the generator and the macro are one contract.
  #[test]
  fn the_spellings_tailor_generates_all_parse() {
    let m = motion! {
        enter: slide_up 16.;
        duration: 400.;
        delay: 60.;
        ease: in_out sine;
        repeat: forever;
        alternate;
        margins;
    };
    assert_eq!(m.duration, 400.0);
    assert_eq!(m.delay, 60.0);
    assert_eq!(m.loops, Loop::Forever);
    // `margins` moved the slide off the inset.
    assert_eq!(m.sample(0.0).number(Prop::MarginTop), Some(16.0));
    let _ = motion! { enter: fade; duration: 260.; ease: out cubic; };
  }

  #[test]
  fn repeat_words_and_counts() {
    assert_eq!(motion! { repeat: once; }.loops, Loop::Times(1));
    assert_eq!(motion! { repeat: forever; }.loops, Loop::Forever);
    assert_eq!(motion! { repeat: 5; }.loops, Loop::Times(5));
  }

  #[test]
  fn an_empty_block_is_a_default_motion() {
    assert_eq!(motion! {}, Motion::new());
  }

  #[test]
  fn the_block_still_chains() {
    // Timing with no track to spend it on is a zero-length motion, so the
    // chained setters go on something that actually moves.
    let m = motion! { duration: 100; opacity: 0 => 1; }
      .repeat(2)
      .alternate(true);
    assert_eq!(m.total_ms(), 200.0);
  }

  fn leg(from: f32, to: f32) -> Motion {
    motion! { duration: 100; ease: linear; x: from => to; }
  }

  #[test]
  fn sequences_queue_and_place() {
    let s = sequence![leg(0.0, 10.0), leg(10.0, 20.0)];
    assert_eq!(s.len(), 2);
    assert_eq!(s.iteration_ms(), 200.0);

    let overlapped = sequence![
        leg(0.0, 10.0),
        rel(-50) => leg(10.0, 20.0),
        with(0) => leg(20.0, 30.0),
        abs(400) => leg(30.0, 40.0),
    ];
    assert_eq!(overlapped.len(), 4);
    assert_eq!(overlapped.iteration_ms(), 500.0);
  }

  #[test]
  fn a_label_anchors_a_sequence_entry() {
    let placed = Sequence::new()
      .add(leg(0.0, 10.0))
      .label("settled", crate::At::End);
    let s = sequence![label("settled", 50) => leg(10.0, 20.0)];
    // The macro can only read labels a builder placed, so the two halves
    // meet here rather than inside one block.
    assert_eq!(s.len(), 1);
    assert_eq!(
      placed.resolve(&crate::At::Label("settled".into(), 0.0)),
      100.0
    );
  }

  #[test]
  fn a_trailing_comma_is_fine_either_way() {
    assert_eq!(sequence![leg(0.0, 1.0)].len(), 1);
    assert_eq!(sequence![leg(0.0, 1.0),].len(), 1);
    assert_eq!(sequence![].len(), 0);
  }
}
