//! Shared ids, errors, and policy enums for the ring family — no ring logic.
//!
//! Tier 0 of the ring family's 33 crates — the concurrency write-path implementation.
//! Every other crate in the family depends on this one; this one depends on
//! nothing, including no error crate — that is what lets tier 0 compile in
//! isolation and what makes the family's dependency forest acyclic by
//! construction.
//!
//! The "no ring logic" in the description is a rule, not a summary.
//! `docs/decision/121_workstream_008_contract_gaps_ruled.md` § 5 rules that this
//! crate owns the *discriminants* of [`WaitKind`] and [`OverflowPolicy`] while
//! `ring_wait` and `ring_overflow` own the handlers that act on them. A
//! behaviour that dispatches on a policy does not belong here.
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | `id` | [`Seq`] and [`SlotIndex`] — the never-wrapping position and the wrapping one |
//! | `capacity` | [`Capacity`] — a slot count validated to a power of two |
//! | `policy` | [`WaitKind`] and [`OverflowPolicy`] — the two configuration enums |
//! | `error` | [`RingError`] — the one error type the family returns |
//!
//! Features delivered here: `docs/feature/167_sequence_slot_index_and_power_of_two_capacity.md`
//! (the vocabulary half), `docs/feature/173_wait_kind_and_strategies.md` and
//! `docs/feature/174_overflow_policy_enum_and_handlers.md` (the enum halves).

#![no_std]
#![deny(missing_docs)]

// Core-only, and now says so. Rationale at `ring_overflow/src/lib.rs`'s own
// attribute — the property is transitive, so it is asserted in all three of this
// crate, `ring_stats`, and `ring_overflow` or in none of them. This is the one
// that matters most: 33 crates depend on it, so a `use std::` here would be a
// `std` dependency for the entire family.

mod capacity;
mod error;
mod id;
mod policy;

pub use capacity::Capacity;
pub use error::RingError;
pub use id::{Seq, SlotIndex};
pub use policy::{OverflowPolicy, WaitKind};
