# Integration: Four Predicates and the One That Is Called

### Scope

- **Purpose**: Account for the outgoing edge to `ring_gating` — the crate that exists to answer *may a producer take these slots* — and for the fact that this crate, its principal caller, uses one of its four public predicates and re-derives a second inline.
- **Responsibility**: Census the four predicates and their call sites, show that the inline re-derivation is not laziness but a correctness requirement, and name what would break if the duplication were removed.
- **In Scope**: `GatingSet::headroom`, `admits`, `check`, `limit`, and every call site of each.
- **Out of Scope**: The retry loop those predicates sit inside — see [`algorithm/001`](../algorithm/001_the_gate_inside_the_retry.md).

### The Four Predicates

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
grep -E '^  pub fn (headroom|admits|check|limit)' ring_gating/src/lib.rs

# every call site of each, library and test, across the family
for p in headroom admits check limit; do
  echo "-- $p"
  command grep -r "\.$p(" ring_*/src/*.rs ring_*/tests/*.rs 2>/dev/null \
    | grep -vE '/// |^\S+: *//'
done
```

Live output:

```
  pub fn headroom( &self, producer : Seq ) -> usize
  pub fn admits( &self, producer : Seq, count : usize ) -> bool
  pub fn check( &self, producer : Seq, count : usize ) -> Result< (), RingError >
  pub fn limit( &self ) -> Option< Seq >
-- headroom
ring_claim/src/lib.rs:    self.consumers.headroom( self.claimed() )
ring_claim/src/lib.rs:    while count <= self.consumers.headroom( current )
ring_claim/src/lib.rs:    while let granted @ 1.. = max.min( self.consumers.headroom( current ) )
ring_gating/src/lib.rs:    count <= self.headroom( producer )
ring_gating/src/lib.rs:    if count > self.headroom( producer )
ring_mpsc/src/lib.rs:    self.claimer.headroom()
ring_barrier/tests/barrier_test.rs:  assert_eq!( set.headroom( Seq( 1_000 ) ), 4, "the producer is clamped to one lap" );
ring_barrier/tests/barrier_test.rs:  assert_eq!( empty.headroom( Seq( 8 ) ), 8, "a producer with nobody behind it may write" );
ring_claim/tests/allocation_test.rs:      core::hint::black_box( claimer.headroom() );
ring_claim/tests/claim_test.rs:    assert_eq!( claimer.headroom(), 8 - taken );
ring_claim/tests/claim_test.rs:  assert_eq!( claimer.headroom(), 0 );
ring_gating/tests/gating_test.rs:  assert_eq!( set.headroom( Seq( 3 ) ), 1 );
ring_gating/tests/gating_test.rs:  assert_eq!( set.headroom( Seq( 4 ) ), 0 );
ring_gating/tests/gating_test.rs:    assert_eq!( set.headroom( Seq( 8 ) ), 2 );
ring_gating/tests/gating_test.rs:  assert_eq!( all_moving.headroom( Seq( 8 ) ), 8 );
ring_gating/tests/gating_test.rs:  assert_eq!( one_stalled.headroom( Seq( 8 ) ), 0, "two fast consumers buy nothing" );
ring_gating/tests/gating_test.rs:      set.headroom( Seq( published ) ),
ring_gating/tests/gating_test.rs:    assert_eq!( set.headroom( Seq( producer ) ), 0, "producer {producer} against a stalled consumer" );
ring_gating/tests/gating_test.rs:    assert_eq!( ungated.headroom( Seq( producer ) ), 4, "at producer {producer}" );
ring_gating/tests/gating_test.rs:  assert_eq!( ungated.headroom( Seq::ZERO ), gated_at_zero.headroom( Seq::ZERO ), "same while empty" );
ring_gating/tests/gating_test.rs:  assert_ne!( ungated.headroom( Seq( 4 ) ), gated_at_zero.headroom( Seq( 4 ) ), "and different after a lap" );
ring_gating/tests/gating_test.rs:  assert_eq!( set.headroom( Seq( limit.0 - 1 ) ), 1, "one slot left just below the limit" );
ring_gating/tests/gating_test.rs:  assert_eq!( set.headroom( limit ), 0, "none at it" );
ring_gating/tests/gating_test.rs:      let headroom = set.headroom( producer );
ring_publish/tests/handshake_test.rs:    assert_eq!( claimer.headroom(), 3 );
ring_publish/tests/handshake_test.rs:    assert_eq!( claimer.headroom(), 0 );
-- admits
ring_barrier/src/lib.rs:    ring_wait::wait_until( kind, spins, || self.admits( from, count ) )?;
ring_barrier/tests/allocation_test.rs:  let ( calls, bytes, answer ) = measure( || barrier.admits( Seq::ZERO, 4 ) );
ring_barrier/tests/barrier_test.rs:            barrier.admits( Seq( from ), count ),
ring_barrier/tests/barrier_test.rs:  assert!( Barrier::over( &nothing_published ).admits( Seq::ZERO, 0 ) );
ring_barrier/tests/barrier_test.rs:  assert!( Barrier::over( &[] ).admits( Seq::ZERO, 0 ), "even with no dependencies at all" );
ring_barrier/tests/barrier_test.rs:  assert!( empty.admits( Seq::ZERO, 0 ), "a zero-length request is trivially satisfied" );
ring_bench/tests/bench_test.rs:    assert!( candidate.admits( 1 ), "{} refuses a single producer", candidate.name() );
ring_bench/tests/bench_test.rs:      Some( ceiling ) => assert!( !candidate.admits( ceiling + 1 ) ),
ring_bench/tests/bench_test.rs:      None => assert!( candidate.admits( 64 ) ),
ring_gating/tests/gating_test.rs:  while set.admits( producer, 1 )
ring_gating/tests/gating_test.rs:  assert!( !set.admits( producer, 1 ), "full" );
ring_gating/tests/gating_test.rs:  assert!( set.admits( producer, 1 ) );
ring_gating/tests/gating_test.rs:  assert!( !set.admits( producer, 2 ), "one slot released is one slot, not two" );
ring_gating/tests/gating_test.rs:  assert!( set.admits( Seq( 3 ), 1 ), "3 ahead of 4 slots: the last slot is free" );
ring_gating/tests/gating_test.rs:  assert!( !set.admits( Seq( 4 ), 1 ), "4 ahead of 4 slots: the next claim would land on the consumer" );
ring_gating/tests/gating_test.rs:    assert!( ungated.admits( Seq( producer ), 4 ) );
ring_gating/tests/gating_test.rs:          set.admits( Seq( producer ), count ),
ring_gating/tests/gating_test.rs:  assert!( full.admits( Seq( 4 ), 0 ) );
-- check
ring_gating/tests/gating_test.rs:  assert_eq!( set.check( producer, 1 ), Err( RingError::Full ) );
ring_gating/tests/gating_test.rs:    if set.check( producer, batch ).is_ok()
ring_gating/tests/gating_test.rs:  let err = set.check( Seq::ZERO, 5 ).unwrap_err();
ring_gating/tests/gating_test.rs:  assert_eq!( set.check( Seq( 3 ), 2 ), Err( RingError::Full ) );
ring_gating/tests/gating_test.rs:  assert_eq!( set.check( Seq( 4 ), 4 ), Err( RingError::Full ) );
ring_gating/tests/gating_test.rs:  assert!( set.check( Seq( 4 ), 5 ).unwrap_err().is_configuration() );
ring_gating/tests/gating_test.rs:          set.check( Seq( producer ), count ).is_ok(),
ring_gating/tests/gating_test.rs:  assert!( full.check( Seq( 4 ), 0 ).is_ok() );
-- limit
ring_claim/tests/claim_test.rs:          let limit = consumers.limit().expect( "one consumer" );
ring_gating/tests/gating_test.rs:  let limit = set.limit().expect( "one consumer, so a limit exists" );
ring_gating/tests/gating_test.rs:  assert_eq!( ungated.limit(), None );
ring_gating/tests/gating_test.rs:  assert_eq!( set.limit(), Some( Seq( 13 ) ) );
ring_gating/tests/gating_test.rs:  let limit = set.limit().unwrap();
```

| Predicate | Line | Returns | Library callers, family-wide |
|-----------|-----:|---------|------------------------------|
| `headroom` | `:217` | `usize` | `ring_claim` ×3, `ring_gating` itself ×2 |
| `admits` | `:237` | `bool` | **0** |
| `check` | `:265` | `Result< (), RingError >` | **0** |
| `limit` | `:303` | `Option< Seq >` | **0** |

Three of the four have no library caller anywhere in the 33 crates. Two of the
three are answered inline, in this file, a few lines apart.

### CL4 — The Crate `ring_gating` Was Built For Calls One of Its Four Predicates

`ring_claim` is the only library in the family that calls `GatingSet` at all
outside `ring_gating` itself, and all three of its call sites are the same
method:

| Site | Call | Role |
|------|------|------|
| `src/lib.rs:384` | `self.consumers.headroom( self.claimed() )` | the public `Claimer::headroom` accessor |
| `src/lib.rs:404` | `while count <= self.consumers.headroom( current )` | `claim`'s loop condition |
| `src/lib.rs:488` | `while let granted @ 1.. = max.min( self.consumers.headroom( current ) )` | `claim_up_to`'s loop condition |

`ring_mpsc:805` reaches the same value, but through `Claimer::headroom` — so it
is this crate's accessor, not a fourth direct call.

The predicate that returns a *number* is used; the three that return a
*verdict* are not. That is not an accident of taste, and the next two findings
are why.

### CL5 — `GatingSet::check` Is `claim`'s Two Guards, and Calling It Would Be the Bug

`GatingSet::check` (`ring_gating:265-276`):

```rust
if count > self.capacity.get()
{
  return Err( RingError::BatchTooLarge { requested : count, capacity : self.capacity.get() } );
}
if count > self.headroom( producer )
{
  return Err( RingError::Full );
}
Ok( () )
```

`Claimer::claim` (`src/lib.rs:390-414`), with the loop elided:

```rust
if count > self.consumers.capacity().get()
{
  return Err( RingError::BatchTooLarge { requested : count, capacity : … } );
}
…
while count <= self.consumers.headroom( current )
{ … }
Err( RingError::Full )
```

Same two conditions, same order, same two error variants, same payloads. The
first guard is genuine duplication and could be replaced by a call today. The
second cannot be, and the reason is the whole point of the crate:

| | `check`'s second guard | `claim`'s second guard |
|--|------------------------|------------------------|
| Shape | `if count > headroom( producer )` | `while count <= headroom( current )` |
| Evaluated | once, before anything | on every retry, against the re-read cursor |
| `producer` / `current` | supplied by the caller, fixed | the value the failed compare-exchange returned |
| Relationship to the CAS | before it | **inside** it |

Substituting `check` into `claim` would move the gate outside the retry, which
is precisely the design the module doc rejects at `src/lib.rs:52-64`:

> A `fetch_add` claim is shorter and wrong. It advances the cursor
> unconditionally, so the gating check has to happen *before* it — and between
> that check and the add, another producer can take the space the check just
> found.

`check` is a correct answer to *is there room right now*. `claim` needs *is
there room at the value I am about to exchange against*, and those are the same
question only in the absence of a second producer. So the family's most
call-shaped predicate has no callers because its one plausible caller is the one
place its shape is wrong.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -F 'Not a step in a claim' ring_gating/src/lib.rs
# every `.check(` in the family's source. The ones that are not doc-comment
# lines are the production callers, and the count below is how many that is.
command grep -r '\.check(' ring_*/src/*.rs | command grep -vc '///'
# The command above can legitimately exit nonzero -- a count of zero, a diff
# that differs -- and the sweep that keeps this output current skips any block
# that exits nonzero, so without this the block freezes instead of refreshing.
:
```

Live output:

```
  /// # Not a step in a claim
0
```

**Disposition:** applied — the count is unchanged and correct at zero, so
nothing here was a defect to fix; what was missing was any record, reachable
from `check` itself, of *why* the one obvious caller must not call it. A
`# Not a step in a claim` section now sits in `GatingSet::check`'s own
documentation, stating that `ring_claim::Claimer::claim` open-codes both guards
deliberately, that substituting this call would hoist the gate out of the retry
and restore exactly the check-then-act window `ring_claim`'s module doc rejects,
and that the correct callers are the ones wanting an answer rather than a claim
— a diagnostic, an admission test, a caller deciding whether to attempt anything
at all. Now prints: `/// # Not a step in a claim`

### CL6 — Three Uncalled Predicates, Three Different Reasons

The three are not uncalled for one reason, and the distinction matters because
only one of them is a candidate for deletion:

| Predicate | Why nothing calls it | Should it exist? |
|-----------|----------------------|------------------|
| `admits` | it is `count <= headroom( producer )` — one line, and callers that want a bool write it | **arguable** — pure sugar over the called predicate |
| `check` | its shape is the pre-CAS check `claim` must not perform (CL5) | **yes** — it is the correct API for a caller that is not racing |
| `limit` | it answers a diagnostic question, not a control-flow one | **yes** — and its doc says so |

`limit`'s own documentation (`ring_gating:296-299`) states the intent plainly:

> Exposed separately from [`headroom`] because a diagnostic wants the absolute
> position — "blocked at sequence 128" localises a stall, while "0 slots free"
> does not.

Its only caller outside `ring_gating`'s own tests is
`tests/claim_test.rs:524`, in this crate — a concurrency test that needs the
absolute ceiling to assert no grant ever passes it. So `limit` has exactly one
outside user in the family and it is a test in the crate that does not call it
from its library. It is being used as designed; the design is for a reader, and
the only reader so far is an assertion.

`admits` is the one with no defence beyond symmetry. It has no library caller,
its body is a single comparison against the predicate that *is* called, and
`ring_barrier` has an identically-named method (`ring_barrier:238`) with a
different signature that accounts for most search hits — so the name is also the
family's one collision between two public predicates.

### What the Edge Actually Buys

Stripped to what is called, this crate's dependency on `ring_gating` is one
method returning a `usize`. It is still the right edge, because that `usize` is
where the family's back-pressure policy lives:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A7 -F '  pub fn headroom( &self, producer : Seq ) -> usize' ring_gating/src/lib.rs
grep -E '^ring_' ring_gating/Cargo.toml
```

Live output:

```
  pub fn headroom( &self, producer : Seq ) -> usize
  {
    self.slowest().map_or( self.capacity.get(), | slowest |
    {
      ring_seqno::free_slots( producer, slowest, self.capacity )
    } )
  }

ring_types = { path = "../ring_types" }
ring_cursor = { path = "../ring_cursor" }
ring_seqno = { path = "../ring_seqno" }
```

`headroom` resolves to `ring_seqno::free_slots( producer, slowest, capacity )`
when any consumer exists, and to the full capacity when none does — the
ungated case. Both branches are policy this crate deliberately does not hold,
and the second is why an ungated `Claimer` never blocks. It is also how this
crate reaches Tier 1 arithmetic without declaring an edge to it
([`integration/001`](001_two_dependents_that_split_one_feature.md) § CL3).

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_the_gate_inside_the_retry.md](../algorithm/001_the_gate_inside_the_retry.md) | Why the gate is the loop condition rather than a call before it |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_two_constructors_of_a_range.md](../api/002_the_two_constructors_of_a_range.md) | The two error variants both guards produce |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_compare_exchange_rather_than_fetch_add.md](../decisions/001_compare_exchange_rather_than_fetch_add.md) | The rejected design `check`'s shape belongs to |

### Integrations

| File | Relationship |
|------|--------------|
| [001_two_dependents_that_split_one_feature.md](001_two_dependents_that_split_one_feature.md) | The three edges this one is part of |

### Sources

| File | Relationship |
|------|--------------|
| `ring_gating/src/lib.rs:222-228` | `headroom`, and its delegation to `ring_seqno::free_slots` |
| `ring_gating/src/lib.rs:242-245` | `admits`, one comparison over `headroom` |
| `ring_gating/src/lib.rs:247-294` | `check`, its `# Errors` contract, and the two guards |
| `ring_gating/src/lib.rs:296-324` | `limit`, and its stated diagnostic purpose |
| `ring_claim/src/lib.rs:384` | The accessor call site |
| `ring_claim/src/lib.rs:411-437` | `claim`'s two guards, one duplicated and one that cannot be |
| `ring_claim/src/lib.rs:481-488` | `claim_up_to`'s gate, in the same position |
| `ring_barrier/src/lib.rs:238` | The other `admits`, with a different signature |

### Tests

| File | Relationship |
|------|--------------|
| `tests/claim_test.rs:508-543` | The family's only outside caller of `GatingSet::limit` |
| `tests/manual/readme.md § C3` | The gate is asserted to *be* the loop condition, not a call preceding it |
