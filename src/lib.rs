//! # pass_lang
//!
//! A pass manager: an ordered pipeline of optimization and transform passes, and
//! the plugin seam capability crates register their passes into.
//!
//! A compiler improves and lowers a program by running a series of *passes* over
//! it — constant folding, dead-code elimination, inlining, lowering steps. The
//! pieces those passes share — running in a defined order, repeating until the
//! program stops changing, stopping cleanly when one fails, recording what
//! happened — are the same regardless of *what* the passes rewrite. pass-lang is
//! that shared machinery, and nothing more.
//!
//! It is generic over the unit a pass rewrites. A [`Pass<T>`] transforms a `T` —
//! an intermediate representation, a single function, a whole module, an abstract
//! syntax tree, or a struct bundling an IR with the diagnostics and analysis a
//! pass needs. A [`PassManager<T>`] holds passes in registration order and runs
//! them; it is the scheduler and never touches the unit itself. The crate owns no
//! IR and wires no first-party dependency — the same shape as LLVM's pass manager
//! (generic over Module / Function / Loop) or Cranelift's pass pipeline.
//!
//! ## Running a pipeline
//!
//! Implement [`Pass`] for each transform, register the transforms with
//! [`PassManager::add`], and run them — once with [`PassManager::run`], or
//! repeatedly to a fixpoint with [`PassManager::run_to_fixpoint`]. Each run
//! returns a [`Report`] of what every pass did.
//!
//! ## Example
//!
//! ```
//! use pass_lang::{Outcome, Pass, PassError, PassManager};
//!
//! // The unit is whatever a pass rewrites — here, a list of integers.
//! struct DropZeros;
//! impl Pass<Vec<i64>> for DropZeros {
//!     fn name(&self) -> &'static str {
//!         "drop-zeros"
//!     }
//!     fn run(&mut self, unit: &mut Vec<i64>) -> Result<Outcome, PassError> {
//!         let before = unit.len();
//!         unit.retain(|&x| x != 0);
//!         Ok(Outcome::from_changed(unit.len() != before))
//!     }
//! }
//!
//! let mut pm = PassManager::new();
//! pm.add(DropZeros);
//!
//! let mut unit = vec![0, 1, 0, 2, 3];
//! let report = pm.run(&mut unit).expect("the pass does not fail");
//!
//! assert_eq!(unit, vec![1, 2, 3]);
//! assert_eq!(report.changes(), 1);
//! ```
//!
//! ## Features
//!
//! - `std` (default) — the standard library; without it the crate is `#![no_std]`
//!   and needs only `alloc`.
//! - `serde` — derives `serde::Serialize` for the reporting types ([`Outcome`],
//!   [`PassRun`], [`Report`]) so a run report can be logged or inspected.
//!
//! ## Stability
//!
//! The public surface is frozen and stable as of `1.0.0`: it follows Semantic
//! Versioning, with no breaking changes before `2.0`. The full surface and the
//! SemVer promise are catalogued in
//! [`docs/API.md`](https://github.com/jamesgober/pass-lang/blob/main/docs/API.md#semver-promise).

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]
#![forbid(unsafe_code)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::unreachable,
    clippy::dbg_macro,
    clippy::print_stdout,
    clippy::print_stderr
)]

extern crate alloc;

mod error;
mod manager;
mod pass;
mod report;

pub use error::PassError;
pub use manager::PassManager;
pub use pass::{Outcome, Pass};
pub use report::{PassRun, Report};

/// Compiles and runs the `rust` code blocks in `README.md` and `docs/API.md` as
/// part of `cargo test`, so the published examples cannot drift from the API.
///
/// Present only while collecting doctests (`#[cfg(doctest)]`); it is not part of
/// the public surface and does not appear in the built library or its docs.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
#[doc = include_str!("../docs/API.md")]
pub struct MarkdownDocTests;
