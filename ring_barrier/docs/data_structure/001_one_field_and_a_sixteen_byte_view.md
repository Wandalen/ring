# Data Structure: One Field and a Sixteen-Byte View

### Scope

- **Purpose**: Describe the crate's only type by its layout, and account for the ratio between what it costs and what it points at.
- **Responsibility**: Give the field, the size, the padding on the far side of the pointer, and what a barrier does *not* store.
- **In Scope**: Memory.
- **Out of Scope**: The type's identity as an API — see [`api/002`](../api/002_the_borrow_is_the_whole_type.md).

### The Whole Type

```rust
// ring_barrier/src/lib.rs:80-84
#[ derive( Debug, Clone, Copy ) ]
pub struct Barrier< 'a >
{
  dependencies : &'a [ PaddedCursor ],
}
```

One private field. No capacity, no cached frontier, no consumer position, no
index, no flag.

### Sixteen Bytes Pointing at Sixty-Four Each

| | Bytes | Alignment |
|--|------:|----------:|
| `Barrier` | 16 | 8 |
| `&[ PaddedCursor ]` | 16 | 8 |
| One `PaddedCursor` | 64 | 64 |
| A barrier over 2 dependencies | 16 | pointing at 128 |
| A barrier over 8 dependencies | 16 | pointing at 512 |

`PaddedCursor` is `ring_align::CACHE_LINE` wide and aligned — `64` on every
target the family builds for — because two cursors in one struct must land on
different cache lines rather than merely at different addresses
(`ring_cursor/src/lib.rs:125-144`). So a barrier is a
16-byte handle onto an array where **every element occupies a full cache line
to hold eight bytes of sequence.**

That 8-in-64 ratio is `ring_align`'s deliberate cost and not this crate's
business, but it is what makes the borrow load-bearing rather than stylistic: a
barrier that owned copies of its dependencies would be copying 64 bytes per
dependency to hold positions that go stale the instant they are read.

### What Is Not Stored, and What That Buys

| Not stored | Consequence |
|------------|-------------|
| The frontier | Every read is fresh; no invalidation, no staleness window this crate controls |
| The consumer's own position | `from` is a parameter on three methods, so one barrier serves many consumers |
| Capacity | The arithmetic cannot clamp even by accident — [`invariant/002`](../invariant/002_capacity_never_enters_the_arithmetic.md) |
| A `Vec` | Construction allocates nothing; only *reading* allocates, and in another crate — [`non_functional_requirement/001`](../non_functional_requirement/001_every_frontier_read_allocates_nothing.md) |

The third row is the one worth pausing on. `available` takes `from : Seq` rather
than reading a stored position, which means a single `Barrier` value is
correct for any number of consumers reading behind the same dependencies. The
family does not currently exercise that — `ring_consume::Consumer` pairs one
barrier with one position — but nothing in the type prevents it, and the
alternative shape (a barrier that remembers *whose* barrier it is) would.

### The Asymmetry With `GatingSet`

`Barrier` and `ring_gating`'s `GatingSet` hold the same element type and store it
oppositely:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -A5 'pub struct GatingSet' ring_gating/src/lib.rs
grep -A4 'pub struct Barrier'   ring_barrier/src/lib.rs
```

Live output:

```
pub struct GatingSet
{
  cursors : Vec< PaddedCursor >,
  capacity : Capacity,
}

pub struct Barrier< 'a >
{
  dependencies : &'a [ PaddedCursor ],
}
```

| | `GatingSet` | `Barrier` |
|--|-------------|-----------|
| Cursors | `Vec< PaddedCursor >`, owned | `&'a [ PaddedCursor ]`, borrowed |
| Capacity | `Capacity` field | absent |
| Size | 32 bytes + heap | 16 bytes, no heap |
| Construction allocates | Yes (`consumers > 0`) | Never |
| `Copy` | No | Yes |
| Has a destructor | Via the `Vec` | No |

Neither is the better shape in general; each matches who owns what. A producer
creates the consumer cursors it gates on, so owning them is right. A consumer
waits on cursors somebody else created, so borrowing them is right. The pattern
that names this is
[`pattern/001`](../pattern/001_the_borrowed_view_and_the_owned_set.md), and the
one place it was forced rather than chosen is
[`decisions/002`](../decisions/002_a_slice_rather_than_an_aggregate.md).

### BR27 — Formatting a Barrier Reads Every Dependency's Atomic

`Barrier` derives `Debug`, and `PaddedCursor` derives it too. `AtomicU64`'s
`Debug` implementation performs a load. So `{:?}` on a barrier over eight
dependencies performs eight atomic reads of state other threads are writing —
from a formatting call, in a log line, at whatever ordering the standard library
picked rather than the `Acquire` every deliberate read in this family uses.

Nothing is unsound about it. It is worth recording because the crate is
otherwise scrupulous about where its loads happen: `frontier` delegates to
`ring_cursor::slowest` precisely so the ordering lives in one place, and a
derive placed on line 80 quietly opens a second door to the same memory that
answers to nobody's ordering choice.

```sh
cd "$(git rev-parse --show-toplevel)"
grep "derive" ring_barrier/src/lib.rs ring_cursor/src/lib.rs | head -4
# the crate's deliberate reads, all in one delegated place
grep "Ordering::" ring_barrier/src/lib.rs || echo '(the crate names no ordering of its own)'
# control: the identical expression over the crate that owns the loads
grep -c "Ordering::" ring_cursor/src/lib.rs
```

Live output:

```
ring_barrier/src/lib.rs:#[derive(Debug, Clone, Copy)]
ring_cursor/src/lib.rs:#[derive(Debug, Default)]
ring_cursor/src/lib.rs:#[derive(Debug)]
/// published[0].store(Seq(5), Ordering::Release);
    ///     cursor.store(Seq(10 + i as u64), Ordering::Release);
    /// cursors[0].store(Seq(4), Ordering::Release);
    /// cursors[0].store(Seq(3), Ordering::Release);
    /// cursors[0].store(Seq(6), Ordering::Release);
14
```

### BR28 — Ten of Thirty-Three Crates Carry a Lifetime, and This Is the Smallest

A lifetime parameter on a struct is the family's marker for *a view over
somebody else's memory*, and ten crates carry one. `Barrier` is the smallest of
them: one field, sixteen bytes, no owned state of any kind, `Copy`.

`GatingSet` — the producer-side half of the same feature — has no lifetime and
is not `Copy`, because it owns its cursors. That asymmetry is the entire design
difference between the two crates reduced to a derive line, and neither crate's
documentation states it in those terms.

```sh
cd "$(git rev-parse --show-toplevel)"
grep -rl "struct [A-Za-z]*< *'a" --include=*.rs ring_*/src/ \
  | sed 's|ring/||;s|/src/lib.rs||' | sort
# the consumer half (ring_barrier) and producer half (ring_gating), side by side
grep "derive" ring_barrier/src/lib.rs ring_gating/src/lib.rs | grep -v '//' | head -4
```

Live output:

```
ring_barrier
ring_claim
ring_consume
ring_core
ring_flush
ring_handle
ring_mpsc
ring_shutdown
ring_spsc
ring_tls
ring_barrier/src/lib.rs:#[derive(Debug, Clone, Copy)]
ring_gating/src/lib.rs:#[derive(Debug)]
```

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_nine_methods_over_one_borrowed_slice.md](../api/001_nine_methods_over_one_borrowed_slice.md) | The nine methods over this one field |
| [../api/002_the_borrow_is_the_whole_type.md](../api/002_the_borrow_is_the_whole_type.md) | `Copy`, `Send`, `Sync`, and the escaping `&'a` |

### Data Structures

| File | Relationship |
|------|--------------|
| [002_the_slices_three_provenances.md](002_the_slices_three_provenances.md) | Where the pointed-at cursors come from |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_a_slice_rather_than_an_aggregate.md](../decisions/002_a_slice_rather_than_an_aggregate.md) | Why the field is a borrow |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_capacity_never_enters_the_arithmetic.md](../invariant/002_capacity_never_enters_the_arithmetic.md) | The absent field, stated as a property |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_every_frontier_read_allocates_nothing.md](../non_functional_requirement/001_every_frontier_read_allocates_nothing.md) | The allocation that is not in this type |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_the_borrowed_view_and_the_owned_set.md](../pattern/001_the_borrowed_view_and_the_owned_set.md) | The two shapes as one pattern |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_the_traits_derived_and_the_traits_absent.md](../type/002_the_traits_derived_and_the_traits_absent.md) | What the three derives cost at this size |

### Sources

| File | Relationship |
|------|--------------|
| `ring_barrier/src/lib.rs:80-84` | The struct |
| `ring_cursor/src/lib.rs:125-144` | `PaddedCursor`'s size and alignment |
| `ring_align/src/lib.rs:36` | `CACHE_LINE = 64` |
| `ring_gating/src/lib.rs` | `GatingSet`, for the ownership contrast |

### Tests

| File | Relationship |
|------|--------------|
| `tests/barrier_test.rs:419-434` | The field survives round-tripping through `dependencies()` |
| `tests/barrier_test.rs:436-449` | A barrier over a `GatingSet`'s cursors reads that set and not a copy |
