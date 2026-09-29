//! Family-neutral validation machinery for the ring family.
//!
//! Holds the seeded workload generator and the byte-parity oracle that grade
//! the `ring_*` family, plus the stage gates under `gate/`. Depends on no
//! `ring_*` crate and must never gain such a dependency: a grader that imports
//! what it grades cannot run until the graded thing already builds, which
//! would make every stage gate unrunnable until the last stage finished.
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | `workload` | [`Workload`] — the seeded item sequence, reproducible from its seed alone |
//! | `accumulator` | [`Accumulator`] and [`Write`] — the two semantics a write sequence folds under |
//! | `oracle` | [`ByteParity`] and [`Parity`] — whether two completed tables agree, and where they stop |
//!
//! The two components answer one question between them. Workstream 008's smoke
//! test is *"all patterns produce byte-identical final tables"*, which needs a
//! sequence every candidate can be driven with and a rule for deciding whether
//! the tables that come back agree. Neither half is meaningful alone: a
//! sequence nothing grades measures nothing, and an oracle with no common input
//! compares two different experiments.
//!
//! The design this crate must satisfy is specified under `docs/`:
//!
//! - `docs/readme.md` — scope and related crates
//! - `../../docs/plan/008_ring_write_path_staged.md` — the stages these gates grade

#![deny(missing_docs)]

mod accumulator;
mod oracle;
mod workload;

pub use accumulator::{Accumulator, Write};
pub use oracle::{ByteParity, Parity};
pub use workload::{Item, PayloadArchetype, Workload};
