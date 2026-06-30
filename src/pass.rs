//! The plugin-seam trait and the per-run outcome it reports.

use crate::error::PassError;

/// Whether a pass changed the unit on a given run.
///
/// A pass returns [`Outcome::Changed`] when it modified the unit and
/// [`Outcome::Unchanged`] when it left it untouched. The
/// [`PassManager`](crate::PassManager) uses this signal to decide whether a
/// fixpoint loop has settled (a full sweep with no `Changed` is a fixpoint) and
/// to build the [`Report`](crate::Report). Reporting `Unchanged` after mutating
/// the unit breaks fixpoint termination, so the value is taken at its word — keep
/// it honest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum Outcome {
    /// The pass modified the unit.
    Changed,
    /// The pass left the unit unchanged.
    Unchanged,
}

impl Outcome {
    /// True if this is [`Outcome::Changed`].
    ///
    /// # Examples
    ///
    /// ```
    /// use pass_lang::Outcome;
    ///
    /// assert!(Outcome::Changed.changed());
    /// assert!(!Outcome::Unchanged.changed());
    /// ```
    #[must_use]
    #[inline]
    pub fn changed(self) -> bool {
        matches!(self, Outcome::Changed)
    }

    /// Build an outcome from a "did it change?" flag.
    ///
    /// The common shape of a [`Pass::run`] body: compute the transform, then
    /// report whether it actually altered the unit.
    ///
    /// # Examples
    ///
    /// ```
    /// use pass_lang::Outcome;
    ///
    /// assert_eq!(Outcome::from_changed(true), Outcome::Changed);
    /// assert_eq!(Outcome::from_changed(false), Outcome::Unchanged);
    /// ```
    #[must_use]
    #[inline]
    pub fn from_changed(changed: bool) -> Self {
        if changed {
            Outcome::Changed
        } else {
            Outcome::Unchanged
        }
    }
}

/// A single transform or analysis over a unit of type `T` — the plugin seam.
///
/// Implement this trait to define a pass. The unit `T` is whatever the pass
/// rewrites: an intermediate representation, a single function, a module, an
/// abstract syntax tree, or a struct bundling an IR with the diagnostics and
/// analysis state the pass needs. The manager is generic over `T` and never
/// inspects it — a pass is the only thing trusted to read or mutate the unit.
///
/// A pass is registered with [`PassManager::add`](crate::PassManager::add) and
/// run in registration order. It must be `'static`: it may own state across
/// runs, but it may not borrow from outside the manager.
///
/// # Contract
///
/// - [`name`](Self::name) returns a stable, static identifier used in the
///   [`Report`](crate::Report) and in error context. It must not change between
///   runs of the same pass.
/// - [`run`](Self::run) transforms `unit` in place and reports an
///   [`Outcome`]: [`Changed`](Outcome::Changed) only when the unit was actually
///   modified, so a fixpoint loop can terminate. A pass that cannot proceed
///   returns a [`PassError`] instead of panicking.
///
/// # Examples
///
/// A pass that drops zero entries from a list, reporting whether it removed any:
///
/// ```
/// use pass_lang::{Outcome, Pass, PassError};
///
/// struct DropZeros;
///
/// impl Pass<Vec<i64>> for DropZeros {
///     fn name(&self) -> &'static str {
///         "drop-zeros"
///     }
///
///     fn run(&mut self, unit: &mut Vec<i64>) -> Result<Outcome, PassError> {
///         let before = unit.len();
///         unit.retain(|&x| x != 0);
///         Ok(Outcome::from_changed(unit.len() != before))
///     }
/// }
///
/// let mut unit = vec![0, 1, 0, 2];
/// assert_eq!(DropZeros.run(&mut unit).unwrap(), Outcome::Changed);
/// assert_eq!(unit, vec![1, 2]);
/// // Running again changes nothing.
/// assert_eq!(DropZeros.run(&mut unit).unwrap(), Outcome::Unchanged);
/// ```
pub trait Pass<T> {
    /// A stable, static name for this pass, used in reports and error context.
    fn name(&self) -> &'static str;

    /// Run the pass over `unit`, transforming it in place.
    ///
    /// Return [`Outcome::Changed`] if and only if the unit was modified, so the
    /// manager's fixpoint loop can tell when the pipeline has settled. Return a
    /// [`PassError`] — never a panic — if the pass cannot proceed; the manager
    /// stops the pipeline and reports which pass failed.
    fn run(&mut self, unit: &mut T) -> Result<Outcome, PassError>;
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_changed_reports_variant() {
        assert!(Outcome::Changed.changed());
        assert!(!Outcome::Unchanged.changed());
    }

    #[test]
    fn test_from_changed_maps_bool() {
        assert_eq!(Outcome::from_changed(true), Outcome::Changed);
        assert_eq!(Outcome::from_changed(false), Outcome::Unchanged);
    }

    struct Noop;
    impl Pass<i64> for Noop {
        fn name(&self) -> &'static str {
            "noop"
        }
        fn run(&mut self, _unit: &mut i64) -> Result<Outcome, PassError> {
            Ok(Outcome::Unchanged)
        }
    }

    #[test]
    fn test_trait_is_object_safe() {
        // If `Pass<T>` were not object-safe this would not compile.
        let _boxed: alloc::boxed::Box<dyn Pass<i64>> = alloc::boxed::Box::new(Noop);
    }
}
