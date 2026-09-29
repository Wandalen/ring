# data_structure

Two representations, measured. `TypedSlot< T >` is exactly its `Option< T >` at
every `T` tried — the niche optimisation finds a discriminant to hide in, so the
newtype costs nothing and composes into further `Option` layers for free.
`BytesSlot< N >` is an inline array plus a `usize`, so it costs sixteen bytes to
carry eight and has no niche at any `N`.

Both instances are about the length field. The first prices it: a fixed word plus
alignment padding, worst exactly at the sizes the family uses — at `N == 8`,
twelve of twenty-four consumer instantiations, the slot is one hundred percent
overhead.
The second
follows the missing niche outward: every `Option` layer over a `BytesSlot` costs
a full extra word, on the shape that is already half overhead, so the two costs
compound instead of trading off.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Sixteen Bytes to Carry Eight](001_sixteen_bytes_to_carry_eight.md) | SL21, SL22 — a fixed word of overhead at every `N`, and why a narrower field was not chosen |
| 002 | [The Niche `Option` Finds and the Array Does Not](002_the_niche_option_finds_and_the_array_does_not.md) | SL23, SL24 — a free newtype, and a wrapping cost that compounds with the length field |

### Measured Layouts

| Type | size | align | Overhead |
|------|-----:|------:|----------|
| `BytesSlot< 0 >` | 8 | 8 | ∞ |
| `BytesSlot< 1 >` | 16 | 8 | 15× |
| `BytesSlot< 8 >` | 16 | 8 | 100% |
| `BytesSlot< 16 >` | 24 | 8 | 50% |
| `BytesSlot< 64 >` | 72 | 8 | 12.5% |
| `TypedSlot< u32 >` | 8 | 4 | 0 — identical to `Option< u32 >` |
| `TypedSlot< String >` | 24 | 8 | 0 — identical to `Option< String >` |
| `TypedSlot< () >` | 1 | 1 | 0 |

The crates that build rings out of `BytesSlot` instantiate it at four widths — 4,
8, 16, 32 — every one of them in the band where the length field dominates.
`ring_slot`'s own suite adds 0, 1, 2 and a single 4096, all boundary fixtures
rather than ring payloads.

### The Two Costs Compound

`Option< TypedSlot< u32 > >` is 8 bytes: the niche absorbs the outer layer at no
cost. `Option< BytesSlot< 8 > >` is 24 against the slot's own 16 — eight bytes
for a discriminant, on a type that was already eight bytes of length field
carrying eight bytes of payload.

Nothing in the family wraps a slot in an `Option` today, so nobody pays it. It is
recorded because the shape that would pay is the shape least able to afford it.

### Why `usize` Is Still Right

A `u8` length fits every `N` the family uses and would make `BytesSlot< 8 >`
nine bytes instead of sixteen. It would also need `N <= u8::MAX` asserted
somewhere const-generic bounds make awkward, and a cast at every use — `read`'s
range, `write`'s comparison, `len`'s return. `usize` matches `slice::len`, needs
no cast anywhere, and cannot overflow for any `N`. The trade is defensible; that
it was a trade is unrecorded.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# the declaration whose second field is the whole subject
command grep -m1 -A7 -F '/// assert!( !slot.is_empty() );' ring_slot/src/lib.rs | tail -n 6

# every N the consuming crates instantiate, then the ones only this crate reaches
# the (::)? is load-bearing: the turbofish form is more than half of all sites
grep -rhoE 'BytesSlot(::)?< *[0-9]+ *>' \
  $( ls ring_*/src/*.rs ring_*/tests/*.rs | grep -v '^ring_slot/' ) \
  | tr -d ' ' | sed 's|::||' | sort -t'<' -k2 -n | uniq -c
grep -rhoE 'BytesSlot(::)?< *[0-9]+ *>' ring_slot/src/lib.rs ring_slot/tests/*.rs \
  | tr -d ' ' | sed 's|::||' | sort -t'<' -k2 -n | uniq -c

# every Option layer over a slot, anywhere
grep -rn 'Option< *TypedSlot\|Option< *BytesSlot' ring_*/src/*.rs || echo '  none — the wrapping cost is latent'
```

Layout figures are produced by a release probe calling `core::mem::size_of` and
`align_of` on each type; the numbers are quoted in the instances.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SL21 | `ring_slot` | **measured cost** | The length field costs a fixed word plus alignment at every `N`, and all four widths the consuming crates use sit in the band where it dominates — 100% overhead at `N == 8` |
| SL22 | `ring_slot` | n/a — observation | A `u8` length fits every `N` in use; `usize` is still right for cast-freedom and `slice::len` parity, and the trade is recorded nowhere |
| SL23 | `ring_slot` | n/a — observation | `TypedSlot< T >` is byte-identical to `Option< T >` at every `T` measured, so the newtype is free and composes into further `Option`s for free |
| SL24 | `ring_slot` | **measured cost** | `BytesSlot< N >` has no niche at any `N`, so every wrapping layer costs a full word — compounding with the length field rather than trading against it |
