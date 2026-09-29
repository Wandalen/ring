# api

Sixteen public items across two types, and the surface's most interesting
property is what it declines to do. Every `must_use` in the crate is unmessaged
— bare `#[ must_use ]`, no sentence — where `ring_claim`'s carries the most
severe message in the family. That contrast is a decision, and it is the right
one: a `Claim` not published wedges the ring, an `Available` not consumed costs
nothing at all.

The second instance takes the two commits as a pair and finds that the contract
documents when each is *safe* and never when each is *right* — which is the
crate's most likely real-world defect.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Sixteen Public Items](001_sixteen_public_items.md) | CN18, CN19 — three unannotated methods each correctly unannotated, and a crate where every `must_use` is deliberately silent |
| 002 | [The Two Commits](002_the_two_commits.md) | CN20, CN21 — a contract covering safety and not fitness, and a fallible function that cannot fail where it is meant to be used |

### The Whole Surface

| Type | Items | `const` | `must_use` |
|------|------:|--------:|-----------:|
| `Available` | 6 methods | 5 | 5 bare |
| `Consumer< 'a >` | 8 methods | 3 | 6 bare |
| the two structs | 2 | — | — |
| **total** | **16** | **8** | **11 bare, 0 messaged** |

The three methods without `must_use` are `commit` (returns `Result`, covered by
`must_use` on `Result` itself), `commit_available` (its purpose is the side
effect), and `sequences` (returns an iterator, and the lint on `Iterator`
covers it). Each absence has a reason; none is an oversight.

### The `must_use` Decision, Stated

Twelve messaged annotations in 33 crates — § Regenerate below prints every one
of them, and every one names a cost of letting the value go unused: a range
claimed and never published, a reservation that publishes an unwritten slot, a
record dropped instead of read, an outcome ignored so a misconfigured
`OnBarrier` stays silent.

`ring_consume` has none, correctly: dropping an `Available` has no consequence.
The value is a permission, not an obligation — the read half's fundamental
asymmetry against the write half, expressed as an annotation choice
([`pattern/001`](../pattern/001_the_half_open_range_as_a_value.md) CN40).

### Where the Contract Stops

`commit` and `commit_available` are documented for the conditions under which
each will not fail. Neither says which to reach for. The distinction that
matters is not safety but irreversibility: `commit_available` discards
everything currently available in one call, with no argument to get wrong and no
error to catch, and is therefore the easier call *and* the one that loses more
([`lifecycle/001`](../lifecycle/001_a_sequence_from_published_to_committed.md)
CN31).

A caller who read a partial batch and reaches for the shorter name has silently
skipped the remainder. Nothing in either doc comment warns of it.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order

# the whole surface, with its annotations
command grep -E '^\s*(pub (const )?fn|#\[ must_use)' ring_consume/src/lib.rs

# every messaged must_use in the family — expect twelve, none here
command grep -r 'must_use = ' ring_*/src/*.rs | sed 's|ring/||'

# the three unannotated methods — what covers each is its return type, so the
# signature alone is the evidence. A fixed -B window is not: the lines above
# these three are whatever the doc block happens to end with, which drifts.
command grep -E '^  pub fn (commit|commit_available|sequences)\(' ring_consume/src/lib.rs

# doctest fences — expect one pair per public item
command grep -c '^/// ```' ring_consume/src/lib.rs

# the crate's only `# Errors` contract — anchored on the section itself rather
# than on a fixed offset above `pub fn commit`, which the doc block has outgrown.
# There is exactly one: `commit_available` returns a `Seq`, so it has none.
command grep -A6 '# Errors' ring_consume/src/lib.rs
```

Live output:

```
  #[ must_use ]
  pub const fn new( start : Seq, len : u64 ) -> Self
  #[ must_use ]
  pub const fn start( self ) -> Seq
  #[ must_use ]
  pub const fn end( self ) -> Seq
  #[ must_use ]
  pub const fn len( self ) -> u64
  #[ must_use ]
  pub const fn is_empty( self ) -> bool
  pub fn sequences( self ) -> impl Iterator< Item = Seq >
  #[ must_use ]
  pub const fn new( cursor : &'a PaddedCursor, barrier : Barrier< 'a > ) -> Self
  #[ must_use ]
  pub const fn cursor( &self ) -> &'a PaddedCursor
  #[ must_use ]
  pub const fn barrier( &self ) -> Barrier< 'a >
  #[ must_use ]
  pub fn position( &self ) -> Seq
  #[ must_use ]
  pub fn available( &self ) -> Available
  #[ must_use ]
  pub fn available_up_to( &self, max : u64 ) -> Available
  pub fn commit( &self, through : Seq ) -> Result< Seq, RingError >
  pub fn commit_available( &self ) -> Seq
ring_atomic/src/lib.rs:  #[ must_use = "the returned sequence is the claim — dropping it claims a range nobody will use" ]
ring_claim/src/lib.rs:#[ must_use = "a claimed range that is never published strands its slots and stalls every consumer" ]
ring_flush/src/lib.rs:#[ must_use = "an ignored outcome is exactly how a misconfigured OnBarrier stays silent" ]
ring_shutdown/src/lib.rs:  #[ must_use = "a Stopped is the only route to a drain; bind it, or bind `_` to close and nothing else" ]
ring_shutdown/src/lib.rs:  #[ must_use = "this is the record itself, not a copy — dropping it loses it" ]
ring_shutdown/src/lib.rs:#[ must_use = "a Wake::Closed means stop, not publish" ]
ring_slot/src/lib.rs:  #[ must_use = "this is the only way a payload leaves the slot; dropping it here destroys the record" ]
ring_spsc/src/lib.rs:#[ must_use = "a reservation publishes on drop; dropping it immediately publishes an unwritten slot" ]
ring_spsc/src/lib.rs:#[ must_use = "a batch commits on drop; dropping it immediately discards the records it covers" ]
ring_testkit/src/lib.rs:#[ must_use = "nothing frees this — dropping the reference leaks the ring with no way to reach it again" ]
ring_testkit/src/lib.rs:#[ must_use = "nothing frees either allocation — dropping the pair leaks both with no way to reach them again" ]
ring_testkit/src/lib.rs:  #[ must_use = "the Outcome is the measurement — `script.run( &mut ring );` as a statement drives the ring and discards everything it observed" ]
  pub fn sequences( self ) -> impl Iterator< Item = Seq >
  pub fn commit( &self, through : Seq ) -> Result< Seq, RingError >
  pub fn commit_available( &self ) -> Seq
4
  /// # Errors
  ///
  /// [`RingError::Empty`] when `through` is past what is available: the
  /// consumer would be freeing slots it has not read. [`RingError::Empty`] also
  /// when `through` is behind the current position, which would re-read slots
  /// the producer has already been cleared to reuse.
  ///
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CN18 | `ring_consume` | n/a — observation | The three methods with no `must_use` are each covered by their return type; the count invites a false finding and none is available |
| CN19 | `ring_consume` | n/a — observation | Every `must_use` here is unmessaged, correctly — an `Available` is a permission, not an obligation, unlike the family's twelve messaged annotations |
| CN20 | `ring_consume` | **latent hazard** | The two commits' contracts state when each is safe and never when each is right; `commit_available` after a partial read silently discards the remainder |
| CN21 | `ring_consume` | **misleading doc** | `commit` cannot return `Err` for a `through` that came from `available()`, so its `Result` costs every correct call site an `unwrap` and benefits only incorrect ones — and its doctest shows the failing case rather than the idiom |
