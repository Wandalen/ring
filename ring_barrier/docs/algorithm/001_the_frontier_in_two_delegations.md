# Algorithm: The Frontier in Two Delegations

### Scope

- **Purpose**: Trace `frontier` and `available` from the call site to the arithmetic, and show that this crate contributes one `map_or` and nothing else.
- **Responsibility**: Give the chain step by step, name what each step costs, and compare its depth against the producer half's.
- **In Scope**: How a barrier answer is computed.
- **Out of Scope**: `wait_for`'s two-phase structure — see [`002`](002_wait_for_asks_twice.md).

### The Two Readings

```rust
// ring_barrier/src/lib.rs:191-219
pub fn frontier( &self ) -> Option< Seq >
{ ring_cursor::slowest( self.dependencies ) }

pub fn available( &self, from : Seq ) -> u64
{ self.frontier().map_or( 0, | frontier | from.distance_to( frontier ) ) }
```

Four lines, of which this crate authored the `map_or` and the choice of `0`.
Everything else is a call.

### The Chain

| Step | Where | Tier | Does |
|-----:|-------|:----:|------|
| 1 | `Barrier::available` | 5 | Resolves the empty case to `0` |
| 2 | `Barrier::frontier` | 5 | Hands the slice on |
| 3 | `ring_cursor::slowest` | 3 | Loads every cursor at `GATING`, collects a `Vec< Seq >` |
| 4 | `ring_seqno::slowest` | 1 | `.iter().copied().min()` |
| 5 | `Seq::distance_to` | 0 | `later.0.saturating_sub( self.0 )` |

```sh
cd "$(git rev-parse --show-toplevel)"
grep -vE "^[[:space:]]*(///|//!)" ring_barrier/src/lib.rs \
  | grep -oE 'ring_(cursor|seq|wait)::[a-z_]+' | sort | uniq -c
# 1 ring_cursor::slowest
# 1 ring_wait::wait_until
```

Live output:

```
      1 ring_cursor::slowest
      1 ring_wait::wait_until
```

Two delegated calls in the whole crate, one per method that needs one. There is
no `ring_seqno` in the manifest at all — step 4 is reached through `ring_cursor`,
and step 5 through `Seq`'s own inherent method on a type this crate already
imports for its signatures.

### Against the Producer Half

`ring_gating::headroom` takes six steps through four crates to reach one
subtraction ([`ring_gating`'s algorithm/001](../../../ring_gating/docs/algorithm/001_headroom_in_two_delegations.md)).
This side takes five, and the missing step is the reason:

| | `ring_gating::headroom` | `Barrier::available` |
|--|-------------------------|----------------------|
| Fold | `ring_cursor::slowest` | `ring_cursor::slowest` — the same function |
| Arithmetic | `ring_seqno::free_slots( producer, slowest, capacity )` | `Seq::distance_to` |
| Empty case | `capacity` | `0` |
| Declared deps | `ring_types`, `ring_cursor`, `ring_seqno` | `ring_types`, `ring_cursor`, `ring_wait` |

The gating side needs `ring_seqno::free_slots` because its answer is *clamped* —
`capacity.saturating_sub( in_flight )` requires a capacity and a subtraction in
that order. This side needs no clamp, so its arithmetic collapses into the one
saturating subtraction `Seq` already carries, and the third dependency slot goes
to `ring_wait` instead.

That is [`integration/002`](../integration/002_the_dependency_that_is_not_ring_seq.md)
stated as an algorithm: **the two halves differ by exactly one clamp, and the
clamp is what costs a crate.**

### Where the Cost Is

| Step | Cost | Before `b7e075ca` |
|-----:|------|-------------------|
| 3 | — | **One heap allocation per call** — `.collect()` into a `Vec< Seq >` |
| 3 | One `Acquire`-strength load per dependency | unchanged |
| 4 | One comparison per dependency | unchanged |
| 5 | One `saturating_sub` | unchanged |

Everything in the left column is proportional to the dependency count and
unavoidable. The allocation was not: `ring_cursor::slowest` collected positions
into a `Vec` only because `ring_seqno::slowest` takes `&[ Seq ]` rather than an
iterator, and it now folds the cursors directly. See
[`non_functional_requirement/001`](../non_functional_requirement/001_every_frontier_read_allocates_nothing.md).

`Barrier::admits` calls `available`, so it allocated too, and `wait_for` calls
`admits` once per spin — so a `wait_for` with a 10,000-spin budget performed up
to **10,000 allocations** before giving up. That was not a hypothetical shape:
it is the exact call in `a_consumer_waiting_on_a_producer_thread_makes_progress`.
That row is now the crate's cheapest, and it is measured rather than argued —
`tests/allocation_test.rs` spends the same 10,000-spin budget and asserts zero.

### The `map_or` Is the Whole Decision

Step 1 is the only line in the chain this crate could have written differently,
and every alternative is a different crate's answer:

| Written | Means | Whose answer |
|---------|-------|--------------|
| `map_or( 0, … )` | No dependencies, nothing readable | **this crate** |
| `map_or( capacity, … )` | No dependencies, unbounded | `ring_gating` |
| `map_or( u64::MAX, … )` | No dependencies, no bound at all | nobody |
| `unwrap_or( Seq::ZERO )` then subtract | Collapses "no dependencies" into "dependencies at zero" | a bug |

The argument for `0` is in
[`decisions/001`](../decisions/001_zero_for_a_barrier_over_nothing.md), and the
fold's refusal to pick an identity itself — which is what leaves the choice
here — is `ring_seqno`'s.

### BR22 — A Successful Wait Folds the Dependencies One More Time Than It Checked Them

`wait_for` runs the fold twice per outcome, not once. The predicate handed to
`ring_wait::wait_until` is `|| self.admits( from, count )`, and `admits` calls
`available`, which calls `frontier`, which is the fold. So every attempt folds
the whole slice — and after the wait succeeds, line 286 folds it once more to
produce the value returned.

A wait that succeeds on attempt *n* therefore performs *n + 1* folds over
*len()* cursors. At the family's `DEFAULT_SPINS` of 1024 with eight
dependencies that is 8,200 atomic loads for one answer, and the crate documents
none of it — the cost is a consequence of composing three single-purpose
functions, each of which is cheap on its own.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A5 -F '  pub fn wait_for( &self, from : Seq, count : u64, kind : WaitKind, spins : usize )' ring_barrier/src/lib.rs
# each link in the chain, and the fold at the end of it
grep -E "self\.available|self\.frontier|ring_cursor::slowest" ring_barrier/src/lib.rs
```

Live output:

```
  pub fn wait_for( &self, from : Seq, count : u64, kind : WaitKind, spins : usize )
  -> Result< Seq, RingError >
  {
    ring_wait::wait_until( kind, spins, || self.admits( from, count ) )?;
    self.frontier().ok_or( RingError::Empty )
  }
//! [`ring_cursor::slowest`] and lives in neither of them.
    ring_cursor::slowest( self.dependencies )
    self.frontier().map_or( 0, | frontier | from.distance_to( frontier ) )
    count <= self.available( from )
    self.frontier().ok_or( RingError::Empty )
```

### BR23 — The Sequence Returned Is Not the One the Wait Succeeded On

The rustdoc calls the return value *"the frontier at the moment the wait
succeeded"*. It is the frontier at a moment strictly after that: `wait_until`
returns when `admits` last saw enough, and line 286 then reads the cursors
again. Between those two reads any dependency may advance.

This is the behaviour the crate wants — the paragraph below the `# Errors`
section argues for it explicitly, that a consumer which waited for one item and
found six should drain six. The sentence describing it is what is off by one
read. The distinction matters because it is the difference between a value that
was checked against `count` and a value that was not: the returned frontier is
only guaranteed to be at least as far as the one `admits` accepted, never
exactly it.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A11 -F '  /// Wait until at least `count` sequences are readable from `from`, then' ring_barrier/src/lib.rs
# the two reads, four lines apart
grep "wait_until\|self.frontier()" ring_barrier/src/lib.rs | tail -3
```

Live output:

```
  /// Wait until at least `count` sequences are readable from `from`, then
  /// report the frontier.
  ///
  /// The returned sequence is the frontier as re-read immediately after the
  /// wait succeeded, not the frontier at the exact instant it succeeded — a
  /// dependency may have advanced between the two reads, so the value is
  /// only guaranteed to be at least as far as what was checked. It is also
  /// not `from + count` — a consumer that waited for one item and found six
  /// should drain six, and returning the requested count instead would
  /// throw away the batch that waiting just discovered.
  ///
  /// # Errors
    self.frontier().map_or( 0, | frontier | from.distance_to( frontier ) )
    ring_wait::wait_until( kind, spins, || self.admits( from, count ) )?;
    self.frontier().ok_or( RingError::Empty )
```

The rustdoc now names the second read explicitly:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A11 -F '  /// Wait until at least `count` sequences are readable from `from`, then' ring_barrier/src/lib.rs
```

Live output:

```
  /// Wait until at least `count` sequences are readable from `from`, then
  /// report the frontier.
  ///
  /// The returned sequence is the frontier as re-read immediately after the
  /// wait succeeded, not the frontier at the exact instant it succeeded — a
  /// dependency may have advanced between the two reads, so the value is
  /// only guaranteed to be at least as far as what was checked. It is also
  /// not `from + count` — a consumer that waited for one item and found six
  /// should drain six, and returning the requested count instead would
  /// throw away the batch that waiting just discovered.
  ///
  /// # Errors
```

**Disposition:** applied — `wait_for`'s rustdoc in `src/lib.rs` now says the
returned sequence is re-read immediately after the wait succeeded rather than
"at the moment" it succeeded, naming the gap this instance's own finding
describes. Now prints: `The returned sequence is the frontier as re-read immediately after the`

### Algorithms

| File | Relationship |
|------|--------------|
| [002_wait_for_asks_twice.md](002_wait_for_asks_twice.md) | The one method that reads the chain more than once |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_zero_for_a_barrier_over_nothing.md](../decisions/001_zero_for_a_barrier_over_nothing.md) | Why the `map_or` default is `0` |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_the_dependency_that_is_not_ring_seq.md](../integration/002_the_dependency_that_is_not_ring_seq.md) | Why the third dependency is `ring_wait` and not `ring_seqno` |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_capacity_never_enters_the_arithmetic.md](../invariant/002_capacity_never_enters_the_arithmetic.md) | The absence that makes step 5 a bare subtraction |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_three_barrier_readings.md](../item/001_the_three_barrier_readings.md) | The three readings this chain serves |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_every_frontier_read_allocates_nothing.md](../non_functional_requirement/001_every_frontier_read_allocates_nothing.md) | Step 3's cost, and who pays it |

### Sources

| File | Relationship |
|------|--------------|
| `ring_barrier/src/lib.rs:191-219` | Steps 1 and 2 |
| `ring_cursor/src/lib.rs:120-123` | Step 3, and the allocation |
| `ring_seqno/src/lib.rs:133-136` | Step 4 |
| `ring_types/src/id.rs:82-85` | Step 5 |

### Tests

| File | Relationship |
|------|--------------|
| `tests/barrier_test.rs:97-117` | The fold, at every size from one to eight with the minimum at every index |
| `tests/barrier_test.rs:152-162` | Step 5, across the whole range below the frontier |
| `tests/barrier_test.rs:164-177` | Step 5's saturation, from the frontier to well past it |
| `tests/barrier_test.rs:234-246` | Step 1's default arm |
