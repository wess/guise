//! Entity-level tests on gpui's test harness (`#[gpui::test]` +
//! `TestAppContext` — the same rig zed's own tests use). These cover the
//! wiring pure unit tests can't reach: signals, bindings, form observers,
//! entity events, and the theme global.
//!
//! Observer effects flush between `cx.update` blocks, so assertions that
//! depend on an observer firing sit in their own block.

use gpui::AppContext as _;
use gpui::TestAppContext;

use crate::input::{Date, DatePicker, Select};
use crate::reactive::{validators, Form, Signal};
use crate::theme::{theme, Color, Theme};
use crate::{Carousel, CarouselEvent};

#[gpui::test]
fn signal_binding_and_lens_round_trip(cx: &mut TestAppContext) {
    let count = cx.update(|cx| Signal::new(cx, 5_i32));
    let binding = count.binding();
    cx.update(|cx| {
        assert_eq!(binding.get(cx), 5);
        binding.set(cx, 9);
        assert_eq!(count.get(cx), 9);
    });

    #[derive(Clone, PartialEq)]
    struct Settings {
        muted: bool,
    }
    let settings = cx.update(|cx| Signal::new(cx, Settings { muted: false }));
    let muted = settings.lens(|s| s.muted, |s, v| s.muted = v);
    cx.update(|cx| {
        muted.set(cx, true);
        assert!(settings.read(cx).muted);
        // Mapped bindings convert both ways.
        let as_text = muted.map(|b| b.to_string(), |s: String| s == "true");
        assert_eq!(as_text.get(cx), "true");
        as_text.set(cx, "false".to_string());
        assert!(!settings.read(cx).muted);
    });
}

#[gpui::test]
fn form_validates_and_revalidates_live(cx: &mut TestAppContext) {
    let form = cx.update(|cx| {
        Form::new(cx)
            .field(cx, "email", "")
            .rule("email", validators::required())
            .rule("email", validators::email())
            .field(cx, "confirm", "")
            .rule_form("confirm", validators::equals_field("email", "Must match"))
    });

    cx.update(|cx| {
        assert!(!form.validate(cx));
        assert!(form.error(cx, "email").is_some());
        assert!(!form.is_valid(cx));
    });

    // Fixing the field re-validates it live (it carried an error) — the
    // observer fires on the effect flush between these blocks.
    cx.update(|cx| form.set(cx, "email", "a@b.com"));
    cx.update(|cx| {
        assert_eq!(form.error(cx, "email"), None);
        assert!(form.touched("email"));
    });

    cx.update(|cx| form.set(cx, "confirm", "a@b.com"));
    cx.update(|cx| {
        let values = form.submit(cx).expect("form should validate");
        assert_eq!(values["email"], "a@b.com");
    });

    // Cross-field: change email, confirm no longer matches.
    cx.update(|cx| form.set(cx, "email", "other@b.com"));
    cx.update(|cx| assert!(!form.validate(cx)));
}

#[gpui::test]
fn select_bind_follows_the_signal_both_ways(cx: &mut TestAppContext) {
    let choice = cx.update(|cx| Signal::new(cx, 2_usize));
    let select = cx.update(|cx| cx.new(|cx| Select::new(cx).data(["a", "b", "c"])));
    cx.update(|cx| Select::bind(&select, &choice, cx));

    // The signal is the source of truth: the picker adopts it immediately…
    cx.update(|cx| assert_eq!(select.read(cx).selected_index(), Some(2)));

    // …and follows later writes.
    cx.update(|cx| choice.set(cx, 0));
    cx.update(|cx| assert_eq!(select.read(cx).selected_index(), Some(0)));
}

#[gpui::test]
fn datepicker_bind_adopts_signal_writes(cx: &mut TestAppContext) {
    let date = Date::new(2026, 7, 14).unwrap();
    let picked = cx.update(|cx| Signal::new(cx, None::<Date>));
    let picker = cx.update(|cx| cx.new(DatePicker::new));
    cx.update(|cx| DatePicker::bind(&picker, &picked, cx));

    cx.update(|cx| assert_eq!(picker.read(cx).selected_date(), None));
    cx.update(|cx| picked.set(cx, Some(date)));
    cx.update(|cx| assert_eq!(picker.read(cx).selected_date(), Some(date)));
}

#[gpui::test]
fn carousel_navigates_and_emits(cx: &mut TestAppContext) {
    use std::cell::RefCell;
    use std::rc::Rc;

    let deck = cx.update(|cx| {
        cx.new(|cx| {
            Carousel::new(cx)
                .slide(|_, _| gpui::Empty)
                .slide(|_, _| gpui::Empty)
                .slide(|_, _| gpui::Empty)
        })
    });
    let seen: Rc<RefCell<Vec<usize>>> = Rc::default();
    let log = seen.clone();
    cx.update(|cx| {
        cx.subscribe(&deck, move |_deck, event: &CarouselEvent, _cx| {
            log.borrow_mut().push(event.0);
        })
        .detach();
    });

    deck.update(cx, |deck, cx| {
        deck.next(cx);
        deck.next(cx);
        deck.next(cx); // wraps to 0
        deck.prev(cx); // wraps back to 2
        deck.go_to(1, cx);
        deck.go_to(1, cx); // no-op, no event
    });
    assert_eq!(*seen.borrow(), vec![1, 2, 0, 2, 1]);
    cx.update(|cx| assert_eq!(deck.read(cx).current(), 1));
}

#[gpui::test]
fn theme_presets_install_and_resolve(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::catppuccin().init(cx);
        let t = theme(cx);
        assert!(t.scheme.is_dark());
        assert_eq!(t.primary(), Color::hex("#89b4fa"));
        assert_eq!(t.body(), Color::hex("#1e1e2e"));

        // Swapping the global restyles everything that reads theme(cx).
        Theme::solarized_light().init(cx);
        let t = theme(cx);
        assert!(!t.scheme.is_dark());
        assert_eq!(t.primary(), Color::hex("#268bd2"));
    });
}
