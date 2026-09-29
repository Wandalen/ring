# Data Structure: A `Vec` and a Limit

### Scope

- **Purpose**: Record the data structure this crate declares, since [`001`](001_thread_local_append_log.md) documents a bump-allocated byte region and the source declares neither bytes nor a bump pointer.
- **Responsibility**: `TlsBuffer`'s two fields, what makes the accumulation allocation-free, and what the type is not.
- **In Scope**: `items : Vec< T >` and `limit : usize`, and the relationship between `limit` and `Vec`'s own capacity.
- **Out of Scope**: The flush that empties it (→ [`../algorithm/002`](../algorithm/002_the_fused_claim_and_drain.md)); the region layout `001` leaves open, which was answered by building something else.

### Two Fields

```sh
cd "$(git rev-parse --show-toplevel)"/ring_tls
grep -B 3 -A 2 'items : Vec< T >,' src/lib.rs
printf 'bump pointers, tags, regions, epochs in the source: '
grep -vE '^\s*(//|///|//!)' src/lib.rs \
| grep -ciE 'bump|tag|region|epoch|memcpy|to_le_bytes'
# The command above can legitimately exit nonzero -- a count of zero, a diff
# that differs -- and the sweep that keeps this output current skips any block
# that exits nonzero, so without this the block freezes instead of refreshing.
:
```

Live output:

```
#[ derive( Debug ) ]
pub struct TlsBuffer< T >
{
  items : Vec< T >,
  limit : usize,
}
bump pointers, tags, regions, epochs in the source: 0
```

`items` is a `Vec< T >` of the caller's own type — typed, not a byte stream.
`limit` is the refusal bound, held separately from `Vec::capacity` on purpose:
`Vec` may reserve more than asked, and a buffer whose refusal point moved with
the allocator's rounding would make the flush count depend on the allocator.

**The accumulation is allocation-free because of one line.**
`Vec::with_capacity( limit )` reserves the whole bound at construction, and
`push` is refused at `limit`, so `Vec`'s growth path is unreachable. That is
the entire mechanism — no arena, no bump pointer, no reset.

### What It Is Not

| The corpus specifies | The source declares |
|----------------------|---------------------|
| A contiguous byte region with a monotonic bump pointer | `Vec< T >`, typed |
| A one-byte tag before each record | Nothing — `T` is whatever the caller staged |
| `reset`, rewinding the pointer | `discard`, calling `Vec::clear` |
| Per-record little-endian operand bytes | Values, moved |

The right reading is not that the source is wrong. `Vec< T >` is the better
structure for the consumers that exist: all three stage typed values and none
needs a walkable byte stream. The earlier design was written for a consumer
whose records are opcodes, which does need a walkable byte stream.

### Sources

| File | Relationship |
|------|-----------------|
| `src/lib.rs` | Declares both fields |
| `001_thread_local_append_log.md` | Specifies the structure that was not built |
| `../../readme.md` | Already states the built shape, and contradicts `001` in doing so |

### TL13 — The Refusal Bound Is Held Separately From the Allocation

`Vec::with_capacity( n )` may reserve more than `n`. A `push` refused at
`self.items.capacity()` would therefore refuse at a point the allocator chose,
and the flush count would vary by platform.

Holding `limit` separately costs one `usize` per buffer and makes
`a_zero_capacity_buffer_accepts_nothing_and_is_full_from_the_start` a statement
about the API rather than about `Vec`.

### TL14 — Growth Is Unreachable by Arithmetic, Not by Type

The zero-allocation guarantee rests on one comparison in one function. `Vec`
is fully capable of growing and would do so without complaint.

`push` is currently the only insertion point, so the guarantee holds. A
`push_within_capacity`, an `extend`, or a `Vec`-returning accessor would each
break it without touching any line that mentions the invariant, and the crate's
own tests assert the *effect* (no growth) rather than the mechanism.

```sh
cd "$(git rev-parse --show-toplevel)"
grep -c 'sole insertion point' ring_tls/src/lib.rs
```

Live output:

```
1
```

**Disposition:** applied — `push`'s doc comment in `ring_tls/src/lib.rs` now
states it is the sole insertion point into `items` and names the guarantee
that rests on that fact, so a future contributor adding a second insertion
path meets the warning before writing one. `push` itself also now asserts
`self.items.capacity()` is unchanged after every insertion
(`debug_assert!` two lines below the `push`), which catches the mechanism
directly rather than only the effect, for any regression internal to `push`
itself. A bypass added elsewhere in the crate is a documentation trap, not a
type-level one — no lint construct closes that residual gap.
Verified via `cargo test -p ring_tls --all-features`, 2026-09-04 — ring_tls's
22 unit tests plus 7 doctests all pass.
Now prints: `1`

### TL15 — The Structure Is Typed, Which Is Why the Byte-Region Consumer Cannot Use It

All three consumers stage typed values — `u32` in `ring_flush` and
`ring_testkit`, a POD `Record` in `ring_bench`. None wants a tag byte, and none
would gain from one.

The design in [`001`](001_thread_local_append_log.md) was written for a consumer
that does: a consumer staging opcodes, which have to be walked without
knowing their types. Reading 3 of
[`../decisions/001`](../decisions/001_the_corpus_specifies_an_api_the_crate_did_not_build.md)
follows from this — the design record is not stale, it is filed against the
wrong crate.

### TL16 — The Crate README Describes Both Designs, Four Paragraphs Apart

```sh
cd "$(git rev-parse --show-toplevel)"/ring_tls
printf 'the opening sentence:\n'; command grep -m1 -A2 -F 'Per-thread, bump-allocated, zero-lock append log: every thread appends to its' readme.md
printf 'and the buffer-layout paragraph:\n'; command grep -m1 -A2 -F 'not to this crate. The buffer layout is not:' readme.md
printf 'the dependency sentence:\n'; command grep -m1 -A1 -F 'Depends on [`ring_types`](../ring_types/readme.md) and' readme.md
printf 'the actual [dependencies]:\n'
sed -n '/^\[dependencies\]/,/^$/p' Cargo.toml | grep '^ring_'
```

Live output:

```
the opening sentence:
Per-thread, bump-allocated, zero-lock append log: every thread appends to its
own buffer with no atomics and no mutexes; a consolidation step later reads
each thread's buffer.
and the buffer-layout paragraph:
belongs to the consumer, not to this crate. The buffer layout is not:
`TlsBuffer<T>` is a `Vec<T>` reserved once to its own refusal bound, which is
what makes the accumulation free.
the dependency sentence:
Depends on [`ring_types`](../ring_types/readme.md) and
[`ring_slot`](../ring_slot/readme.md). One of the 33 `ring_*` crates that make
the actual [dependencies]:
ring_types = { path = "../ring_types" }
ring_atomic = { path = "../ring_atomic" }
ring_batch = { path = "../ring_batch" }
```

Three defects in one file. The opening calls the crate bump-allocated and
atomic-free — the first is the unbuilt design and the second is false for
`flush_into`, which costs exactly one. The layout paragraph then states the
built shape correctly, contradicting the opening. And the dependency sentence
names `ring_types` (a real dependency) alongside `ring_slot` (dev-only),
omitting `ring_atomic` and `ring_batch` entirely.

**The readme was updated in place, twice, without re-reading the top of it.**
That is the same failure as the nineteen instances at a smaller scale, and it
is visible in a thirty-seven-line file — which is what makes it worth recording
separately from
[`../decisions/001`](../decisions/001_the_corpus_specifies_an_api_the_crate_did_not_build.md).
