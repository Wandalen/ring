# lifecycle

Two lives, and only one of them is the set's. The type itself has a degenerate
lifecycle — construct, read, drop, with no state in between. The lifecycle worth
documenting is the sequence the set *causes*: a producer advancing until a
stalled consumer stops it, which is the sequence this crate is built to gate
correctly.

### Overview Table

| ID | Name | Span |
|----|------|------|
| 001 | [A Set from Construction to Drop](001_a_set_from_construction_to_drop.md) | `new` → borrows handed out → reads → deallocation, with one state throughout |
| 002 | [The Producer Walking a Lap Against a Stall](002_the_producer_walking_a_lap_against_a_stall.md) | Eight admissions, a refusal, a released slot, one more admission |

### The Two, Side by Side

| | The set's | The producer's |
|--|-----------|----------------|
| States | 1 | Unbounded — a position per step |
| Transitions | 2 (construct, drop) | One per claim and one per consumer store |
| Driven by | The ring that owns it | The caller's loop, which this crate does not contain |
| Ends | When the ring drops | Only if the caller bounds it |

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| GT31 | An ungated set | n/a — observation | It allocates nothing at all — `Vec::with_capacity( 0 )` reports the alignment as its pointer, so the shape this crate argues hardest about is free to represent |
| GT32 | The absent `Drop` | n/a — observation | Four `Drop` impls exist in all 33 crates and all four are publish-on-drop guards; the gating chain from `GatingSet` down to `AtomicSeq` has no destructor at any level, so a set's end of life is a deallocation with no consumer notified |
| GT33 | The lifecycle itself | n/a — observation | Construct, read, drop. With no `&mut self` method and no `Drop` impl there is no state transition between the first and the last, so the structure has no lifecycle in the usual sense — all the motion is inside cursors it owns and does not drive |
| GT34 | The retry loop | n/a — observation | The crate contains zero loops, spin hints, yields and sleeps, eight lines below documentation prescribing a retry loop. All three matches for those words are in doc comments |
| GT35 | `ring_poll::push_within` | n/a — observation | The family's one bounded retry over a refusal, and it consumes no part of the error taxonomy: `try_push` returns `Result< (), T >`, so there is no `RingError` to classify — which is *why* `is_transient` has 27 test callers and no production caller |
| GT36 | `is_transient`'s documentation | **misleading doc** | It opens with "without anything else changing" and closes with "a peer's progress clears" — only the second is true, and a caller trusting the first live-locks against a stalled consumer |
| GT37 | The lap test | n/a — coverage | `a_producer_never_passes_the_limit_over_a_full_lap_with_batches` is the only one of twenty-three that walks a lap, so the cyclic behaviour this instance traces rests on a single test; the other twenty-two assert points rather than motion |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the constructor --'
command grep '^  pub fn new' ring_gating/src/lib.rs
echo '  -- destructors, mutators, loops, spin hints, yields and sleeps here --'
command grep -cE 'impl.*Drop for|&mut self|loop|spin_loop|yield_now|sleep' ring_gating/src/lib.rs || true
echo '  -- all of which are doc mentions --'
command grep -E 'loop|spin_loop|yield_now|sleep' ring_gating/src/lib.rs
echo '  -- control: the identical Drop expression where the family does write one --'
command grep -rE 'impl.*Drop for' --include=*.rs ring_*/src/ | sed 's|ring/||'
```

Live output:

```
  -- the constructor --
  pub fn new( capacity : Capacity, consumers : usize ) -> Self
  -- destructors, mutators, loops, spin hints, yields and sleeps here --
3
  -- all of which are doc mentions --
//! moves, and a retry loop that could not tell them apart would spin forever on
  /// loop must stop rather than spin. [`RingError::Full`] when the claim would
  /// loop should keep going.
  -- control: the identical Drop expression where the family does write one --
ring_mpsc/src/lib.rs:impl< S > Drop for Reserved< '_, S >
ring_mpsc/src/lib.rs:impl< S > Drop for Batch< '_, S >
ring_spsc/src/lib.rs:impl< S > Drop for Reservation< '_, S >
ring_spsc/src/lib.rs:impl< S > Drop for Batch< '_, S >
```

**One constructor, no destructor, no mutator, and no loop.** The three matches
in the second arm are all in doc comments — the crate prescribes a retry loop it
does not contain. The control finds four `Drop` impls in the family and all four
are borrow guards in the two rings, so the zero here is a real absence rather
than a missed pattern.
