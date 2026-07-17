//! Form state: values, validators, and errors keyed by field name.
//!
//! Two layers:
//!
//! - [`FormState`] — the pure model (values + validators + errors), unit
//!   testable with no gpui. Hold it in a [`Signal`] via [`use_form`] when a
//!   plain map is all you need.
//! - [`Form`] — the reactive layer: **every field is its own
//!   `Signal<String>`**, so it plugs straight into any input's `bind`
//!   (`TextInput::bind(&input, form.signal("email"), cx)`). Rules can see
//!   the whole form (cross-field), errors live in a signal views can watch,
//!   and fields that failed validation re-validate live as they're edited.
//!
//! ```ignore
//! let form = Form::new(cx)
//!     .field(cx, "email", "")
//!     .rule("email", validators::required())
//!     .rule("email", validators::email())
//!     .field(cx, "confirm", "")
//!     .rule("confirm", validators::equals_field("email", "Emails must match"));
//!
//! TextInput::bind(&email_input, form.signal("email"), cx);
//! // in the submit handler:
//! if form.validate(cx) { save(form.value(cx, "email")); }
//! // in render:
//! Field::new().error_opt(form.error(cx, "email"))
//! ```

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use gpui::App;

use super::signal::Signal;

/// A validator: returns `Some(message)` when the value is invalid.
pub type Validator = Box<dyn Fn(&str) -> Option<String> + 'static>;

/// Form values + validators + the errors produced by the last validation.
#[derive(Default)]
pub struct FormState {
    values: HashMap<&'static str, String>,
    errors: HashMap<&'static str, String>,
    validators: HashMap<&'static str, Validator>,
}

impl FormState {
    pub fn new() -> Self {
        FormState::default()
    }

    /// Register a field with an initial value (builder form).
    pub fn field(mut self, name: &'static str, initial: impl Into<String>) -> Self {
        self.values.insert(name, initial.into());
        self
    }

    /// Attach a validator to a field (builder form).
    pub fn validator(mut self, name: &'static str, validator: Validator) -> Self {
        self.validators.insert(name, validator);
        self
    }

    /// The current value of a field (empty string if unset).
    pub fn value(&self, name: &str) -> &str {
        self.values.get(name).map(String::as_str).unwrap_or("")
    }

    /// Set a field's value and clear its error.
    pub fn set(&mut self, name: &'static str, value: impl Into<String>) {
        self.values.insert(name, value.into());
        self.errors.remove(name);
    }

    /// Validate one field, recording or clearing its error. Returns validity.
    pub fn validate_field(&mut self, name: &'static str) -> bool {
        if let Some(validator) = self.validators.get(name) {
            let value = self.values.get(name).map(String::as_str).unwrap_or("");
            match validator(value) {
                Some(message) => {
                    self.errors.insert(name, message);
                    return false;
                }
                None => {
                    self.errors.remove(name);
                }
            }
        }
        true
    }

    /// Validate every field with a validator. Returns whether all passed.
    pub fn validate(&mut self) -> bool {
        let names: Vec<&'static str> = self.validators.keys().copied().collect();
        let mut ok = true;
        for name in names {
            ok &= self.validate_field(name);
        }
        ok
    }

    /// The error message for a field, if the last validation produced one.
    pub fn error(&self, name: &str) -> Option<&str> {
        self.errors.get(name).map(String::as_str)
    }

    /// Whether there are no recorded errors.
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

/// A snapshot of every field's value, passed to [`Rule`]s so they can
/// cross-reference other fields.
pub type FormValues = HashMap<&'static str, String>;

/// A form-aware validator: sees the field's value and the whole form.
/// Plain [`Validator`]s lift into rules automatically via [`Form::rule`].
pub type Rule = Box<dyn Fn(&str, &FormValues) -> Option<String> + 'static>;

/// Built-in validators.
pub mod validators {
    use super::{FormValues, Rule, Validator};

    /// Fails when the trimmed value is empty.
    pub fn required() -> Validator {
        Box::new(|v: &str| {
            if v.trim().is_empty() {
                Some("Required".to_string())
            } else {
                None
            }
        })
    }

    /// Fails when the value is shorter than `n` characters.
    pub fn min_len(n: usize) -> Validator {
        Box::new(move |v: &str| {
            if v.chars().count() < n {
                Some(format!("Must be at least {n} characters"))
            } else {
                None
            }
        })
    }

    /// Fails when the value is longer than `n` characters.
    pub fn max_len(n: usize) -> Validator {
        Box::new(move |v: &str| {
            if v.chars().count() > n {
                Some(format!("Must be at most {n} characters"))
            } else {
                None
            }
        })
    }

    /// A permissive `a@b.c` email shape check.
    pub fn email() -> Validator {
        Box::new(|v: &str| {
            let ok = v
                .split_once('@')
                .map(|(user, domain)| {
                    !user.is_empty() && domain.contains('.') && !domain.starts_with('.')
                })
                .unwrap_or(false);
            if ok {
                None
            } else {
                Some("Enter a valid email".to_string())
            }
        })
    }

    /// Fails when the value doesn't parse as a number.
    pub fn numeric() -> Validator {
        Box::new(|v: &str| {
            if v.trim().parse::<f64>().is_ok() {
                None
            } else {
                Some("Enter a number".to_string())
            }
        })
    }

    /// Fails when the value parses below `min` (non-numbers fail too).
    pub fn min_value(min: f64) -> Validator {
        Box::new(move |v: &str| match v.trim().parse::<f64>() {
            Ok(n) if n >= min => None,
            _ => Some(format!("Must be at least {min}")),
        })
    }

    /// Fails when the value parses above `max` (non-numbers fail too).
    pub fn max_value(max: f64) -> Validator {
        Box::new(move |v: &str| match v.trim().parse::<f64>() {
            Ok(n) if n <= max => None,
            _ => Some(format!("Must be at most {max}")),
        })
    }

    /// Fails when the value isn't one of the allowed options.
    pub fn one_of(options: &'static [&'static str]) -> Validator {
        Box::new(move |v: &str| {
            if options.contains(&v) {
                None
            } else {
                Some("Not an allowed value".to_string())
            }
        })
    }

    /// Custom check: `pred` returns whether the value is valid.
    pub fn matches(pred: impl Fn(&str) -> bool + 'static, message: &'static str) -> Validator {
        Box::new(move |v: &str| {
            if pred(v) {
                None
            } else {
                Some(message.to_string())
            }
        })
    }

    /// Cross-field: fails unless this value equals the named field's
    /// ("confirm password"). A [`Rule`], for [`super::Form::rule_form`].
    pub fn equals_field(other: &'static str, message: &'static str) -> Rule {
        Box::new(move |v: &str, values: &FormValues| {
            if values.get(other).map(String::as_str) == Some(v) {
                None
            } else {
                Some(message.to_string())
            }
        })
    }
}

/// The reactive form. Cheap to clone (`Rc`-shared) and `'static`, so it can
/// be captured by handlers. Field order is registration order.
pub struct Form {
    inner: Rc<FormInner>,
}

struct FormInner {
    order: RefCell<Vec<&'static str>>,
    fields: RefCell<HashMap<&'static str, Signal<String>>>,
    rules: RefCell<HashMap<&'static str, Vec<Rule>>>,
    errors: Signal<HashMap<&'static str, String>>,
    touched: RefCell<HashSet<&'static str>>,
}

impl Clone for Form {
    fn clone(&self) -> Self {
        Form {
            inner: self.inner.clone(),
        }
    }
}

impl Form {
    pub fn new(cx: &mut App) -> Self {
        Form {
            inner: Rc::new(FormInner {
                order: RefCell::new(Vec::new()),
                fields: RefCell::new(HashMap::new()),
                rules: RefCell::new(HashMap::new()),
                errors: Signal::new(cx, HashMap::new()),
                touched: RefCell::new(HashSet::new()),
            }),
        }
    }

    /// Register a field with an initial value. Each field is a
    /// `Signal<String>`; edits mark it touched, and a field carrying an error
    /// re-validates live as it changes.
    pub fn field(self, cx: &mut App, name: &'static str, initial: impl Into<String>) -> Self {
        let signal = Signal::new(cx, initial.into());
        let form = self.clone();
        cx.observe(signal.entity(), move |_observed, cx| {
            form.inner.touched.borrow_mut().insert(name);
            if form.inner.errors.read(cx).contains_key(name) {
                form.validate_field(cx, name);
            }
        })
        .detach();
        self.inner.order.borrow_mut().push(name);
        self.inner.fields.borrow_mut().insert(name, signal);
        self
    }

    /// Attach a plain [`Validator`] to a field. Multiple rules run in order;
    /// the first failure wins.
    pub fn rule(self, name: &'static str, validator: Validator) -> Self {
        self.rule_form(name, Box::new(move |value, _values| validator(value)))
    }

    /// Attach a form-aware [`Rule`] (cross-field checks like
    /// [`validators::equals_field`]).
    pub fn rule_form(self, name: &'static str, rule: Rule) -> Self {
        self.inner
            .rules
            .borrow_mut()
            .entry(name)
            .or_default()
            .push(rule);
        self
    }

    /// The field's value signal — plug it into `TextInput::bind` and friends.
    /// Panics on an unregistered name (a typo you want loud).
    pub fn signal(&self, name: &str) -> Signal<String> {
        self.inner
            .fields
            .borrow()
            .get(name)
            .unwrap_or_else(|| panic!("guise: unknown form field {name:?}"))
            .clone()
    }

    /// The errors signal (field -> message). `watch` it to re-render on
    /// validation changes.
    pub fn errors(&self) -> Signal<HashMap<&'static str, String>> {
        self.inner.errors.clone()
    }

    pub fn value(&self, cx: &App, name: &str) -> String {
        self.signal(name).get(cx)
    }

    pub fn set(&self, cx: &mut App, name: &str, value: impl Into<String>) {
        self.signal(name).set_if_changed(cx, value.into());
    }

    /// Every field's current value, keyed by name.
    pub fn values(&self, cx: &App) -> FormValues {
        let fields = self.inner.fields.borrow();
        fields
            .iter()
            .map(|(name, signal)| (*name, signal.get(cx)))
            .collect()
    }

    /// The current error for a field, if any.
    pub fn error(&self, cx: &App, name: &str) -> Option<String> {
        self.inner.errors.read(cx).get(name).cloned()
    }

    /// Whether the field has been edited since registration.
    pub fn touched(&self, name: &str) -> bool {
        self.inner.touched.borrow().contains(name)
    }

    /// Run one field's rules. Returns validity and updates the errors signal.
    pub fn validate_field(&self, cx: &mut App, name: &'static str) -> bool {
        let values = self.values(cx);
        let value = values.get(name).cloned().unwrap_or_default();
        let failure = {
            let rules = self.inner.rules.borrow();
            rules
                .get(name)
                .and_then(|list| list.iter().find_map(|rule| rule(&value, &values)))
        };
        let ok = failure.is_none();
        self.inner.errors.update(cx, |errors| match failure {
            Some(message) => {
                errors.insert(name, message);
            }
            None => {
                errors.remove(name);
            }
        });
        ok
    }

    /// Run every field's rules (in registration order). Returns whether all
    /// passed; the errors signal ends up reflecting exactly this pass.
    pub fn validate(&self, cx: &mut App) -> bool {
        let names: Vec<&'static str> = self.inner.order.borrow().clone();
        let mut ok = true;
        for name in names {
            ok &= self.validate_field(cx, name);
        }
        ok
    }

    /// Whether the last validation left no errors.
    pub fn is_valid(&self, cx: &App) -> bool {
        self.inner.errors.read(cx).is_empty()
    }

    /// Validate and hand back the values on success — the submit-handler
    /// one-liner.
    pub fn submit(&self, cx: &mut App) -> Option<FormValues> {
        if self.validate(cx) {
            Some(self.values(cx))
        } else {
            None
        }
    }
}

/// Create a reactive form: a [`Signal`] wrapping the given [`FormState`].
pub fn use_form(cx: &mut App, state: FormState) -> Signal<FormState> {
    Signal::new(cx, state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_and_min_len() {
        let req = validators::required();
        assert!(req("").is_some());
        assert!(req("  ").is_some());
        assert!(req("x").is_none());

        let min = validators::min_len(3);
        assert!(min("ab").is_some());
        assert!(min("abc").is_none());
    }

    #[test]
    fn email_shape() {
        let email = validators::email();
        assert!(email("nope").is_some());
        assert!(email("a@b").is_some());
        assert!(email("a@b.com").is_none());
    }

    #[test]
    fn length_and_numeric_bounds() {
        let max = validators::max_len(3);
        assert!(max("abcd").is_some());
        assert!(max("abc").is_none());

        let num = validators::numeric();
        assert!(num("12.5").is_none());
        assert!(num(" 7 ").is_none());
        assert!(num("seven").is_some());

        let min = validators::min_value(18.0);
        assert!(min("17").is_some());
        assert!(min("18").is_none());
        assert!(min("x").is_some());

        let max = validators::max_value(100.0);
        assert!(max("101").is_some());
        assert!(max("99.9").is_none());
    }

    #[test]
    fn one_of_and_matches() {
        let choice = validators::one_of(&["red", "green", "blue"]);
        assert!(choice("green").is_none());
        assert!(choice("mauve").is_some());

        let upper = validators::matches(|v| v.chars().any(char::is_uppercase), "Need a capital");
        assert!(upper("hello").is_some());
        assert_eq!(upper("Hello"), None);
    }

    #[test]
    fn equals_field_reads_the_other_value() {
        let rule = validators::equals_field("password", "Must match");
        let mut values = FormValues::new();
        values.insert("password", "hunter2".into());
        assert!(rule("hunter2", &values).is_none());
        assert_eq!(rule("hunter3", &values), Some("Must match".to_string()));
        // Missing other field never matches.
        assert!(rule("", &FormValues::new()).is_some());
    }

    #[test]
    fn set_clears_error_then_validate_repopulates() {
        let mut form = FormState::new()
            .field("name", "")
            .validator("name", validators::required());
        assert!(!form.validate());
        assert_eq!(form.error("name"), Some("Required"));

        form.set("name", "Ada");
        // set clears the field's error eagerly.
        assert_eq!(form.error("name"), None);
        assert!(form.validate());
        assert!(form.is_valid());
    }
}
