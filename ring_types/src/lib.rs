//! Shared ids, errors, and policy enums for the ring family, with no ring logic.
//!
//! Tier 0 of the ring family's 33 crates, which implement the concurrency write-path.
//! Most of the family depends on this one; this one depends on nothing, not
//! even an error crate. That is what lets tier 0 compile in isolation and what
//! makes the family's dependency forest acyclic by construction.
//!
//! The "no ring logic" in the description is a rule, not a summary. This
//! crate owns the *discriminants* of [`WaitKind`] and [`OverflowPolicy`] while
//! `ring_wait` and `ring_overflow` own the handlers that act on them. A
//! behaviour that dispatches on a policy does not belong here.
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | `id` | [`Seq`], the never-wrapping position, and [`SlotIndex`], the wrapping one |
//! | `capacity` | [`Capacity`], a slot count validated to a power of two |
//! | `policy` | [`WaitKind`] and [`OverflowPolicy`], the two configuration enums |
//! | `error` | [`RingError`], the one error type the ring path returns |
//!
//! Features delivered here: the vocabulary half of the sequence-to-slot-index
//! and power-of-two-capacity feature, and the enum halves of the wait-kind and
//! overflow-policy features.

#![no_std]
#![deny(missing_docs)]

// Core-only, and now says so. The rationale is at `ring_overflow/src/lib.rs`'s own
// attribute. The property is transitive, so it is asserted in all three of this
// crate, `ring_stats`, and `ring_overflow` or in none of them. This is the one
// that matters most, since most of the family depends on it. A `use std::` here
// would be a `std` dependency for every crate that does.

mod capacity;
mod error;
mod id;
mod policy;

pub use capacity::Capacity;
pub use error::RingError;
pub use id::{Seq, SlotIndex};
pub use policy::{OverflowPolicy, WaitKind};
