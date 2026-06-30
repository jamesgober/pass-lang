//! Property-based tests for the section-4 invariants in `dev/DIRECTIVES.md`:
//! determinism, fixpoint termination, fixpoint stability, and the report
//! accounting for every pass that ran.

use pass_lang::{Outcome, Pass, PassError, PassManager};
use proptest::prelude::*;

/// Drop zero entries. Idempotent.
struct DropZeros;
impl Pass<Vec<i64>> for DropZeros {
    fn name(&self) -> &'static str {
        "drop-zeros"
    }
    fn run(&mut self, unit: &mut Vec<i64>) -> Result<Outcome, PassError> {
        let before = unit.len();
        unit.retain(|&value| value != 0);
        Ok(Outcome::from_changed(unit.len() != before))
    }
}

/// Halve every value greater than one. Drives values down toward one, so a
/// pipeline containing it reaches a fixpoint in a bounded number of sweeps.
struct Halve;
impl Pass<Vec<i64>> for Halve {
    fn name(&self) -> &'static str {
        "halve"
    }
    fn run(&mut self, unit: &mut Vec<i64>) -> Result<Outcome, PassError> {
        let mut changed = false;
        for value in unit.iter_mut() {
            if *value > 1 {
                *value /= 2;
                changed = true;
            }
        }
        Ok(Outcome::from_changed(changed))
    }
}

/// Clamp every value to at most one hundred. Idempotent.
struct ClampHigh;
impl Pass<Vec<i64>> for ClampHigh {
    fn name(&self) -> &'static str {
        "clamp-high"
    }
    fn run(&mut self, unit: &mut Vec<i64>) -> Result<Outcome, PassError> {
        let mut changed = false;
        for value in unit.iter_mut() {
            if *value > 100 {
                *value = 100;
                changed = true;
            }
        }
        Ok(Outcome::from_changed(changed))
    }
}

/// A fresh three-pass pipeline. Passes are stateless, so a new manager per run
/// is equivalent to reusing one.
fn pipeline() -> PassManager<Vec<i64>> {
    let mut pm = PassManager::new();
    pm.add(DropZeros).add(Halve).add(ClampHigh);
    pm
}

proptest! {
    /// The same pipeline over the same input always produces the same unit and
    /// an identically shaped report.
    #[test]
    fn determinism(input in prop::collection::vec(-1000i64..1000, 0..64)) {
        let mut a = input.clone();
        let report_a = pipeline().run(&mut a).expect("no pass fails");

        let mut b = input;
        let report_b = pipeline().run(&mut b).expect("no pass fails");

        prop_assert_eq!(a, b);
        prop_assert_eq!(report_a.changes(), report_b.changes());
        prop_assert_eq!(report_a.runs().len(), report_b.runs().len());
        prop_assert_eq!(report_a.converged(), report_b.converged());
    }

    /// `run_to_fixpoint` never exceeds its iteration bound, and a reported
    /// convergence is real: a further sweep changes nothing.
    #[test]
    fn fixpoint_is_bounded_and_stable(
        input in prop::collection::vec(0i64..100_000, 0..48),
        bound in 1usize..40,
    ) {
        let mut unit = input;
        let report = pipeline().run_to_fixpoint(&mut unit, bound).expect("no pass fails");

        prop_assert!(report.iterations() <= bound);

        if report.converged() {
            let settled = unit.clone();
            let confirm = pipeline().run(&mut unit).expect("no pass fails");
            prop_assert_eq!(&unit, &settled);
            prop_assert_eq!(confirm.changes(), 0);
            prop_assert!(confirm.converged());
        }
    }

    /// Every pass that runs is recorded: across a fixpoint run the report holds
    /// exactly `passes * iterations` entries.
    #[test]
    fn report_accounts_for_every_pass(
        input in prop::collection::vec(0i64..100_000, 0..48),
        bound in 0usize..30,
    ) {
        let passes = 3;
        let mut unit = input;
        let report = pipeline().run_to_fixpoint(&mut unit, bound).expect("no pass fails");
        prop_assert_eq!(report.runs().len(), passes * report.iterations());
    }
}
