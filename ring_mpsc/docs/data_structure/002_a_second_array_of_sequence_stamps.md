# Data Structure: A Second Array of Sequence Stamps

### Scope

- **Purpose**: Record that publication state is a second array parallel to the slots, state why it is a stamp rather than a flag, and record the padding decision.
- **Responsibility**: The stamp array's element type, its parallel relationship to the slots, and the deliberate absence of padding.
- **In Scope**: `stamps : Box< [ AtomicSeq ] >` and its `UNSTAMPED` initial value.
- **Out of Scope**: The single-array slot storage (→ [`001`](001_sequence_stamped_ring.md)); the orderings the stamps are accessed with (→ [`../item/002`](../item/002_five_public_ordering_constants.md)).

### Two Arrays, One Index

| Array | Element | Holds |
|-------|---------|-------|
| `slots` | `UnsafeCell< S >` | The payload |
| `stamps` | `AtomicSeq` | The sequence whose payload currently occupies that slot |

Both are indexed by `seq & capacity.mask()`. A slot is published exactly when
its stamp equals the sequence addressing it — not "is set", but "equals this
one".

**That equality is what makes the marker lap-safe.** A boolean flag would say
"occupied" and could not distinguish this lap's record from the previous lap's;
the stamp says *which* record, so a reader on lap 2 looking at a slot still
carrying lap 1's stamp sees a mismatch rather than a stale success
(→ [`../pitfall/002`](../pitfall/002_a_stale_stamp_reads_as_unpublished_not_as_wrong.md)).

`UNSTAMPED` is `Seq( u64::MAX )`, chosen so lap zero has a value no valid
sequence can equal.

### The Padding Decision

```sh
cd "$(git rev-parse --show-toplevel)"/ring_mpsc
printf 'PaddedCursor uses:  '; grep -c 'PaddedCursor' src/lib.rs
printf 'stamps field type:  '; grep -n 'stamps : ' src/lib.rs
```

Live output:

```
PaddedCursor uses:  4
stamps field type:  336:  stamps : Box< [ AtomicSeq ] >,
```

The stamp array is **unpadded on purpose**, and the source states the reasoning:
`PaddedCursor` would make this array 64 times the size of the payload array for
a small `S`, to prevent a false-sharing contention that does not arise. Two
producers writing adjacent stamps are two producers that claimed adjacent
sequences — a handful of stores on one line, not a contended loop.

**The two cursors are padded and the stamps are not**, and that asymmetry is the
finding: padding is applied where contention is structural and withheld where it
is transient.

### Cost

One `u64` per slot, atomic, allocated once at construction. For a ring of small
records that is a real fraction of the footprint, and it is the price of a
publication marker that survives wrapping.

### MP11 — Cursors Are Padded and Stamps Are Not

```sh
cd "$(git rev-parse --show-toplevel)"/ring_mpsc
printf 'PaddedCursor mentions:   '; grep -c 'PaddedCursor' src/lib.rs
printf 'stamps field:            '; grep -n 'stamps : ' src/lib.rs
printf 'the padding test:        '
grep -hoE '^\s*fn [a-z_0-9]+' tests/*.rs | sed 's/^ *fn //' | grep 'distinct_cache_lines'
```

Live output:

```
PaddedCursor mentions:   4
stamps field:            336:  stamps : Box< [ AtomicSeq ] >,
the padding test:        the_claim_cursor_and_the_consumer_cursor_are_on_distinct_cache_lines
```

Padding the stamps would cost 64 bytes per slot — for a ring of `u32` records,
sixteen times the payload array. The source argues the contention it would
prevent does not arise: two producers writing adjacent stamps claimed adjacent
sequences, which is a few stores on one line rather than a loop.

**The cursors are the opposite case**: the claim cursor and the consumer cursor
are read and written by every participant on every operation, so their
false-sharing is structural. `the_claim_cursor_and_the_consumer_cursor_are_on_distinct_cache_lines`
asserts the padding actually took effect.

The recipe above is anchored `^\s*fn` and not `^fn`, which is not a detail. This
crate is the only one in the family whose integration tests live inside a `mod`
— the ordinary body is gated `#[ cfg( not( loom ) ) ]`, so every test in it is
indented one level. A column-anchored pattern matches nothing here while
matching everything in `ring_core` and `ring_registry`, and `grep -c` reports
that as a clean `0` rather than as an error. **The first draft of this recipe
carried exactly that zero**, which read as "the padding is unasserted" for a
test that has existed all along.

**Disposition:** declined — this instance's own text concludes the asymmetric
padding is correct ("the source argues the contention it would prevent does
not arise") and separately records a recipe-anchoring mistake already caught
and corrected in the Regenerate block above (`^\s*fn`, not `^fn`); neither
half implies a source or doc fix beyond what
`data_structure/002_a_second_array_of_sequence_stamps.md` already records.

### MP12 — `UNSTAMPED` Is `Seq( u64::MAX )` and Nothing Forbids Reaching It

`Seq( u64::MAX )` is a value `Seq` can hold, so the sentinel is distinguishable
from a real sequence only because no run reaches it. `ring_types`'
`seq_does_not_wrap_within_any_reachable_workload` is the argument, and it is an
argument about workloads rather than about types.

The bound is enormous — at one publish per nanosecond, `u64::MAX` is roughly 585
years — so this is a correct engineering decision and not a latent bug. Recorded
because the sentinel's safety is a numerical claim that lives in a different
crate's test, not a property of the declaration here.
