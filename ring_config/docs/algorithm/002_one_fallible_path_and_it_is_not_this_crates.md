# Algorithm: One Fallible Path, and It Is Not This Crate's

### Scope

**Purpose:** Record the crate's single failure path — where it starts, what it can
carry, and how much of it this crate authors.

**Responsibility:** `new`'s `Result`, the one `?`, the delegated rejection, and the
gap between the error type's declared surface and its reachable one.

**In Scope:** `ring_config/src/lib.rs:65`, `:71`;
`ring_types/src/capacity.rs:44`, `:48`; `ring_types/src/error.rs:42-80`.

**Out of Scope:** Why `new` is the only non-`const` function is
[`workaround/001`](../workaround/001_the_question_mark_forecloses_const.md). The
two clamps, which correct rather than reject, are
[`decisions/002`](../decisions/002_clamping_instead_of_a_question_mark_mid_chain.md).

---

## The One Failure Path

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- every fallible signature in this crate --'
command grep 'Result<\|Result <' ring_config/src/lib.rs
echo '  -- the one ? in the crate, and what it propagates from --'
command grep 'Capacity::new( slots )?' ring_config/src/lib.rs
echo '  -- every Err this crate constructs itself --'
command grep -c 'Err(\|Err (' ring_config/src/lib.rs || true
echo '  -- variants RingError declares --'
command grep -m1 -B1 -A35 -F '  /// A capacity of zero was requested. A ring with no slots can never accept' ring_types/src/error.rs | command grep -c '^  [A-Z]' || true
echo '  -- and the two the suite pins as reachable from here --'
command grep 'unwrap_err()' ring_config/tests/config_test.rs
echo '  -- every Err this crate delegates to --'
command grep 'Err( RingError' ring_types/src/capacity.rs
```

Live output:

```
  -- every fallible signature in this crate --
  pub fn new( slots : usize ) -> Result< Self, RingError >
  -- the one ? in the crate, and what it propagates from --
        capacity : Capacity::new( slots )?,
  -- every Err this crate constructs itself --
0
  -- variants RingError declares --
9
  -- and the two the suite pins as reachable from here --
  assert_eq!( RingConfig::new( 0 ).unwrap_err(), RingError::CapacityZero );
  assert_eq!( RingConfig::new( 7 ).unwrap_err(), RingError::CapacityNotPowerOfTwo( 7 ) );
  -- every Err this crate delegates to --
      return Err( RingError::CapacityZero );
      return Err( RingError::CapacityNotPowerOfTwo( slots ) );
```

---

### RC3 — The Crate Constructs No Error of Its Own

One function in `ring_config` returns a `Result`, and its only `Err` arrives
through a `?` on `Capacity::new`. Grep finds no `Err(` constructed anywhere in the
crate. Every rejection `RingConfig::new` can perform was written in `ring_types`.

**Finding.** The module comment says "This crate holds the record and its
validation." Half of that is delegated: the rejecting half. What is local is the
*correcting* half — two clamps that repair rather than reject, mapping a producer
count of zero to one and a batch outside `1..=capacity` into range.

So the crate's treatment of bad input splits three ways by field. Capacity is
rejected, by someone else, loudly, with a named error. Producers and batch are
accepted, locally, at a different value than the caller asked for. Wait kind and
overflow policy cannot be wrong at all, because their types have no invalid value.

Each of the three is documented where it happens, and the clamps unusually well
for this family — `with_producers` gives its reason ("clamping keeps the setter
infallible so a builder chain does not need a `?` in its middle") and `with_batch`
gives a different one ("a batch larger than the ring can never be served however
much draining happens"). What no document states is the shape: that "validation"
covers three distinct fates, that only one of them produces a value the caller can
inspect, and that which fate a field meets was decided field by field rather than
by a rule.

---

### RC4 — Nine Declared Variants, Two Reachable, and the Type Is `#[ non_exhaustive ]`

`RingConfig::new` returns `Result< Self, RingError >`. `RingError` declares nine
variants and carries `#[ non_exhaustive ]`. `Capacity::new` — the only source of
this crate's errors — constructs exactly two of them, and the suite pins both.

**Finding.** A caller matching on `RingConfig::new`'s error therefore faces nine
variants plus a mandatory `_` arm, to handle two outcomes: a zero capacity and a
non-power-of-two capacity. Seven of the nine — `Full`, `Empty`, `Closed`,
`NameTaken`, `NameUnknown`, `BatchTooLarge`, `PolicyUnsupported` — describe runtime
conditions of a *running* ring and cannot arise from validating a number.

The cost is not correctness, and it is not the documentation either. `new`'s
`# Errors` section reads "Whatever [`Capacity::new`] rejects — a zero or
non-power-of-two capacity", which names both reachable outcomes exactly. That is
what makes the gap worth recording: the prose already knows the answer is two, and
the type it documents cannot say so. The signature a compiler sees, and that any
`match` is written against, is the nine-variant one.

`BatchTooLarge { requested, capacity }` is the variant worth noting against this
crate specifically: it names exactly the condition `with_batch` handles, and
`with_batch` does not return it — it clamps instead
([`decisions/002`](../decisions/002_clamping_instead_of_a_question_mark_mid_chain.md)).
The family already has a name for the error this crate chose not to raise.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`decisions/002`](../decisions/002_clamping_instead_of_a_question_mark_mid_chain.md) | The other treatment of bad input, and why |
| [`workaround/001`](../workaround/001_the_question_mark_forecloses_const.md) | What the one `?` costs the constructor |
| [`invariant/001`](../invariant/001_two_ranges_that_cannot_be_violated.md) | What the clamps guarantee instead |
| [`algorithm/001`](001_four_setters_and_the_one_that_reads_a_second_field.md) | The rest of the computation |

### Sources

| Fact | Where |
|------|-------|
| The only fallible signature | `ring_config/src/lib.rs:65` |
| The only `?`, and no locally-constructed `Err` | `ring_config/src/lib.rs:71` |
| The `# Errors` section naming both reachable outcomes | `ring_config/src/lib.rs:56-58` |
| The two clamps, each documented with its own reason | `ring_config/src/lib.rs:112-114`, `:128-130` |
| Nine declared variants, `#[ non_exhaustive ]` | `ring_types/src/error.rs:42-80` |
| The two this crate can propagate | `ring_types/src/capacity.rs:44`, `:48` |
| The variant naming the condition `with_batch` clamps | `ring_types/src/error.rs:63-71` |

### Tests

| Test | Covers |
|------|--------|
| `capacity_is_validated_at_construction` | Both reachable errors, and two accepted capacities |
| `batch_clamps_into_one_through_capacity` | The condition that produces no error at all |
| `defaults_are_the_documented_ones` | The state a successful construction lands in |
