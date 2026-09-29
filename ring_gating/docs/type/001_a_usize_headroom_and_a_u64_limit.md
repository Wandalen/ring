# Type: A `usize` Headroom and a `u64` Limit

### Scope

- **Purpose**: Account for the crate's width choices — why a count is `usize` and a position is `u64` — and locate every conversion between them.
- **Responsibility**: Give the rule, find the one cast this crate writes and the two it inherits, and record where the family's two halves disagree.
- **In Scope**: Integer widths across the public surface.
- **Out of Scope**: The 32-bit truncation in `free_slots` — owned and documented by [`ring_seqno` `pitfall/002`](../../../ring_seqno/docs/pitfall/002_reading_free_slots_on_a_narrow_target.md).

### The Rule

| Kind | Width | Why |
|------|-------|-----|
| A **count** of slots | `usize` | Bounded by capacity, which is a `usize` because it indexes memory |
| A **position** in the sequence | `u64` (inside `Seq`) | Never wraps in practice; must not narrow on a 32-bit target |

Applied across the surface:

| Method | Returns / takes | Kind |
|--------|-----------------|------|
| `len` | `usize` | count |
| `capacity` | `Capacity( usize )` | count |
| `headroom` | `usize` | count |
| `admits( _, count : usize )` | `usize` | count |
| `check( _, count : usize )` | `usize` | count |
| `slowest` | `Option< Seq >` | position |
| `limit` | `Option< Seq >` | position |
| `cursor( index : usize )` | `usize` | an index, which is a count of elements |

The rule is applied without exception. A `Seq` never appears where a quantity is
meant, and a `usize` never appears where a position is.

### The One Cast This Crate Writes

```sh
cd "$(git rev-parse --show-toplevel)"
grep -vE "^[[:space:]]*(///|//!)" ring_gating/src/lib.rs | grep ' as '
# 323:    self.slowest().map( | s | s.advanced_by( self.capacity.get() as u64 ) )
```

Live output:

```
    self.slowest().map( | s | s.advanced_by( self.capacity.get() as u64 ) )
```

**One `as` in the whole crate**, and it is the safe direction: `usize → u64`
widens on every target Rust supports. It appears in `limit`, where a capacity —
a count — has to be added to a position, and the position's width wins.

That is the correct direction for this conversion. The count is bounded by the
ring's size; the position is not.

### The Two Conversions It Inherits

| Conversion | Where | Direction | Risk |
|------------|-------|-----------|------|
| `u64 → usize` | `ring_seqno::free_slots`, on `distance_to`'s result | **narrowing** | Truncates above 2³² on a 32-bit target |
| `usize → u64` | `Seq::advanced_by`'s argument | widening | None |

The narrowing one reaches this crate through `headroom` and is `ring_seqno`'s
finding, not this crate's — recorded there as a reachable defect on 32-bit
targets. `ring_gating` cannot fix it and does not restate it; it inherits it, as
every `free_slots` caller does.

### `limit`'s Addition Is Not Saturating

```rust
// ring_gating/src/lib.rs:323
self.slowest().map( | s | s.advanced_by( self.capacity.get() as u64 ) )
```

`Seq::advanced_by` is `Self( self.0 + n )` — plain addition. So `limit()` on a set
whose slowest consumer is within `capacity` of `u64::MAX` overflows: a panic in a
debug build, a wrap in release.

| Property | Value |
|----------|-------|
| Reachable | At ~1.8 × 10¹⁹ sequences |
| At 1M messages/second | ~584,000 years |
| Same as | `Seq::next`, and every other `advanced_by` caller |
| Workspace has `overflow-checks` override? | No — debug panics, release wraps |

Worth stating and not worth fixing. It is the family-wide condition recorded as
`ring_seqno` finding S11, not a `ring_gating` defect, and the crate is consistent
with its siblings in not guarding it.

### G15 — The Two Halves Disagree on a Count's Width

`ring_gating` and `ring_barrier` implement the same feature from two sides
([`integration/002`](../integration/002_the_other_half_of_feature_178.md)) and
name their predicate identically. The signatures differ:

```rust
// ring_gating
pub fn admits( &self, producer : Seq, count : usize ) -> bool

// ring_barrier
pub fn admits( &self, from : Seq, count : u64 ) -> bool
```

| | quantity method | count parameter |
|---|---|---|
| `GatingSet` | `headroom -> usize` | `usize` |
| `Barrier` | `available -> u64` | `u64` |

Each is internally consistent, and each follows from its own third dependency:

- `GatingSet::headroom` ends in `ring_seqno::free_slots`, which returns `usize`
  because it subtracts from a `Capacity( usize )`.
- `Barrier::available` ends in `Seq::distance_to`, which returns `u64` because it
  subtracts two positions and never touches a capacity.

So the divergence is a consequence of the dependency split G9 identifies, not an
oversight — `ring_barrier` has no capacity to bound against, so it has no reason
to reach `usize` at all.

**The cost is at the joint.** A consumer chained behind a producer reads
`Barrier::available` (a `u64`) and a producer reads `GatingSet::headroom` (a
`usize`); code holding both must convert, and on a 32-bit target that conversion
is the narrowing one. Nothing in the family does this today — no crate calls both
— so the collision is latent rather than live.

| Resolution | Cost |
|------------|------|
| `Barrier::available -> usize` | Wrong on 32-bit: an available count is bounded by a position distance, not a capacity |
| `GatingSet::headroom -> u64` | Forces every caller to narrow before indexing; moves the same cast one level up |
| Leave it | A conversion at the joint, whenever a crate needs both |

The second is worth noting as the near-miss it is: `headroom`'s result is used to
index memory in every caller, so `usize` is where the value has to end up. Making
it `u64` would relocate the truncation rather than remove it.

### GT51 — The Two Halves Disagree on a Count

```
ring_gating/src/lib.rs:242:  pub fn admits( &self, producer : Seq, count : usize ) -> bool
ring_barrier/src/lib.rs:238:  pub fn admits( &self, from : Seq, count : u64 ) -> bool
```

Same method name, same question, two widths. A caller holding one count and
asking both `GatingSet` and `Barrier` casts between them, and neither doc comment
mentions the other side.

**Finding.** `ring_gating` and `ring_barrier` disagree on it: `GatingSet::admits` takes `usize`, `Barrier::admits` takes `u64`, and neither doc comment mentions the other side

---

### GT52 — Where the Two Widths Meet

```
323:      self.capacity.get() as u64
      Capacity::get -> usize ; Seq::advanced_by -> u64
      the only `as` in the crate
```

The disagreement costs exactly one line, in the one method that has to express
a size as a position. On every target the workspace builds for it is a
widening.

**Finding.** In `limit`, the crate's only `as` conversion — a widening on every target the workspace builds for, and the single line where the disagreement in GT51 costs anything

---


### Types

| File | Relationship |
|------|--------------|
| [002_send_and_sync_without_unsafe.md](002_send_and_sync_without_unsafe.md) | The other thing the field types decide for this crate |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_reading_that_returns_a_position.md](../api/002_the_reading_that_returns_a_position.md) | The count/position split as a return-shape question |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_headroom_in_two_delegations.md](../algorithm/001_headroom_in_two_delegations.md) | Where the inherited narrowing enters |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_the_other_half_of_feature_178.md](../integration/002_the_other_half_of_feature_178.md) | G9 — the dependency split G15 follows from |

### Items

| File | Relationship |
|------|--------------|
| [../item/002_the_four_accessors_and_the_limit.md](../item/002_the_four_accessors_and_the_limit.md) | `limit` and `capacity` in their catalogue entries |

### Sources

| File | Relationship |
|------|--------------|
| `ring_gating/src/lib.rs:323` | The crate's one cast |
| `ring_types/src/capacity.rs:23` | `Capacity( usize )` |
| `ring_types/src/id.rs:25` | `Seq( pub u64 )` |
| `ring_types/src/id.rs:65-68` | `advanced_by` — plain addition |
| `ring_seqno/src/lib.rs:95-99` | The inherited narrowing |
| `ring_barrier/src/lib.rs:216,238` | `available` and `admits` at `u64` |

### Tests

| File | Relationship |
|------|--------------|
| `tests/gating_test.rs:176-188` | `headroom` saturating rather than wrapping, across 60 positions |
| `tests/gating_test.rs:291-311` | `limit`'s addition, at positions well below any overflow |
| `tests/gating_test.rs:190-205` | A producer at `u32::MAX as u64`, past a 32-bit boundary |
