//! A constant-folding and algebraic-simplification pipeline over a small
//! arithmetic expression tree, scheduled by `pass-lang`.
//!
//! This is the shape a real optimizer takes: each transform is a [`Pass`], the
//! manager runs them to a fixpoint (folding exposes more simplification, which
//! exposes more folding), and the [`Report`](pass_lang::Report) records what
//! every pass did. The final pipeline also shows the error path — folding an
//! expression that overflows fails cleanly with the pass named.
//!
//! Run it with:
//!
//! ```text
//! cargo run --example expr_optimizer
//! ```

use pass_lang::{Outcome, Pass, PassError, PassManager};

/// A tiny arithmetic expression: integer constants combined with `+` and `*`.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Expr {
    Const(i64),
    Add(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
}

use Expr::{Add, Const, Mul};

fn add(l: Expr, r: Expr) -> Expr {
    Add(Box::new(l), Box::new(r))
}

fn mul(l: Expr, r: Expr) -> Expr {
    Mul(Box::new(l), Box::new(r))
}

/// Fold a `Const op Const` subtree into a single constant, bottom-up.
///
/// Folding an expression whose value would not fit in `i64` is a failure, not a
/// silent wrap: the pass returns a [`PassError`] and the pipeline stops.
struct ConstFold;

impl ConstFold {
    fn fold(e: &mut Expr) -> Result<bool, PassError> {
        let mut changed = match e {
            Const(_) => false,
            Add(l, r) | Mul(l, r) => {
                let left = Self::fold(l)?;
                let right = Self::fold(r)?;
                left || right
            }
        };

        let folded = match e {
            Add(l, r) => match (&**l, &**r) {
                (Const(a), Const(b)) => Some(
                    a.checked_add(*b)
                        .ok_or_else(|| PassError::new(format!("overflow folding {a} + {b}")))?,
                ),
                _ => None,
            },
            Mul(l, r) => match (&**l, &**r) {
                (Const(a), Const(b)) => Some(
                    a.checked_mul(*b)
                        .ok_or_else(|| PassError::new(format!("overflow folding {a} * {b}")))?,
                ),
                _ => None,
            },
            Const(_) => None,
        };

        if let Some(value) = folded {
            *e = Const(value);
            changed = true;
        }
        Ok(changed)
    }
}

impl Pass<Expr> for ConstFold {
    fn name(&self) -> &'static str {
        "const-fold"
    }

    fn run(&mut self, unit: &mut Expr) -> Result<Outcome, PassError> {
        Ok(Outcome::from_changed(Self::fold(unit)?))
    }
}

/// Apply the algebraic identities `x + 0`, `0 + x`, `x * 1`, `1 * x`, and the
/// annihilator `x * 0`, bottom-up.
struct Simplify;

impl Simplify {
    fn simplify(e: &mut Expr) -> bool {
        let mut changed = match e {
            Const(_) => false,
            Add(l, r) | Mul(l, r) => {
                let left = Self::simplify(l);
                let right = Self::simplify(r);
                left || right
            }
        };

        let replaced = match e {
            Add(l, r) => match (&**l, &**r) {
                (Const(0), _) => Some((**r).clone()),
                (_, Const(0)) => Some((**l).clone()),
                _ => None,
            },
            Mul(l, r) => match (&**l, &**r) {
                (Const(0), _) | (_, Const(0)) => Some(Const(0)),
                (Const(1), _) => Some((**r).clone()),
                (_, Const(1)) => Some((**l).clone()),
                _ => None,
            },
            Const(_) => None,
        };

        if let Some(value) = replaced {
            *e = value;
            changed = true;
        }
        changed
    }
}

impl Pass<Expr> for Simplify {
    fn name(&self) -> &'static str {
        "simplify"
    }

    fn run(&mut self, unit: &mut Expr) -> Result<Outcome, PassError> {
        Ok(Outcome::from_changed(Self::simplify(unit)))
    }
}

fn main() {
    // ((2 * 1) + 0) * (3 + 4)  ==>  14
    let mut expr = mul(
        add(mul(Const(2), Const(1)), Const(0)),
        add(Const(3), Const(4)),
    );
    println!("before: {expr:?}");

    let mut pm = PassManager::new();
    pm.add(Simplify).add(ConstFold);

    match pm.run_to_fixpoint(&mut expr, 32) {
        Ok(report) => {
            println!("after:  {expr:?}");
            println!(
                "sweeps={} changes={} converged={}",
                report.iterations(),
                report.changes(),
                report.converged(),
            );
            for run in report.runs() {
                println!("  {:<12} {:?}", run.name(), run.outcome());
            }
            assert_eq!(expr, Const(14));
        }
        Err(err) => println!("pipeline failed: {err}"),
    }

    println!();

    // The error path: i64::MAX * 2 cannot be folded into an i64.
    let mut overflow = mul(Const(i64::MAX), Const(2));
    let mut folder = PassManager::new();
    folder.add(ConstFold);
    match folder.run(&mut overflow) {
        Ok(_) => println!("unexpected success"),
        Err(err) => {
            println!("expected failure: {err}");
            assert_eq!(err.pass(), "const-fold");
        }
    }
}
