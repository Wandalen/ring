# integration

Seven crates depend on `ring_slot` and four of them never name a concrete shape —
they take `S : Slot` and pass it through. That genericity survives three layers:
storage in `ring_store`, both ring implementations above it, and `ring_event`'s
recycle. The fourth layer drops it.

Both instances measure a reach rather than assert one. The first follows the
inbound edges up until `ring_core` hard-wires `TypedSlot< T >` at thirteen sites,
which forecloses the family's unified handle for exactly the traffic the second
shape exists to carry. The second follows `BytesSlot` outward and finds one
library consumer, four test suites, and a bench that `ring_slot`'s own argument
calls for and `ring_bench` does not contain.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Seven Dependents and Four That Stay Generic](001_seven_dependents_and_four_that_stay_generic.md) | SL1, SL2 — genericity holding across three layers, and the layer that drops it |
| 002 | [Where the Second Shape Stops](002_where_the_second_shape_stops.md) | SL3, SL4 — one library consumer for `BytesSlot`, and a promised bench that does not exist |

### The Genericity Ladder

| Layer | Crate | Generic over `Slot`? |
|-------|-------|:--------------------:|
| storage | `ring_store` | ✔ `Buffer< S >` |
| ring | `ring_spsc` | ✔ `Ring< S >` |
| ring | `ring_mpsc` | ✔ `Ring< S >` |
| protocol | `ring_event` | ✔ `publish_into`/`drain_from`/`recycle` |
| handle | `ring_core` | ✘ **`TypedSlot< T >` at thirteen sites** |

A caller may pick either shape and everything from storage through drain follows.
Reaching the family's unified handle requires the typed shape, so a `BytesSlot`
ring must drop to a concrete ring crate — and opaque host traffic is precisely
what `BytesSlot` was introduced for.

### `BytesSlot`'s Whole Reach

| Crate | Mentions | Shipped code |
|-------|---------:|-------------:|
| `ring_event` | 12 | **4** |
| `ring_spsc` | 8 | 0 |
| `ring_store` | 3 | 0 |
| `ring_mpsc` | 2 | 0 |

Forty-six hits across the family, four of them library code — two trait impls in
`ring_event`, one per direction. The rest are doctests and tests, which is the
abstraction working: three crates demonstrate the shape and need no code for it.

### The Bench That Is Not There

`ring_slot`'s own module comment calls for benching both shapes, since a typed
slot and a byte slot have different copy costs under the same protocol. That
copy-cost claim is the crate's founding argument
([`decisions/001`](../decisions/001_two_shapes_rather_than_one.md) SL6).
`ring_bench` benches `TypedSlot< Record >` on both ring implementations and
mentions `BytesSlot` zero times.

Both bench sites are already `Ring< TypedSlot< Record > >` over a generic ring,
so the counterpart is a type substitution plus a `Fill` that `ring_event` already
supplies.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# every crate depending on ring_slot — its own manifest names itself, so drop it
grep -rln 'ring_slot' ring_*/Cargo.toml | sed 's|/Cargo.toml||' \
  | grep -v '^ring_slot$' | sort

# the four that stay generic
grep -rnE '^pub struct (Buffer|Ring)<|S : Slot' ring_store/src/lib.rs \
  ring_mpsc/src/lib.rs ring_spsc/src/lib.rs ring_event/src/lib.rs | sort

# and the layer that does not
grep -c 'TypedSlot' ring_core/src/lib.rs

# BytesSlot's reach, shipped code against total
for f in $( grep -rl 'BytesSlot' ring_*/src/*.rs ring_*/tests/*.rs 2>/dev/null \
  | grep -v '^ring_slot/' | sort ); do
  printf '%-38s total %2d   shipped code %d\n' "$f" \
    "$( grep -c 'BytesSlot' "$f" )" \
    "$( grep -vE '^[[:space:]]*//' "$f" | grep -c 'BytesSlot' )"
done

# the promised bench, and what the bench crate holds — the feature's own
# "008_ring_write_path" assignment lived in a pre-implementation design corpus
# external to this repository, unreachable since extraction (quote preserved
# as a historical note in integration/002.md, SL4); what remains checkable:
grep -n 'TypedSlot\|BytesSlot' ring_bench/src/lib.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SL1 | family | n/a — observation | Genericity over `Slot` survives three layers — storage, both rings, and `ring_event`'s recycle — with no coordination between them |
| SL2 | `ring_core` | n/a — doc gap | `ring_core` hard-wires `TypedSlot< T >` at thirteen sites, so the traffic `BytesSlot` exists for has no route through the family's unified handle, and nothing states the limit |
| SL3 | family | n/a — observation | `BytesSlot` has exactly one library consumer — `ring_event`'s two trait impls — against 46 mentions family-wide, 42 of them doctests and tests |
| SL4 | `ring_bench` | **wrong doc** | `ring_slot`'s own argument calls for the bench that justifies two shapes; `ring_bench` benches `TypedSlot` twice and never instantiates `BytesSlot` |
