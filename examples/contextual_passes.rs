//! Passes that share context. The unit bundles the program being transformed with
//! a diagnostics log, so each pass can both rewrite the code and record what it
//! did.
//!
//! This is how a real pipeline threads a diagnostics sink — or an analysis cache,
//! or a symbol table — through every pass without reaching for a global: make the
//! context part of the unit. The manager stays generic; the passes share whatever
//! the unit holds.
//!
//! Run it with:
//!
//! ```text
//! cargo run --example contextual_passes
//! ```

use pass_lang::{Outcome, Pass, PassError, PassManager};

/// The unit: a list of integer opcodes plus a log the passes append to.
struct Session {
    code: Vec<i64>,
    log: Vec<String>,
}

/// Remove no-op (`0`) opcodes, noting how many were dropped.
struct StripNops;
impl Pass<Session> for StripNops {
    fn name(&self) -> &'static str {
        "strip-nops"
    }
    fn run(&mut self, unit: &mut Session) -> Result<Outcome, PassError> {
        let before = unit.code.len();
        unit.code.retain(|&op| op != 0);
        let removed = before - unit.code.len();
        if removed == 0 {
            return Ok(Outcome::Unchanged);
        }
        unit.log
            .push(format!("strip-nops: removed {removed} no-op(s)"));
        Ok(Outcome::Changed)
    }
}

/// Reject the program if any opcode is outside the valid `0..=9` range.
struct Validate;
impl Pass<Session> for Validate {
    fn name(&self) -> &'static str {
        "validate"
    }
    fn run(&mut self, unit: &mut Session) -> Result<Outcome, PassError> {
        if let Some(&bad) = unit.code.iter().find(|&&op| !(0..=9).contains(&op)) {
            return Err(PassError::new(format!("opcode {bad} out of range 0..=9")));
        }
        unit.log
            .push(format!("validate: {} opcode(s) ok", unit.code.len()));
        Ok(Outcome::Unchanged)
    }
}

fn main() {
    let mut session = Session {
        code: vec![1, 0, 5, 0, 3],
        log: Vec::new(),
    };

    let mut pm = PassManager::new();
    pm.add(StripNops).add(Validate);

    let report = pm.run(&mut session).expect("the program is valid");

    println!("code: {:?}", session.code);
    println!("log:");
    for line in &session.log {
        println!("  {line}");
    }
    println!(
        "({} passes ran, {} changed the unit)",
        report.runs().len(),
        report.changes(),
    );
    assert_eq!(session.code, vec![1, 5, 3]);

    println!();

    // A program with an out-of-range opcode is rejected by `validate`.
    let mut invalid = Session {
        code: vec![1, 42],
        log: Vec::new(),
    };
    let mut checker = PassManager::new();
    checker.add(StripNops).add(Validate);
    if let Err(err) = checker.run(&mut invalid) {
        println!("rejected by `{}`: {}", err.pass(), err.message());
        assert_eq!(err.pass(), "validate");
    }
}
