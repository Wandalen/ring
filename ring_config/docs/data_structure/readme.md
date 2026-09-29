# data_structure

One `struct` of five fields: a `Capacity` newtype over `usize`, two fieldless
one-byte enums, and two raw `usize` counts. Measured, that is twenty-six bytes of
field in thirty-two bytes of struct at alignment eight — six bytes no field order
can recover, because they follow from the field types rather than their order.

The six wasted bytes buy one property back. `Option< RingConfig >` is also
thirty-two bytes: the two one-byte enums carry unused bit patterns the compiler
claims as a niche, so an optional configuration costs nothing over a mandatory
one. And passing the record by value rather than by pointer costs about four times
as much per call — a ratio worth knowing and a quantity, 1.6 ns, that no consumer
in the family pays at a rate where it matters.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_twenty_six_bytes_of_fields_in_thirty_two_of_struct.md) | Twenty-Six Bytes of Fields in Thirty-Two of Struct | The measured layout, the padding, and the niche `Option` uses |
| [002](002_four_times_a_reference_and_nobody_pays_it.md) | Four Times a Reference, and Nobody Pays It | The paired by-value/by-reference measurement and the family's even split |

## Padding That Field Order Cannot Fix

Three eight-byte fields and two one-byte fields round to thirty-two under
alignment eight in every arrangement, and `repr(Rust)` already reorders freely, so
the six bytes are not a declaration-order mistake anyone can correct by editing.

What would shrink the record is narrowing the two counts: `producers` is a thread
count and `batch` is bounded above by `capacity`, and `u32` for both gives
eighteen bytes of field in twenty-four bytes of struct. The niche survives that
change, since it comes from the enums rather than the integers. The case for
making it is nonetheless weak — the record is built once per ring and read through
a reference by every low-level consumer — which is why both facts are recorded and
neither is a recommendation.

## An Even Split, Along Tier Rather Than Cost

Nine sites outside this crate name a `RingConfig`. Four take `&RingConfig`, and
all four build a ring directly: `ring_core::Ring::new` and `new_crossbeam`,
`ring_mpsc::Ring::with_config`, `ring_spsc::Ring::with_config`. Five take it by
value, and all five compose rather than build: `ring_factory`'s two `build`
methods and its stored field, `ring_bench`'s constructor and its stored field.

The measurement says neither side is paying meaningfully — 2.1–2.3 ns against
0.58 ns, roughly 1.6 ns per call, at a call rate of once per ring. So the split is
coherent, costless, and written down nowhere, which leaves nothing to keep it
coherent. `ring_factory` re-exports the type and then takes it by value, so which
convention a reader meets depends on which crate they arrived through.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the record as declared, and the types it is built from --'
command grep -m1 -A8 -F '#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]' ring_config/src/lib.rs
command grep -n 'derive\|pub struct Capacity\|pub enum WaitKind\|pub enum OverflowPolicy' ring_types/src/capacity.rs ring_types/src/policy.rs 
echo '  -- every signature and field naming a RingConfig outside this crate --'
command grep -rn ': *&\?RingConfig' --include=*.rs */src | command grep -v '^ring_config/' | command grep -v '//\|use ring_config' 
echo '  -- how many by-reference signatures each crate carries --'
command grep -rc ': *&RingConfig' --include=*.rs */src 2>/dev/null | command grep -v ':0$' 
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| RC9 | `ring_config` | n/a — observation | The record measures 32 bytes at alignment 8 holding 26 bytes of field — three eight-byte (`Capacity`, `producers`, `batch`) and two one-byte (`WaitKind`, `OverflowPolicy`) — and no field order removes the six bytes of padding, since `repr(Rust)` already reorders freely and the shortfall follows from the types; narrowing the two counts to `u32` would give 18 bytes in 24, which is recorded rather than recommended because the record is built once per ring |
| RC10 | `ring_config` | n/a — observation | `Option< RingConfig >` measures 32 bytes, the same as `RingConfig`, because the two fieldless one-byte enums carry unused bit patterns the compiler claims as a niche while neither `Capacity` (a plain `usize` newtype) nor either raw `usize` offers one — so an optional configuration is free, nothing in the crate records that, and the property would be silently lost if a field type changed |
| RC11 | `ring_config` | **measured cost** | Passing the record by value across a non-inlined call boundary costs 2.126 ns against 0.577 ns for a shared reference (median of nine paired repetitions, ratio 3.69x; second run 2.327/0.582, ratio 4.00x; spread 2.33x–6.44x under a load average near 43 on 16 cores), so the ratio is four-fold and the quantity is 1.6 ns per call — around six hundred million passes to lose a second |
| RC12 | `ring_config` | n/a — unenforced | Nine sites outside this crate name a `RingConfig`: four take `&RingConfig` and all four build a ring directly (`ring_core::Ring::new`, `new_crossbeam`, `ring_mpsc`/`ring_spsc` `with_config`), five take it by value and all five compose one (`ring_factory`'s two `build` methods and its field, `ring_bench`'s constructor and its field) — a split that falls exactly on tier, costs neither side anything measurable, and is stated in no document, while `ring_factory` re-exports the type so which convention a reader meets depends on their route in |
