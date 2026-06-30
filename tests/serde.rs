//! Serialization of the reporting types behind the `serde` feature.
//!
//! Only present when built with `--features serde`; otherwise the file is empty.

#![cfg(feature = "serde")]

use pass_lang::{Outcome, Pass, PassError, PassManager};

struct Inc;
impl Pass<i64> for Inc {
    fn name(&self) -> &'static str {
        "inc"
    }
    fn run(&mut self, unit: &mut i64) -> Result<Outcome, PassError> {
        *unit += 1;
        Ok(Outcome::Changed)
    }
}

#[test]
fn report_serializes_to_expected_json() {
    let mut pm = PassManager::new();
    pm.add(Inc);

    let mut unit = 0;
    let report = pm.run(&mut unit).expect("the pass does not fail");

    let value: serde_json::Value = serde_json::to_value(&report).expect("the report serializes");

    assert_eq!(value["iterations"], 1);
    assert_eq!(value["converged"], false);
    assert!(value["runs"].is_array());
    assert_eq!(value["runs"][0]["name"], "inc");
    assert_eq!(value["runs"][0]["outcome"], "Changed");
}

#[test]
fn outcome_serializes_as_its_variant_name() {
    assert_eq!(
        serde_json::to_value(Outcome::Changed).expect("serializes"),
        serde_json::json!("Changed"),
    );
    assert_eq!(
        serde_json::to_value(Outcome::Unchanged).expect("serializes"),
        serde_json::json!("Unchanged"),
    );
}
