# Decisions: `Fail` Returns an Error, Not a Resolution

### Scope

**Purpose:** Record that the two functions disagree about how to express the same
outcome — `resolve` returns `Err( RingError::Full )` where `would_resolve` returns
`Resolution::Refused` — and what follows from that split.

**Responsibility:** The two `Fail` arms, the variant `resolve` can never produce,
and every place `Refused` is constructed.

**In Scope:** `ring_overflow/src/lib.rs:204`, `:235`, `:75`;
`ring_types/src/error.rs:54`.

**Out of Scope:** What the `Result` costs is
[`data_structure/002`](../data_structure/002_a_one_byte_outcome_in_a_twenty_four_byte_result.md).
The absent fourth variant is
[`decisions/001`](001_the_fourth_variant_that_is_not_there.md).

---

## One Policy, Two Answers

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the same policy, in the two functions --'
sed -n '/^    OverflowPolicy::Fail => Err( RingError::Full ),$/p;/^    OverflowPolicy::Fail => Resolution::Refused,$/p' ring_overflow/src/lib.rs
echo '  -- every construction of Refused, anywhere in the workspace --'
command grep -r 'Resolution::Refused' --include=*.rs . | sed 's|ring/||'
echo '  -- and the two names the one outcome carries --'
command grep '^  Refused,' ring_overflow/src/lib.rs
command grep '^  Full,' ring_types/src/error.rs
```

Live output:

```
  -- the same policy, in the two functions --
    OverflowPolicy::Fail => Err( RingError::Full ),
    OverflowPolicy::Fail => Resolution::Refused,
  -- every construction of Refused, anywhere in the workspace --
ring_overflow/tests/overflow_test.rs:  assert_eq!( would_resolve( OverflowPolicy::Fail ), Resolution::Refused );
ring_overflow/tests/overflow_test.rs:      Resolution::Refused => "refused",
ring_overflow/tests/overflow_test.rs:  assert_eq!( name( Resolution::Refused ), "refused" );
ring_overflow/tests/overflow_test.rs:  assert!( !Resolution::Refused.lost_an_item() );
ring_overflow/tests/overflow_test.rs:  assert!( !Resolution::Refused.accepted_incoming() );
ring_overflow/tests/overflow_test.rs:  assert!( !Resolution::Refused.lost_an_item() );
ring_overflow/tests/overflow_test.rs:  assert!( !Resolution::Refused.accepted_incoming() );
ring_overflow/tests/overflow_test.rs:  assert_eq!( would_resolve( OverflowPolicy::Fail ), Resolution::Refused );
ring_overflow/tests/overflow_test.rs:    assert_eq!( would_resolve( one_arrival_at_a_full_ring ), Resolution::Refused );
ring_overflow/tests/overflow_test.rs:  assert_ne!( a, Resolution::Refused );
ring_overflow/src/lib.rs:/// assert!( !Resolution::Refused.lost_an_item() );
ring_overflow/src/lib.rs:  /// assert!( Resolution::ALL.contains( &Resolution::Refused ) );
ring_overflow/src/lib.rs:  /// assert!( !Resolution::Refused.accepted_incoming() );
ring_overflow/src/lib.rs:  // OverflowPolicy::Fail )` — `Resolution::Refused.lost_an_item()` is `false`,
ring_overflow/src/lib.rs:/// assert_eq!( would_resolve( OverflowPolicy::Fail ), Resolution::Refused );
ring_overflow/src/lib.rs:    OverflowPolicy::Fail => Resolution::Refused,
ring_core/src/lib.rs:        Resolution::EvictedOldest | Resolution::Refused => Err( record ),
  -- and the two names the one outcome carries --
  Refused,
  Full,
```

---

### OV15 — `resolve` Cannot Return the Third Variant It Documents

`resolve` returns `Ok( Resolution )` on two policies and `Err( RingError::Full )`
on the third. `Resolution::Refused` therefore never appears in `resolve`'s output
for any input — the variant exists, is exported, is tested, and is unreachable
through the function the crate names as its main entry point.

The whole workspace constructs `Refused` in fourteen places, all fourteen inside
this crate, and exactly one of them is production code: `would_resolve`'s own
`Fail` arm at `src/lib.rs:235`. Every other is a doctest or a test assertion.

**Finding.** So the two halves of the crate disagree about what a refusal is.
`would_resolve` says it is a resolution, one of three, carrying no more weight
than the other two. `resolve` says it is an error, in a different type, in the
other half of a `Result`. Both are defensible in isolation and the crate does not
say which reading it intends.

The consequence is asymmetry in the pair the crate is built around: for two of the
three policies the two functions agree exactly and the suite proves it
(`resolve_agrees_with_would_resolve`), and for the third they return values that
cannot even be compared without unwrapping one of them. The agreement test
necessarily excludes the case where the disagreement lives.

---

### OV16 — The Same Outcome Carries Two Names in Two Type Systems

A publish refused under `OverflowPolicy::Fail` is `Resolution::Refused` at
`ring_overflow/src/lib.rs:75` and `RingError::Full` at
`ring_types/src/error.rs:54`. One event, two spellings, two crates, two enums.

They are not synonyms in the ordinary sense — `RingError::Full` is a family-wide
error a dozen call sites raise, and `Resolution::Refused` is a local outcome —
but for this crate's one purpose they denote the identical situation, and nothing
in either declaration mentions the other.

**Finding.** The duplication is where drift would begin. If `RingError` ever
gained a more specific full-ring error — a `FullAndClosed`, say — `resolve` would
need to choose between it and `Full`, and `would_resolve` would have no
corresponding choice to make, because its side of the mapping has exactly one
variant for the case. The two functions would then diverge in a way
`resolve_agrees_with_would_resolve` cannot detect, since that test already skips
the `Fail` policy.

Recording it because the crate's own summary calls `would_resolve` "the pure half
of `resolve`", which is true for two policies out of three and false for the
third: the pure half returns a resolution, the effectful half returns an error,
and the word "half" implies a correspondence that the third arm does not have.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`decisions/001`](001_the_fourth_variant_that_is_not_there.md) | Why there are three variants and not four |
| [`data_structure/002`](../data_structure/002_a_one_byte_outcome_in_a_twenty_four_byte_result.md) | What the `Result` shape costs |
| [`workaround/002`](../workaround/002_one_outcome_expressed_in_two_type_systems.md) | Living with the two spellings |
| [`algorithm/001`](../algorithm/001_two_mappings_over_three_policies.md) | The two mappings, side by side |

### Sources

| Fact | Where |
|------|-------|
| `resolve`'s `Fail` arm | `ring_overflow/src/lib.rs:204` |
| `would_resolve`'s `Fail` arm | `ring_overflow/src/lib.rs:235` |
| `Resolution::Refused`'s declaration | `ring_overflow/src/lib.rs:75` |
| `RingError::Full`'s declaration | `ring_types/src/error.rs:54` |
| Every construction of `Refused` | Census above |

### Tests

| Test | Covers |
|------|--------|
| `resolve_agrees_with_would_resolve` | The two policies where the halves do agree |
| `fail_hands_the_decision_back_as_an_error` | That the third returns `Err` |
| `a_refusal_loses_nothing` | `Refused`'s own readings, on a literal |
| *(to create)* | Nothing asserts `resolve` never yields `Ok( Refused )` — the exclusion is structural and untested |
