# Pattern: The Guard That Makes the Next Line Legal

### Scope

- **Purpose**: Record the guard-then-subtract shape, what it costs, and where the family still uses it.
- **Responsibility**: Why D1 had to be tested before D2, what happened if it was not, and what replaced the requirement.
- **In Scope**: `check_seqs`'s subtraction; the eight unguarded subtractions left in the family's source; the profile settings that decide how they fail.
- **Out of Scope**: What D1 and D2 mean (→ [`invariant/001`](../invariant/001_cursor_invariants_over_a_live_ring.md)); the saturating arithmetic in the *ring* that motivates the crate (→ [`pitfall/001`](../pitfall/001_saturating_arithmetic_reports_health.md)).

### Abstract

A pattern that was in two lines:

```rust
if consumer.0 > producer.0 { return Err( ... ); }   // D1
if producer.0 - consumer.0 > capacity { ... }       // D2
```

The second line performed an unsigned subtraction that was sound **only** because
the first line returned. Reversing them, or inserting anything between them that
could return early, turned a diagnosis into a wraparound — and this crate ships
with no `overflow-checks`, so the wraparound would have been silent in release.

The pattern was correct as written. It was recorded because **nothing enforced
it**: not a type, not a test, not a comment at the site. It is now written a
third way, in which there is nothing left to enforce — `checked_sub` makes the
absent case the `else` branch, so D1 *is* the subtraction failing rather than a
separate block that has to come first.

**The pattern did not go away; this crate stopped being an instance of it.** The
census below finds eight unguarded subtractions still in the family's source, and
one of them — `ring_spsc`'s `capacity - occupancy` — is the same underflow
[`invariant/002`](../invariant/002_the_conservation_law_and_why_it_holds.md)'s
DB35 reaches from the other direction. Two findings arriving at one line from
opposite ends is the census earning its cost.

### Pattern

**Name:** guard-then-subtract.

**Shape:** an unsigned subtraction whose non-negativity is established by a
preceding early return rather than by the types of its operands.

**Where it appeared here:** `check_seqs`. `producer.0` and `consumer.0` are both
`u64`. The subtraction `producer.0 - consumer.0` was meaningful only in the half
of the input space where `consumer.0 <= producer.0`, and the D1 block was what
restricted the input to that half.

**Why the ordering was also the correct diagnostic ordering**, which is what made
it comfortable rather than obviously fragile: D1 is the more fundamental defect. A
consumer ahead of its producer means the ring's own arithmetic is meaningless, so
asking "how far ahead is the producer" is not a question worth answering. The
compiler-facing reason and the reader-facing reason pointed the same way, and
`api/001`'s guarantee A5 states the reader-facing one. That agreement is exactly
what made the coupling easy to leave alone for as long as it was left alone.

**What replaced it:** the same two questions, asked in one expression.
`producer.0.checked_sub( consumer.0 )` returns `None` in precisely the D1 case, so
the `else` branch *is* the D1 report and the bound check runs on a value that
cannot have wrapped. The diagnostic ordering is preserved — D1 still wins a pair
that breaks both — but it is now a consequence of the arithmetic rather than of
the source order, and there is no second block that could be moved.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
S=ring_debug/src/lib.rs
echo '-- what check_seqs asks now, and in what order --'
awk '/^fn check_seqs/{ i = 1 } i && /let Some/{ p = 1 } p; p && /^  Ok/{ exit }' $S
echo '-- unguarded subtractions still in the family source --'
command grep -rE '[]a-zA-Z0-9_)] - [a-zA-Z0-9_(]' --include='*.rs' ring_*/src \
  | command grep -vE ': *//' | sed 's|ring/||; s|: *|  |' | sort
printf 'unguarded:  %s\n' "$( command grep -rnE '[]a-zA-Z0-9_)] - [a-zA-Z0-9_(]' --include='*.rs' ring_*/src | command grep -vcE ': *//' )"
printf 'checked:    %s\n' "$( command grep -rhoE '\.checked_sub\(' --include='*.rs' ring_*/src | wc -l )"
printf 'saturating: %s\n' "$( command grep -rhoE '\.saturating_sub\(' --include='*.rs' ring_*/src | wc -l )"
echo '-- what governs them in each profile --'
printf 'overflow-checks anywhere in the root manifest: %s\n' "$( command grep -c 'overflow-checks' Cargo.toml || true )"
printf 'profile sections in the root manifest:         %s\n' "$( command grep -c '^\[profile' Cargo.toml || true )"
echo '-- and whether any test pins the diagnostic ordering --'
T=ring_debug/tests/debug_test.rs
printf 'tests naming ProducerLappedConsumer: %s\n' "$( command grep -c 'ProducerLappedConsumer' $T )"
printf 'tests naming ConsumerAheadOfProducer: %s\n' "$( command grep -c 'ConsumerAheadOfProducer' $T )"
```

Live output:

```
-- what check_seqs asks now, and in what order --
  let Some( pending ) = producer.0.checked_sub( consumer.0 )
  else
  {
    return Err( Violation::ConsumerAheadOfProducer { producer, consumer } );
  };

  if pending > capacity.get() as u64
  {
    return Err
    (
      Violation::ProducerLappedConsumer { producer, consumer, capacity : capacity.get() }
    );
  }

  Ok( () )
-- unguarded subtractions still in the family source --
ring_bench/src/lib.rs  self.offered - self.received
ring_bench/src/lib.rs  self.reported - self.received
ring_bench/src/lib.rs  stats.record_drop( workload.config().overflow(), ( offered - received ) as u64 );
ring_core/src/lib.rs  ProducerInner::Crossbeam( queue ) => queue.capacity() - queue.len(),
ring_mpsc/src/lib.rs  if end == from { None } else { Some( Seq( end.0 - 1 ) ) }
ring_poll/src/lib.rs  self.lost += offered - moved;
ring_types/src/capacity.rs  self.0 - 1
unguarded:  7
checked:    3
saturating: 6
-- what governs them in each profile --
overflow-checks anywhere in the root manifest: 0
profile sections in the root manifest:         8
-- and whether any test pins the diagnostic ordering --
tests naming ProducerLappedConsumer: 3
tests naming ConsumerAheadOfProducer: 6
```

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_cursor_invariants_over_a_live_ring.md](../invariant/001_cursor_invariants_over_a_live_ring.md) | D1 and D2 as invariants, independent of the order they are tested in |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_checking_a_pair_without_touching_it.md](../algorithm/001_checking_a_pair_without_touching_it.md) | The read strategy the two blocks operate on |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_saturating_arithmetic_reports_health.md](../pitfall/001_saturating_arithmetic_reports_health.md) | The same failure shape one level up — arithmetic that hides rather than reports |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | `check_seqs` |

### Tests

| Test | Relationship |
|------|--------------|
| `a_pair_breaking_both_reports_the_invisible_one` | The diagnostic ordering, asserted — and the closest the suite comes to pinning the positional requirement without pinning it |
| `a_consumer_ahead_of_its_producer_is_caught` | The D1 half, which is exactly the input on which the reordered version would wrap |
| `a_producer_exactly_a_lap_ahead_is_full_not_lapped` | The D2 boundary, where the subtraction's result equals the capacity |

### DB9 — the soundness of the subtraction was positional, and now there is no position

**What was found.** The requirement "D1's block precedes D2's block" was
load-bearing and was recorded in exactly zero places that a change would have to
pass through:

| Would it catch a reordering? | |
|---|---|
| The type system | No — both are `u64`; the subtraction compiles either way |
| A debug build | Yes, by panic — `u64` subtraction overflow panics under `debug-assertions` |
| A release build | **No** — no `overflow-checks` in any profile in the workspace root manifest, so it wraps to a near-`u64::MAX` value |
| The test suite | **No** — `a_pair_breaking_both_reports_the_invisible_one` asserts *which* violation is reported when both hold, which is the diagnostic ordering; a pair breaking only D1 would wrap and return `ProducerLappedConsumer` with an absurd capacity, and no test constructs the assertion that would catch it |
| A comment at the site | No — the comment on `check_seqs` explains why it is *split out* from `check`, not why its two halves are ordered |

**And the wrapped value is the worst possible one.** `producer.0 - consumer.0`
with the operands the wrong way round wraps to something on the order of `u64::MAX`,
which is greater than any `capacity`, so the reordered function returns
`Err( ProducerLappedConsumer )` — a violation, plausibly shaped, for a ring whose
actual defect is D1. The failure is not a crash or an obviously-wrong number; it
is **the wrong diagnosis, delivered confidently**, in the crate whose entire job
is to deliver the right one.

That is the same failure shape as
[`pitfall/001`](../pitfall/001_saturating_arithmetic_reports_health.md), one level
up: there the *ring's* saturating arithmetic hides a defect from a caller, here
the *checker's* wrapping arithmetic would misname one. A crate written to catch
arithmetic that lies was one edit away from lying the same way.

**The row that mattered is the one every entry in the table shares: none of them
is a mechanism.** A comment records the requirement for a reader who is already
looking; a test would catch a reordering only if someone wrote the one assertion
nobody had thought to write. The table was five ways of saying that the ordering
was held in place by whoever read it last. So the fix is not another row — it is
removing the thing the rows were failing to guard.

`producer.0.checked_sub( consumer.0 )` returns `None` in exactly the D1 case.
That makes D1 the `else` branch of the subtraction rather than a block that has
to precede it: there is no second block to move, no operand order to reverse, and
no state in which the bound check runs on a wrapped value. The type system now
answers "would it catch a reordering?" with *there is no reordering to catch*,
which is the only answer in that column that does not depend on somebody
remembering.

**What it did not fix is the more interesting half.** The shape is not this
crate's; it is the family's. Eight unguarded subtractions remain in `ring_*/src`,
and the census in the recipe above lists every one. `ring_spsc`'s
`capacity - occupancy` is the same line DB35 arrives at from the conservation-law
side, and `ring_mpsc`'s `Seq( end.0 - 1 )` underflows on `end == Seq( 0 )` unless
its own guard holds — which is this pattern again, one crate over, unexamined.

**Disposition:** applied — `check_seqs` uses `producer.0.checked_sub( consumer.0 )`
with the `None` branch reporting `ConsumerAheadOfProducer`, so the D1-before-D2
ordering the finding was about is no longer expressible rather than merely
documented, and the diagnostic preference it also encoded is preserved by
`a_pair_breaking_both_reports_the_invisible_one`. The eight remaining unguarded
subtractions in the family are now counted rather than assumed absent — the
census is what corrected this document's own earlier count of seven, which it
had been carrying while its recipe printed eight directly beneath it — a count
since dropped again to seven, per the correction below.
Now prints: `unguarded:  7`

**Correction (2026-09-28):** `ring_spsc`'s `capacity - occupancy` — named above
and in the Abstract as one of the eight — has since been guarded
(`Fix(free_capacity_underflow_on_a_precondition_violation)` in
`ring_spsc/src/lib.rs`, now `saturating_sub`), so the census the Regenerate
recipe prints has dropped to seven and no longer names `ring_spsc`. The Abstract
and the paragraph above describe the family as it stood when written;
`ring_mpsc`'s `Seq( end.0 - 1 )` is untouched by this fix and is still one of the
seven, unexamined. The matching correction to the line DB35 reaches from the
conservation-law side is in
[`invariant/002`](../invariant/002_the_conservation_law_and_why_it_holds.md); the
one to the Error Handling row it feeds is in
[`integration/001`](../integration/001_reaching_the_cursors_of_a_live_ring.md)'s
DB29.

### DB10 — `debug-assertions` is doing the work an assertion should


The one thing that would catch the reordering is a debug build, and it catches it
by panicking inside a diagnostic — which is a poor place to learn about it, since
a `check` that panics is strictly worse than one that returns a wrong `Violation`
for any caller running checks in a test teardown.

**The family relies on this mechanism family-wide and has never chosen it.** No
profile in the workspace root manifest sets `overflow-checks` either way, so every
`usize`/`u64` subtraction in all 33 crates panics in `dev` and wraps in `release`
by default rather than by decision. This crate is where the consequence is
sharpest, because it is the crate whose output is a claim *about* arithmetic.

The cheap fix is not a profile change — it is one line at the subtraction site
turning the positional requirement into a checked one, at a cost of nothing in a
function that already branches twice. It is recorded rather than applied because
changing `check_seqs` mid-corpus would invalidate the very measurements this
instance rests on; the finding is the record that the choice is open.
