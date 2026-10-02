# `ring_spsc` opts out of the workspace unsafe-code deny so two threads can use disjoint slots of one array

Status: Accepted

## Context

The workspace sets `unsafe-code = "deny"` and `undocumented_unsafe_blocks = "deny"`. `ring_spsc::Ring` is one slot
array that the producer writes while the consumer reads, each through a shared reference. `UnsafeCell` is Rust's only
way to mutate through a shared reference, and no safe type expresses "two threads, one array, index sets that are
disjoint at runtime". The sets here are disjoint because of cursor values. The producer writes only the slot it has
claimed and not yet published, and the consumer reads only slots that are published and not yet committed.
`slice::split_at_mut` splits at a fixed index, while this boundary moves with every publish and commit.

Because the invariant is stated in cursors, only a crate that holds both the storage and the cursors can encapsulate
it. `ring_store` and `ring_slot` hold storage only. `bench_harness/gate/declared/ring/unsafe_allowlist.txt` names this
crate, and gate G6 requires this file to justify the opt-out.

## Decision

`ring_spsc` carries `#![allow(unsafe_code)]`, and its unsafe code is confined to these items:

- `unsafe impl<S: Send> Sync for Ring<S>`, because `Ring` holds `Buffer<UnsafeCell<S>>` and the compiler will not
  derive `Sync` through an `UnsafeCell`;
- the private `unsafe fn Ring::slot` and `Ring::slot_mut`, each one deref of a single slot's `UnsafeCell`;
- their call sites in `Reservation`'s `Deref` and `DerefMut` impls and in `Batch::get` and `Batch::get_mut`.

These rules bound it:

- No unsafe API is public. A `Reservation` comes only from `Producer::claim`, and a `Batch` only from
  `Consumer::drain` or `Consumer::drain_up_to`. Each checks the precondition before it builds the guard.
- `slot` requires the consumer, on a sequence bounded by a `ring_cursor::GATING` load. `slot_mut` requires the slot's
  sole owner, which is either the producer on its claimed, unpublished sequence or the consumer through
  `Batch::get_mut`. Each function's `# Safety` section states this, and every block has a `// SAFETY:` comment.
- Visibility comes from the `ring_spsc::HANDOFF` (`Release`) store on one end's cursor, paired with the other end's
  `GATING` (`Acquire`) load.
- The `UnsafeCell` wraps each slot, not the whole `Buffer`. The whole-buffer form materialised `&mut Buffer<S>` over
  the entire allocation on every slot access, and Miri reported a retag conflict between a producer and a consumer
  that touched different slots.
- `Ring::split` takes `&mut self`, neither end is `Clone`, and a `PhantomData<Cell<()>>` field makes both ends `Send`
  and not `Sync`. Each split therefore yields exactly one producer and one consumer.

The bound is `S: Send`, not `S: Sync`. A record is written on the producer's thread and read on the consumer's, so it
moves between threads and is never used from both at once. `S: Sync` would be stricter than the argument needs and
would exclude payload types that are `Send` but not `Sync`.

`ring_mpsc` carries the identical `unsafe impl` line on a different argument, many producers and a `Producer` that
must be `Sync`. See [its record](../../../ring_mpsc/docs/workaround/readme.md).

## Alternatives considered

- **Put the unsafe code in `ring_store` or `ring_slot`.** A storage-only crate cannot state the cursor invariant, so
  no safe API it offers can be sound. A `SharedBuffer::split()` was considered and found unsound for that reason.
- **One `UnsafeCell` around the whole `Buffer`.** Unsound, as Miri showed.
- **Safe code only.** Nothing safe expresses runtime-disjoint access to one array, so the ring could not be written.

## Consequences

- The argument that makes the crate sound lives in comments and in the guard types. The compiler checks none of the
  disjointness.
- `both_ends_are_send_and_neither_is_sync` in `tests/spsc_test.rs` asserts the `Send` half. The negatives the
  argument rests on (no `Clone`, no second split while a pair is live, neither end `Sync`, no batch borrow outliving
  its commit) are `compile_fail` doctests in the crate's `//!`. They run only under `cargo test --doc`, which
  `just test` and CI run. A nextest-only run, such as `just test_only`, checks none of them.
- The loom model, `mod exhaustive` in `tests/spsc_test.rs`, runs only under `--cfg loom`. The Loom CI workflow runs it
  on `master` pushes and nightly, and no recipe runs it. No recipe or workflow runs Miri, which is what found the
  whole-buffer aliasing.
- Delete the opt-out when a safe abstraction below this crate, from `std` or a dependency, can express exclusive access
  to one element of a shared array gated by a runtime cursor comparison. None exists. The `Sync` impl retires with
  it, since the compiler can derive `Sync` only once that exclusivity is expressed in a type.
