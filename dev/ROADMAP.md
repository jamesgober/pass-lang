# pass-lang - Roadmap

> Path from scaffold to a stable 1.0. Hard parts are front-loaded; each phase has hard exit criteria.
> Master plan: ../../_strategy/LANG_COLLECTION.md
>
> **Anti-deferral rule:** no listed hard task moves to a later phase unless this file records the move and the reason.

## v0.1.0 - Scaffold (DONE)
Compiles, CI green, structure correct, no domain logic.
- [x] Manifest, README, CHANGELOG, REPS, dual license, CI, deny, clippy, rustfmt.

## v0.2.0 - Core (THE HARD PART, NOT DEFERRED) (DONE)
The pass manager: ordered optimization/transform passes and the plugin seam capability crates register into.
Exit criteria:
- [x] Every public item has rustdoc + a runnable example.
- [x] Core invariants property-tested (full DIRECTIVES + API authored at this stage).

### Dependency wiring — not wired, with reason (anti-deferral rule)
`ir-lang` is **not** wired at v0.2.0. The pass manager is a generic scheduling
engine, not a transform over one concrete IR:

- A pass manager's job is to order passes, run them over a unit, iterate to a
  fixpoint, and report what happened. None of that depends on what the unit *is* —
  it is the same machinery whether the passes rewrite a high-level IR, a mid-level
  IR, a machine-level IR, or an AST. Coupling the manager to one concrete IR would
  bind it to a single layer and pull in a dependency the scheduler never reads.
- The manager is therefore generic over the unit type `T`. A consumer brings the
  concrete unit (`ir-lang`'s `Function`, its own IR, anything) and implements the
  `Pass<T>` plugin-seam trait for each transform. The seam *is* the trait; the
  manager never inspects or mutates the unit itself — only registered passes do.
  This is the same shape as LLVM's pass manager (generic over Module / Function /
  Loop) and Cranelift's pass pipeline.

This keeps the crate self-contained with zero first-party dependencies — the same
precedent `ir-lang` set by leaving `ast-lang` / `type-lang` unwired (a generic
substrate has no one concrete type to bind to). A concrete `ir-lang` integration,
if it earns its place, is an additive later-minor convenience, not a v0.2.0
requirement.

### Scope held for a later minor (additive, non-breaking)
Analysis passes with cached, invalidatable results, and inter-pass dependency
declaration (a pass requiring another's result), are **not** in v0.2.0. The unit
is run through a flat, registration-ordered pipeline with optional fixpoint
iteration. An analysis cache and a dependency scheduler are additive — they arrive
in a later 0.x minor without breaking the v0.2.0 surface.

## v1.0.0 - API freeze (DONE)
Public surface stable and frozen until 2.0.
- [x] docs/API.md marked stable; SemVer promise recorded.
- [x] Full test + benchmark suite green on all three platforms.

The 0.2.0 surface is frozen as the 1.0 contract; no breaking change. The freeze
pre-sizes the run report, inlines the hot-path methods, adds the `contextual_passes`
example, and ships the stability docs. Post-1.0 additive candidates (each a minor
release): analysis passes with cached, invalidatable results, and inter-pass
dependency declaration.
