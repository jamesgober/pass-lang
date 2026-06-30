# pass-lang &mdash; Engineering Directives

> Engineering standards and the definition of done for this project. Read alongside `REPS.md` (root, authoritative) and `dev/ROADMAP.md` (current phase). If anything here conflicts with `REPS.md`, `REPS.md` wins.

---

## 0. Philosophy

This library is built and maintained to a production standard and treated as a flagship piece of work. Plan the full path, then build one verified step at a time. "Good enough" is treated as a defect. pass-lang is the scheduler every later optimization and transform runs through: each pass trusts that it is handed the unit in a defined order, that a failure stops the pipeline cleanly instead of corrupting half a compilation, and that "nothing changed" is reported honestly so a fixpoint loop can terminate. A pass manager that reorders silently, swallows a pass error, or loops forever on an oscillating pass is not a cosmetic bug — it is a miscompile or a hang that surfaces far from its cause. The public surface stays small on purpose: order passes, run them over a unit, and report what happened — nothing more.

---

## 1. What this is

pass-lang is the pass manager: it orders optimization and transform passes and runs them over a unit of compilation, and it is the plugin seam that capability crates register their passes into. It is generic over the unit type — a function, a module, an intermediate representation, an AST — and owns no IR of its own. A consumer brings a concrete unit type `T`, implements the `Pass<T>` trait for each transform, registers them in order, and runs the pipeline. The manager schedules and runs; it never inspects or rewrites the unit itself — only the registered passes do. It sits in the SEMA tier of the `-lang` language-construction family, above the IR and the analyses the passes operate on, and below the driver that assembles a full compilation pipeline. It owns scheduling, sequencing, fixpoint iteration, and reporting only — the passes, the analyses, and the IR live in their own crates.

---

## 2. Engineering law (non-negotiable)

- **Performance** — peak is the baseline; the pipeline is a flat, pre-ordered list walked once per run with no per-pass allocation on the hot path; dispatch to a pass is a single indirect call amortised over the whole unit it rewrites, never a per-node cost; no "faster" claim without `criterion` numbers.
- **Correctness** — the invariants in section 4 are covered by property tests; passes run in registration order, deterministically; `Changed`/`Unchanged` is honest; fixpoint iteration terminates; a pass error halts the pipeline at a defined point with the failing pass named.
- **Security** — a registered pass is the only thing trusted to mutate the unit; a pass failure is a defined error, never UB, never a panic, never an unbounded loop; fixpoint iteration is bounded so a pathological pass cannot hang the driver.
- **Architecture** — SOLID, KISS, YAGNI; one responsibility (scheduling, not transformation); the manager is generic over the unit and adds no dependency on any concrete IR; the plugin seam is the `Pass<T>` trait and nothing wider.
- **Cross-platform** — Linux/macOS/Windows first-class, verified by CI; nothing here is platform-specific, and it stays that way.
- **Error handling** — every fallible path (a pass that cannot proceed) returns `Result` per the documented contract; no panics in shipping code.
- **Production-ready** — `#![forbid(unsafe_code)]` and `#![deny(missing_docs)]` from the first commit; no stray `println!`/`dbg!`; every public item has rustdoc with a runnable example.

---

## 3. Definition of done

1. Compiles clean on Linux/macOS/Windows, stable and MSRV 1.85.
2. `fmt`, `clippy -D warnings`, `test --all-features`, `cargo doc -D warnings` clean.
3. `cargo audit` + `cargo deny check` pass.
4. No `unwrap`/`expect`/`todo!`/`dbg!` in shipping code.
5. A Tier-1 API exists and headlines the docs.
6. Property tests cover every section-4 invariant.
7. Hot-path changes carry benchmarks; no regression over 5%.
8. Docs and `CHANGELOG.md` updated; the matching `docs/release/vX.Y.Z.md` written before the tag.

---

## 4. Project-specific invariants

- Passes run in the order they were registered. The same pipeline over the same unit produces the same result every time; ordering is never silently rewritten.
- The manager mutates the unit only by running a registered pass. It never reads or rewrites the unit on its own — scheduling is its single responsibility.
- A pass that reports `Unchanged` is taken at its word: the report and any fixpoint loop treat it as a no-op for that sweep. A pass that mutates the unit reports `Changed`.
- Fixpoint iteration terminates. It stops when a full sweep reports no change, or at the documented iteration bound — it never loops unboundedly on a pass whose output oscillates.
- A pass failure is contained. When a pass returns an error the pipeline stops at that pass, the error names the pass that failed and why, and the caller is never left unsure where the run halted. A pass error is never a panic.
- Every pass that runs is accounted for in the run's report; the report never claims a pass ran that did not, or omits one that did.
- No global or shared mutable state. A `PassManager<T>` owns its pipeline; two managers never alias one another's passes or reports.
