//! The ordered pass pipeline and the loops that run it.

use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::error::PassError;
use crate::pass::Pass;
use crate::report::Report;

/// An ordered pipeline of passes over a unit of type `T`.
///
/// A `PassManager` holds passes in the order they were registered and runs them
/// over a unit. It is the scheduler, and scheduling is its only job: it never
/// reads or rewrites the unit itself — it only calls the passes, in order, and
/// records what they report. Registering a pass with [`add`](Self::add) is the
/// plugin seam: any crate can contribute a transform by implementing
/// [`Pass<T>`](crate::Pass) and adding it here.
///
/// Two ways to run a pipeline:
///
/// - [`run`](Self::run) makes a single sweep — every pass once, in order. This
///   is the common case.
/// - [`run_to_fixpoint`](Self::run_to_fixpoint) repeats the sweep until a full
///   pass over the pipeline changes nothing, or an iteration bound is reached.
///   Use it when passes feed one another and a transform can expose further work
///   (constant folding exposing dead code, which exposes more folding).
///
/// The manager is single-threaded by design: a pipeline is an inherently ordered
/// sequence of mutations, so it carries no atomic overhead. Dispatch to a pass is
/// one indirect call amortised over the whole unit that pass rewrites.
///
/// # Examples
///
/// ```
/// use pass_lang::{Outcome, Pass, PassError, PassManager};
///
/// struct Double;
/// impl Pass<i64> for Double {
///     fn name(&self) -> &'static str { "double" }
///     fn run(&mut self, u: &mut i64) -> Result<Outcome, PassError> {
///         *u *= 2;
///         Ok(Outcome::Changed)
///     }
/// }
///
/// let mut pm = PassManager::new();
/// pm.add(Double).add(Double);
///
/// let mut unit = 3;
/// let report = pm.run(&mut unit).unwrap();
///
/// assert_eq!(unit, 12);            // 3 -> 6 -> 12
/// assert_eq!(report.runs().len(), 2);
/// ```
pub struct PassManager<T> {
    passes: Vec<Box<dyn Pass<T>>>,
}

impl<T> PassManager<T> {
    /// Create an empty pipeline.
    ///
    /// # Examples
    ///
    /// ```
    /// use pass_lang::PassManager;
    ///
    /// let pm = PassManager::<i64>::new();
    /// assert!(pm.is_empty());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self { passes: Vec::new() }
    }

    /// Register a pass at the end of the pipeline.
    ///
    /// Returns `&mut Self` so registrations can be chained. This is the plugin
    /// seam: the pass must be `'static`, and it runs after every pass already
    /// registered.
    ///
    /// # Examples
    ///
    /// ```
    /// use pass_lang::{Outcome, Pass, PassError, PassManager};
    ///
    /// struct Noop(&'static str);
    /// impl Pass<i64> for Noop {
    ///     fn name(&self) -> &'static str { self.0 }
    ///     fn run(&mut self, _: &mut i64) -> Result<Outcome, PassError> { Ok(Outcome::Unchanged) }
    /// }
    ///
    /// let mut pm = PassManager::new();
    /// pm.add(Noop("first")).add(Noop("second"));
    /// assert_eq!(pm.len(), 2);
    /// ```
    pub fn add(&mut self, pass: impl Pass<T> + 'static) -> &mut Self {
        self.passes.push(Box::new(pass));
        self
    }

    /// The number of registered passes.
    #[must_use]
    #[inline]
    pub fn len(&self) -> usize {
        self.passes.len()
    }

    /// Whether the pipeline has no passes.
    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.passes.is_empty()
    }

    /// Run every pass once, in registration order.
    ///
    /// Each pass transforms `unit` in place; the returned [`Report`] lists every
    /// pass that ran with the [`Outcome`](crate::Outcome) it reported. If a pass
    /// returns a [`PassError`], the pipeline stops at that pass — later passes do
    /// not run — and the error is returned with the failing pass's name stamped
    /// in.
    ///
    /// The report's [`converged`](crate::Report::converged) is `true` when the
    /// sweep changed nothing (the unit is already at a fixpoint for this
    /// pipeline), and [`iterations`](crate::Report::iterations) is always `1`.
    ///
    /// # Errors
    ///
    /// Returns the [`PassError`] of the first pass that fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use pass_lang::{Outcome, Pass, PassError, PassManager};
    ///
    /// struct FailIfNegative;
    /// impl Pass<i64> for FailIfNegative {
    ///     fn name(&self) -> &'static str { "fail-if-negative" }
    ///     fn run(&mut self, u: &mut i64) -> Result<Outcome, PassError> {
    ///         if *u < 0 { return Err(PassError::new("unit went negative")); }
    ///         Ok(Outcome::Unchanged)
    ///     }
    /// }
    ///
    /// let mut pm = PassManager::new();
    /// pm.add(FailIfNegative);
    ///
    /// let mut unit = -1;
    /// let err = pm.run(&mut unit).unwrap_err();
    /// assert_eq!(err.pass(), "fail-if-negative");
    /// assert_eq!(err.message(), "unit went negative");
    /// ```
    pub fn run(&mut self, unit: &mut T) -> Result<Report, PassError> {
        let mut report = Report::with_capacity(self.passes.len());
        let mut changed = false;
        for pass in &mut self.passes {
            let outcome = pass.run(unit).map_err(|e| e.in_pass(pass.name()))?;
            changed |= outcome.changed();
            report.record(pass.name(), outcome);
        }
        report.finalize(1, !changed);
        Ok(report)
    }

    /// Repeat the pipeline until it settles, or until `max_iters` sweeps run.
    ///
    /// Each sweep runs every pass once, in order. After a sweep in which no pass
    /// reported [`Changed`](crate::Outcome::Changed), the unit is at a fixpoint
    /// and the loop stops with [`converged`](crate::Report::converged) `true`. If
    /// `max_iters` sweeps run while the unit is still changing, the loop stops
    /// with `converged` `false` — the bound guarantees termination even if a pass
    /// oscillates. Passing `max_iters == 0` performs no sweeps.
    ///
    /// The [`Report`] accumulates every pass execution across every sweep, so its
    /// [`runs`](crate::Report::runs) length is the sum over sweeps (a short final
    /// sweep on error aside).
    ///
    /// # Errors
    ///
    /// Returns the [`PassError`] of the first pass that fails, on whichever sweep
    /// it fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use pass_lang::{Outcome, Pass, PassError, PassManager};
    ///
    /// // Halve toward 1; idempotent once it reaches 1.
    /// struct Halve;
    /// impl Pass<i64> for Halve {
    ///     fn name(&self) -> &'static str { "halve" }
    ///     fn run(&mut self, u: &mut i64) -> Result<Outcome, PassError> {
    ///         if *u <= 1 { return Ok(Outcome::Unchanged); }
    ///         *u /= 2;
    ///         Ok(Outcome::Changed)
    ///     }
    /// }
    ///
    /// let mut pm = PassManager::new();
    /// pm.add(Halve);
    ///
    /// let mut unit = 16;
    /// let report = pm.run_to_fixpoint(&mut unit, 32).unwrap();
    /// assert_eq!(unit, 1);
    /// assert!(report.converged());
    /// ```
    pub fn run_to_fixpoint(&mut self, unit: &mut T, max_iters: usize) -> Result<Report, PassError> {
        let mut report = Report::with_capacity(self.passes.len());
        let mut iterations = 0;
        let mut converged = false;
        while iterations < max_iters {
            iterations += 1;
            let mut changed = false;
            for pass in &mut self.passes {
                let outcome = pass.run(unit).map_err(|e| e.in_pass(pass.name()))?;
                changed |= outcome.changed();
                report.record(pass.name(), outcome);
            }
            if !changed {
                converged = true;
                break;
            }
        }
        report.finalize(iterations, converged);
        Ok(report)
    }
}

impl<T> Default for PassManager<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::pass::Outcome;
    use alloc::vec;
    use alloc::vec::Vec;

    /// Records the order in which passes ran, by pushing its tag onto the unit.
    struct Tag(&'static str);
    impl Pass<Vec<&'static str>> for Tag {
        fn name(&self) -> &'static str {
            self.0
        }
        fn run(&mut self, unit: &mut Vec<&'static str>) -> Result<Outcome, PassError> {
            unit.push(self.0);
            Ok(Outcome::Changed)
        }
    }

    /// Subtracts one while positive; reports `Unchanged` at zero (idempotent).
    struct Decrement;
    impl Pass<i64> for Decrement {
        fn name(&self) -> &'static str {
            "decrement"
        }
        fn run(&mut self, unit: &mut i64) -> Result<Outcome, PassError> {
            if *unit <= 0 {
                return Ok(Outcome::Unchanged);
            }
            *unit -= 1;
            Ok(Outcome::Changed)
        }
    }

    /// Always fails.
    struct Boom;
    impl Pass<i64> for Boom {
        fn name(&self) -> &'static str {
            "boom"
        }
        fn run(&mut self, _unit: &mut i64) -> Result<Outcome, PassError> {
            Err(PassError::new("intentional"))
        }
    }

    /// Flips a value between 0 and 1 every run — never converges.
    struct Flip;
    impl Pass<i64> for Flip {
        fn name(&self) -> &'static str {
            "flip"
        }
        fn run(&mut self, unit: &mut i64) -> Result<Outcome, PassError> {
            *unit = 1 - *unit;
            Ok(Outcome::Changed)
        }
    }

    #[test]
    fn test_new_is_empty() {
        let pm = PassManager::<i64>::new();
        assert!(pm.is_empty());
        assert_eq!(pm.len(), 0);
    }

    #[test]
    fn test_default_matches_new() {
        let pm: PassManager<i64> = PassManager::default();
        assert!(pm.is_empty());
    }

    #[test]
    fn test_add_counts_passes() {
        let mut pm = PassManager::new();
        pm.add(Decrement).add(Decrement);
        assert_eq!(pm.len(), 2);
    }

    #[test]
    fn test_run_empty_pipeline_is_converged_noop() {
        let mut pm = PassManager::<i64>::new();
        let mut unit = 7;
        let report = pm.run(&mut unit).unwrap();
        assert_eq!(unit, 7);
        assert!(report.runs().is_empty());
        assert_eq!(report.iterations(), 1);
        assert!(report.converged());
    }

    #[test]
    fn test_run_preserves_registration_order() {
        let mut pm = PassManager::new();
        pm.add(Tag("a")).add(Tag("b")).add(Tag("c"));
        let mut unit: Vec<&'static str> = vec![];
        let report = pm.run(&mut unit).unwrap();
        assert_eq!(unit, vec!["a", "b", "c"]);
        let names: Vec<_> = report.runs().iter().map(|r| r.name()).collect();
        assert_eq!(names, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_run_single_sweep_converged_when_unchanged() {
        let mut pm = PassManager::new();
        pm.add(Decrement);
        let mut unit = 0; // already at fixpoint
        let report = pm.run(&mut unit).unwrap();
        assert!(report.converged());
        assert_eq!(report.changes(), 0);
    }

    #[test]
    fn test_run_single_sweep_not_converged_when_changed() {
        let mut pm = PassManager::new();
        pm.add(Decrement);
        let mut unit = 5;
        let report = pm.run(&mut unit).unwrap();
        assert_eq!(unit, 4);
        assert!(!report.converged());
        assert_eq!(report.changes(), 1);
    }

    #[test]
    fn test_run_to_fixpoint_settles() {
        let mut pm = PassManager::new();
        pm.add(Decrement);
        let mut unit = 4;
        let report = pm.run_to_fixpoint(&mut unit, 100).unwrap();
        assert_eq!(unit, 0);
        assert!(report.converged());
        // Four sweeps decrement (4->3->2->1->0), a fifth confirms no change.
        assert_eq!(report.iterations(), 5);
        assert_eq!(report.changes(), 4);
        assert_eq!(report.runs().len(), 5);
    }

    #[test]
    fn test_run_to_fixpoint_respects_bound_when_oscillating() {
        let mut pm = PassManager::new();
        pm.add(Flip);
        let mut unit = 0;
        let report = pm.run_to_fixpoint(&mut unit, 10).unwrap();
        assert_eq!(report.iterations(), 10);
        assert!(!report.converged());
        assert_eq!(report.runs().len(), 10);
    }

    #[test]
    fn test_run_to_fixpoint_zero_iters_does_nothing() {
        let mut pm = PassManager::new();
        pm.add(Decrement);
        let mut unit = 9;
        let report = pm.run_to_fixpoint(&mut unit, 0).unwrap();
        assert_eq!(unit, 9);
        assert_eq!(report.iterations(), 0);
        assert!(!report.converged());
        assert!(report.runs().is_empty());
    }

    #[test]
    fn test_run_halts_at_failing_pass_and_names_it() {
        let mut pm = PassManager::new();
        pm.add(Decrement).add(Boom).add(Decrement);
        let mut unit = 5;
        let err = pm.run(&mut unit).unwrap_err();
        assert_eq!(err.pass(), "boom");
        assert_eq!(err.message(), "intentional");
        // The first Decrement ran (5 -> 4); the pass after Boom did not.
        assert_eq!(unit, 4);
    }

    #[test]
    fn test_run_to_fixpoint_propagates_failure() {
        let mut pm = PassManager::new();
        pm.add(Boom);
        let mut unit = 0;
        let err = pm.run_to_fixpoint(&mut unit, 5).unwrap_err();
        assert_eq!(err.pass(), "boom");
    }
}
