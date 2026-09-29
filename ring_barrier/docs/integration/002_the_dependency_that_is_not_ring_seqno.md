# Integration: The Dependency That Is Not `ring_seqno`

### Scope

- **Purpose**: Account for the one slot in which this crate's dependency list differs from its sibling's, and show that the difference is a clamp rather than a preference.
- **Responsibility**: Compare the two manifests, trace why `ring_seqno` is unnecessary here and unavoidable there, and note what the third slot actually buys.
- **In Scope**: The third `[dependencies]` line.
- **Out of Scope**: The full edge census — see [`001`](001_three_dependencies_and_one_dependent.md).

### Two Manifests, One Difference

```sh
cd "$(git rev-parse --show-toplevel)"
grep -A4 '^\[dependencies\]' ring_barrier/Cargo.toml
grep -A4 '^\[dependencies\]' ring_gating/Cargo.toml
```

Live output:

```
[dependencies]
ring_types = { path = "../ring_types" }
ring_cursor = { path = "../ring_cursor" }
ring_wait = { path = "../ring_wait" }

[dependencies]
ring_types = { path = "../ring_types" }
ring_cursor = { path = "../ring_cursor" }
ring_seqno = { path = "../ring_seqno" }
```

| Slot | `ring_gating` | `ring_barrier` |
|------|---------------|----------------|
| 1 | `ring_types` | `ring_types` |
| 2 | `ring_cursor` | `ring_cursor` |
| 3 | **`ring_seqno`** | **`ring_wait`** |

`ring_gating` and `ring_barrier` agree on their first two dependencies exactly and
disagree on the third. Neither picked its third slot for balance.

### Why `ring_gating` Cannot Drop `ring_seqno`

Its answer is clamped:

```rust
// ring_gating/src/lib.rs — headroom
ring_seqno::free_slots( producer, slowest, capacity )
```

`free_slots` needs a capacity, a producer position, a consumer position, and a
subtraction in a specific order — `capacity.saturating_sub( in_flight )`. There
is no inherent method on `Seq` that does that, because it is not a fact about
two sequences; it is a fact about two sequences *and a ring*. So the arithmetic
has to live somewhere, and `ring_seqno` is where the family keeps sequence
arithmetic.

### Why `ring_barrier` Does Not Need It

Its answer is not clamped:

```rust
// ring_barrier/src/lib.rs:218
self.frontier().map_or( 0, | frontier | from.distance_to( frontier ) )
```

`Seq::distance_to` is a `const fn` on `Seq` itself, in `ring_types` — slot 1,
already declared for the type's own sake. The whole of this crate's arithmetic
is one method on a type it imports for its signatures, so the third slot is
free.

```sh
cd "$(git rev-parse --show-toplevel)"
grep -vE "^[[:space:]]*(///|//!)" ring_barrier/src/lib.rs | grep -c 'ring_seqno'   # 0
# The command above can legitimately exit nonzero -- a count of zero, a diff
# that differs -- and the sweep that keeps this output current skips any block
# that exits nonzero, so without this the block freezes instead of refreshing.
:
```

Live output:

```
0
```

**Zero** — and `ring_seqno` is nonetheless reached on every call, at step 4 of the
chain, through `ring_cursor::slowest`
([`algorithm/001`](../algorithm/001_the_frontier_in_two_delegations.md)).
Transitive use is not a reason to declare an edge; naming a symbol is, and this
crate names none.

That is the whole content of the difference:

> **The two halves differ by exactly one clamp, and the clamp is what costs a
> crate.**

### What the Third Slot Bought Instead

`ring_wait` is Tier 4 and depends on `ring_types` and `ring_cursor` — this
crate's other two dependencies and nothing else. So the manifest reads as a
strict extension:

| | Declares |
|--|----------|
| `ring_wait` | `ring_types`, `ring_cursor` |
| `ring_barrier` | `ring_types`, `ring_cursor`, **`ring_wait`** |

One tier up, one crate added, no new leaves. And it is used by exactly one
method:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -vE "^[[:space:]]*(///|//!)" ring_barrier/src/lib.rs | grep -c 'ring_wait'   # 1
```

Live output:

```
1
```

That method is `wait_for`, which has **zero callers outside its own test file**
([`item/002`](../item/002_the_five_accessors_and_the_wait.md) § BR4). A third of
the dependency list serves the one method nothing in the family calls.

| | |
|--|--|
| Is the edge unused? | No — `wait_for` is real, tested, and correct |
| Is it *exercised outside tests*? | No |
| Would dropping `wait_for` remove the edge? | Yes, entirely |
| Should it be dropped? | Not on this evidence — this crate is expected to block, and a barrier that can only be polled pushes the spin loop into every caller |

Recorded because the shape is worth seeing: the crate's cheapest, most-used
methods need only its first two dependencies, and its third exists for a
capability nothing has yet taken up.

### Where the Boundary Would Move

| If | Then |
|----|------|
| A caller needs a capacity-clamped consumer bound | `ring_seqno` returns, and the two crates' manifests converge — which would be a signal the split is wrong |
| `wait_for` is deleted | `ring_wait` goes, and the crate is `ring_types` + `ring_cursor` — three tiers of delegation for one `map_or` |
| ~~`ring_cursor::slowest` moves to an iterator signature~~ — **happened, `b7e075ca`** | Nothing changed here, and the allocation went with it — [`non_functional_requirement/001`](../non_functional_requirement/001_every_frontier_read_allocates_nothing.md) |

### BR34 — Every Arithmetic Helper This Crate Might Have Used Lives in a Crate It Does Not Name

`ring_seqno` exports `free_slots`, `has_room`, `in_flight`, `pending` and
`laps_between` — the family's arithmetic over pairs of sequences. `ring_barrier`
depends on none of it, and computes its one quantity with
`Seq::distance_to` from `ring_types` instead.

That is the correct edge to skip: every `ring_seqno` helper takes a capacity, and
this crate's whole invariant is that capacity does not enter the arithmetic. But
it means the one crate named for sequence arithmetic is unreachable from the one
crate whose only job is a sequence subtraction, and the reason is a parameter
none of the five helpers could be given without breaking it.

```sh
cd "$(git rev-parse --show-toplevel)"
grep -E "^pub (const )?fn " ring_seqno/src/lib.rs
# every one takes a capacity; this crate's answer must not
grep -c "capacity" ring_seqno/src/lib.rs
grep -c "capacity" ring_barrier/src/lib.rs || echo 0
```

Live output:

```
pub fn laps_between( earlier : Seq, later : Seq, capacity : Capacity ) -> u64
pub fn may_claim( producer : Seq, consumer : Seq, capacity : Capacity ) -> bool
pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity ) -> usize
pub fn pending( producer : Seq, consumer : Seq ) -> u64
pub fn slowest( cursors : &[ Seq ] ) -> Option< Seq >
10
5
```

### Integrations

| File | Relationship |
|------|--------------|
| [001_three_dependencies_and_one_dependent.md](001_three_dependencies_and_one_dependent.md) | Every edge, including the reverse ones |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_the_frontier_in_two_delegations.md](../algorithm/001_the_frontier_in_two_delegations.md) | The clamp comparison, step by step |
| [../algorithm/002_wait_for_asks_twice.md](../algorithm/002_wait_for_asks_twice.md) | What `ring_wait` is used for |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_capacity_never_enters_the_arithmetic.md](../invariant/002_capacity_never_enters_the_arithmetic.md) | The absence that makes slot 3 free |

### Items

| File | Relationship |
|------|--------------|
| [../item/002_the_five_accessors_and_the_wait.md](../item/002_the_five_accessors_and_the_wait.md) | `wait_for`'s zero non-test callers |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_a_non_blocking_wait_must_look_exactly_once.md](../non_functional_requirement/002_a_non_blocking_wait_must_look_exactly_once.md) | The contract `ring_wait` supplies |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_a_u64_distance_and_a_usize_headroom.md](../type/001_a_u64_distance_and_a_usize_headroom.md) | The width difference the clamp causes |

### Sources

| File | Relationship |
|------|--------------|
| `ring_barrier/Cargo.toml` | Slot 3 |
| `ring_gating/Cargo.toml` | The other slot 3 |
| `ring_wait/Cargo.toml` | Two dependencies, both already here |
| `ring_types/src/id.rs:82-85` | `Seq::distance_to`, the arithmetic that needs no crate |
| `ring_seqno/src/lib.rs` | `free_slots`, the arithmetic that does |

### Tests

| File | Relationship |
|------|--------------|
| `tests/barrier_test.rs:179-194` | `available_ignores_capacity_entirely` — the clamp's absence asserted |
| `tests/barrier_test.rs:326-339` | The `ring_wait` edge exercised at its strictest |
