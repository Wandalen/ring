# Data Structure: The Absent Stamp Array

### Scope

- **Purpose**: Record what this crate's storage does *not* contain, and why the absence is load-bearing rather than incidental.
- **Responsibility**: The per-slot publication marker the multi-producer sibling needs and this crate does not.
- **In Scope**: The comparison between `ring_spsc`'s two-cursor storage and `ring_mpsc`'s stamped storage.
- **Out of Scope**: The two cursors themselves (→ [`001`](001_two_cursor_ring.md)); the orderings on them (→ [`../invariant/002`](../invariant/002_no_lock_in_the_path.md)).

### The Measurement

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'ring_mpsc stamp mentions:  '; grep -c 'stamps' ring_mpsc/src/lib.rs
printf 'ring_spsc stamp mentions:  '; grep -c 'stamps' ring_spsc/src/lib.rs
printf 'ring_spsc RMW operations:  '
grep -vE '^\s*(//|///|//!)' ring_spsc/src/lib.rs | grep -cE 'compare_exchange|fetch_'
printf 'ring_mpsc RMW operations:  '
grep -vE '^\s*(//|///|//!)' ring_mpsc/src/lib.rs | grep -cE 'compare_exchange|fetch_'
# The command above can legitimately exit nonzero -- a count of zero, a diff
# that differs -- and the sweep that keeps this output current skips any block
# that exits nonzero, so without this the block freezes instead of refreshing.
:
```

Live output:

```
ring_mpsc stamp mentions:  12
ring_spsc stamp mentions:  0
ring_spsc RMW operations:  0
ring_mpsc RMW operations:  0
```

Two crates, the same `UnsafeCell`-wrapped slot buffer, the same wrapping reuse —
one stamp array between them.

### Why the Absence Holds

A stamp exists to answer "is the record at this slot published?" for a reader
that cannot infer it from cursors alone. Three conditions have to hold for that
question to be open:

1. Slots are reused across laps, so "occupied" is ambiguous. **True here.**
2. A reader can address a slot before its writer has finished. **True here.**
3. Publication completes out of claim order, so the producer cursor is not a
   frontier. **False here** — one producer, so the cursor advances only after
   the record is written, and it *is* the frontier.

The third condition is the whole difference. It is a property of producer
cardinality, not of storage, which is why two crates with identical buffers
differ by an entire array.

### What It Buys

One `u64` per slot, not allocated. No atomic store per publish beyond the cursor
advance itself. And no stale-stamp trap — the failure mode `ring_mpsc`'s
`pitfall/002` documents has no analogue here, because there is no stamp to go
stale.

### Sources

| File | Relationship |
|------|-----------------|
| `src/lib.rs` | The storage that has no stamp array |
| `../../../ring_mpsc/src/lib.rs` | The sibling storage that does |
| `../../../ring_mpsc/docs/data_structure/002_a_second_array_of_sequence_stamps.md` | The instance this one is the negative of |

### SP11 — The Third Condition Is About Cardinality, Not Storage

That is what makes the absence structural rather than a size optimization. Slots
are reused across laps here exactly as in the sibling, and a reader can address a
slot before its writer finishes exactly as in the sibling. What differs is that
the producer cursor advances *after* the write rather than at claim time, so it
is a frontier.

**A second producer would reintroduce the whole array**, not a variant of it —
which is why `ring_mpsc` is a separate crate rather than a feature flag on this
one.

### SP12 — The Absence Also Deletes a Documented Trap

The sibling's `pitfall/002` describes a publication test that is correct on lap
one and wrong on lap two — `stamp != UNSTAMPED` where `stamp == seq` was meant.
There is no stamp here, so the trap cannot be written.

Worth stating because the two crates' `pitfall/002` files are otherwise
positionally parallel and describe unrelated hazards: a wrong publication test
there, an unobservable counterpart departure here
(→ [`../pitfall/002`](../pitfall/002_a_departed_counterpart_is_indistinguishable_from_a_slow_one.md)).
