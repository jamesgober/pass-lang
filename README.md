<h1 align="center">
    <img width="99" alt="Rust logo" src="https://raw.githubusercontent.com/jamesgober/rust-collection/72baabd71f00e14aa9184efcb16fa3deddda3a0a/assets/rust-logo.svg">
    <br>
    <b>pass-lang</b>
    <br>
    <sub><sup>PASS MANAGER</sup></sub>
</h1>

<div align="center">
    <a href="https://crates.io/crates/pass-lang"><img alt="Crates.io" src="https://img.shields.io/crates/v/pass-lang"></a>
    <a href="https://crates.io/crates/pass-lang"><img alt="Downloads" src="https://img.shields.io/crates/d/pass-lang?color=%230099ff"></a>
    <a href="https://docs.rs/pass-lang"><img alt="docs.rs" src="https://img.shields.io/docsrs/pass-lang"></a>
    <a href="https://github.com/jamesgober/pass-lang/actions"><img alt="CI" src="https://github.com/jamesgober/pass-lang/actions/workflows/ci.yml/badge.svg"></a>
    <a href="https://github.com/rust-lang/rfcs/blob/master/text/2495-min-rust-version.md"><img alt="MSRV" src="https://img.shields.io/badge/MSRV-1.85%2B-blue"></a>
</div>

<br>

<div align="left">
    <p>
        pass-lang is the pass manager a compiler runs its optimizations and transforms through. A <code>PassManager</code> is an ordered pipeline of passes over a unit of compilation: it runs them in registration order, can iterate to a fixpoint, and reports what changed. It is generic over the unit type &mdash; a function, a module, an intermediate representation, an AST &mdash; and owns no IR of its own. A consumer brings the unit and implements the <code>Pass</code> trait, which is the plugin seam capability crates register their passes into. It is a SEMA-tier crate of the <code>-lang</code> language-construction family.
    </p>
    <br>
    <hr>
    <p>
        <strong>MSRV is 1.85+</strong> (Rust 2024 edition).
    </p>
    <blockquote>
        <strong>Status: stable (1.0).</strong> The public API is frozen and follows Semantic Versioning &mdash; no breaking changes before <code>2.0</code>. See <a href="./docs/API.md#semver-promise"><code>the SemVer promise</code></a> and <a href="./CHANGELOG.md"><code>CHANGELOG.md</code></a>.
    </blockquote>
</div>

<hr>
<br>

## Installation

```toml
[dependencies]
pass-lang = "1.0"
```

Or from the terminal:

```bash
cargo add pass-lang
```

<br>

## Usage

Implement [`Pass`](./docs/API.md#pass) for each transform, register them in order with [`PassManager::add`](./docs/API.md#passmanageradd), and run the pipeline. Each pass reports whether it changed the unit, so the manager can drive the pipeline to a fixpoint and tell you what happened.

```rust
use pass_lang::{Outcome, Pass, PassError, PassManager};

// The unit is whatever a pass rewrites — here, a list of integers.
struct DropZeros;
impl Pass<Vec<i64>> for DropZeros {
    fn name(&self) -> &'static str {
        "drop-zeros"
    }
    fn run(&mut self, unit: &mut Vec<i64>) -> Result<Outcome, PassError> {
        let before = unit.len();
        unit.retain(|&x| x != 0);
        Ok(Outcome::from_changed(unit.len() != before))
    }
}

// Halve every value greater than one.
struct Halve;
impl Pass<Vec<i64>> for Halve {
    fn name(&self) -> &'static str {
        "halve"
    }
    fn run(&mut self, unit: &mut Vec<i64>) -> Result<Outcome, PassError> {
        let mut changed = false;
        for x in unit.iter_mut() {
            if *x > 1 {
                *x /= 2;
                changed = true;
            }
        }
        Ok(Outcome::from_changed(changed))
    }
}

let mut pm = PassManager::new();
pm.add(DropZeros).add(Halve);

let mut unit = vec![0, 8, 0, 4];
let report = pm.run_to_fixpoint(&mut unit, 16).expect("no pass fails");

assert_eq!(unit, vec![1, 1]); // zeros dropped, 8 and 4 halved to 1
assert!(report.converged());
```

A pass that cannot proceed returns a [`PassError`](./docs/API.md#passerror) instead of panicking; the manager stops the pipeline and names the pass that failed. The unit type is anything a pass rewrites — an IR, a function, a module, an AST, or a struct bundling an IR with the diagnostics and analysis a pass needs.

Runnable examples: [`examples/expr_optimizer.rs`](./examples/expr_optimizer.rs) (constant folding and algebraic simplification to a fixpoint, with the error path), [`examples/text_normalizer.rs`](./examples/text_normalizer.rs) (the same machinery over a `String`), and [`examples/contextual_passes.rs`](./examples/contextual_passes.rs) (a unit that bundles code with a diagnostics log, so passes share context without a global).

```bash
cargo run --example expr_optimizer
cargo run --example text_normalizer
cargo run --example contextual_passes
```

<br>

## Documentation

- [API Reference](./docs/API.md) — every public item, with examples.
- [ROADMAP](./dev/ROADMAP.md) — the path to a frozen 1.0.
- [CHANGELOG](./CHANGELOG.md).

<hr>
<br>

## Contributing

See <a href="./dev/DIRECTIVES.md"><code>dev/DIRECTIVES.md</code></a> for engineering standards and the definition of done. Before a PR: `cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all-features` must be clean.

<br>

<div id="license">
    <h2>License</h2>
    <p>Licensed under either of</p>
    <ul>
        <li><b>Apache License, Version 2.0</b> &mdash; <a href="./LICENSE-APACHE">LICENSE-APACHE</a></li>
        <li><b>MIT License</b> &mdash; <a href="./LICENSE-MIT">LICENSE-MIT</a></li>
    </ul>
    <p>at your option.</p>
</div>

<div align="center">
  <h2></h2>
  <sup>COPYRIGHT <small>&copy;</small> 2026 <strong>James Gober <me@jamesgober.com>.</strong></sup>
</div>
