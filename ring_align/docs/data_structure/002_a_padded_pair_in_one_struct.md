# Data Structure: A Padded Pair in One Struct

### Scope

- **Purpose**: Document the arrangement the crate's guarantee is actually about — two padded fields in one owning type — and why holding both halves in one type is what discharges the obligation the wrapper alone cannot.
- **Responsibility**: Give the layout, the size arithmetic, the failure the arrangement forecloses, and the real instance of it in the family.
- **In Scope**: The two-field layout; `CursorPair` as its realisation; the 2-vs-3-line cost question.
- **Out of Scope**: The single wrapper's own shape, which is [`data_structure/001`](001_the_cache_aligned_wrapper.md); what the cursors mean, which is [`ring_cursor`](../../../ring_cursor/readme.md)'s.

### Layout

```rust
struct TwoCursors
{
  producer : CacheAligned< u64 >,
  consumer : CacheAligned< u64 >,
}
```

```
 line 0                          line 1
┌───────────────────────────────┬───────────────────────────────┐
│ producer                      │ consumer                      │
│ payload 8B │ padding 56B      │ payload 8B │ padding 56B      │
│ offset 0                      │ offset 64                     │
└───────────────────────────────┴───────────────────────────────┘
```

**A write to `producer` invalidates line 0 only.** That is the entire product of
the crate. `consumer` sits in line 1 and its cached copy on another core
survives.

The unpadded form, for contrast — the negative control the test suite builds:

```
 line 0
┌────────────┬────────────┬───────────────────────────────────────┐
│ producer 8B│ consumer 8B│ 48 bytes of whatever follows          │
└────────────┴────────────┴───────────────────────────────────────┘
```

Both in line 0. Every write by either invalidates the other's copy — two
logically independent variables made into a hardware dependency, which is
false sharing exactly.

### Why the Arrangement, Not the Wrapper, Carries the Property

The wrapper guarantees its own size and alignment
(→ [`data_structure/001`](001_the_cache_aligned_wrapper.md)). It cannot
guarantee that a *second* hot value was also wrapped — and one padded field
beside one unpadded field still contends, because false sharing is symmetric.

So the property lives at the level of the owning struct, and the owning struct
is where it can be discharged **by construction**: a type that holds both
cursors, and offers no way to obtain one without the other, cannot be built in
the failing configuration. There is no check involved. The failing arrangement
is simply not expressible.

That is what `ring_cursor::CursorPair` is:

```sh
cd "$(git rev-parse --show-toplevel)"
grep "struct CursorPair" -A6 ring_cursor/src/lib.rs
```

Live output:

```
pub struct CursorPair
{
  producer : PaddedCursor,
  consumer : PaddedCursor,
  capacity : Capacity,
}
```

**This is the general shape worth carrying to the other 32 crates:** where a
guarantee needs two things to be true together, the type that owns both is the
only place it can be made structural rather than conventional.

### The Cost, and Where It Is Bounded

Two padded cursors cost 128 bytes to hold 16 bytes of state. `ring_cursor`
asserts the total does not drift past that:

```rust
assert!( size >= 2 * CACHE_LINE, "two cursors at minimum, got {size}" );
assert!( size <= 3 * CACHE_LINE, "capacity must not cost a whole extra line, got {size}" );
assert_eq!( core::mem::align_of::< CursorPair >(), CACHE_LINE );
```

**The upper bound is the interesting assertion.** `CursorPair` carries a
capacity alongside the two cursors, and an unlucky field order would give that
capacity a line of its own — 192 bytes for 24 bytes of state. The test permits
3 lines and forbids 4, which is a real constraint on a future field being added
without thought, and it is stated as a bound rather than an exact number so
that adding a small field does not require editing the test to a new magic
value.

Note this assertion lives in `ring_cursor`, not here. This crate has no
dependency on its consumer and never sees the pair
(→ [`integration/002`](../integration/002_why_the_constant_lives_here.md)); the
arrangement is documented here because it is what the crate's guarantee is
*for*, and tested there because that is where the type is.

### The Arrangement Declined

The family contains one documented decision **not** to build this arrangement,
and it is the honest other side of the argument.
`ring_mpsc`'s `stamps` array holds one `AtomicSeq` per slot, unpadded
(`ring_mpsc/src/lib.rs:330-335`):

> One stamp per slot, holding the sequence whose payload currently occupies it.
> Unpadded on purpose: [`PaddedCursor`] would make this array 64 times the size
> of the payload array for a small `S`, to prevent a false-sharing contention
> that does not arise — two producers writing adjacent stamps are two producers
> that claimed adjacent sequences, which is a handful of stores on one line
> rather than a contended loop.

**Two writers on one line is not automatically false sharing.** The false-sharing
pathology is a *loop*: two cores repeatedly writing their own variable and repeatedly
losing the line to each other. Two producers each storing once to adjacent
stamps pay a couple of transfers and move on. The padding would cost 64× the
array for a small slot type and buy nothing.

So the arrangement in this instance is the right answer for **two contended
variables written in a loop by different cores**, and the wrong answer for **n
variables written once each**. The cost model is the same in both cases — a
line per value — and only the access pattern decides whether it is worth
paying. That distinction is not visible in any type signature, which is why it
lives in a comment at the declaration.

### What This Arrangement Still Does Not Fix

| # | Residual | Why the pair does not help |
|---|----------|----------------------------|
| R1 | The constant is wrong for the host | Both fields are 64 apart on a 128-byte-line machine; the diagram above is drawn to the wrong scale (→ [`pitfall/001`](../pitfall/001_a_constant_too_small_buys_nothing.md)) |
| R2 | A third hot value is placed adjacent to the pair by an outer struct | The pair's own lines are clean; the neighbour shares whichever line the pair's last padding occupies — except it cannot, because the pair ends on a boundary too. **R2 is actually foreclosed**, and it is worth writing out to see why: whole-line sizing composes upward |
| R3 | The two cursors are separated into different allocations | Each is alone in its line and the contention is gone — this is fine, and the pair is a convenience rather than a requirement in that case |

**R2 is the one that resolves rather than persists,** and it is the payoff of
insisting on whole-line *sizing* rather than alignment alone: because
`CursorPair` is itself a multiple of 64 bytes, anything the compiler places
after it starts on a fresh line. The property is transitive up the struct
hierarchy, which alignment alone would not give.

### AL12 — The Family's One Padded Pair Carries a Third, Unpadded Field

Exactly one struct in the 33 crates holds two padded fields, and it holds three
fields in total:

```rust
// ring_cursor/src/lib.rs:247
pub struct CursorPair
{
  producer : PaddedCursor,
  consumer : PaddedCursor,
  capacity : Capacity,
}
```

`capacity` is an unpadded `usize` newtype sharing the struct with two things the
crate went to some trouble to separate.

**Finding.** It needs no line of its own, and the reason is supplied by this
crate rather than by `ring_cursor`: the two 64-aligned fields ahead of it push
it past both their lines in any field order the compiler picks. Its separation
is a consequence of the wrapper's alignment rather than a decision anybody made
— which is worth writing down, because a fourth field added to that struct would
inherit the same accident without inheriting any guarantee that it holds.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_rounding_a_payload_up_to_whole_lines.md](../algorithm/002_rounding_a_payload_up_to_whole_lines.md) | Why whole-line sizing is what makes R2 resolve |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_wrapper_surface.md](../api/002_the_wrapper_surface.md) | W1 and W2 — the caller failures this arrangement forecloses |

### Data Structures

| File | Relationship |
|------|--------------|
| [001_the_cache_aligned_wrapper.md](001_the_cache_aligned_wrapper.md) | The single wrapper whose guarantee is insufficient alone |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_why_the_constant_lives_here.md](../integration/002_why_the_constant_lives_here.md) | Why the assertions on this arrangement live in the consumer rather than here |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_two_wrapped_fields_never_share_a_line.md](../invariant/001_two_wrapped_fields_never_share_a_line.md) | P6 — the caller obligation this arrangement discharges by construction |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_size_of_proves_nothing_about_addresses.md](../pitfall/002_size_of_proves_nothing_about_addresses.md) | The unpadded diagram above is the negative control, and why it is load-bearing |

### Sources

| File | Relationship |
|------|--------------|
| [`../invariant/001_two_wrapped_fields_never_share_a_line.md`](../invariant/001_two_wrapped_fields_never_share_a_line.md) | The unpadded diagram is the failure this invariant prevents, drawn; the padded diagram is the invariant itself, drawn |
| `ring_cursor/src/lib.rs:247-252` | `CursorPair`'s three fields — the arrangement realised |
| `ring_mpsc/src/lib.rs:330-335` | The arrangement declined, with the access-pattern reasoning |

### Tests

| File | Relationship |
|------|--------------|
| `tests/align_test.rs` | `two_wrapped_fields_land_on_different_lines` builds exactly the `TwoCursors` struct above; `two_unwrapped_fields_share_a_line` builds the contrast diagram |
| `ring_cursor/tests/cursor_test.rs` | The 2-to-3-line bound and the alignment of the real `CursorPair` |
