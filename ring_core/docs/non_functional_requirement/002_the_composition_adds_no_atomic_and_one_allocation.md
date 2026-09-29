# Non-Functional Requirement: The Composition Adds No Atomic and One Allocation

### Scope

- **Purpose**: State the two cost thresholds this crate holds itself to as a composition layer — no atomic of its own, and no allocation on the in-house paths — and measure both.
- **Responsibility**: The thresholds, their measurement recipes, and the one place the second is not met.
- **In Scope**: `src/lib.rs`'s atomic and allocation footprint, per backend arm.
- **Out of Scope**: Why the atomic threshold exists (→ [`../invariant/001`](../invariant/001_no_atomic_of_its_own.md)); the backend swap requirement (→ [`001`](001_backend_swap_is_a_build_flag.md)).

### Threshold 1 — Zero Atomics

`ring_spsc` asserts zero read-modify-writes across a run and every publish
reaches it through a method here, so a counter added in this crate would break
that assertion with nothing in `ring_spsc`'s own dependency tree to blame.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_core
printf 'Atomic in code:     '; grep -vE '^\s*(//|///|//!)' src/lib.rs | grep -c 'Atomic'
printf 'Atomic in prose:    '; grep -c 'Atomic' src/lib.rs
printf 'unsafe:             '; grep -c 'unsafe' src/lib.rs
printf 'ring_stats dep:     '; grep -c 'ring_stats' Cargo.toml
# The command above can legitimately exit nonzero -- a count of zero, a diff
# that differs -- and the sweep that keeps this output current skips any block
# that exits nonzero, so without this the block freezes instead of refreshing.
:
```

Live output:

```
Atomic in code:     0
Atomic in prose:    1
unsafe:             0
ring_stats dep:     0
```

**Met.** The single `Atomic` occurrence is the module documentation saying there
is none.

### Threshold 2 — No Allocation on the In-House Paths

```sh
cd "$(git rev-parse --show-toplevel)"/ring_core
grep 'Vec\|collect\|to_vec\|Box' src/lib.rs | grep -vE ':[0-9]+: *(//|///|//!)'
```

Live output:

```
  /// let mut out = Vec::new();
  pub fn try_recv_batch( &mut self, out : &mut Vec< T > ) -> usize
        let taken = ( 0..len ).filter_map( | _ | queue.pop() ).collect::< Vec< _ > >();
```

**Met for the two in-house arms and not for crossbeam.** `try_recv_batch`'s
`Vec` parameter is the caller's, extended in place. Its crossbeam arm is the
exception: it collects into an intermediate `Vec` before extending, because
`ArrayQueue::pop` cannot be bounded any other way without risking non-termination
under a live producer.

That intermediate is one allocation per batch drain, on the backend the module
documentation already calls interim. Recording it here rather than in
`workaround/001` keeps the threshold honest — the requirement is met on the paths
the family ships and missed on the one it means to replace.

### CO41 — The One Allocation Is on the Backend Meant to Be Replaced

```sh
cd "$(git rev-parse --show-toplevel)"/ring_core
grep 'Vec\|collect\|to_vec\|Box' src/lib.rs | grep -vE '^ *(//|///|//!)'
```

Live output:

```
  pub fn try_recv_batch( &mut self, out : &mut Vec< T > ) -> usize
        let taken = ( 0..len ).filter_map( | _ | queue.pop() ).collect::< Vec< _ > >();
```

Three hits. Line 585 (was 559) is a doc example. Line 590 (was 563) is the
caller's `&mut Vec< T >` parameter, which the in-house arms push into directly.
**Line 614 (was 587) is the one allocation**:
`( 0..len ).filter_map( | _ | queue.pop() ).collect::< Vec< _ > >()`. All three
citations drifted since this instance was last regenerated — most of it
pre-dating this session, plus 11 more lines from this session's own
`Fix(decision_121_link_pointed_at_docsrs_not_the_ruling)` correction to
`src/lib.rs`'s module doc comment (2026-09-11).

It exists because `ArrayQueue::pop` gives no way to bound a drain without
counting, and counting into the caller's buffer directly would leave it
partially extended if the count were wrong. The allocation buys that safety on
the backend the module documentation already calls interim
(→ [`../workaround/001`](../workaround/001_crossbeam_queue_as_interim_backend.md)).

**Disposition:** declined — this instance's own text traces the allocation to
`ArrayQueue::pop`'s lack of a bounded drain, on the backend
`workaround/001_crossbeam_queue_as_interim_backend.md` already records as
interim; the fix is that backend's eventual removal on its own documented
condition, not a change to this crate's source or
`non_functional_requirement/002_the_composition_adds_no_atomic_and_one_allocation.md`
now.

### CO42 — The Threshold Is Stated Here and Enforced by `ring_spsc`

`ring_spsc` asserts zero read-modify-writes across a run. Every publish through
a `ring_core::Ring` on the SPSC backend reaches that assertion, so an atomic
added here would break it — in `ring_spsc`'s suite, reported against
`ring_spsc`, with nothing naming this crate.

That is enforcement, and it is enforcement with the wrong return address. A
failure would send a reader to the crate that did not change. Recorded alongside
CO24, which records the same gap from the invariant's side.
