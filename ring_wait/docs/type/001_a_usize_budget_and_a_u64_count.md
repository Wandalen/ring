# Type: A `usize` Budget and a `u64` Count

### Scope

- **Purpose**: Account for the crate's two integer domains — attempts in `usize`, sequence distances in `u64` — show that they never mix, and record what the plain types fail to prevent.
- **Responsibility**: Give every integer in the public surface, trace each to where its width was decided, and name the two values a stronger type would have excluded.
- **In Scope**: `spins`, `attempt`, `count`, and the `usize` in `Ok`.
- **Out of Scope**: `Result` itself — see [`002`](002_one_return_type_and_the_one_must_use.md).

### Every Integer in the Surface

```sh
cd "$(git rev-parse --show-toplevel)"
grep -E "^pub (const )?fn|^pub const|^-> " ring_wait/src/lib.rs
```

Live output:

```
pub const DEFAULT_SPINS : usize = 1024;
pub const fn escalation_hint( kind : WaitKind ) -> Option< WaitKind >
pub fn pause( kind : WaitKind, attempt : usize ) -> bool
pub fn wait_until< F >( kind : WaitKind, spins : usize, mut ready : F ) -> Result< usize, RingError >
pub fn wait< F >( kind : WaitKind, ready : F ) -> Result< usize, RingError >
pub fn for_space( pair : &CursorPair, kind : WaitKind, spins : usize ) -> Result< usize, RingError >
pub fn for_data( pair : &CursorPair, count : u64, kind : WaitKind, spins : usize )
-> Result< usize, RingError >
```

| Where | Name | Type | Counts |
|-------|------|------|--------|
| `:63` | `DEFAULT_SPINS` | `usize` | attempts |
| `:112` | `attempt` | `usize` | attempts |
| `:179` | `spins` | `usize` | attempts |
| `:179` | `Ok`'s payload | `usize` | attempts |
| `:239` | `spins` | `usize` | attempts |
| `:265` | `spins` | `usize` | attempts |
| `:265` | **`count`** | **`u64`** | **ring items** |

Six `usize` and one `u64`, and the one is the odd one for a reason.

### The Two Domains

| | Attempts | Ring items |
|--|----------|------------|
| Type | `usize` | `u64` |
| Decided by | the loop — it indexes iterations | `Seq( pub u64 )`, `ring_types/src/id.rs:25` |
| Reaches this crate via | the caller's argument | `CursorPair::pending() -> u64` |
| Ultimately produced by | nothing — it *is* the count | `Seq::distance_to`, `id.rs:82-85` |
| Width on a 64-bit target | 64 | 64 |
| Width on a 32-bit target | **32** | 64 |

`count : u64` is not a stylistic inconsistency. It is the width of a sequence
distance, fixed three crates away:

```rust
// ring_types/src/id.rs:25
pub struct Seq( pub u64 );

// ring_types/src/id.rs:82-85
pub const fn distance_to( self, later : Self ) -> u64
{
  later.0.saturating_sub( self.0 )
}
```

A ring's sequence numbers are monotonic and never wrap in practice, so `u64` is
the family's choice regardless of target width. An attempt index is a loop
counter on the machine actually running, so `usize` is the family's choice for
that. `for_data` sits on the seam and takes one of each.

The two never mix — no arithmetic in this crate combines them, and no cast
appears anywhere:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -vE "^[[:space:]]*//" ring_wait/src/lib.rs \
  | grep -E " as (u64|usize)|try_into|from\(" \
  || echo '(no cast or conversion in this crate)'
# control: the identical expression over ring_publish, which has exactly one
grep -vE "^[[:space:]]*//" ring_publish/src/lib.rs \
  | grep -E " as (u64|usize)|try_into|from\("
```

Live output:

```
(no cast or conversion in this crate)
    let end = start.advanced_by( len as u64 );
```

**No match**, while the control returns `ring_publish`'s single `len as u64`, so
the crate really does convert nothing. On a 32-bit target the two would be
genuinely different widths and
the code would still compile unchanged, because the seam is a parameter boundary
rather than an expression.

### `Capacity` Is the Third Domain, and This Crate Never Sees It

```rust
// ring_types/src/capacity.rs:23
pub struct Capacity( usize );
```

`may_claim` needs it, but takes it from the `CursorPair` rather than from this
crate's caller — so `for_space` has no capacity parameter and cannot get one
wrong ([`data_structure/001`](../data_structure/001_a_crate_with_no_type_of_its_own.md)).
That is worth noticing: of the three integer-shaped domains in the family, the
one with a validating newtype is the one this crate never touches.

### What the Plain Types Do Not Prevent

Two values pass the type checker and mean something the API does not want:

| Value | Accepted as | Consequence |
|-------|-------------|-------------|
| `spins = 0` | a budget | clamped to 1 at `:183` — a *silent* correction, invisible to the caller |
| `count = 0` | an item count | `pending() >= 0` is a tautology; `Ok( 0 )` on an empty ring |

Both are handled, and handled differently. The first is corrected and the second
is not, because there is no correct answer for it — "wait for zero items"
succeeds immediately by any reading, and the caller who asked probably meant
something else.

`ring_poll` shows what the alternative looks like for the first:

```rust
// ring_poll/src/lib.rs:137-140
pub const fn new( attempts : usize ) -> Self
{
  if attempts == 0 { Self( 1 ) } else { Self( attempts ) }
}
```

Same rule, but in a constructor, so `Budget::new( 0 ).attempts()` returns `1` and
a caller can *see* the correction happened
([`algorithm/001`](../algorithm/001_one_loop_and_the_two_ways_out.md) § WT17).
This crate's clamp is inside a loop header and reaches nobody.

Nothing equivalent exists for `count`. A `NonZeroU64` would have excluded the
tautology at compile time, and the reason not to reach for one is the same as
everywhere else here: `for_data` has no production caller
([`api/001`](../api/001_seven_items_and_the_one_with_a_caller.md) § WT1), so the
value of the stronger type is currently zero and its cost — every caller
unwrapping a `NonZeroU64` — is not.

### Why `usize` and Not a Newtype for the Budget

`Budget` exists, in `ring_poll`, and is the obvious candidate for sharing. It
cannot be:

| | `ring_wait` | `ring_poll` |
|--|-------------|-------------|
| Has the newtype | no | `Budget`, `:84-113` |
| May depend on the other | yes, but the type is there | **no** — banned by its own test |

The crate that owns the useful newtype is the one forbidden from being depended
on in the direction that would help
([`integration/001`](../integration/001_two_dependencies_two_dependents_and_a_roster.md)).
So `spins : usize` is not a decision so much as the only remaining option, and
the clamp is written twice as a consequence.


### WT49 — The Success Value and the Budget Share a Type and No Meaning

`wait_until` takes a `usize` budget and returns a `usize` count. The two are
different quantities in the same type, so the compiler accepts feeding one back
as the other.

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'pub fn wait_until' ring_wait/src/lib.rs
# the two usize positions, and what the returned one counts
grep 'return Ok( attempt )\|for attempt in 0..spins' ring_wait/src/lib.rs
```

Live output:

```
pub fn wait_until< F >( kind : WaitKind, spins : usize, mut ready : F ) -> Result< usize, RingError >
  for attempt in 0..spins.max( 1 )
      return Ok( attempt );
```

The signature is `( kind : WaitKind, spins : usize, ready : F ) -> Result< usize, RingError >`.
The argument is a maximum; the return is the index at which the predicate
answered, which is the number of pauses that happened (WT25). `wait_until( k, n, p )`
followed by `wait_until( k, that_result, p )` typechecks and means nothing —
a budget derived from how long the last wait took, which is neither a policy nor a
measurement of anything the caller wanted.

Nothing suggests anyone has done it. It is recorded because
[`type/001`](001_a_usize_budget_and_a_u64_count.md)'s subject is the two integer
domains this crate keeps apart, and this is a third pair it does not: `spins` and
the returned attempt count are as distinct as `spins` and `count`, and unlike
those two they share a type and a function. `ring_poll` avoids the same collision
by construction — its budget is a `Budget` newtype and its progress report is a
`Progress`, so neither can be passed where the other belongs (WT44).

### Types

| File | Relationship |
|------|--------------|
| [002_one_return_type_and_the_one_must_use.md](002_one_return_type_and_the_one_must_use.md) | The `Result` these integers travel inside |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_one_loop_and_the_two_ways_out.md](../algorithm/001_one_loop_and_the_two_ways_out.md) | WT17 — the clamp, and where the other copy lives |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_a_crate_with_no_type_of_its_own.md](../data_structure/001_a_crate_with_no_type_of_its_own.md) | WT14 — no type declared here at all |
| [../data_structure/002_the_budget_and_the_attempt_index.md](../data_structure/002_the_budget_and_the_attempt_index.md) | The budget as a value, and count-not-duration |

### Items

| File | Relationship |
|------|--------------|
| [../item/002_the_loop_the_wrapper_and_the_two_questions.md](../item/002_the_loop_the_wrapper_and_the_two_questions.md) | The `count = 0` corner in context |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_two_dependencies_two_dependents_and_a_roster.md](../integration/001_two_dependencies_two_dependents_and_a_roster.md) | Why `Budget` cannot be reused here |

### Sources

| File | Relationship |
|------|--------------|
| `ring_types/src/id.rs:25,82-85` | `Seq` and `distance_to`, which fix `count`'s width |
| `ring_types/src/capacity.rs:23` | The validating newtype this crate never handles |
| `ring_seqno/src/lib.rs:112-115` | `pending`, the `u64` that reaches the predicate |
| `ring_poll/src/lib.rs:84-113` | `Budget`, the newtype that cannot be shared |

### Tests

| File | Relationship |
|------|--------------|
| `tests/wait_test.rs:206-221` | `spins = 0`, corrected silently |
| `tests/wait_test.rs:223-237` | Budgets 1 through 7, honoured exactly |
| `tests/wait_test.rs:280-291` | `count` from 1 to 3 against 3 pending — the zero case is outside this loop, and is the row below |
| `tests/wait_test.rs:383-400` | `count = 0` on the same empty ring that answers `Err( Empty )` for one — the tautology at `:118`, `Ok( 0 )` under all four strategies |
