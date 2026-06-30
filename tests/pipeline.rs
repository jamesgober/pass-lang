//! End-to-end tests of a multi-pass pipeline through the public API.

use pass_lang::{Outcome, Pass, PassError, PassManager};

/// The unit under transformation: a list of stack-machine opcodes encoded as
/// integers, where `0` is a no-op.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Program {
    ops: Vec<i64>,
}

/// Remove no-op (`0`) opcodes.
struct RemoveNops;
impl Pass<Program> for RemoveNops {
    fn name(&self) -> &'static str {
        "remove-nops"
    }
    fn run(&mut self, unit: &mut Program) -> Result<Outcome, PassError> {
        let before = unit.ops.len();
        unit.ops.retain(|&op| op != 0);
        Ok(Outcome::from_changed(unit.ops.len() != before))
    }
}

/// Collapse a run of identical adjacent opcodes into one.
struct DedupAdjacent;
impl Pass<Program> for DedupAdjacent {
    fn name(&self) -> &'static str {
        "dedup-adjacent"
    }
    fn run(&mut self, unit: &mut Program) -> Result<Outcome, PassError> {
        let before = unit.ops.len();
        unit.ops.dedup();
        Ok(Outcome::from_changed(unit.ops.len() != before))
    }
}

/// Reject a program containing a negative opcode.
struct RejectNegative;
impl Pass<Program> for RejectNegative {
    fn name(&self) -> &'static str {
        "reject-negative"
    }
    fn run(&mut self, unit: &mut Program) -> Result<Outcome, PassError> {
        if let Some(bad) = unit.ops.iter().find(|&&op| op < 0) {
            return Err(PassError::new(format!("negative opcode {bad}")));
        }
        Ok(Outcome::Unchanged)
    }
}

#[test]
fn pipeline_runs_in_order_and_reports() {
    let mut pm = PassManager::new();
    pm.add(RemoveNops).add(DedupAdjacent);

    let mut program = Program {
        ops: vec![1, 0, 1, 1, 2, 0, 2],
    };
    let report = pm.run(&mut program).expect("no pass fails");

    // RemoveNops first: [1, 1, 1, 2, 2]; DedupAdjacent then: [1, 2].
    assert_eq!(program.ops, vec![1, 2]);

    let names: Vec<_> = report.runs().iter().map(|r| r.name()).collect();
    assert_eq!(names, vec!["remove-nops", "dedup-adjacent"]);
    assert_eq!(report.changes(), 2);
    assert_eq!(report.iterations(), 1);
}

#[test]
fn pipeline_reaches_fixpoint() {
    // RemoveNops can expose new adjacent duplicates for DedupAdjacent, so a
    // single sweep is not enough: [1,0,1] -> remove-nops -> [1,1] -> needs a
    // second sweep for dedup to collapse it.
    let mut pm = PassManager::new();
    pm.add(RemoveNops).add(DedupAdjacent);

    let mut program = Program {
        ops: vec![1, 0, 1, 0, 1],
    };
    let report = pm.run_to_fixpoint(&mut program, 16).expect("no pass fails");

    assert_eq!(program.ops, vec![1]);
    assert!(report.converged());
    assert!(report.iterations() >= 2);
    // Every sweep runs both passes.
    assert_eq!(report.runs().len(), 2 * report.iterations());
}

#[test]
fn pipeline_stops_at_failing_pass() {
    let mut pm = PassManager::new();
    pm.add(RemoveNops).add(RejectNegative).add(DedupAdjacent);

    let mut program = Program {
        ops: vec![1, 0, -3, 1],
    };
    let err = pm.run(&mut program).expect_err("the program is rejected");

    assert_eq!(err.pass(), "reject-negative");
    assert_eq!(err.message(), "negative opcode -3");
    // RemoveNops ran (dropped the 0); DedupAdjacent never ran.
    assert_eq!(program.ops, vec![1, -3, 1]);
}

#[test]
fn empty_pipeline_is_a_converged_noop() {
    let mut pm = PassManager::<Program>::new();
    let mut program = Program { ops: vec![1, 2, 3] };
    let report = pm.run(&mut program).expect("nothing to run");

    assert_eq!(program.ops, vec![1, 2, 3]);
    assert!(report.runs().is_empty());
    assert!(report.converged());
}
