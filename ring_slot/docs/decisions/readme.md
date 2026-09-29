# decisions

Two rulings made this crate what it is: carry two slot shapes rather than one,
and represent a byte payload's extent as a length rather than an initialisation
flag. Both are recorded in the source. Neither is measured.

The first ruling's mechanism turned out stronger than the ruling required — a
trait, rather than a convention, which is why four crates stayed generic across
three layers with no coordination. Its justification is a cost asymmetry between
copying a typed payload and copying bytes, argued in prose and benchmarked
nowhere. The second ruling buys the crate its `unsafe`-freedom at a fixed
`N`-byte zeroing cost per slot, paid once because a ring allocates its slots once
— and it leaves one distinction unrepresentable that a whole paragraph in
`ring_event`, two crates away, explains better than anything here.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Two Shapes Rather Than One](001_two_shapes_rather_than_one.md) | SL5, SL6 — a trait where a convention would have sufficed, and a founding argument with no measurement |
| 002 | [A Length Rather Than a Flag](002_a_length_rather_than_a_flag.md) | SL7, SL8 — `unsafe`-freedom bought with a bounded zeroing cost, and the distinction a length cannot make |

### Ruling One: Two Shapes

| | Decided | Alternative |
|---|---|---|
| Shapes | `TypedSlot< T >` and `BytesSlot< N >` | One shape, generic over payload |
| Enforced by | A trait both implement | Convention, or a marker |
| Consequence | Shapes are indistinguishable above the slot boundary | Every consumer branches per shape |
| Justification | Different copy costs under the same protocol | — |
| Measured | **No** | — |

The mechanism exceeds the requirement, and that is the finding: the ruling asked
only that both shapes use the same claim, gating, and drain. The trait makes them
*indistinguishable*, which is strictly stronger and is what let `ring_store`,
`ring_spsc`, `ring_mpsc`, and `ring_event::recycle` all stay generic without
coordinating.

The justification is the weak part. The copy-cost asymmetry is the crate's
founding argument, stated in `ring_slot`'s own module comment, which calls for
it to be measured. `ring_bench` benches `TypedSlot< Record >` twice and never
instantiates `BytesSlot`
([`integration/002`](../integration/002_where_the_second_shape_stops.md) SL4).

### Ruling Two: A Length

| | Decided | Alternative |
|---|---|---|
| Extent | `len : usize` | `MaybeUninit` + an initialised flag |
| `unsafe` required | **None** | Yes, at every read |
| Construction cost | `N` zero-bytes, once per slot | None |
| Distinguishes empty from zero-length | **No** | Yes |

The zeroing cost is bounded by the ring's own allocate-once behaviour, so it is
paid once per slot for the ring's life rather than per publish. In exchange the
workspace's `unsafe-code = "deny"` lint applies to this crate without exception.

What it gives up is one distinction: a slot holding a zero-length payload is
indistinguishable from an empty one. `ring_event::Peek` names the cause, prices
the alternative at one flag byte per slot, points at where the distinction
actually lives — the published-sequence handshake — and names the cheaper
substitute, `TypedSlot< () >`. That paragraph is the best thing written about
this crate and it is two crates away from the type it describes.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# the cost-asymmetry claim, as the crate states it
command grep -m1 -A2 -F '//! The reason for both is a cost asymmetry — force everything through bytes and' ring_slot/src/lib.rs

# feature 182's Workstreams row assigning the measurement — historical source,
# external to this repository and unreachable since extraction; the quote it
# once produced is preserved as a historical note in integration/002.md (SL4)

# what the bench crate actually instantiates
grep -n 'TypedSlot\|BytesSlot' ring_bench/src/lib.rs

# the allocate-once premise the zeroing cost rests on
command grep -m1 -A3 -F '/// The shape for traffic arriving from outside the process, decoded later by' ring_slot/src/lib.rs

# no unsafe in the code — the one hit is the module comment claiming exactly that
grep -n 'unsafe' ring_slot/src/lib.rs
grep -vE '^[[:space:]]*//' ring_slot/src/lib.rs | grep -c 'unsafe'

# and the lint that would catch it
grep -n 'unsafe-code' Cargo.toml

# the paragraph about this crate that lives two crates away
grep -n -B2 -A12 'pub trait Peek' ring_event/src/lib.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SL5 | `ring_slot` | n/a — observation | The ruling is enforced by a trait rather than a convention, which is strictly stronger than the ruling required and is why four crates stayed generic across three layers |
| SL6 | `ring_slot` | n/a — unenforced | The founding copy-cost asymmetry is argued in prose, calls for its own measurement, and is measured nowhere |
| SL7 | `ring_slot` | n/a — observation | The length representation buys workspace-wide `unsafe`-freedom at an `N`-byte zeroing cost bounded by the ring's allocate-once behaviour |
| SL8 | `ring_event` | n/a — doc gap | A length cannot distinguish empty from zero-length; the complete explanation of that limitation lives in `ring_event::Peek`, two crates from the type it describes |
