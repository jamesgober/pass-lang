//! The record a [`PassManager`](crate::PassManager) run produces.

use alloc::vec::Vec;

use crate::pass::Outcome;

/// One entry in a [`Report`]: a pass that ran and what it did.
///
/// Entries appear in execution order. When a pipeline is run to a fixpoint, a
/// pass that runs on three sweeps contributes three entries, one per sweep.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct PassRun {
    name: &'static str,
    outcome: Outcome,
}

impl PassRun {
    /// The [`name`](crate::Pass::name) of the pass that ran.
    ///
    /// # Examples
    ///
    /// ```
    /// use pass_lang::{Outcome, Pass, PassError, PassManager};
    ///
    /// struct Touch;
    /// impl Pass<i64> for Touch {
    ///     fn name(&self) -> &'static str { "touch" }
    ///     fn run(&mut self, u: &mut i64) -> Result<Outcome, PassError> {
    ///         *u += 1;
    ///         Ok(Outcome::Changed)
    ///     }
    /// }
    ///
    /// let mut pm = PassManager::new();
    /// pm.add(Touch);
    /// let mut unit = 0;
    /// let report = pm.run(&mut unit).unwrap();
    /// assert_eq!(report.runs()[0].name(), "touch");
    /// ```
    #[must_use]
    #[inline]
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// What the pass reported on that run.
    #[must_use]
    #[inline]
    pub fn outcome(&self) -> Outcome {
        self.outcome
    }
}

/// A record of one [`PassManager`](crate::PassManager) run.
///
/// A `Report` is the observability hook: it lists every pass that ran, in order,
/// with the [`Outcome`] each reported, and summarizes the run — how many sweeps
/// it took, whether it reached a fixpoint, and how many passes changed the unit.
/// It is produced by [`PassManager::run`](crate::PassManager::run) and
/// [`PassManager::run_to_fixpoint`](crate::PassManager::run_to_fixpoint); you
/// read it, you do not build it.
///
/// # Examples
///
/// ```
/// use pass_lang::{Outcome, Pass, PassError, PassManager};
///
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
/// let mut unit = 8;
/// let report = pm.run_to_fixpoint(&mut unit, 16).unwrap();
///
/// assert_eq!(unit, 1);                 // 8 -> 4 -> 2 -> 1
/// assert!(report.converged());         // a final sweep changed nothing
/// assert_eq!(report.iterations(), 4);  // three halving sweeps + one confirming sweep
/// assert_eq!(report.changes(), 3);     // three sweeps reported Changed
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Report {
    runs: Vec<PassRun>,
    iterations: usize,
    converged: bool,
}

impl Report {
    /// Start a report sized for at least `capacity` pass executions, so a single
    /// sweep records without reallocating its run list.
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            runs: Vec::with_capacity(capacity),
            iterations: 0,
            converged: false,
        }
    }

    /// Append the outcome of one pass execution.
    #[inline]
    pub(crate) fn record(&mut self, name: &'static str, outcome: Outcome) {
        self.runs.push(PassRun { name, outcome });
    }

    /// Stamp the aggregate fields once the run is complete.
    pub(crate) fn finalize(&mut self, iterations: usize, converged: bool) {
        self.iterations = iterations;
        self.converged = converged;
    }

    /// Every pass execution, in the order it happened.
    ///
    /// # Examples
    ///
    /// ```
    /// use pass_lang::{Outcome, Pass, PassError, PassManager};
    ///
    /// struct A;
    /// impl Pass<i64> for A {
    ///     fn name(&self) -> &'static str { "a" }
    ///     fn run(&mut self, _: &mut i64) -> Result<Outcome, PassError> { Ok(Outcome::Unchanged) }
    /// }
    /// struct B;
    /// impl Pass<i64> for B {
    ///     fn name(&self) -> &'static str { "b" }
    ///     fn run(&mut self, _: &mut i64) -> Result<Outcome, PassError> { Ok(Outcome::Unchanged) }
    /// }
    ///
    /// let mut pm = PassManager::new();
    /// pm.add(A).add(B);
    /// let mut unit = 0;
    /// let report = pm.run(&mut unit).unwrap();
    ///
    /// let names: Vec<_> = report.runs().iter().map(|r| r.name()).collect();
    /// assert_eq!(names, ["a", "b"]);
    /// ```
    #[must_use]
    #[inline]
    pub fn runs(&self) -> &[PassRun] {
        &self.runs
    }

    /// How many pass executions reported [`Outcome::Changed`].
    ///
    /// # Examples
    ///
    /// ```
    /// use pass_lang::{Outcome, Pass, PassError, PassManager};
    ///
    /// struct Inc;
    /// impl Pass<i64> for Inc {
    ///     fn name(&self) -> &'static str { "inc" }
    ///     fn run(&mut self, u: &mut i64) -> Result<Outcome, PassError> {
    ///         *u += 1;
    ///         Ok(Outcome::Changed)
    ///     }
    /// }
    ///
    /// let mut pm = PassManager::new();
    /// pm.add(Inc);
    /// let mut unit = 0;
    /// assert_eq!(pm.run(&mut unit).unwrap().changes(), 1);
    /// ```
    #[must_use]
    pub fn changes(&self) -> usize {
        self.runs.iter().filter(|r| r.outcome.changed()).count()
    }

    /// The number of full sweeps over the pipeline.
    ///
    /// [`PassManager::run`](crate::PassManager::run) always reports `1`.
    /// [`PassManager::run_to_fixpoint`](crate::PassManager::run_to_fixpoint)
    /// reports how many sweeps it actually performed.
    #[must_use]
    #[inline]
    pub fn iterations(&self) -> usize {
        self.iterations
    }

    /// Whether the final sweep made no change — the pipeline reached a fixpoint.
    ///
    /// For [`run`](crate::PassManager::run) this is `true` when the single sweep
    /// changed nothing. For
    /// [`run_to_fixpoint`](crate::PassManager::run_to_fixpoint) it is `true` when
    /// a sweep settled before the iteration bound was hit, and `false` when the
    /// bound was reached with the unit still changing.
    #[must_use]
    #[inline]
    pub fn converged(&self) -> bool {
        self.converged
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_report_defaults() {
        let report = Report::with_capacity(0);
        assert!(report.runs().is_empty());
        assert_eq!(report.changes(), 0);
        assert_eq!(report.iterations(), 0);
        assert!(!report.converged());
    }

    #[test]
    fn test_record_appends_in_order() {
        let mut report = Report::with_capacity(0);
        report.record("a", Outcome::Changed);
        report.record("b", Outcome::Unchanged);
        let names: alloc::vec::Vec<_> = report.runs().iter().map(PassRun::name).collect();
        assert_eq!(names, ["a", "b"]);
    }

    #[test]
    fn test_changes_counts_only_changed() {
        let mut report = Report::with_capacity(0);
        report.record("a", Outcome::Changed);
        report.record("b", Outcome::Unchanged);
        report.record("c", Outcome::Changed);
        assert_eq!(report.changes(), 2);
    }

    #[test]
    fn test_finalize_sets_aggregate() {
        let mut report = Report::with_capacity(0);
        report.finalize(3, true);
        assert_eq!(report.iterations(), 3);
        assert!(report.converged());
    }

    #[test]
    fn test_pass_run_accessors() {
        let mut report = Report::with_capacity(0);
        report.record("only", Outcome::Changed);
        let run = report.runs()[0];
        assert_eq!(run.name(), "only");
        assert_eq!(run.outcome(), Outcome::Changed);
    }
}
