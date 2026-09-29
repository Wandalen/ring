//! Feature 179's compile-fail suite — the family's only test that gets redder
//! as the surface grows.
//!
//! Every other reached-test in this family asserts that something happens.
//! This one asserts that seven programs are *rejected*, which is the only shape
//! that fails when a method is **added** — see
//! `docs/non_functional_requirement/001_proven_by_code_that_must_not_compile.md`
//! for why that asymmetry is the point rather than a curiosity.
//!
//! # Why this is its own test binary
//!
//! `trybuild` spawns a nested `cargo` build. Keeping it out of
//! `tests/handle_test.rs` means the fast suite stays fast and a compiler
//! upgrade that shifts diagnostic wording fails one binary, not the crate's
//! whole test surface.
//!
//! # When a case fails after a toolchain upgrade
//!
//! The pinned `.stderr` files record rustc's exact wording, which is not
//! stable across releases. The question to ask is not "does the message
//! match" but **"is the program still rejected"** — that is the actual
//! requirement. `TRYBUILD=overwrite cargo test --test ui_test` regenerates the
//! files, and is also exactly how a real regression would get accepted, so a
//! changed `.stderr` is a change to what this crate guarantees.
//!
//! # When a case fails because a crate *moved*
//!
//! Fix(the_forbidden_programs_are_rejected): `producer_shared_across_threads`
//! failed from the moment this workstream's crates moved out of `module/` into
//! `ring/`. Root cause: a pinned `.stderr` records rustc's `--> $WORKSPACE/…`
//! note lines, which spell the path of every crate in the required-because
//! chain. `$WORKSPACE` normalizes the repo root and nothing below it, so three
//! lines went on naming `module/ring_spsc` and `module/ring_core` after both
//! had moved. Pitfall: this presents exactly like the toolchain drift above,
//! and that section sends you to `TRYBUILD=overwrite` — which would have
//! accepted the relocation *and* any real diagnostic change riding in the same
//! file, silently. Diff the expected and actual blocks first. If every
//! differing line is a `-->` path, edit those lines and leave the rest; the
//! program is still rejected for the same reason, which is the only thing this
//! suite actually guarantees.

#![ cfg( test ) ]

/// The seven programs that must not compile.
///
/// | Case | Asserts | Origin |
/// |---|---|---|
/// | `producer_drains` | No drain on the publishing end | [Feature 179](../../../docs/feature/179_producer_and_consumer_handles.md)'s stated criterion |
/// | `consumer_publishes` | No publish on the draining end | Feature 179's stated criterion |
/// | `producer_clones` | No second producer | This crate — V3, a data race if it lands |
/// | `consumer_clones` | No second consumer | This crate — C5, the same defect wearing "it's only reading" |
/// | `producer_shared_across_threads` | Not `Sync` | This crate — no static assertion can express a negative bound |
/// | `producer_try_clones` | `ring_core`'s duplication is not forwarded | This crate — N2, and the `Deref` detector |
/// | `ring_used_after_split` | The ring is gone once split | This crate — T1, the by-value narrowing |
///
/// **Five of the seven are additions**, and they are the ones covering the
/// edits that actually get made. The two the acceptance table names cover the
/// edit nobody makes: a drain on `Producer` is obviously wrong to its author.
#[ test ]
fn the_forbidden_programs_are_rejected()
{
  let cases = trybuild::TestCases::new();
  cases.compile_fail( "tests/ui/producer_drains.rs" );
  cases.compile_fail( "tests/ui/consumer_publishes.rs" );
  cases.compile_fail( "tests/ui/producer_clones.rs" );
  cases.compile_fail( "tests/ui/consumer_clones.rs" );
  cases.compile_fail( "tests/ui/producer_shared_across_threads.rs" );
  cases.compile_fail( "tests/ui/producer_try_clones.rs" );
  cases.compile_fail( "tests/ui/ring_used_after_split.rs" );
}
