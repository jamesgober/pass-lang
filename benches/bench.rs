//! Criterion benchmarks for the pass pipeline.
//!
//! `run` measures a single sweep of a three-pass pipeline over a one-thousand
//! element unit; `run_to_fixpoint` measures a pipeline driven to convergence.
//! Both reuse one manager across iterations so the figure is the cost of running
//! the pipeline, not of building it.

use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};
use pass_lang::{Outcome, Pass, PassError, PassManager};

/// Add one to every element. Always reports `Changed`.
struct Increment;
impl Pass<Vec<i64>> for Increment {
    fn name(&self) -> &'static str {
        "increment"
    }
    fn run(&mut self, unit: &mut Vec<i64>) -> Result<Outcome, PassError> {
        for value in unit.iter_mut() {
            *value += 1;
        }
        Ok(Outcome::Changed)
    }
}

/// Drop every zero element. Reports `Changed` only if it removed one.
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

/// Decrement every positive element toward zero. Reports `Changed` while any
/// element is still positive — drives a fixpoint that takes `max` sweeps.
struct Decrement;
impl Pass<Vec<i64>> for Decrement {
    fn name(&self) -> &'static str {
        "decrement"
    }
    fn run(&mut self, unit: &mut Vec<i64>) -> Result<Outcome, PassError> {
        let mut changed = false;
        for value in unit.iter_mut() {
            if *value > 0 {
                *value -= 1;
                changed = true;
            }
        }
        Ok(Outcome::from_changed(changed))
    }
}

fn bench_run(c: &mut Criterion) {
    let mut pm = PassManager::new();
    pm.add(Increment).add(DropZeros).add(Increment);

    c.bench_function("run/3-pass/1k", |b| {
        b.iter_batched(
            || (-500..500i64).collect::<Vec<_>>(),
            |mut unit| {
                let report = pm.run(black_box(&mut unit)).expect("passes do not fail");
                black_box(report);
            },
            BatchSize::SmallInput,
        );
    });
}

fn bench_fixpoint(c: &mut Criterion) {
    let mut pm = PassManager::new();
    pm.add(Decrement);

    c.bench_function("fixpoint/decrement-to-zero/cap64", |b| {
        b.iter_batched(
            || vec![32i64; 256],
            |mut unit| {
                let report = pm
                    .run_to_fixpoint(black_box(&mut unit), 64)
                    .expect("passes do not fail");
                black_box(report);
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, bench_run, bench_fixpoint);
criterion_main!(benches);
