# Type: Six Derives on a Fieldless Enum

### Scope

**Purpose:** Record what the six derives on `Resolution` buy, where each is
exercised, and which are needed by anything outside this crate.

**Responsibility:** The derive list, the one test that uses most of it, and the
absent seventh derive the sibling enum carries.

**In Scope:** `ring_overflow/src/lib.rs:55`;
`ring_overflow/tests/overflow_test.rs:269-283`;
`ring_types/src/policy.rs:104`.

**Out of Scope:** The layout the derives sit on is
[`data_structure/001`](../data_structure/001_one_byte_and_two_hundred_fifty_three_spare_niches.md).
The two inherent methods are [`item/002`](../item/002_the_enum_and_its_two_readings.md).

---

## Six Derives, and Where Each Lands

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the derive list, and the comment now guarding the absent seventh --'
command grep -m1 -B2 -F '#[ derive( Debug, Clone, Copy, PartialEq, Eq, Hash ) ]' ring_overflow/src/lib.rs
echo '  -- the sibling enum, one derive longer --'
command grep -m1  -A2 -F '/// assert!( !WaitKind::Park.is_non_blocking() );' ring_types/src/policy.rs | tail -n 1
echo '  -- where Copy, PartialEq and Hash are each exercised --'
command grep -m1 -A10 -F '  let a = Resolution::EvictedOldest;' ring_overflow/tests/overflow_test.rs
echo '  -- and every use of the type outside this crate --'
command grep 'Resolution' ring_core/src/lib.rs
```

Live output:

```
  -- the derive list, and the comment now guarding the absent seventh --
// makes the *count* enforced; this comment is the only thing standing between
// the derive list and the reader who would otherwise "fix" it.
#[ derive( Debug, Clone, Copy, PartialEq, Eq, Hash ) ]
  -- the sibling enum, one derive longer --
#[ derive( Debug, Clone, Copy, PartialEq, Eq, Hash, Default ) ]
  -- where Copy, PartialEq and Hash are each exercised --
  let a = Resolution::EvictedOldest;
  let b = a;
  assert_eq!( a, b );
  assert_ne!( a, Resolution::Refused );

  let mut counts = std::collections::HashMap::new();
  for policy in OverflowPolicy::ALL
  {
    *counts.entry( would_resolve( policy ) ).or_insert( 0 ) += 1;
  }
  assert_eq!( counts.len(), 3, "three policies must key three distinct buckets" );
  -- and every use of the type outside this crate --
use ring_overflow::{ would_resolve, Resolution };
        Resolution::DroppedIncoming => Ok( () ),
        Resolution::EvictedOldest | Resolution::Refused => Err( record ),
```

---

### OV33 — Every Derive Is Exercised, and Only by This Crate's Own Suite

All six derives are used, and `a_resolution_is_a_plain_comparable_value` uses five
of them in eleven lines: `Copy` at `let b = a;` (the original stays live for the
next assertion), `PartialEq` and `Eq` in `assert_eq!`/`assert_ne!`, and `Eq` plus
`Hash` together as the bound on a `HashMap` key. `Debug` is used by every
assertion message in the file, including that one's `"{policy:?}"`.

Outside the crate, the type appears four times over three lines: an import and
two `match` arms that name all three variants between them. None of that needs
a derive — pattern matching is structural, and the arms compare nothing.

**Finding.** So the derive list is fully justified by the tests and entirely
unexercised by the consumer. That is not an argument for removing any of it —
`Copy` on a one-byte enum is free, and a public outcome type that could not be
compared or printed would be hostile — but it does mean the capabilities are
inferred from what a value like this should support rather than from what any
caller has asked for.

The one to watch is `Hash`. It is a real commitment on a public type, exercised by
one line in one test, and used to key three buckets that a three-element array
would key more cheaply. It earns its place as documentation of intent — a
resolution is a value you may tabulate — rather than as a response to a need.

---

### OV34 — The Missing Seventh Derive Is the Right Omission and the Undocumented One

`OverflowPolicy` derives seven: the same six plus `Default`, with `#[ default ]`
on `DropNewest`. `Resolution` derives six.

The asymmetry is correct. A policy is configuration, and a ring built without one
stated needs a defined starting point. A resolution is caused — every value is
produced by an event that already happened — so `Resolution::default()` would be a
value nothing produced, and its existence would let a caller construct an outcome
that never occurred.

**Finding.** This is the crate's sharpest type-level decision and it was
invisible. The two declarations sit eight lines apart across two crates and differ
by one token; a reader comparing them saw an inconsistency, and a future author
normalising the family's derive lists would have added `Default` to `Resolution`
in good faith and broken the thing that makes the type meaningful.

The guard against that is a comment. There was none — no note on the declaration,
no mention in the module doc, and no test that would fail. Adding `Default`
compiled, passed the entire suite, and changed nothing observable until someone
called it.

The comment now sits directly above the derive list, where the normalisation would
be typed. It states the asymmetry, why it is correct, and that the two
declarations are one token apart in two crates — so the reader who came to make
them match finds the reason they do not before making the edit rather than after.

**Disposition:** applied — a source comment added above `Resolution`'s derive list
in `ring_overflow/src/lib.rs`. A comment rather than a test because the
omission is not testable: Rust offers no way to assert a trait is *not*
implemented, and the negative-reasoning tricks that approximate it are more
fragile than the thing they would guard. The comment is placed at the point of
edit, which is the only guard available and the one a normaliser cannot miss. Now prints: `// the derive list and the reader who would otherwise "fix" it.`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`data_structure/001`](../data_structure/001_one_byte_and_two_hundred_fifty_three_spare_niches.md) | The layout these derives sit on |
| [`type/002`](002_three_variants_and_two_questions.md) | What the variants mean |
| [`item/002`](../item/002_the_enum_and_its_two_readings.md) | The inherent methods beside the derived ones |
| [`decisions/001`](../decisions/001_the_fourth_variant_that_is_not_there.md) | Why no resolution is uncaused |

### Sources

| Fact | Where |
|------|-------|
| The derive list | `ring_overflow/src/lib.rs:55` |
| `OverflowPolicy`'s seven | `ring_types/src/policy.rs:104` |
| Five derives in eleven lines | `ring_overflow/tests/overflow_test.rs:272-282` |
| The consumer's four mentions | `ring_core/src/lib.rs:80`, `:413-414` |

### Tests

| Test | Covers |
|------|--------|
| `a_resolution_is_a_plain_comparable_value` | `Copy`, `PartialEq`, `Eq`, `Hash` |
| `distinct_policies_give_distinct_resolutions` | `PartialEq`, via `Vec::contains` |
| `each_policy_maps_to_its_own_resolution` | `PartialEq` and `Debug`, via `assert_eq!` |
| *(to create)* | Nothing would fail if `Default` were added, so the omission is unguarded |
