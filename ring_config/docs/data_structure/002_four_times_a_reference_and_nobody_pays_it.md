# Data Structure: Four Times a Reference, and Nobody Pays It

### Scope

**Purpose:** Measure what passing the thirty-two-byte record by value costs
against passing a pointer to it, then account for who in the family actually pays
that cost.

**Responsibility:** The paired measurement, its spread, and the by-value /
by-reference split across every consumer signature.

**In Scope:** `ring_core/src/lib.rs:163`, `:196`;
`ring_mpsc/src/lib.rs:408`; `ring_spsc/src/lib.rs:323`;
`ring_factory/src/lib.rs:150`, `:197`, `:240`;
`ring_bench/src/lib.rs:208`, `:226`.

**Out of Scope:** Where the thirty-two bytes come from is
[`data_structure/001`](001_twenty_six_bytes_of_fields_in_thirty_two_of_struct.md).
Why the setters take `mut self` at all is
[`pattern/001`](../pattern/001_a_consuming_builder_over_a_copy_record.md).

---

## How Every Consumer Takes It

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- every signature and field naming a RingConfig outside this crate --'
command grep -r ': *&\?RingConfig' --include=*.rs */src | command grep -v '^ring_config/' | command grep -v '//\|use ring_config' | sed 's|^ring/||'
```

Live output:

```
  -- every signature and field naming a RingConfig outside this crate --
ring_bench/src/lib.rs:  config : RingConfig,
ring_bench/src/lib.rs:  pub const fn new( config : RingConfig ) -> Self
ring_core/src/lib.rs:  pub fn new( config : &RingConfig ) -> Result< Self, RingError >
ring_core/src/lib.rs:  pub fn new_crossbeam( config : &RingConfig ) -> Result< Self, RingError >
ring_factory/src/lib.rs:  pub fn build< S : Send >( &self, cfg : RingConfig ) -> Result< Split< S >, BuildError >
ring_factory/src/lib.rs:    cfg : RingConfig,
ring_factory/src/lib.rs:  pub fn build_crossbeam< S : Send >( &self, cfg : RingConfig ) -> Result< Split< S >, BuildError >
ring_mpsc/src/lib.rs:  pub fn with_config( config : &RingConfig ) -> Self
ring_spsc/src/lib.rs:  pub fn with_config( config : &RingConfig ) -> Self
```

## The Measured Cost of a Copy

Two `#[ inline( never ) ]` functions reading one field, one taking the record by
value and one taking a shared reference, timed back-to-back inside each of nine
repetitions so both variants meet the same machine conditions. `black_box` wraps
the argument on both sides so the copy cannot be folded away, and the ratio is
taken per repetition rather than between the two medians. Two independent runs,
`--release`, `aarch64-unknown-linux-gnu`:

```rust
#[ inline( never ) ]
fn by_value( c : RingConfig ) -> usize { c.capacity().get() }

#[ inline( never ) ]
fn by_reference( c : &RingConfig ) -> usize { c.capacity().get() }

// inside each of nine repetitions, back to back:
let v = time_value( &cfg ).as_secs_f64() / f64::from( ITERS ) * 1e9;
let r = time_reference( &cfg ).as_secs_f64() / f64::from( ITERS ) * 1e9;
val.push( v ); refs.push( r ); ratio.push( v / r );
```

```
Run 1:
    by_value       median 2.126 ns   min 2.122   max 3.994
    by_reference   median 0.577 ns   min 0.574   max 0.911
    ratio          median 3.69x      min 2.33x     max 6.44x
Run 2:
    by_value       median 2.327 ns   min 2.258   max 3.713
    by_reference   median 0.582 ns   min 0.574   max 0.614
    ratio          median 4.00x      min 3.69x     max 6.09x
```

The two ratio medians land at 3.69x and 4.00x, and `by_reference`'s own minimum is
identical across runs to three decimal places at 0.574 ns — but the ratio spread is
wide on both, 2.33x to 6.44x, and `by_value`'s minimum moves from 2.122 to 2.258.
The machine carried a load average near 43 on sixteen cores throughout, from
an unrelated mutation-testing sweep. Pairing inside each repetition is what keeps
the medians usable under that; the spread is the honest cost of measuring when the
machine is busy, and it is reported rather than smoothed.

---

### RC11 — A By-Value Call Costs About Four Times a By-Reference One

Copying thirty-two bytes across a call boundary the optimizer cannot see through
runs at roughly 2.1–2.3 ns against 0.58 ns for passing a pointer — a median ratio
of 3.69x on the first run and 4.00x on the second. The absolute difference is
about 1.6 ns per call.

**Finding.** The ratio is large and the quantity is small, which is the whole
character of the measurement. Four times a very cheap operation is still a very
cheap operation: at 1.6 ns, a caller would need on the order of six hundred
million passes to lose a second.

The number is worth having because the record is the family's one shared
configuration type, and "should this take `&RingConfig` or `RingConfig`?" is a
question every new consumer has to answer. The answer this measurement supports is
that it does not matter at any call rate a configuration is plausibly passed at,
which is a more useful thing to know than a ratio alone.

**Disposition:** declined — this instance's own Finding concludes the cost does
not matter at any plausible call rate (six hundred million passes to lose one
second); the measurement is descriptive, not a defect, and no source or doc
fix is implied beyond what is already recorded in
`data_structure/002_four_times_a_reference_and_nobody_pays_it.md`.

---

### RC12 — The Family Splits Evenly on the Question, and the Split Tracks Tier Rather Than Cost

Nine sites outside this crate name a `RingConfig` in a signature or a field. Four
take a shared reference — `ring_core::Ring::new` and `new_crossbeam`,
`ring_mpsc::Ring::with_config`, `ring_spsc::Ring::with_config`. Five take it by
value — `ring_factory`'s `build` and `build_crossbeam` plus the field they store
it in, and `ring_bench`'s constructor and its field.

The line falls exactly on tier. Every crate that builds a ring directly takes a
reference; every crate that *composes* one — the factory, the benchmark harness —
takes ownership and stores it.

**Finding.** That is a coherent split and no document states it as a rule. It
reads as a convention someone followed rather than a decision anyone recorded, and
the measurement above says neither side is paying meaningfully for its choice — so
the split survives on consistency alone, with nothing written down to keep it
consistent.

The one asymmetry worth naming: `ring_factory` re-exports `RingConfig`
(`src/lib.rs:104`) and then takes it by value, so a caller reaching the type
through the factory sees the owning convention, and a caller reaching it through
`ring_config` directly sees the borrowing one. Both are correct; a reader
comparing two call sites has no way to tell which they are looking at is
deliberate.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`data_structure/001`](001_twenty_six_bytes_of_fields_in_thirty_two_of_struct.md) | Where the thirty-two bytes come from |
| [`pattern/001`](../pattern/001_a_consuming_builder_over_a_copy_record.md) | Why the setters consume `self` and what `Copy` does to that |
| [`integration/002`](../integration/002_a_re_export_that_carries_the_record_but_not_its_vocabulary.md) | The `ring_factory` re-export, and who arrives by which route |
| [`non_functional_requirement/001`](../non_functional_requirement/001_a_record_that_allocates_nothing_and_is_read_once.md) | The cost profile this measurement feeds |

### Sources

| Fact | Where |
|------|-------|
| Four by-reference signatures | `ring_core/src/lib.rs:163`, `:196`; `ring_mpsc/src/lib.rs:408`; `ring_spsc/src/lib.rs:323` |
| Five by-value sites | `ring_factory/src/lib.rs:150`, `:197`, `:240`; `ring_bench/src/lib.rs:208`, `:226` |
| The re-export the owning callers reach through | `ring_factory/src/lib.rs:104` |
| 3.69x and 4.00x median, 2.33x–6.44x spread | Probe above, two runs |
| Machine load ~43/16 cores during both runs | `uptime`, taken between the runs |

### Tests

| Test | Covers |
|------|--------|
| `the_record_is_copy_and_compares_by_value` | That the by-value form is legal at all |
| `each_setter_is_independent` | The consuming chain the by-value convention comes from |
| `every_named_field_is_carried` | That a copy carries everything a reference would reach |
