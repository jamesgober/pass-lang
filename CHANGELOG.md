<h1 align="center">
    <img width="90px" height="auto" src="https://raw.githubusercontent.com/jamesgober/jamesgober/main/media/icons/hexagon-3.svg" alt="Triple Hexagon">
    <br><b>CHANGELOG</b>
</h1>
<p>
  All notable changes to <code>pass-lang</code> will be documented in this file. The format is based on <a href="https://keepachangelog.com/en/1.1.0/">Keep a Changelog</a>,
  and this project adheres to <a href="https://semver.org/spec/v2.0.0.html/">Semantic Versioning</a>.
</p>

---

## [Unreleased]

### Added

### Changed

### Fixed

### Security

---

## [1.0.0] - 2026-06-30

API freeze. The `0.2.0` surface is ratified as the `1.0` contract: it follows
Semantic Versioning, with no breaking changes before `2.0`. There is no breaking
change from `0.2.0` &mdash; a `0.2.0` program compiles and behaves identically.

### Added

- `docs/API.md` marked stable, with a `## SemVer promise` section.
- `examples/contextual_passes.rs` &mdash; a unit that bundles the program with a
  diagnostics log, so passes share context without a global.

### Changed

- `Report` pre-sizes its run list to the pass count, and the hot-path methods on
  `Outcome`, `PassRun`, and `Report` are `#[inline]`. No API or behavioural change;
  benchmarks confirm no regression.

---

## [0.2.0] - 2026-06-30

The pass manager core. A generic, self-contained pipeline: passes are scheduled and
run over a unit of any type, with fixpoint iteration and a run report. No
first-party dependency is wired &mdash; the manager is generic over the unit, so it
binds to no concrete IR.

### Added

- `Pass<T>` &mdash; the plugin-seam trait. A pass names itself and transforms a unit
  of type `T` in place, reporting an `Outcome` or failing with a `PassError`.
- `Outcome` &mdash; `Changed` / `Unchanged`, the honest per-run signal that drives
  fixpoint termination, with `changed()` and `from_changed(bool)`.
- `PassManager<T>` &mdash; the ordered pipeline. `new`, `add` (the plugin seam),
  `len` / `is_empty`, `run` (one sweep), and `run_to_fixpoint` (repeat to a fixpoint
  under a bound that guarantees termination). Implements `Default`.
- `Report` and `PassRun` &mdash; the observability hook: every pass execution in
  order, with `runs()`, `changes()`, `iterations()`, and `converged()`.
- `PassError` &mdash; a pass-failure error carrying the failing pass's name and a
  reason; hand-rolled `Display` and `core::error::Error`.
- `serde` feature &mdash; derives `Serialize` for the reporting types so a run report
  can be logged or inspected.
- `examples/expr_optimizer.rs` and `examples/text_normalizer.rs`; criterion
  benchmarks for `run` and `run_to_fixpoint`; property tests for determinism,
  fixpoint termination and stability, and report accounting.
- `dev/DIRECTIVES.md` and the full `docs/API.md` covering every public item.

---

## [0.1.0] - 2026-06-18

Initial scaffold and repository bootstrap. No domain logic yet &mdash; this release establishes the structure, tooling, and quality gates the implementation will be built on.

### Added

- `Cargo.toml` with crate metadata, Rust 2024 edition, MSRV 1.85.
- Dual `Apache-2.0 OR MIT` license files.
- `README.md`, `CHANGELOG.md`, and a documentation skeleton.
- `REPS.md` compliance baseline.
- `.github/workflows/ci.yml` CI matrix; `deny.toml`, `clippy.toml`, `rustfmt.toml`.
- `dev/DIRECTIVES.md` and `dev/ROADMAP.md` (committed engineering standards + plan).

[Unreleased]: https://github.com/jamesgober/pass-lang/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/jamesgober/pass-lang/compare/v0.2.0...v1.0.0
[0.2.0]: https://github.com/jamesgober/pass-lang/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/jamesgober/pass-lang/releases/tag/v0.1.0
