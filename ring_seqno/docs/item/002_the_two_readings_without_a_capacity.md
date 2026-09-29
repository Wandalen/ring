# Item: The Two Readings Without a Capacity

### Scope

- **Purpose**: Inventory `pending` and `slowest`, the two functions that take no `Capacity`, and account for how little each adds over what it calls.
- **Responsibility**: Give both signatures, constness and coverage, and show that one is a rename and the other is a fold — and that both earn their place for reasons that are not arithmetic.
- **In Scope**: `pending` and `slowest`.
- **Out of Scope**: The three capacity readings — see [`001`](001_the_three_capacity_readings.md).

### The Inventory

```sh
cd "$(git rev-parse --show-toplevel)"
awk '/^\/\/\/ assert_eq!\( pending\( Seq\( 2 \), Seq\( 2 \) \), 0 \);$/{ n1 = NR } n1 && NR >= n1 + 2 && NR <= n1 + 6 { print } /^\/\/\/ assert_eq!\( slowest\( &\[\] \), None \);$/{ n2 = NR } n2 && NR >= n2 + 2 && NR <= n2 + 6 { print }' ring_seqno/src/lib.rs
```

Live output:

```
#[ must_use ]
pub fn pending( producer : Seq, consumer : Seq ) -> u64
{
  consumer.distance_to( producer )
}
#[ must_use ]
pub fn slowest( cursors : &[ Seq ] ) -> Option< Seq >
{
  cursors.iter().copied().min()
}
```

| Function | Signature | `const`? | Cast | Body | Line |
|----------|-----------|:--------:|:----:|------|-----:|
| `pending` | `( producer : Seq, consumer : Seq ) -> u64` | ❌ *(could be)* | none | `consumer.distance_to( producer )` | 112 |
| `slowest` | `( cursors : &[ Seq ] ) -> Option< Seq >` | ❌ *(cannot be)* | none | `cursors.iter().copied().min()` | 133 |

Both are `#[ must_use ]`. Neither performs a cast, so neither is affected by the
`usize` width hazard that splits the other three
([`001`](001_the_three_capacity_readings.md) § The Casts Do Not Agree).

### `pending` Adds Nothing But a Name and an Order

**Finding SQ29.** The body is one method call:

```rust
pub fn pending( producer : Seq, consumer : Seq ) -> u64
{
  consumer.distance_to( producer )
}
```

`Seq::distance_to` is `pub` and `const` on a type every caller already imports.
So `pending( p, c )` is exactly `c.distance_to( p )` — same value, same type,
one fewer function in the world. What the wrapper contributes:

| Contribution | Assessment |
|--------------|------------|
| A domain name | Real. "Pending" is the ring's word; "distance to" is the arithmetic's |
| An argument order matching its three siblings | Real, and it is the *reason* the order is reversed relative to the method it calls |
| Any computation | None |

The second row is the interesting one. `distance_to` reads
`earlier.distance_to( later )`; `pending` reads `pending( producer, consumer )` —
later first. So the wrapper exists partly *to* flip the order, which makes it
consistent with `may_claim` and `free_slots` and inconsistent with
`laps_between`, the one function that kept `distance_to`'s order. See
[`api/002`](../api/002_the_argument_order_split.md).

**Whether a name is worth a function is a real question, and the family answers
it inconsistently.** Two crates compute the same "how much is readable" quantity
by different routes:

```rust
// ring_barrier/src/lib.rs:217-220
pub fn available( &self, from : Seq ) -> u64
{
  self.frontier().map_or( 0, | frontier | from.distance_to( frontier ) )
}

// ring_consume/src/lib.rs:338-344
let position = self.position();
let readable = self.barrier.frontier()
  .map_or( 0, | frontier | ring_seqno::pending( frontier, position ) );
```

**Finding SQ20.** These are the same expression. `pending( frontier, position )`
is `position.distance_to( frontier )`, which is `from.distance_to( frontier )`
with `from = position`. `ring_consume` holds a `Barrier` (`src/lib.rs:199`) and
depends on `ring_barrier`, so `self.barrier.available( position )` was available
to it — and `Barrier::available` has **no callers outside its own crate**:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r '\.available(' ring_*/src/*.rs | grep -vE ':\s*///'
```

Live output:

```
ring_barrier/src/lib.rs:    count <= self.available( from )
ring_consume/src/lib.rs:    let run = self.available();
ring_consume/src/lib.rs:    let run = self.available();
ring_consume/src/lib.rs:    let run = self.available();
ring_core/src/lib.rs:      ConsumerInner::Spsc( consumer ) => consumer.available(),
ring_core/src/lib.rs:      ConsumerInner::Mpsc( consumer ) => consumer.available(),
ring_mpsc/src/lib.rs:    self.available() == 0
```

One hit is `Barrier::admits` calling its own method; the other six are
`Consumer::available` in `ring_consume`, `ring_mpsc` and `ring_core` — a
different function on a different type.

So `ring_barrier` exports a method that expresses this exactly, nobody outside
uses it, and the one crate that would has re-expanded its body inline through
`ring_seqno::pending`.

That is the mirror image of `slowest`'s story below, and the same rule explains
both: `Barrier::available` needed *no* adaptation at the call site, so reaching
past it to the lower-level function was as easy as calling it. `slowest` needed
adaptation at every tier, so every tier wrote a wrapper and called down. See
[`integration/002`](../integration/002_how_the_fold_crossed_four_tiers.md).

Neither duplicate is a defect — both compute the right answer — and neither is
repaired here.

### `slowest` Is the Only Function That Cannot Be `const`

Four of the crate's five functions accept `const fn`, verified by compiling them
([`api/001`](../api/001_five_functions_and_no_types.md)). `slowest` is the
exception, and genuinely so: `Iterator::min` is not a `const` operation, and no
rewriting of the body around a slice index would preserve the `Option` return for
an empty slice as cleanly.

It was also the only function here whose *caller* paid a cost this crate's
signature imposes. `&[ Seq ]` cannot be produced from `&[ PaddedCursor ]` without
materialising the loads, so `ring_cursor::slowest` heap-allocated a `Vec< Seq >`
on every call — in the loop condition of `ring_claim::claim`'s compare-exchange
retry. Commit `b7e075ca` settled it by removing the caller rather than the cost:
that function folds the loads in place with `.min()` now and no longer calls this
one at all, which leaves `slowest` here reachable only from its own tests.
Recorded in full at
[`ring_cursor` `nfr/002`](../../../ring_cursor/docs/non_functional_requirement/002_the_gating_read_allocates_nothing.md)
and, from this side, at
[`non_functional_requirement/001`](../non_functional_requirement/001_every_reading_is_allocation_free.md).

### The `Option` Is the Whole Design

`slowest`'s return type is the one place this crate makes a decision rather than
performing an arithmetic. Returning `Option< Seq >` rather than `Seq::ZERO` keeps
"no consumers" separable from "a consumer at position zero", and its two
downstream consumers resolve that `None` to **opposite** values — full capacity
in `ring_gating`, zero in `ring_barrier`. See
[`decisions/001`](../decisions/001_none_rather_than_zero_for_an_empty_set.md) and
[`type/002`](../type/002_the_option_that_slowest_returns.md).

### Coverage

| Function | Unit tests | Doctest | Cross-crate callers |
|----------|-----------:|:-------:|---------------------|
| `pending` | 1 — `pending_counts_unread` (`:104-110`) | ✅ `:99-105` | `ring_cursor:389`, `ring_consume:342` |
| `slowest` | 2 — `:113-122`, `:125-131` | ✅ `:120-126` | **none** — `ring_cursor` stopped delegating at `b7e075ca` |

`pending`'s single test is the thinnest coverage in the crate, and it is
defensible: the function has no branches, no capacity, and no arithmetic of its
own. Its three assertions cover the three cases that exist — ahead, equal, and
behind:

```rust
assert_eq!( pending( Seq( 5 ), Seq( 2 ) ), 3 );
assert_eq!( pending( Seq( 2 ), Seq( 2 ) ), 0 );
assert_eq!( pending( Seq( 2 ), Seq( 5 ) ), 0, "a consumer cannot be ahead" );
```

The third is the one carrying weight — it pins the saturation, which is the only
behaviour `pending` has that is not obvious from its name.

Neither function appears in `positions_many_laps_apart_stay_comparable`, the
test that guards the never-fold invariant. For `slowest` that is correct — it
does no capacity arithmetic and folding cannot affect a minimum. For `pending` it
is a small gap: a folded implementation would return a wrong pending count for
positions many laps apart, and no test would notice.

### SQ29 — A Function That Is Its Own Callee

The entire body, reproduced in full:

```
pub fn pending( producer : Seq, consumer : Seq ) -> u64
{
  consumer.distance_to( producer )
}
```

**Finding.** `pending` is `Seq::distance_to` with the arguments swapped and nothing added.

---

### SQ30 — The Parameter List as a Statement About Range

Two functions decline the parameter, and they are the two that would have no use for it:

```
laps_between( … , capacity )   answer bounded by nothing, but scaled by capacity
may_claim   ( … , capacity )   answer is a verdict about capacity
free_slots  ( … , capacity )   answer in 0..=capacity
pending     ( … )              answer unbounded
slowest     ( … )              answer is a position, not a count
```

**Finding.** `pending` and `slowest` are the two functions that take no `Capacity`, and they are also the two whose answers are not bounded by one — the parameter list states the mathematical fact.

---

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_four_readings_of_one_subtraction.md](../algorithm/001_four_readings_of_one_subtraction.md) | `pending` as the bare subtraction the other three build on |
| [../algorithm/002_the_slowest_fold.md](../algorithm/002_the_slowest_fold.md) | The fold, and the chain that carries it |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_argument_order_split.md](../api/002_the_argument_order_split.md) | Why `pending` reverses `distance_to`'s order |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_none_rather_than_zero_for_an_empty_set.md](../decisions/001_none_rather_than_zero_for_an_empty_set.md) | The `Option`, and its two opposite defaults |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_how_the_fold_crossed_four_tiers.md](../integration/002_how_the_fold_crossed_four_tiers.md) | Why `slowest` was wrapped and `Barrier::available` was bypassed |

### Items

| File | Relationship |
|------|--------------|
| [001_the_three_capacity_readings.md](001_the_three_capacity_readings.md) | The three that take a `Capacity` and perform a cast |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_the_option_that_slowest_returns.md](../type/002_the_option_that_slowest_returns.md) | What the `Option` promises |

### Sources

| File | Relationship |
|------|--------------|
| `ring_seqno/src/lib.rs:101-136` | Both functions and their docs |
| `ring_types/src/id.rs:82-85` | `distance_to`, which `pending` renames |
| `ring_barrier/src/lib.rs:216-219` | `available` — the duplicate, with no external callers |
| `ring_consume/src/lib.rs:213, 338-344` | Holds a `Barrier`, and re-expands its body |
| `ring_cursor/src/lib.rs:120-123` | `slowest`'s only cross-crate caller, and the `Vec` |

### Tests

| File | Relationship |
|------|--------------|
| `tests/seq_test.rs:104-110` | `pending`'s three cases |
| `tests/seq_test.rs:113-122` | The minimum, wherever it sits |
| `tests/seq_test.rs:125-131` | `None` is not `Some( ZERO )` |
