# Invariant: `is_published` Is Exclusive of the Frontier

### Scope

- **Purpose**: State the read-side invariant — `is_published( seq )` is exactly `seq < published()`, exclusive at the boundary — and record the off-by-one it is written against.
- **Responsibility**: State the invariant, show the two exhaustive tests that establish it, explain why the exclusive convention rather than inclusive, and trace what an inclusive reading would produce.
- **In Scope**: `is_published` (`src/lib.rs:229-232`) and its relationship to `published`.
- **Out of Scope**: How the frontier gets where it is — see [`invariant/001`](001_the_frontier_moves_only_by_compare_exchange.md).

### The Invariant

```rust
pub fn is_published( &self, seq : Seq ) -> bool
{
  seq < self.published()
}
```

For every `seq` and at every moment: `is_published( seq )` is true if and only if
`seq` is strictly less than the current frontier. The frontier itself is **not**
published. `published()` is *one past* the last readable sequence — the same
convention as a Rust half-open range `0..published`, `slice.len()`, and every
other exclusive-end index in the language.

`src/lib.rs:122` states it in the accessor's own summary — *"How far publication
has reached — one past the last readable sequence"* — and `:214-215` states the
consequence the feature is graded on:

> The consumer-facing question feature 170 is graded on: a slot claimed but not
> published must answer `false`.

### PB20 — The Boundary Is Asserted Exhaustively, Twice

Two tests establish it, and they fail for different reasons.

**Point by point at a fixed frontier** — `tests/publish_test.rs:194-210`:

```rust
publisher.publish( Seq::ZERO, 3 );

assert!(  publisher.is_published( Seq::ZERO ) );
assert!(  publisher.is_published( Seq( 1 ) ) );
assert!(  publisher.is_published( Seq( 2 ) ), "the last readable one" );
assert!( !publisher.is_published( Seq( 3 ) ), "the frontier itself is not readable" );
assert!( !publisher.is_published( Seq( 4 ) ) );
```

Five points around one boundary: the last readable, the frontier, and one past
it. `:197-201` says why this is the test the doc example is not:

> The boundary the doc example gestures at and this pins down: `published()` is
> one *past* the last readable sequence, so the frontier itself must answer
> false. Off by one here hands a consumer a slot no producer has finished
> writing — the exact failure the loom model catches from the other direction.

**Every point at every frontier** — `tests/publish_test.rs:223-247`:

```rust
const REACH : u64 = 16;

for frontier in 0..REACH
{
  assert_eq!( publisher.published(), Seq( frontier ) );

  for candidate in 0..REACH + 4
  {
    assert_eq!(
      publisher.is_published( Seq( candidate ) ),
      candidate < frontier,
      "at frontier {frontier}, asking about {candidate}"
    );
  }

  publisher.publish( Seq( frontier ), 1 );
}
```

Sixteen frontiers × twenty candidates = **320 assertions**, comparing the method
against the predicate it claims to be, on both sides of a moving boundary and
four positions beyond the highest frontier reached. `:226-227` states the intent:
*"rather than trusting the two methods to stay consistent, check the whole range
on both sides of a moving frontier."*

The second test subsumes the first at every point it covers. Both are kept
because they fail differently: the first names the boundary in its assertion
messages and fails at a readable line, the second fails with a
`"at frontier N, asking about M"` message that localises an arbitrary
inconsistency. A single off-by-one in the comparison operator would fail both;
a subtler defect — say, `is_published` reading a stale cached frontier — would
fail only the second.

The third test, `:212-221`, covers the degenerate frontier: on an untouched
publisher, `is_published` is false for `0..8`. `Seq::ZERO < Seq::ZERO` is false,
so *nothing* is published before anything is — which is the correct answer and
the one an inclusive comparison would get wrong at exactly one point.

### Why Exclusive

The convention is forced, not chosen. `try_publish( start, len )` sets the
frontier to `start + len`, so after publishing `0..3` the frontier is `3` and the
readable sequences are `0, 1, 2`. An inclusive `is_published` would have to be
`seq <= published() - 1`, which:

- underflows at `published() == Seq::ZERO`, the initial state, unless
  special-cased;
- makes the empty case (`published() == 0`, nothing readable) and the
  one-item case (`published() == 1`, sequence 0 readable) differ by a saturating
  subtraction rather than by a comparison;
- disagrees with `ring_claim::Claim::start()` + `len()`, which uses the same
  half-open convention, so the two ends of the handshake would need a conversion
  at the seam.

The alternative was never live. What *is* worth recording is that the exclusive
reading makes `is_published` and `published` mutually derivable in one direction
only: `is_published( seq )` is computable from `published()`, but `published()`
is not recoverable from any single `is_published` answer. That asymmetry is why
both are on the surface rather than one
([`item/001`](../item/001_the_three_readings_of_the_cursor.md)).

### What an Inclusive Reading Would Produce

Exactly one extra sequence would answer `true`: the frontier itself — the first
sequence of the range a producer has claimed and may be writing right now.

| | Correct | Off by one |
|--|---------|-----------|
| Frontier at 3, ask about 2 | `true` — written and published | `true` |
| Frontier at 3, ask about **3** | **`false`** — claimed, perhaps, but not written | **`true`** |
| Empty publisher, ask about 0 | `false` | `true` — reads a slot never touched |

The middle row is this crate's central requirement, stated at
`src/lib.rs:22-23` as *"the feature's central requirement is that a consumer
never sees it"*. The doc example's own comment at `:226` says it in four words:
*"claimed, perhaps, but not published"*.

The failure is a read of uninitialised or stale memory, and — like everything
else in this crate — it passes single-threaded tests, because there the write
completes before anything can read. `tests/handshake_test.rs:77-165` is what
catches it structurally, by giving the producer a real slot to write and asking
whether the consumer was ever handed it early; the assertion message is
`"available offered a slot the producer had claimed but not written"`, and
`tests/manual/readme.md § P1` records both mutations failing there with
`left: 0`.

### The Atomic Load Nobody Counts

`is_published` calls `published()`, which is `self.cursor.load( GATING )` — one
`Acquire` load. So the method costs exactly one atomic load per call, and a
caller asking about *n* sequences pays *n* of them, each returning a possibly
different frontier.

Nothing in the crate batches this and nothing needs to: the real consumer path
uses `ring_consume::Consumer::available`, which reads the frontier once through
a `Barrier` and returns a whole range.
[`non_functional_requirement/001`](../non_functional_requirement/001_what_a_publication_costs.md)
records the cost; the reason it never bites is that `is_published` is a
*question* API for tests and diagnostics, and the production read path is the
barrier.

The 320-assertion test above therefore issues 320 atomic loads plus 16
compare-exchanges, and runs in microseconds — which is why exhaustiveness was
affordable here and is not affordable in the loom model
([`workaround/001`](../workaround/001_the_loom_seam_and_its_only_user.md)).

### PB51 — The Graded Question Is Asked Thirteen Times, Never Where Interleavings Are Enumerated

```sh
cd "$(git rev-parse --show-toplevel)"/ring_publish
# every call site of the graded question, and the gate on the file holding them
grep -c 'is_published' tests/publish_test.rs
grep '^#!\[ cfg' tests/publish_test.rs
# the loom model never asks it …
grep -c 'is_published' tests/handshake_test.rs || true
# … and here is the model, and what it reads instead
grep 'cfg( loom )' tests/handshake_test.rs
grep -c 'published()' tests/handshake_test.rs
```

Live output:

```
13
#![ cfg( not( loom ) ) ]
0
#[ cfg( loom ) ]
5
```

Thirteen call sites, every one of them in `tests/publish_test.rs`, whose first
attribute is `#![ cfg( not( loom ) ) ]`. The file carrying the entire evidence
for this invariant is excluded by construction from the loom build.

`src/lib.rs:214-215` names `is_published` as "the consumer-facing question
this crate is graded on: a slot claimed but not published must answer
`false`". Two of those thirteen sites do run concurrently — the file uses
`std::thread::scope` twice — so the predicate is exercised under real threads.
Real threads *sample* interleavings. The model that *enumerates* them is
`handshake_test.rs`'s `#[ cfg( loom ) ] mod exhaustive`, and it calls
`is_published` zero times; it reads `published()` and compares sequences
directly.

That substitution is sound — `is_published` is exactly `seq < published()`, so
the model checks the predicate's meaning without calling the predicate. It
leaves one derivable line between what is enumerated and what the feature
grades. The gap is thin rather than hollow, and the direction is worth naming:
the assertion that would close it is
`assert!( !publisher.is_published( seq ) )` at an interleaving point the model
already reaches, and nothing prevents adding it there.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_six_methods_and_no_caller.md](../api/001_six_methods_and_no_caller.md) | `is_published` and `published` on the same surface |

### Invariants

| File | Relationship |
|------|--------------|
| [001_the_frontier_moves_only_by_compare_exchange.md](001_the_frontier_moves_only_by_compare_exchange.md) | The monotone exact frontier that makes this a complete answer |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_three_readings_of_the_cursor.md](../item/001_the_three_readings_of_the_cursor.md) | The three read methods, and why all three exist |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_a_slot_from_claim_to_visibility.md](../lifecycle/001_a_slot_from_claim_to_visibility.md) | The moment a sequence crosses from `false` to `true` |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_what_a_publication_costs.md](../non_functional_requirement/001_what_a_publication_costs.md) | One `Acquire` load per question |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_conflating_the_two_cursors.md](../pitfall/002_conflating_the_two_cursors.md) | The larger form of the same read-too-early failure |

### Sources

| File | Relationship |
|------|--------------|
| `ring_publish/src/lib.rs:122-136,212-232` | `published`'s "one past" contract and `is_published`'s comparison |
| `ring_claim/src/lib.rs:80-93` | `Claim`'s matching half-open `start`/`len` convention |
| `ring_cursor/src/lib.rs:89` | `GATING`, the `Acquire` every question costs |

### Tests

| File | Relationship |
|------|--------------|
| `tests/publish_test.rs:194-210` | Five points around one boundary, with the reason named |
| `tests/publish_test.rs:212-221` | Nothing is published before anything is |
| `tests/publish_test.rs:223-247` | 320 assertions — every candidate at every frontier |
| `tests/handshake_test.rs:77-165` | The same requirement, from the consumer's side, over every interleaving |
| `tests/manual/readme.md § P1` | Both mutations failing at the slot assertion with `left: 0` |
