# pass-lang &mdash; API Reference

> Complete reference for every public item in `pass-lang`, with examples.
> **Status: stable (1.0).** The surface below is the `1.0` contract; it follows
> [Semantic Versioning](#semver-promise) and will not change in a breaking way
> before `2.0`. See [`../dev/ROADMAP.md`](../dev/ROADMAP.md).

## Table of contents

- [Overview](#overview)
- [Installation](#installation)
- [Quick start](#quick-start)
- [The model](#the-model)
- [`Pass`](#pass)
  - [`Pass::name`](#passname)
  - [`Pass::run`](#passrun)
- [`Outcome`](#outcome)
  - [`Outcome::changed`](#outcomechanged)
  - [`Outcome::from_changed`](#outcomefrom_changed)
- [`PassManager`](#passmanager)
  - [`PassManager::new`](#passmanagernew)
  - [`PassManager::add`](#passmanageradd)
  - [`PassManager::len` / `is_empty`](#passmanagerlen--is_empty)
  - [`PassManager::run`](#passmanagerrun)
  - [`PassManager::run_to_fixpoint`](#passmanagerrun_to_fixpoint)
- [`Report`](#report)
  - [`Report::runs`](#reportruns)
  - [`Report::changes`](#reportchanges)
  - [`Report::iterations`](#reportiterations)
  - [`Report::converged`](#reportconverged)
- [`PassRun`](#passrun-1)
- [`PassError`](#passerror)
  - [`PassError::new`](#passerrornew)
  - [`PassError::pass` / `message`](#passerrorpass--message)
- [Feature flags](#feature-flags)
- [SemVer promise](#semver-promise)

---

## Overview

pass-lang is the pass manager: it orders optimization and transform passes and
runs them over a unit of compilation, and it is the plugin seam capability crates
register their passes into. It is generic over the unit a pass rewrites — an
intermediate representation, a single function, a module, an abstract syntax tree,
or a struct bundling an IR with the diagnostics and analysis a pass needs.

A [`Pass<T>`](#pass) is one transform over a unit of type `T`. A
[`PassManager<T>`](#passmanager) holds passes in registration order and runs them;
it is the scheduler and never inspects the unit itself. Each run returns a
[`Report`](#report) of what every pass did. The crate is self-contained: it owns
no IR and wires no first-party dependency — the same shape as LLVM's pass manager
(generic over Module / Function / Loop) or Cranelift's pass pipeline.

---

## Installation

```toml
[dependencies]
pass-lang = "1.0"
```

Or from the terminal:

```bash
cargo add pass-lang
```

MSRV: Rust 1.85 (Rust 2024 edition).

---

## Quick start

Define two passes over a list of integers, run them to a fixpoint, and read the
report:

```rust
use pass_lang::{Outcome, Pass, PassError, PassManager};

// Drop zero entries.
struct DropZeros;
impl Pass<Vec<i64>> for DropZeros {
    fn name(&self) -> &'static str { "drop-zeros" }
    fn run(&mut self, unit: &mut Vec<i64>) -> Result<Outcome, PassError> {
        let before = unit.len();
        unit.retain(|&x| x != 0);
        Ok(Outcome::from_changed(unit.len() != before))
    }
}

// Halve every value greater than one.
struct Halve;
impl Pass<Vec<i64>> for Halve {
    fn name(&self) -> &'static str { "halve" }
    fn run(&mut self, unit: &mut Vec<i64>) -> Result<Outcome, PassError> {
        let mut changed = false;
        for x in unit.iter_mut() {
            if *x > 1 { *x /= 2; changed = true; }
        }
        Ok(Outcome::from_changed(changed))
    }
}

let mut pm = PassManager::new();
pm.add(DropZeros).add(Halve);

let mut unit = vec![0, 8, 0, 4];
let report = pm.run_to_fixpoint(&mut unit, 16).unwrap();

assert_eq!(unit, vec![1, 1]);   // zeros dropped, 8 and 4 halved to 1
assert!(report.converged());
```

---

## The model

A *unit* is whatever a pass rewrites — its type is the `T` in `Pass<T>` and
`PassManager<T>`. Because the manager never reads the unit itself, `T` can be
anything: an IR function, a whole module, or a struct that bundles the IR with the
diagnostics sink and analysis results the passes share.

A *pass* is one transform. You implement [`Pass<T>`](#pass) — two methods, a
[`name`](#passname) and a [`run`](#passrun) — and that is the plugin seam: any
crate can contribute a pass without the manager knowing its type.

A *pass manager* is an ordered pipeline. You register passes with
[`add`](#passmanageradd), in the order they should run, and then drive them:

- [`run`](#passmanagerrun) makes one sweep — every pass once, in order.
- [`run_to_fixpoint`](#passmanagerrun_to_fixpoint) repeats the sweep until a full
  pass changes nothing, or an iteration bound is hit. This is how a transform that
  exposes more work for an earlier pass (folding exposing dead code, which exposes
  more folding) is driven to completion without an unbounded loop.

Both return a [`Report`](#report): every pass execution in order, plus whether the
pipeline settled and how many sweeps it took. A pass that cannot proceed returns a
[`PassError`](#passerror) instead of panicking; the manager stops the pipeline and
names the pass that failed.

---

## `Pass`

```rust,ignore
pub trait Pass<T> {
    fn name(&self) -> &'static str;
    fn run(&mut self, unit: &mut T) -> Result<Outcome, PassError>;
}
```

The plugin-seam trait. Implement it to define a transform or analysis over a unit
of type `T`. A pass is registered with [`PassManager::add`](#passmanageradd) and
must be `'static` — it may own state across runs, but it may not borrow from
outside the manager. The manager is the only scheduler; a pass is the only thing
trusted to read or mutate the unit.

### `Pass::name`

```rust,ignore
fn name(&self) -> &'static str;
```

A stable, static identifier for the pass, used in the [`Report`](#report) and in
[`PassError`](#passerror) context. It must not change between runs of the same
pass.

### `Pass::run`

```rust,ignore
fn run(&mut self, unit: &mut T) -> Result<Outcome, PassError>;
```

Transform `unit` in place. Return [`Outcome::Changed`](#outcome) **if and only if**
the unit was modified, so the manager's fixpoint loop can tell when the pipeline
has settled — reporting `Unchanged` after a mutation breaks termination. Return a
[`PassError`](#passerror) — never a panic — if the pass cannot proceed.

**Example** — a pass that rewrites a unit and reports honestly:

```rust
use pass_lang::{Outcome, Pass, PassError};

struct Negate;
impl Pass<i64> for Negate {
    fn name(&self) -> &'static str { "negate" }
    fn run(&mut self, unit: &mut i64) -> Result<Outcome, PassError> {
        if *unit == 0 {
            return Ok(Outcome::Unchanged); // -0 == 0, nothing changed
        }
        *unit = -*unit;
        Ok(Outcome::Changed)
    }
}

let mut value = 5;
assert_eq!(Negate.run(&mut value).unwrap(), Outcome::Changed);
assert_eq!(value, -5);
```

**Example** — a pass that fails on input it cannot handle:

```rust
use pass_lang::{Outcome, Pass, PassError};

struct RequirePositive;
impl Pass<i64> for RequirePositive {
    fn name(&self) -> &'static str { "require-positive" }
    fn run(&mut self, unit: &mut i64) -> Result<Outcome, PassError> {
        if *unit < 0 {
            return Err(PassError::new("value must be non-negative"));
        }
        Ok(Outcome::Unchanged)
    }
}

let mut value = -1;
assert!(RequirePositive.run(&mut value).is_err());
```

---

## `Outcome`

```rust,ignore
pub enum Outcome {
    Changed,
    Unchanged,
}
```

Whether a pass changed the unit on a given run. The manager uses it to decide
whether a fixpoint loop has settled and to build the [`Report`](#report). `Outcome`
is `Copy`, `Eq`, and `Hash`; with the `serde` feature it derives `Serialize`.

### `Outcome::changed`

```rust,ignore
pub fn changed(self) -> bool;
```

`true` for [`Outcome::Changed`](#outcome).

```rust
use pass_lang::Outcome;

assert!(Outcome::Changed.changed());
assert!(!Outcome::Unchanged.changed());
```

### `Outcome::from_changed`

```rust,ignore
pub fn from_changed(changed: bool) -> Outcome;
```

Build an outcome from a "did it change?" flag — the common tail of a
[`run`](#passrun) body.

```rust
use pass_lang::Outcome;

let before = 3;
let after = 3;
assert_eq!(Outcome::from_changed(before != after), Outcome::Unchanged);
```

---

## `PassManager`

```rust,ignore
pub struct PassManager<T> { /* private */ }
```

An ordered pipeline of passes over a unit of type `T`. Holds passes in
registration order and runs them; scheduling is its only responsibility. It is
single-threaded by design — a pipeline is an inherently ordered sequence of
mutations, so it carries no atomic overhead — and implements
[`Default`](#passmanagernew) as the empty pipeline.

### `PassManager::new`

```rust,ignore
pub fn new() -> PassManager<T>;
```

Create an empty pipeline. `PassManager::default()` is equivalent.

```rust
use pass_lang::PassManager;

let pm = PassManager::<i64>::new();
assert!(pm.is_empty());
```

### `PassManager::add`

```rust,ignore
pub fn add(&mut self, pass: impl Pass<T> + 'static) -> &mut Self;
```

Register a pass at the end of the pipeline and return `&mut Self` so registrations
can be chained. This is the plugin seam: the pass runs after every pass already
registered. The pass must be `'static`.

**Example** — chaining several registrations:

```rust
use pass_lang::{Outcome, Pass, PassError, PassManager};

struct Step(&'static str);
impl Pass<i64> for Step {
    fn name(&self) -> &'static str { self.0 }
    fn run(&mut self, _: &mut i64) -> Result<Outcome, PassError> { Ok(Outcome::Unchanged) }
}

let mut pm = PassManager::new();
pm.add(Step("a")).add(Step("b")).add(Step("c"));
assert_eq!(pm.len(), 3);
```

**Example** — registering in a loop (the `&mut Self` return is simply ignored):

```rust
use pass_lang::{Outcome, Pass, PassError, PassManager};

struct Inc;
impl Pass<i64> for Inc {
    fn name(&self) -> &'static str { "inc" }
    fn run(&mut self, u: &mut i64) -> Result<Outcome, PassError> { *u += 1; Ok(Outcome::Changed) }
}

let mut pm = PassManager::new();
for _ in 0..4 {
    pm.add(Inc);
}
let mut unit = 0;
pm.run(&mut unit).unwrap();
assert_eq!(unit, 4);
```

### `PassManager::len` / `is_empty`

```rust,ignore
pub fn len(&self) -> usize;
pub fn is_empty(&self) -> bool;
```

The number of registered passes, and whether there are none.

```rust
use pass_lang::PassManager;

let pm = PassManager::<i64>::new();
assert_eq!(pm.len(), 0);
assert!(pm.is_empty());
```

### `PassManager::run`

```rust,ignore
pub fn run(&mut self, unit: &mut T) -> Result<Report, PassError>;
```

Run every pass once, in registration order. Each pass transforms `unit` in place;
the returned [`Report`](#report) lists every pass with its
[`Outcome`](#outcome). The report's [`iterations`](#reportiterations) is always
`1`, and [`converged`](#reportconverged) is `true` when the sweep changed nothing.

**Errors.** If a pass returns a [`PassError`](#passerror), the pipeline stops at
that pass — later passes do not run — and the error is returned with the failing
pass's name stamped in.

**Example** — a single sweep that transforms and reports:

```rust
use pass_lang::{Outcome, Pass, PassError, PassManager};

struct Double;
impl Pass<i64> for Double {
    fn name(&self) -> &'static str { "double" }
    fn run(&mut self, u: &mut i64) -> Result<Outcome, PassError> { *u *= 2; Ok(Outcome::Changed) }
}

let mut pm = PassManager::new();
pm.add(Double).add(Double);

let mut unit = 3;
let report = pm.run(&mut unit).unwrap();

assert_eq!(unit, 12);                 // 3 -> 6 -> 12
assert_eq!(report.runs().len(), 2);
assert_eq!(report.changes(), 2);
```

**Example** — a failing pass halts the pipeline and names itself:

```rust
use pass_lang::{Outcome, Pass, PassError, PassManager};

struct Guard;
impl Pass<i64> for Guard {
    fn name(&self) -> &'static str { "guard" }
    fn run(&mut self, u: &mut i64) -> Result<Outcome, PassError> {
        if *u > 10 { return Err(PassError::new("value too large")); }
        Ok(Outcome::Unchanged)
    }
}

let mut pm = PassManager::new();
pm.add(Guard);

let mut unit = 99;
let err = pm.run(&mut unit).unwrap_err();
assert_eq!(err.pass(), "guard");
assert_eq!(err.message(), "value too large");
```

### `PassManager::run_to_fixpoint`

```rust,ignore
pub fn run_to_fixpoint(&mut self, unit: &mut T, max_iters: usize) -> Result<Report, PassError>;
```

Repeat the pipeline until it settles or `max_iters` sweeps run. Each sweep runs
every pass once, in order. After a sweep in which no pass reported
[`Changed`](#outcome), the unit is at a fixpoint and the loop stops with
[`converged`](#reportconverged) `true`. If `max_iters` sweeps run while the unit is
still changing, the loop stops with `converged` `false` — the bound guarantees
termination even if a pass oscillates. `max_iters == 0` performs no sweeps.

**Errors.** Returns the [`PassError`](#passerror) of the first pass that fails, on
whichever sweep it fails.

**Example** — driving a transform to convergence:

```rust
use pass_lang::{Outcome, Pass, PassError, PassManager};

struct Halve;
impl Pass<i64> for Halve {
    fn name(&self) -> &'static str { "halve" }
    fn run(&mut self, u: &mut i64) -> Result<Outcome, PassError> {
        if *u <= 1 { return Ok(Outcome::Unchanged); }
        *u /= 2;
        Ok(Outcome::Changed)
    }
}

let mut pm = PassManager::new();
pm.add(Halve);

let mut unit = 16;
let report = pm.run_to_fixpoint(&mut unit, 32).unwrap();

assert_eq!(unit, 1);
assert!(report.converged());
assert_eq!(report.iterations(), 5);  // 16->8->4->2->1, plus one confirming sweep
```

**Example** — the bound stops an oscillating pass:

```rust
use pass_lang::{Outcome, Pass, PassError, PassManager};

struct Flip;
impl Pass<i64> for Flip {
    fn name(&self) -> &'static str { "flip" }
    fn run(&mut self, u: &mut i64) -> Result<Outcome, PassError> { *u = 1 - *u; Ok(Outcome::Changed) }
}

let mut pm = PassManager::new();
pm.add(Flip);

let mut unit = 0;
let report = pm.run_to_fixpoint(&mut unit, 10).unwrap();

assert_eq!(report.iterations(), 10);
assert!(!report.converged());
```

---

## `Report`

```rust,ignore
pub struct Report { /* private */ }
```

A record of one run: every pass that ran, in order, with the [`Outcome`](#outcome)
each reported, plus the aggregate. Produced by [`run`](#passmanagerrun) and
[`run_to_fixpoint`](#passmanagerrun_to_fixpoint); you read it, you do not build it.
`Report` is `Clone`, `Eq`, and `Default`; with the `serde` feature it derives
`Serialize`.

### `Report::runs`

```rust,ignore
pub fn runs(&self) -> &[PassRun];
```

Every pass execution, in the order it happened. Across a fixpoint run, a pass that
runs on three sweeps appears three times.

```rust
use pass_lang::{Outcome, Pass, PassError, PassManager};

struct Tag(&'static str);
impl Pass<i64> for Tag {
    fn name(&self) -> &'static str { self.0 }
    fn run(&mut self, _: &mut i64) -> Result<Outcome, PassError> { Ok(Outcome::Unchanged) }
}

let mut pm = PassManager::new();
pm.add(Tag("a")).add(Tag("b"));
let mut unit = 0;
let report = pm.run(&mut unit).unwrap();

let names: Vec<_> = report.runs().iter().map(|r| r.name()).collect();
assert_eq!(names, ["a", "b"]);
```

### `Report::changes`

```rust,ignore
pub fn changes(&self) -> usize;
```

How many pass executions reported [`Changed`](#outcome).

```rust
use pass_lang::{Outcome, Pass, PassError, PassManager};

struct Inc;
impl Pass<i64> for Inc {
    fn name(&self) -> &'static str { "inc" }
    fn run(&mut self, u: &mut i64) -> Result<Outcome, PassError> { *u += 1; Ok(Outcome::Changed) }
}

let mut pm = PassManager::new();
pm.add(Inc).add(Inc);
let mut unit = 0;
assert_eq!(pm.run(&mut unit).unwrap().changes(), 2);
```

### `Report::iterations`

```rust,ignore
pub fn iterations(&self) -> usize;
```

The number of full sweeps over the pipeline. [`run`](#passmanagerrun) always
reports `1`; [`run_to_fixpoint`](#passmanagerrun_to_fixpoint) reports how many
sweeps it performed.

### `Report::converged`

```rust,ignore
pub fn converged(&self) -> bool;
```

Whether the final sweep made no change — the pipeline reached a fixpoint. For
[`run`](#passmanagerrun) this is `true` when the single sweep changed nothing; for
[`run_to_fixpoint`](#passmanagerrun_to_fixpoint) it is `true` when a sweep settled
before the bound, and `false` when the bound was hit with the unit still changing.

```rust
use pass_lang::{Outcome, Pass, PassError, PassManager};

struct Noop;
impl Pass<i64> for Noop {
    fn name(&self) -> &'static str { "noop" }
    fn run(&mut self, _: &mut i64) -> Result<Outcome, PassError> { Ok(Outcome::Unchanged) }
}

let mut pm = PassManager::new();
pm.add(Noop);
let mut unit = 0;
let report = pm.run(&mut unit).unwrap();
assert!(report.converged());   // nothing changed, so the unit is at a fixpoint
```

---

## `PassRun`

```rust,ignore
pub struct PassRun { /* private */ }

impl PassRun {
    pub fn name(&self) -> &'static str;
    pub fn outcome(&self) -> Outcome;
}
```

One entry in a [`Report`](#report): the [`name`](#passname) of a pass that ran and
the [`Outcome`](#outcome) it reported. `PassRun` is `Copy`; with the `serde`
feature it derives `Serialize`.

```rust
use pass_lang::{Outcome, Pass, PassError, PassManager};

struct Touch;
impl Pass<i64> for Touch {
    fn name(&self) -> &'static str { "touch" }
    fn run(&mut self, u: &mut i64) -> Result<Outcome, PassError> { *u += 1; Ok(Outcome::Changed) }
}

let mut pm = PassManager::new();
pm.add(Touch);
let mut unit = 0;
let report = pm.run(&mut unit).unwrap();

let run = report.runs()[0];
assert_eq!(run.name(), "touch");
assert_eq!(run.outcome(), Outcome::Changed);
```

---

## `PassError`

```rust,ignore
pub struct PassError { /* private */ }
```

The error a [`Pass`](#pass) returns when it cannot complete. It carries a
human-readable reason; the [`PassManager`](#passmanager) stamps in the name of the
failing pass before returning it, so a caller always knows which pass halted the
pipeline and why. `PassError` is `Clone`, `Eq`, `Display`, and implements
`core::error::Error`.

### `PassError::new`

```rust,ignore
pub fn new(message: impl Into<Cow<'static, str>>) -> PassError;
```

Create an error describing why a pass could not complete. The message accepts a
string literal (no allocation) or an owned `String` (a computed reason). Call it
from inside [`Pass::run`](#passrun); you do not repeat the pass name.

```rust
use pass_lang::PassError;

let from_literal = PassError::new("division by zero");
let from_owned = PassError::new(format!("overflow at index {}", 7));
assert_eq!(from_literal.message(), "division by zero");
assert_eq!(from_owned.message(), "overflow at index 7");
```

### `PassError::pass` / `message`

```rust,ignore
pub fn pass(&self) -> &str;
pub fn message(&self) -> &str;
```

The name of the pass that failed (empty until the error has passed through a
[`PassManager`](#passmanager)), and the reason it failed. The `Display`
representation combines them:

```rust
use pass_lang::{Outcome, Pass, PassError, PassManager};

struct Fail;
impl Pass<i64> for Fail {
    fn name(&self) -> &'static str { "fail" }
    fn run(&mut self, _: &mut i64) -> Result<Outcome, PassError> {
        Err(PassError::new("nope"))
    }
}

let mut pm = PassManager::new();
pm.add(Fail);
let mut unit = 0;
let err = pm.run(&mut unit).unwrap_err();

assert_eq!(err.pass(), "fail");
assert_eq!(err.message(), "nope");
assert_eq!(err.to_string(), "pass `fail` failed: nope");
```

---

## Feature flags

| Feature | Default | Effect |
| --- | --- | --- |
| `std` | yes | Uses the standard library. Without it the crate is `#![no_std]` and needs only `alloc`. |
| `serde` | no | Derives `serde::Serialize` for [`Outcome`](#outcome), [`PassRun`](#passrun-1), and [`Report`](#report) so a run report can be logged or inspected. |

```toml
# no_std build (needs alloc)
pass-lang = { version = "1.0", default-features = false }

# with serializable reports
pass-lang = { version = "1.0", features = ["serde"] }
```

---

## SemVer promise

As of `1.0.0` the public surface is frozen. The crate follows
[Semantic Versioning](https://semver.org):

- No documented item is removed or changed in a breaking way within `1.x`; breaking
  changes wait for `2.0`.
- New functionality is additive and arrives in minor releases.
  [`PassManager`](#passmanager), [`Report`](#report), [`PassRun`](#passrun-1), and
  [`PassError`](#passerror) keep their fields private, so a new method or field is a
  minor change. [`Outcome`](#outcome) is a deliberately complete two-variant enum
  and will not grow.
- The [`Pass`](#pass) trait will not gain a required method within `1.x`; any new
  method ships with a default body, which is additive.
- The MSRV is Rust `1.85`; raising it is a minor change, never a patch.
- Behaviour is part of the contract: passes run in registration order;
  `run_to_fixpoint` always terminates within its bound; a pass failure halts the
  pipeline and names the failing pass; and the `runs` / `changes` / `iterations` /
  `converged` accounting is stable.

This file is updated in lockstep with every release so it always matches the code.

<sub>Copyright &copy; 2026 <strong>James Gober</strong>.</sub>
