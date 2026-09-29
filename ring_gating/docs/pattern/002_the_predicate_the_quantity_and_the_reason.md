# Pattern: The Predicate, the Quantity and the Reason

### Scope

- **Purpose**: Name the three-rung API shape this crate is the family's only complete instance of, and record that the family applies it without agreeing on names.
- **Responsibility**: State the three rungs and what each is for, census the family's instances, and give the one place the rationale is written down.
- **In Scope**: The shape, its instances, and its naming.
- **Out of Scope**: Why the third rung is a `Result` — see [`decisions/002`](../decisions/002_a_result_rather_than_a_bool.md).

### The Three Rungs

| Rung | Question | Returns | For |
|------|----------|---------|-----|
| Quantity | *How many?* | a count | Sizing a batch — the caller decides how much to take |
| Predicate | *May I take this many?* | `bool` | A branch — the caller has already decided |
| Reason | *…and if not, why?* | `Result` | Control flow — retry, or stop |

Each rung is derivable from the one above it, and each exists so a caller does
not have to do the derivation:

```rust
headroom( producer )                     // the quantity
admits( producer, count )  ≡  count <= headroom( producer )
check ( producer, count )  ≡  admits, plus which of two reasons
```

The rungs are not redundancy. A caller that only needs the branch should not have
to know that zero is the boundary, and a caller that must not spin should not
have to infer permanence from a `false`.

### The Rationale, Written Down Once

The family states this argument in exactly one place, and it is not this crate:

> different questions — "how much room" is a batch-sizing input, "may I" is a
> branch — and a caller that only needs the branch should not have to know that
> zero is the boundary.
>
> — `ring_cursor/src/lib.rs:397-399`, on `CursorPair::may_claim`

`ring_gating` argues for its own third rung (`Result` over `bool`) and takes the
first two for granted. `ring_barrier` argues for neither. So the pattern's
justification exists, is good, and lives on the method of a different type in a
different crate — findable only by someone who already knows to look.

### The Family's Instances

| Type | Quantity | Predicate | Reason | Rungs |
|------|----------|-----------|--------|:-----:|
| `ring_cursor::CursorPair` | `free_slots()` | `may_claim()` | — | 2 |
| `ring_barrier::Barrier` | `available( from )` | `admits( from, count )` | — (but `wait_for` returns a `Result`) | 2 |
| `ring_gating::GatingSet` | `headroom( producer )` | `admits( producer, count )` | `check( producer, count )` | **3** |

```sh
cd "$(git rev-parse --show-toplevel)"
grep -rE '^\s*pub (const )?fn (admits|may_claim)\(' ring_*/src/*.rs
# ring_barrier:238, ring_cursor:417, ring_gating:242
```

Live output:

```
ring_barrier/src/lib.rs:  pub fn admits( &self, from : Seq, count : u64 ) -> bool
ring_bench/src/lib.rs:  pub const fn admits( self, producers : usize ) -> bool
ring_cursor/src/lib.rs:  pub fn may_claim( &self ) -> bool
ring_gating/src/lib.rs:  pub fn admits( &self, producer : Seq, count : usize ) -> bool
ring_seqno/src/lib.rs:pub fn may_claim( producer : Seq, consumer : Seq, capacity : Capacity ) -> bool
```

**This crate is the only complete instance.** The two 2-rung instances are not
incomplete by oversight:

- `CursorPair` gates one producer against one consumer, so there is only ever one
  reason a claim is refused. A `Result` would carry no information a `bool` does
  not.
- `Barrier` has no capacity, so `BatchTooLarge` has no analogue — its only
  refusal is `RingError::Empty`, and it is delivered by `wait_for` after the
  spins run out rather than by a check. See
  [`integration/002`](../integration/002_the_other_half_of_feature_178.md).

The third rung appears exactly where a caller can be wrong in two distinguishable
ways, which is the condition it exists for.

### G17 — Three Instances, Three Name Pairs

| Type | Quantity name | Predicate name |
|------|---------------|----------------|
| `CursorPair` | `free_slots` | `may_claim` |
| `Barrier` | `available` | `admits` |
| `GatingSet` | `headroom` | `admits` |

Three names for the quantity, two for the predicate. Every one is defensible in
isolation — `free_slots` is the literal count, `available` is the consumer's
word, `headroom` is the producer's — and together they mean a reader who learns
the shape on one type does not recognise it on the next.

There is a fourth name for the same quantity one layer down: `ring_seqno::free_slots`,
which `headroom` calls. So the value the producer knows as *headroom* is called
*free slots* by the function that computes it.

| Cost | Detail |
|------|--------|
| Recognition | The shape has to be rediscovered per type |
| Search | No single grep finds all instances — this document's uses two alternations |
| Documentation | Each type re-explains, or, as here, does not explain at all |

| Benefit | Detail |
|---------|--------|
| Each name reads correctly at its own call site | `barrier.available( … )` and `gate.headroom( … )` both say the right thing |
| No forced abstraction | Nothing had to become a trait to share a name |

This is a judgement, not a defect, and it is worth recording as one. The family's
own conventions call for one canonical term per concept; three names for one
quantity is a departure that nothing has yet paid for. Whether to converge —
and on which name — is a family-level decision, not this crate's.

### What Would Make the Shape Explicit

| Option | Cost | Effect |
|--------|------|--------|
| A trait with the three methods | Real — generic dispatch on a hot path, and the widths differ (`usize` vs `u64`, see [`type/001`](../type/001_a_usize_headroom_and_a_u64_limit.md)) | Enforces the shape, at a price |
| One naming convention, no trait | A rename across three crates | Recognition, no runtime cost |
| A family-level doc naming the pattern | One document | Discoverability only |

The second is the cheapest real improvement and the widths are the obstacle: a
trait is not viable while `GatingSet::admits` takes `usize` and `Barrier::admits`
takes `u64`, and reconciling those is
[`type/001`](../type/001_a_usize_headroom_and_a_u64_limit.md) § G15's open
question. The third is what this document is.

### GT44 — Four Sites, Three Name Pairs

```
ring_seqno     : free_slots / may_claim      ( free functions )
ring_cursor  : free_slots / may_claim      ( methods over them )
ring_barrier : available  / admits
ring_gating  : headroom   / admits
```

The same two-method shape, four times. The pair that repeats does so across a
layer boundary — a wrapper keeping its callee's names — so the three genuinely
independent choices are the other three.

**Finding.** It appears four times across four crates under three different name pairs — `free_slots`/`may_claim` twice, once as `ring_seqno`'s free functions and once as `ring_cursor`'s methods over them, then `available`/`admits` and `headroom`/`admits` — so the shape is a convention the family follows without naming, and the one pair that repeats does so across a layer boundary rather than between peers

---

### GT45 — The Rationale Survives in One Doc Comment

```
ring_cursor/src/lib.rs, above may_claim:
  /// Kept as its own method because the two readings answer
  /// different questions -- "how much room" is a batch-sizing input,
  /// "may I" is a branch
```

Four sites spell the pair. One argues for it. The reason the family carries two
methods where one would compute the same answer is written down exactly once.

**Finding.** It is written down once, in `ring_cursor::may_claim`'s doc — "how much room" is a batch-sizing input, "may I" is a branch — and nowhere else. Three further sites spell the pair without arguing it, so the reason the family keeps two methods over one survives in a single doc comment on the crate that happens to have written it

---

### GT46 — Only One of the Two Predicates Is Derived

```
ring_gating::admits  -> count <= self.headroom( producer )      ( derived )
ring_cursor::may_claim -> ring_seqno::may_claim( two fresh loads ) ( not )
```

A predicate defined in terms of its own quantity cannot disagree with it. A
predicate that takes its own readings can — and under concurrency, will.

**Finding.** `admits` is literally `count <= self.headroom( producer )` — the predicate defined in terms of the quantity, one line. `ring_cursor::may_claim` is not defined in terms of `free_slots`; it takes its own loads. Same pattern, two crates, opposite implementations, and only one is consistent with its own quantity by construction

---


### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_check_orders_its_two_refusals.md](../algorithm/002_check_orders_its_two_refusals.md) | The third rung, and why its two branches are ordered |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_reading_that_returns_a_position.md](../api/002_the_reading_that_returns_a_position.md) | The five readings, of which three are these rungs |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_a_result_rather_than_a_bool.md](../decisions/002_a_result_rather_than_a_bool.md) | The argument for the third rung, and its lack of callers |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_the_other_half_of_feature_178.md](../integration/002_the_other_half_of_feature_178.md) | Why `Barrier` stops at two rungs |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_three_gating_readings.md](../item/001_the_three_gating_readings.md) | The three rungs catalogued individually |

### Patterns

| File | Relationship |
|------|--------------|
| [001_the_owned_set_with_shared_readers.md](001_the_owned_set_with_shared_readers.md) | The structural pattern this one sits on |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_a_usize_headroom_and_a_u64_limit.md](../type/001_a_usize_headroom_and_a_u64_limit.md) | G15 — the width mismatch that blocks a trait |

### Sources

| File | Relationship |
|------|--------------|
| `ring_cursor/src/lib.rs:392-420` | `may_claim`, and the pattern's only written rationale |
| `ring_barrier/src/lib.rs:216-240` | `available` and `admits` |
| `ring_gating/src/lib.rs:221-294` | All three rungs |
| `ring_seqno/src/lib.rs:95-99` | `free_slots` — the fourth name for the quantity |

### Tests

| File | Relationship |
|------|--------------|
| `tests/gating_test.rs:252-275` | The second and third rungs, asserted to agree over 1,920 states |
| `tests/gating_test.rs:277-287` | The zero-count boundary the first rung hides from the second |
