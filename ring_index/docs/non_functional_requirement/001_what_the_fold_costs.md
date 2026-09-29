# Non-Functional Requirement: What the Fold Costs

### Scope

**Purpose:** Record the measured cost of the fold against the modulo it
replaces, that the module comment's "20–40 cycles" figure is roughly three to
six times the measured one and cites an architecture this host is not, and that
a constant-divisor modulo is free — which is the measurement most likely to make
someone draw the wrong conclusion.

**Responsibility:** The performance claim the crate makes for itself, and
whether the numbers support it.

**In Scope:** `ring_index/src/lib.rs:10-13`; `of` under `-O`, against
`%` with a constant divisor and with a runtime one.

**Out of Scope:** what `run` allocates is
[`non_functional_requirement/002`](002_the_one_allocation_and_the_zero_callers.md).
Whether anything in the family can hold this measurement is
[`integration/002`](../integration/002_the_feature_it_implements_half_of.md)
IX14.

---

## The Claim

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A3 -F '//! constraining capacity to a power of two is that this fold is then a bitmask' ring_index/src/lib.rs
echo "  measured on: $( uname -m ), $( rustc --version )"
```

Live output:

```
//! constraining capacity to a power of two is that this fold is then a bitmask
//! rather than a division — an integer `%` costs on the order of 20–40 cycles on
//! current x86, and it sits on every operation that touches a slot. The claim
//! path is not one of them: `ring_claim`, `ring_publish`, `ring_consume`, and
  measured on: aarch64, rustc 1.97.1 (8bab26f4f 2026-07-14)
```

Three assertions: the fold is cheaper than a division, the division costs 20–40
cycles, and the fold runs on every operation. The third is
[`integration/001`](../integration/001_two_dependents_and_a_third_that_did_it_again.md)
IX11's, and it is false. The first two are measurable, and were measured, on a
machine that is not the one the comment names.

---

### IX21 — The Fold Is Cheaper, by About 7×, Not 20–40×

Three separate `-O` builds, each timing the mask and both forms of `%` against
the same input stream, with a checksum to keep the optimizer honest:

```
  `% 1024`, divisor a constant          0.000 ns  ~ 0.0 cycles
  `% d`, divisor known only at runtime  2.077 ns  ~ 6.2 cycles
  ring_index::of, capacity hidden       0.284 ns  ~ 0.9 cycles
  runtime-divisor modulo costs 7.3x the mask; a constant one costs 0.0x
  (cycles at 3.0 GHz; checksums true true true)

  `% d`, divisor known only at runtime  2.036 ns  ~ 6.1 cycles
  ring_index::of, capacity hidden       0.287 ns  ~ 0.9 cycles
  runtime-divisor modulo costs 7.1x the mask; a constant one costs 0.0x

  `% d`, divisor known only at runtime  2.083 ns  ~ 6.2 cycles
  ring_index::of, capacity hidden       0.288 ns  ~ 0.9 cycles
  runtime-divisor modulo costs 7.2x the mask; a constant one costs 0.0x
```

**Finding.** The direction of the claim holds and the magnitude does not. A
runtime-divisor `%` measured 6.1–6.2 cycles across three runs; the comment says
20–40. The fold measured 0.9 cycles, so the ratio is 7.1–7.3×, not the 22–44×
the comment's numbers imply against a one-cycle mask.

Two things make the gap defensible rather than careless. `aarch64`'s `udiv` is
faster than the x86 `div` the comment describes, so a figure that was accurate
for the named architecture would still be wrong here — the comment is not
claiming something false about x86, it is claiming something about a machine
nobody measured. And 7× is still a large ratio for an operation this hot; the
constraint pays for itself either way.

What is worth recording is that the number is stated with two significant
figures and a named architecture, which reads as measurement, and no measurement
exists anywhere in the repository to back it. The correct form of the claim is
either a range wide enough to cover both architectures or a citation, and it is
neither.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_index
command grep -F 'a 7x ratio rather than the' src/lib.rs
```

Live output:

```
//! cycles against a runtime-divisor `%`'s 6.1–6.2, a 7x ratio rather than the
```

**Disposition:** applied — the module comment no longer states the x86 figure
as though it were measured here; it now discloses that it is asserted for
x86, gives this repository's own build-host measurement (roughly 0.9 cycles
against 6.1–6.2, a 7x ratio), and names the 22–44x gap against the x86 figure.
Now prints: `a 7x ratio rather than the`

---

### IX22 — A Constant-Divisor Modulo Is Free, Which Is the Trap

`% 1024` measured 0.000 ns — the same as doing nothing. The compiler recognizes
a power-of-two literal and emits the mask itself.

**Finding.** Anyone benchmarking this claim the obvious way — writing
`seq % 1024` next to `seq & 1023` and timing both — will measure them as
identical and conclude the power-of-two constraint buys nothing. That conclusion
is wrong, and the reason it is wrong is the whole justification for the
constraint.

The capacity in this family is never a literal at the fold site. It arrives as a
`Capacity` built at runtime from a `RingConfig`, is stored in a struct field,
and is read back through `self.capacity()` — so the divisor is opaque to the
optimizer at every real call site, and the 6.2-cycle figure is the one that
applies. The probe reproduces this by hiding the capacity behind
`std::hint::black_box`, which is what "capacity hidden" in the output means.

So the constraint's value is real and invisible to the naive benchmark. That is
a more useful thing to have written in the module comment than a cycle count for
an architecture the project does not run on, and it is the part the comment does
not say.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_index
command grep -F 'the family'"'"'s capacity is never a' src/lib.rs
```

Live output:

```
//! beside `seq & 1023`) would also mislead: the family's capacity is never a
```

**Disposition:** applied — the module comment now says the part it previously
did not: a same-literal benchmark would mislead, because the family's capacity
is never a compile-time literal at a real fold site, so the runtime-divisor
figure is the one that applies rather than the free constant-divisor one.
Now prints: `the family's capacity is never a`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`decisions/001`](../decisions/001_a_power_of_two_or_nothing.md) | The constraint these numbers are meant to justify |
| [`integration/002`](../integration/002_the_feature_it_implements_half_of.md) | The feature that promised this would be measured, and the crate that cannot measure it |
| [`api/002`](../api/002_the_three_signatures_and_the_const_they_are_not.md) | Why the missing `const` costs nothing at runtime |
| [`non_functional_requirement/002`](002_the_one_allocation_and_the_zero_callers.md) | The cost that is not measured in cycles |

### Sources

| Fact | Where |
|------|-------|
| The 20–40 cycle claim | `ring_index/src/lib.rs:10-13` |
| Host architecture and toolchain | `uname -m`, `rustc --version`, quoted above |
| Cycle counts, three runs | Release probe, quoted above |
| Constant-divisor modulo compiling to a mask | Same probe, row 1 |

### Tests

| Test | Covers |
|------|--------|
| `derivation_is_a_mask` | That the result is the mask — correctness, not cost |
| *(to create)* | Nothing — a cycle count is not a test assertion, and the family has no benchmark target to hold one |
