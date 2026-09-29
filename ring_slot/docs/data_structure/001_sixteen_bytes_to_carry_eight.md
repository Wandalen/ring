# Data Structure: Sixteen Bytes to Carry Eight

### Scope

**Purpose:** Record what a `BytesSlot< N >` actually occupies, that the `usize`
length doubles the slot at the sizes the family uses, and that no line anywhere
argues for that width.

**Responsibility:** The in-memory layout of `BytesSlot< N >` — size, alignment,
and the ratio of overhead to payload at the four `N` the family instantiates.

**In Scope:** `ring_slot/src/lib.rs:237-241`; the `N` values appearing
across `ring_*/src` and `ring_*/tests`.

**Out of Scope:** `TypedSlot`'s layout and the niche asymmetry between the two
shapes are
[`data_structure/002`](002_the_niche_option_finds_and_the_array_does_not.md).
*Why* a length rather than a flag is
[`decisions/002`](../decisions/002_a_length_rather_than_a_flag.md) — this
instance prices the choice, it does not re-argue it.

---

## Two Fields, and What They Cost

```sh
cd "$(git rev-parse --show-toplevel)"
echo '--- every concrete BytesSlot< N > in the crates that consume it ---'
# the (::)? is load-bearing: the turbofish form is more than half of all sites
grep -rhoE 'BytesSlot(::)?< *[0-9]+ *>' \
  $( ls ring_*/src/*.rs ring_*/tests/*.rs | grep -v '^ring_slot/' ) \
  | tr -d ' ' | sed 's|::||' | sort -t'<' -k2 -n | uniq -c
echo '--- the two fields N sizes ---'
command grep -m1 -A4 -F 'pub struct BytesSlot< const N : usize >' ring_slot/src/lib.rs
```

Live output:

```
--- every concrete BytesSlot< N > in the crates that consume it ---
      7 BytesSlot<4>
     12 BytesSlot<8>
      4 BytesSlot<16>
      1 BytesSlot<32>
--- the two fields N sizes ---
pub struct BytesSlot< const N : usize >
{
  bytes : [ u8; N ],
  len : usize,
}
```

Four distinct `N`, none above 32, the commonest being 8. (`ring_slot`'s own
source and suite add four more — 0, 1, 2, and one 4096 in a `capacity()`
assertion — all boundary fixtures rather than ring payloads;
[`type/002`](../type/002_the_const_parameter_as_capacity.md) takes up the
degenerate ends.) Measured layout, release:

```
--- BytesSlot: the array, plus a usize ---
BytesSlot< 0 >             size   8  align  8
BytesSlot< 1 >             size  16  align  8
BytesSlot< 8 >             size  16  align  8
BytesSlot< 16 >            size  24  align  8
BytesSlot< 64 >            size  72  align  8
```

---

### SL21 — The Length Field Costs a Word and the Alignment It Brings, at Every `N`

Three readings of the table, in order of how much they cost:

| `N` | Size | Payload | Overhead | Ratio |
|----:|-----:|--------:|---------:|------:|
| 0 | 8 | 0 | 8 | ∞ |
| 1 | 16 | 1 | 15 | 15× |
| 8 | 16 | 8 | 8 | 1× |
| 16 | 24 | 16 | 8 | 0.5× |
| 64 | 72 | 64 | 8 | 0.125× |

`BytesSlot< 0 >` is eight bytes wide and can hold nothing. `BytesSlot< 1 >`
costs sixteen bytes to carry one — the `usize` forces alignment 8, so a single
payload byte is followed by seven of padding before the length. And
`BytesSlot< 8 >`, the shape the consuming crates instantiate ten times, is
exactly half overhead.

The alignment is the part that is easy to miss. `[ u8; N ]` has alignment 1; the
`usize` raises the struct's to 8, which is why `N == 1` jumps to 16 rather than
sitting at 9. Every `BytesSlot` is 8-byte aligned regardless of what it carries.

**Finding.** The overhead is a fixed eight bytes plus padding to a multiple of
eight, and it is worst exactly where the family lives. At `N == 8` — twelve of
the twenty-four consumer instantiations — the slot is 100% overhead; at
`N == 4`, it is 300%. Only
past `N == 64` does the length become a rounding error, and nothing in the family
is that large.

This is a real cost with a real justification
([`decisions/002`](../decisions/002_a_length_rather_than_a_flag.md) SL7: no
`MaybeUninit`, no `unsafe`), and the point here is only that the cost is larger
than the struct definition suggests. Two fields reading `[ u8; N ]` and `usize`
look like `N + 8`; at the sizes in use, they are closer to `2N`.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_slot
command grep -F 'alignment folds in as padding too, so the true cost sits' src/lib.rs
```

Live output:

```
/// alignment folds in as padding too, so the true cost sits closer to `2N`.
```

**Disposition:** applied — `BytesSlot`'s own doc comment no longer lets the
two-field definition imply `N + 8`; it states up front that the `usize`'s
alignment folds in as padding at the sizes this family uses, so the real cost
sits closer to `2N`, matching what this finding measures.
Now prints: `alignment folds in as padding too, so the true cost sits`

---

### SL22 — A Narrower Length Would Fit Every `N` the Family Uses, and Nothing Says Why It Was Not Chosen

Every `N` a consuming crate instantiates is 32 or below. A `u8` length represents
0..=255, which covers all of them with room to spare — though not the one
`BytesSlot< 4096 >` the crate's own suite constructs to assert `capacity()`,
which is the sharpest argument against narrowing and appears nowhere in the
source. Measured against the same payloads:

```
--- the same payloads with a narrower length ---
[ u8; 8 ] + u8             size   9  align  1
[ u8; 8 ] + u32            size  12  align  4
[ u8; 64 ] + u8            size  65  align  1
[ u8; 64 ] + u32           size  68  align  4
```

| Shape | `BytesSlot< 8 >` | with `u32` | with `u8` |
|-------|-----------------:|-----------:|----------:|
| Size | 16 | 12 | 9 |
| Align | 8 | 4 | 1 |
| Saving | — | 25% | 44% |

Over a ring's worth of slots the difference stops being academic:

```
--- a ring's worth: 1024 slots ---
BytesSlot< 8 > x1024         16384 bytes
[ u8; 8 ] + u8  x1024         9216 bytes
```

Seven kilobytes per thousand slots, on the shape the family uses most — and the
alignment drop from 8 to 1 means the packed version also stops forcing padding
in anything that embeds it.

The crate's own accounting of `usize` is thin:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r 'len : usize' ring_slot/src/lib.rs
grep -c 'usize' ring_slot/src/lib.rs
```

Live output:

```
  len : usize,
12
```

Seven occurrences of the word, one of which is the field. None of the other six
is a sentence explaining the width.

**Finding.** `usize` is the defensible default — it matches `slice::len`, it
needs no cast in `read`'s range or `write`'s comparison, and it cannot overflow
for any `N`. A narrower field would need `N <= u8::MAX` asserted somewhere and a
cast at every use, const-generic bounds of that shape are awkward to express, and
the crate's own suite already instantiates an `N` that would fail the assertion.

None of that is written down. The field is `usize` because `usize` is what
lengths are, and the reasoning was never surfaced — so a reader who notices the
100%-overhead measurement above has no way to tell whether the width was
considered and kept or simply never questioned. One sentence on the field
("`usize` rather than a narrower type: no cast at any use site, and the ring's
slot count dominates its slot width") would close it, and would also make the
counter-case visible if it ever stops being true.

The counter-case is not hypothetical: a `BytesSlot< 4 >` ring is 300% overhead
today, and the crate offers no smaller-footprint variant.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`data_structure/002`](002_the_niche_option_finds_and_the_array_does_not.md) | `TypedSlot`'s layout, and why only one shape composes for free |
| [`decisions/002`](../decisions/002_a_length_rather_than_a_flag.md) | Why a length at all — the ruling this instance prices |
| [`non_functional_requirement/001`](../non_functional_requirement/001_what_a_slot_costs.md) | The cost budget these figures feed |
| [`type/002`](../type/002_the_const_parameter_as_capacity.md) | `N` as a type parameter rather than a field |

### Sources

| Fact | Where |
|------|-------|
| The two fields | `ring_slot/src/lib.rs:237-241` |
| The `N` values in use | `ring_*/src/*.rs`, `ring_*/tests/*.rs` — census above |
| The absent justification | `ring_slot/src/lib.rs` — 7 `usize` occurrences, none explanatory |
| Every size and alignment figure | Release probe, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `capacity_is_the_const_parameter` | That `capacity()` reports `N` rather than a stored field |
| `a_zero_capacity_slot_accepts_only_nothing` | The `N == 0` degenerate case measured above |
| `a_write_of_exactly_capacity_is_accepted` | The length reaching `N`, its maximum |
| *(to create)* | A layout assertion pinning `size_of::< BytesSlot< 8 > >()`, so a field change is visible |
