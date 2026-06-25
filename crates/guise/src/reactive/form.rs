//! A small form-state layer: values, validators, and errors keyed by field name.
//!
//! Pure and unit-testable on its own ([`FormState`]); make it reactive by
//! holding it in a [`Signal`](super::Signal) and mutating through
//! `signal.update(cx, |form| form.set(..))`. [`use_form`] is the shorthand.
//!
//! ```ignore
//! let form = use_form(cx, FormState::new()
//!     .field("email", "")
//!     .validator("email", validators::email()));
//! // later, in a handler:
//! form.update(cx, |f| { f.set("email", "a@b.com"); f.validate(); });
//! ```

use std::collections::HashMap;

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

/// Built-in validators.
pub mod validators {
    use super::Validator;

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
