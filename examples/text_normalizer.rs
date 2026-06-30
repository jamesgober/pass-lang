//! A text-normalization pipeline, to show that `pass-lang` is generic over the
//! unit a pass rewrites — here a [`String`], not an IR.
//!
//! The same machinery — registration order, fixpoint iteration, the run report —
//! drives passes that collapse runs of spaces, strip trailing whitespace, and
//! lowercase the text. Collapsing spaces one pair at a time needs more than one
//! sweep, so `run_to_fixpoint` keeps going until the string stops changing.
//!
//! Run it with:
//!
//! ```text
//! cargo run --example text_normalizer
//! ```

use pass_lang::{Outcome, Pass, PassError, PassManager};

/// Replace each `"  "` (two spaces) with a single space. One pass halves runs of
/// spaces; a fixpoint loop collapses them completely.
struct CollapseSpaces;

impl Pass<String> for CollapseSpaces {
    fn name(&self) -> &'static str {
        "collapse-spaces"
    }

    fn run(&mut self, unit: &mut String) -> Result<Outcome, PassError> {
        if !unit.contains("  ") {
            return Ok(Outcome::Unchanged);
        }
        *unit = unit.replace("  ", " ");
        Ok(Outcome::Changed)
    }
}

/// Strip trailing spaces from the end of the string.
struct TrimTrailing;

impl Pass<String> for TrimTrailing {
    fn name(&self) -> &'static str {
        "trim-trailing"
    }

    fn run(&mut self, unit: &mut String) -> Result<Outcome, PassError> {
        let trimmed = unit.trim_end();
        if trimmed.len() == unit.len() {
            return Ok(Outcome::Unchanged);
        }
        unit.truncate(trimmed.len());
        Ok(Outcome::Changed)
    }
}

/// Lowercase any ASCII uppercase letters in place.
struct Lowercase;

impl Pass<String> for Lowercase {
    fn name(&self) -> &'static str {
        "lowercase"
    }

    fn run(&mut self, unit: &mut String) -> Result<Outcome, PassError> {
        if !unit.bytes().any(|b| b.is_ascii_uppercase()) {
            return Ok(Outcome::Unchanged);
        }
        unit.make_ascii_lowercase();
        Ok(Outcome::Changed)
    }
}

fn main() {
    let mut text = String::from("Hello      World   ");
    println!("before: {text:?}");

    let mut pm = PassManager::new();
    pm.add(CollapseSpaces).add(TrimTrailing).add(Lowercase);

    let report = pm
        .run_to_fixpoint(&mut text, 16)
        .expect("none of these passes can fail");

    println!("after:  {text:?}");
    println!(
        "sweeps={} changes={} converged={}",
        report.iterations(),
        report.changes(),
        report.converged(),
    );

    assert_eq!(text, "hello world");
    assert!(report.converged());
}
