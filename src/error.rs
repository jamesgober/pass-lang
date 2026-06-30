//! The error a pass returns when it cannot complete.

use alloc::borrow::Cow;
use core::fmt;

/// The error a [`Pass`](crate::Pass) returns when it cannot complete its work.
///
/// A pass produces a `PassError` from inside [`Pass::run`](crate::Pass::run)
/// when it hits a condition it cannot transform past — a construct it does not
/// support, an invariant the input violates, an arithmetic operation that would
/// overflow. The error carries a human-readable [`message`](Self::message)
/// explaining what went wrong; the [`PassManager`](crate::PassManager) then
/// stamps in the [`name`](Self::pass) of the pass that failed before returning
/// it, so the caller always knows *which* pass halted the pipeline and *why*.
///
/// The reason is stored as a [`Cow<'static, str>`](alloc::borrow::Cow): a static
/// message (`"unsupported construct"`) costs no allocation, while a computed one
/// (`format!("overflow folding {a} * {b}")`) is owned only when it is built.
///
/// # Examples
///
/// Failing a pass with a static reason:
///
/// ```
/// use pass_lang::{Outcome, Pass, PassError};
///
/// struct RejectEmpty;
/// impl Pass<Vec<i64>> for RejectEmpty {
///     fn name(&self) -> &'static str { "reject-empty" }
///     fn run(&mut self, unit: &mut Vec<i64>) -> Result<Outcome, PassError> {
///         if unit.is_empty() {
///             return Err(PassError::new("the unit is empty"));
///         }
///         Ok(Outcome::Unchanged)
///     }
/// }
/// ```
///
/// Failing with a computed reason:
///
/// ```
/// use pass_lang::PassError;
///
/// let (a, b) = (i64::MAX, 2);
/// let err = PassError::new(format!("overflow folding {a} * {b}"));
/// assert_eq!(err.message(), "overflow folding 9223372036854775807 * 2");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PassError {
    pass: &'static str,
    message: Cow<'static, str>,
}

impl PassError {
    /// Create an error describing why a pass could not complete.
    ///
    /// Call this from inside [`Pass::run`](crate::Pass::run). The `message`
    /// accepts a string literal (no allocation) or an owned [`String`] (a
    /// computed reason). The failing pass's name is filled in by the
    /// [`PassManager`](crate::PassManager) — you do not need to repeat it.
    ///
    /// # Examples
    ///
    /// ```
    /// use pass_lang::PassError;
    ///
    /// let from_literal = PassError::new("division by zero");
    /// let from_owned = PassError::new(String::from("division by zero"));
    /// assert_eq!(from_literal, from_owned);
    /// ```
    #[must_use]
    pub fn new(message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            pass: "",
            message: message.into(),
        }
    }

    /// The name of the pass that failed, or `""` if the error has not yet been
    /// returned through a [`PassManager`](crate::PassManager).
    ///
    /// # Examples
    ///
    /// ```
    /// use pass_lang::PassError;
    ///
    /// // Before the manager stamps it in, the pass name is empty.
    /// assert_eq!(PassError::new("boom").pass(), "");
    /// ```
    #[must_use]
    pub fn pass(&self) -> &str {
        self.pass
    }

    /// The reason the pass could not complete.
    ///
    /// # Examples
    ///
    /// ```
    /// use pass_lang::PassError;
    ///
    /// assert_eq!(PassError::new("unsupported node").message(), "unsupported node");
    /// ```
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Stamp the failing pass's name into the error. Called by the
    /// [`PassManager`](crate::PassManager) as it propagates the failure.
    pub(crate) fn in_pass(mut self, name: &'static str) -> Self {
        self.pass = name;
        self
    }
}

impl fmt::Display for PassError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.pass.is_empty() {
            write!(f, "pass failed: {}", self.message)
        } else {
            write!(f, "pass `{}` failed: {}", self.pass, self.message)
        }
    }
}

impl core::error::Error for PassError {}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use alloc::string::{String, ToString};

    #[test]
    fn test_new_accepts_literal_and_owned() {
        assert_eq!(PassError::new("x"), PassError::new(String::from("x")));
    }

    #[test]
    fn test_message_is_preserved() {
        assert_eq!(
            PassError::new("division by zero").message(),
            "division by zero"
        );
    }

    #[test]
    fn test_pass_is_empty_before_stamping() {
        assert_eq!(PassError::new("boom").pass(), "");
    }

    #[test]
    fn test_in_pass_stamps_name() {
        let err = PassError::new("boom").in_pass("const-fold");
        assert_eq!(err.pass(), "const-fold");
        assert_eq!(err.message(), "boom");
    }

    #[test]
    fn test_display_without_pass_name() {
        assert_eq!(PassError::new("boom").to_string(), "pass failed: boom");
    }

    #[test]
    fn test_display_with_pass_name() {
        let err = PassError::new("boom").in_pass("simplify");
        assert_eq!(err.to_string(), "pass `simplify` failed: boom");
    }

    #[test]
    fn test_error_trait_is_implemented() {
        fn assert_error<E: core::error::Error>(_: &E) {}
        assert_error(&PassError::new("boom"));
    }
}
