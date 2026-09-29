# Data Structure: Owning the Cursors Rather Than Borrowing Them

### Scope

- **Purpose**: Give the argument for `GatingSet` owning a `Vec< PaddedCursor >` rather than borrowing a slice, and show what the ownership costs and prevents.
- **Responsibility**: State the failure modes a borrowed set would allow, establish the memory layout with the test that asserts it, and contrast against `ring_barrier`, which made the opposite choice for the same kind of data.
- **In Scope**: The field, its layout, and the ownership decision.
- **Out of Scope**: That the count is fixed — see [`001`](001_the_set_that_cannot_grow.md).

### The Field

```rust
// ring_gating/src/lib.rs:69-74
#[ derive( Debug ) ]
pub struct GatingSet
{
  cursors : Vec< PaddedCursor >,
  capacity : Capacity,
}
```

Both fields private. `Debug` is the only derive — `Clone` is unavailable
(`AtomicU64` is not `Clone`), and `Default` would have to invent a capacity,
which `Capacity` deliberately refuses to supply.

### What Borrowing Would Allow

The crate states the argument on the type:

> Owns its cursors rather than borrowing them, so that the set and the cursors
> cannot get out of sync — a set holding references to cursors that outlive it,
> or fewer cursors than it was built for, is a bound that reads correctly and
> gates nothing.

Both failure modes are worth spelling out, because "reads correctly and gates
nothing" is the dangerous property:

| If the set borrowed | Failure | What the gate reports |
|---------------------|---------|-----------------------|
| A slice shorter than the consumer count | A real consumer is not in the fold | A **larger** headroom than exists — the producer laps the missing consumer |
| A slice of cursors nobody advances | The fold sees stale zeros | A smaller headroom — safe, merely slow |
| A slice outliving its cursors | Use-after-free | Prevented by the borrow checker, not by design |

Row 1 is the one that matters and it is the permissive direction. A gating set
missing a consumer does not fail loudly; it silently stops gating against that
consumer, which is exactly the corruption this design exists to prevent. Ownership
makes the count and the cursors the same fact rather than two facts that must
agree.

### `ring_barrier` Made the Opposite Choice

The consumer half of the same feature borrows:

```rust
// ring_barrier — the dependencies are a bare slice
Barrier::over( &cursors )   // &[ PaddedCursor ]
```

and argues for it just as explicitly:

> A [`Barrier`] borrows `&[PaddedCursor]`, not a `ring_gating::GatingSet`. A
> `GatingSet` is a producer-side aggregate — it owns consumer cursors and carries
> the capacity that bounds the producer — and a barrier's dependencies are neither
> owned by it nor related to capacity. They are wherever they happen to live.

**Both are right, and the reason they differ is where the cursors come from:**

| | `GatingSet` | `Barrier` |
|---|---|---|
| Who creates the cursors | This type, in `new` | Someone else, before the barrier exists |
| Where they live | Here | `Publisher::cursor`, `GatingSet::cursors`, a bare array in a test |
| Can the count be known here | Yes — it is the constructor argument | No — the caller assembles the dependency list |
| Consequence | Own them | Borrow them |

`ring_barrier`'s module doc records that taking the aggregate instead "is what
made the four-operation handshake in `ring_publish/tests/handshake_test.rs`
unwireable until this signature changed" — a borrowed barrier can depend on a
*publisher's* cursor, which no owning type could ever hold.

So `ring_gating` and `ring_barrier` hold the same kind of data in opposite ways, and
the asymmetry is a consequence of provenance rather than an inconsistency.

### The Layout

`PaddedCursor` is `CacheAligned< AtomicSeq >` — `#[ repr( align( 64 ) ) ]` around
an `AtomicU64`, so 64 bytes of which 8 carry the sequence. A `Vec` gives each
element a full stride, so a set of *n* consumers occupies `64n` bytes with no two
cursors on one line.

That is asserted directly, and the test says why it is asserted *here*:

```rust
// tests/gating_test.rs:341-353
// A `GatingSet` is where several consumers' cursors are most likely to end
// up adjacent, so it is where false sharing would actually bite. The `Vec`
// gives each element a full stride because `PaddedCursor` is a whole line.
let set = GatingSet::new( cap( 4 ), 4 );

for window in set.cursors().windows( 2 )
{
  assert_eq!( window[ 1 ].addr() - window[ 0 ].addr(), 64 );
}
```

**The separation is inherited, not produced.** This crate contains no alignment
attribute, no padding field, and no reference to a cache line:

```sh
cd "$(git rev-parse --show-toplevel)"
# this crate: no alignment attribute, no padding field, no cache line
grep -cE 'align|CACHE_LINE|(^|[^a-z0-9_])64([^0-9]|$)' ring_gating/src/lib.rs || true
# control — the identical expression over the crate that holds the attribute
grep -cE 'align|CACHE_LINE|(^|[^a-z0-9_])64([^0-9]|$)' ring_align/src/lib.rs
```

Live output:

```
0
24
```

The word boundary on `64` is not decoration — a bare `64` matches inside `u64`,
and the file has two `as u64` casts. A looser pattern reports two hits and reads
as if the crate did mention the cache line.

The property comes entirely from `PaddedCursor`'s own `repr`, and this test is
the family's check that a `Vec` does not undo it. That is the right place for it —
`ring_align` asserts the type is 64 bytes, and `ring_gating` asserts that putting
several of them in a row keeps them 64 apart, which is a different claim.

The test's own limit is worth naming: it hard-codes `64`, so on a machine with
128-byte lines it would pass while two cursors shared a line. That is
[`ring_cursor`'s F15](../../../ring_cursor/docs/invariant/001_one_cursor_one_line.md),
inherited here — the clauses are internally consistent and externally wrong on
such a target.

### The Cost

| Cost | Detail |
|------|--------|
| One heap allocation per set | At `new`, sized `64 × consumers`. Once per ring, not per operation |
| 64 bytes per consumer for 8 bytes of state | The point of the padding; deliberate |
| A `Vec` where a boxed slice would say more | 8 extra bytes and an unwanted `push` — see [`001`](001_the_set_that_cannot_grow.md) § *What `Vec` Buys* |
| Cursors cannot be shared with a `Barrier` by value | They are shared by reference instead: `Barrier::over( set.cursors() )` works, and is the chained-consumer case `ring_barrier` documents |

The last row is the important one, and it is not a cost: ownership here does not
prevent the barrier from depending on these cursors, because `cursors()` hands
out exactly the `&[ PaddedCursor ]` a barrier takes. The two designs compose.

### GT13 — One Production Call Site, and It Passes One

```
ring_mpsc/src/lib.rs:380:      consumers : GatingSet::new( capacity, 1 ),
tests constructing a set: ring_barrier, ring_claim, ring_gating, ring_publish
```

Ownership is the decision this instance is about, and the question of who
actually exercises it has one answer in shipping code.

**Finding.** Exactly one call site exists in the production source of all 33 crates, and it passes `consumers = 1`; every multi-consumer construction in the workspace is in a test

---

### GT14 — The Stride Is Inherited, Not Established

```
size_of::< PaddedCursor >() == 64      <- ring_cursor's promise
&cursors[ 1 ] - &cursors[ 0 ] == 64    <- what this crate asserts
```

Owning the cursors is what lets this crate promise they are one per cache line.
The 64 itself comes from somewhere else.

**Finding.** The 64-byte stride is asserted, and it comes from `PaddedCursor`'s size rather than from anything this crate does — so the property that keeps consumers off each other's cache lines is inherited, and this crate's test of it is a test of `ring_cursor`

---


### APIs

| File | Relationship |
|------|--------------|
| [../api/001_eleven_methods_over_one_owned_vec.md](../api/001_eleven_methods_over_one_owned_vec.md) | The accessors this field is reached through |

### Data Structures

| File | Relationship |
|------|--------------|
| [001_the_set_that_cannot_grow.md](001_the_set_that_cannot_grow.md) | The other half of the field's shape |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_the_other_half_of_feature_178.md](../integration/002_the_other_half_of_feature_178.md) | `ring_barrier`'s opposite choice, in full |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_every_gating_read_allocates_nothing.md](../non_functional_requirement/001_every_gating_read_allocates_nothing.md) | The other allocation — the per-read one this field never caused, removed a crate over by `b7e075ca` |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_send_and_sync_without_unsafe.md](../type/002_send_and_sync_without_unsafe.md) | What owning atomics gives the type for free |

### Sources

| File | Relationship |
|------|--------------|
| `ring_gating/src/lib.rs:45-74` | The type, its doc, and the ownership argument |
| `ring_gating/src/lib.rs:147-158` | `cursors()` — the borrow a `Barrier` can consume |
| `ring_barrier/src/lib.rs:30-44` | The opposite choice, argued |
| `ring_align/src/lib.rs:67-73` | `CacheAligned`, where the 64 bytes come from |

### Tests

| File | Relationship |
|------|--------------|
| `tests/gating_test.rs:341-353` | The 64-byte stride across a four-consumer set |
| `tests/gating_test.rs:329-339` | Every cursor starts at zero, and the fold agrees |
| `tests/gating_test.rs:355-359` | The set remembers its capacity |
