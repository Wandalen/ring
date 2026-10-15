# `ring_mpsc` opts out of the workspace unsafe-code deny so many producers can write disjoint slots of one array

Status: Accepted

## Context

The workspace sets `unsafe-code = "deny"` and `undocumented_unsafe_blocks = "deny"`. In `ring_mpsc::Ring`, any number
of producers write slots while the one consumer reads others, all through shared references to one allocation.
`ring_mpsc::Producer` is `Copy` and `Sync` on purpose, since giving each thread its own copy is what multi-producer
means. Which producer may write which slot is a runtime fact about cursor values, settled by a compare-exchange on
the claim cursor: `ring_claim::Claimer::claim` for the ordinary producers, the primary's guessed exchange for
`PrimaryProducer`. The borrow checker reasons about scopes and cannot see it.

The invariant is stated in the claim cursor, the per-slot stamps and the consumer cursor. `ring_store` and `ring_slot`
hold none of those, so the unsafe code has to live in a crate that holds both the storage and the cursors.
`bench_harness/gate/declared/ring/unsafe_allowlist.txt` names this crate, and gate G6 requires this file to justify
the opt-out.

## Decision

`ring_mpsc` carries `#![allow(unsafe_code)]`, and its unsafe code is confined to these items:

- `unsafe impl<S: Send> Sync for Ring<S>`, because the slots are `Buffer<UnsafeCell<S>>` and the compiler will not
  derive `Sync` through an `UnsafeCell`;
- the private `unsafe fn Ring::slot` and `Ring::slot_mut`, each one deref of a single slot's `UnsafeCell`;
- their call sites in `Reserved`'s `Deref` and `DerefMut` impls and in `Batch::get` and `Batch::get_mut`.

These rules bound it:

- No unsafe API is public, and callers carry no obligation. A `Reserved` comes from `Producer::claim`, which grants
  after the `GatingSet` headroom check, or from `PrimaryProducer::claim`, which grants on an exchange won against
  the live cursor and a cached head that only ever lags the consumer's commit (the cache is refreshed at `GATING`).
  A `Batch` comes only from `Consumer::drain` or `Consumer::drain_up_to`,
  which stop at the first slot whose stamp, read at `OBSERVE`, does not equal its sequence. So `slot_mut` is reached
  only for a claimed, unpublished sequence or a published, uncommitted one.
- There is one consumer. `Ring::ends` takes `&mut self`, `Ends::split` takes `&mut self`, and `Consumer` is neither
  `Clone` nor `Sync`.
- Happens-before runs both ways. The `PUBLISH` store and `OBSERVE` load on a stamp carry the payload write to the
  consumer. The `COMMIT` store and `ring_cursor::GATING` load on the consumer cursor carry the payload read back to
  the producers before a slot is reused.
- The `UnsafeCell` wraps each slot, not the whole `Buffer`. The whole-buffer form materialised `&mut Buffer<S>` over
  the entire allocation, so two producers writing different slots aliased all of it, and Miri reported a retag
  conflict.

Maintainers carry two obligations. `Ring::ends` must keep its `&mut self` receiver, because the `Sync` argument rests
on there being one consumer, and the only guard is a `compile_fail` doctest that holds two `Ends` at once. The stamp
test in `Ring::contiguous_end` must stay an equality, because `>=` would read every unwritten stamp as published and
`!= UNSTAMPED` would read a previous lap's stamp as published.

The `Sync` argument also assumes `Ring::ends` runs once per ring. A second call restarts the claim cursor at zero while
the consumer cursor and stamps keep their values, so two live claims can address one slot. `Ring::ends` documents this
as a pitfall, and it stays unsound until the claim cursor carries over between calls.

The bound is `S: Send`, not `S: Sync`. A record is written on a producer's thread and read on the consumer's, so it
moves between threads and is never shared.

`ring_spsc` carries the identical `unsafe impl` line on a different argument, exactly two threads and ends that must
not be `Sync`. See [its record](../../../ring_spsc/docs/workaround/readme.md).

## Alternatives considered

- **Put the unsafe code in `ring_store` or `ring_slot`.** A storage-only crate cannot state the cursor invariant. A
  `SharedBuffer::split()` was considered and found unsound for that reason.
- **One `UnsafeCell` around the whole `Buffer`.** Unsound, as Miri showed.
- **Safe code only.** Nothing safe expresses "each producer has exclusive access to the one slot it claimed", so the
  ring could not be written.

## Consequences

- The `compile_fail` guards in the crate's `//!` (no second consumer by `Clone`, `Consumer` not `Sync`, no second
  split, no second `Ends`, no `Reserved` outliving the ring it borrows) run only under `cargo test --doc`, which
  `verb/test` and CI run. A nextest-only run, such as `verb/test_only`, checks none of them.
- `COMMIT`'s `Release` has no behavioural check. Only the test
  `the_orderings_are_the_ones_the_publication_invariant_names` reads the constant. The crate's `tests/manual/readme.md`
  records that weakening it to `Relaxed` survived both loom models and 120 hardware runs on aarch64.
- The loom models, `mod exhaustive` in `tests/mpsc_test.rs`, cover the publish edge and the drain bound and run only
  under `--cfg loom`. The Loom CI workflow runs them on `master` pushes and nightly, and no verb runs them. No verb or
  workflow runs Miri.
- Delete the opt-out when a safe abstraction, from `std` or a dependency, can express exclusive access to one element
  of a shared array gated by a runtime cursor comparison. None exists. The `Sync` impl retires with it.
