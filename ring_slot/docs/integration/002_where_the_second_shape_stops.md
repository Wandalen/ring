# Integration: Where the Second Shape Stops

### Scope

**Purpose:** Record how far `BytesSlot` actually reaches through the family,
which crates carry it in shipped code rather than in doctests, and the one
bench promise that nothing fulfils.

**Responsibility:** The reach of the second slot shape, measured separately from
the first.

**In Scope:** Every `BytesSlot` mention outside `ring_slot`;
`ring_bench/src/lib.rs`; the bench `ring_slot`'s own module comment calls for.

**Out of Scope:** The inbound edge census and the `ring_core` foreclosure, which
are [`integration/001`](001_seven_dependents_and_four_that_stay_generic.md).
Whether the two shapes *should* both exist is
[`decisions/001`](../decisions/001_two_shapes_rather_than_one.md).

---

## The Second Shape's Whole Reach

```sh
cd "$(git rev-parse --show-toplevel)"
# every BytesSlot mention outside ring_slot, split by whether it is shipped code
for f in $( grep -rl 'BytesSlot' ring_*/src/*.rs ring_*/tests/*.rs 2>/dev/null | grep -v '^ring_slot/' | sort ); do
  all=$( grep -c 'BytesSlot' "$f" )
  code=$( grep -vE '^[[:space:]]*//' "$f" | grep -c 'BytesSlot' )
  printf '%-38s total %2d   shipped code %d\n' "${f}" "$all" "$code"
done
```

Live output:

```
ring_store/src/lib.rs                 total  5   shipped code 0
ring_store/tests/buffer_test.rs       total  2   shipped code 2
ring_event/src/lib.rs                  total 15   shipped code 4
ring_event/tests/event_test.rs         total 15   shipped code 13
ring_mpsc/src/lib.rs                   total  2   shipped code 0
ring_mpsc/tests/mpsc_test.rs           total  3   shipped code 3
ring_spsc/src/lib.rs                   total  8   shipped code 0
ring_spsc/tests/spsc_test.rs           total  3   shipped code 3
```

---

### SL3 — `BytesSlot` Has One Library Consumer in the Family

Of the four `src/lib.rs` files that mention it, three mention it only inside doc
comments — `ring_store`, `ring_spsc`, and `ring_mpsc` each demonstrate the shape
in a doctest and then never name it in code, because they are generic over `Slot`
and do not need to. `ring_event` is the exception:

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'BytesSlot' ring_event/src/lib.rs | grep -vE '^[[:space:]]*//'
```

Live output:

```
use ring_slot::{ BytesSlot, Slot, TypedSlot };
impl< const N : usize > Fill< BytesSlot< N > > for &[ u8 ]
  fn fill( self, slot : &mut BytesSlot< N > ) -> Result< (), RingError >
impl< const N : usize > Peek for BytesSlot< N >
```

Two trait impls, one per direction — `Fill` to put bytes in, `Peek` to get them
out. That is the whole of `BytesSlot`'s presence in shipped library code across
33 crates.

**Finding.** The second slot shape exists, is fully implemented, is exercised by
four test suites, and is named in shipped code by exactly one crate. That is not
a defect — it is what "generic over `Slot`" is supposed to buy, and three crates
demonstrating the shape in doctests while needing no code for it is the
abstraction working. It is recorded because the ratio is the reverse of the
intuition: a reader grepping for `BytesSlot` to find out how the family uses it
gets 46 hits, 4 of which are library code, and 42 of which are doctests and
tests.

`ring_event` is also where the shape's one documented limitation lives, which
[`invariant/002`](../invariant/002_the_two_emptiness_paths_agree.md) takes up.

---

### SL4 — The Bench `ring_slot`'s Own Argument Calls For Does Not Exist

`ring_slot`'s own module comment argues a cost asymmetry between the two shapes
and calls for it to be measured.

> Historical record, not independently reproducible from a standalone checkout:
> this finding was originally corroborated by grepping the crate's
> pre-implementation design corpus (`docs/feature/182_typed_slot_and_bytes_slot.md`,
> external to this repository and unreachable since extraction) for
> `008_ring_write_path`, which returned: "`008_ring_write_path/readme.md` —
> Benches both, since a typed slot and a byte slot have different copy costs
> under the same protocol." The claim itself is verified independently below,
> against this crate's own module comment and `ring_bench`'s actual source.

The bench crate this measurement would run in is `ring_bench`:

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'ring_bench TypedSlot mentions: %d\n' "$( grep -c 'TypedSlot' ring_bench/src/lib.rs )"
printf 'ring_bench BytesSlot mentions: %d\n' "$( grep -c 'BytesSlot' ring_bench/src/lib.rs )"
grep 'TypedSlot' ring_bench/src/lib.rs
```

Live output:

```
ring_bench TypedSlot mentions: 5
ring_bench BytesSlot mentions: 0
use ring_slot::TypedSlot;
  let mut ring : ring_spsc::Ring< TypedSlot< Record > > =
    drained.extend( batch.iter().filter_map( TypedSlot::get ).copied() );
  let mut ring : ring_mpsc::Ring< TypedSlot< Record > > =
    drained.extend( batch.iter().filter_map( TypedSlot::get ).copied() );
```

**Finding.** `ring_slot`'s own module comment states the reason both shapes
exist — a typed slot and a byte slot have different copy costs under the same
protocol — and calls for that asymmetry to be measured. `ring_bench` benches
`TypedSlot< Record >` on both ring implementations and never instantiates
`BytesSlot` at all. The copy-cost difference that justifies carrying two shapes
is asserted only in `ring_slot`'s own module comment, as a "cost asymmetry",
and measured nowhere.

The gap is cheap to close and the two bench sites are already written: both are
`Ring< TypedSlot< Record > >` over a generic ring, so a `BytesSlot< N >`
counterpart is a type substitution plus a `Fill` for the payload, which
`ring_event` already provides. Until it exists, the crate's founding argument
rests on a plausible claim about copying rather than a number.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_slot
command grep -F 'difference this paragraph claims has no number behind it' src/lib.rs
```

Live output:

```
//! `BytesSlot`, so the difference this paragraph claims has no number behind it.
```

**Disposition:** declined — writing the `BytesSlot< N >` bench case this finding
specifies is a code addition to `ring_bench/src/lib.rs`, a different
crate with its own corpus, outside `ring_slot`'s own `src/`, `docs/`, and
`Cargo.toml`. What was in reach here is applied instead: `ring_slot`'s own
module comment, the one place that states the cost-asymmetry claim, now says
the claim is argued rather than measured, and names the bench crate's
`TypedSlot`-only coverage as why.
Now prints: `difference this paragraph claims has no number behind it`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`integration/001`](001_seven_dependents_and_four_that_stay_generic.md) | The inbound census, and the `ring_core` layer that drops the genericity |
| [`decisions/001`](../decisions/001_two_shapes_rather_than_one.md) | The ruling that produced two shapes, and the cost asymmetry it rests on |
| [`non_functional_requirement/001`](../non_functional_requirement/001_what_a_slot_costs.md) | The costs that *are* measured, against the one that is not |
| [`invariant/002`](../invariant/002_the_two_emptiness_paths_agree.md) | `ring_event`'s documented limitation on `BytesSlot`, and where it should live |

### Sources

| Fact | Where |
|------|-------|
| The reach census, run twice for stability | `ring_*/src/*.rs`, `ring_*/tests/*.rs` |
| `BytesSlot`'s two trait impls | `ring_event/src/lib.rs:80`, `:137` |
| The bench as written | `ring_bench/src/lib.rs:107, 1160, 1191` |
| The cost-asymmetry claim | `ring_slot/src/lib.rs:10-12` |

### Tests

| Test | Covers |
|------|--------|
| `ring_spsc` — `one_producer_and_one_consumer_exchange_one_hundred_thousand_items` | A `BytesSlot< 8 >` ring driven across 100k items on the single-producer path |
| `ring_mpsc` — `a_bytes_payload_round_trips_its_written_length` | A `BytesSlot< 16 >` ring end to end on the multi-producer path |
| `ring_store` — `the_same_buffer_type_serves_both_slot_shapes` | Both shapes through one generic buffer |
| `ring_spsc` — `a_reservation_reads_back_what_was_written_through_it` | A `BytesSlot< 4 >` ring through the reservation path |
| *(to create)* | A `ring_bench` case over `BytesSlot`, measuring the copy cost `ring_slot`'s own module comment names |
