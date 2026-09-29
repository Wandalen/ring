# Algorithm: Classifying an Error Into Configuration or Traffic

### Scope

- **Purpose**: Specify the two membership sets behind [`is_configuration`](../item/associated_function/004_ring_error_is_configuration.md) and [`is_transient`](../item/associated_function/005_ring_error_is_transient.md), and record that the two together leave three of nine variants unclassified.
- **Responsibility**: State the abstract and the algorithm.
- **In Scope**: Both membership sets; the third bucket neither predicate names; the responses each class implies.
- **Out of Scope**: What each variant means (→ [`type/002`](../type/002_ring_error.md)); the two variants nothing constructs (→ [`pitfall/002`](../pitfall/002_two_name_errors_nothing_constructs.md)).

### Abstract

**Two predicates over one nine-variant enum, answering two different questions
that a caller confuses at its peril.** `is_configuration` asks whether the
failure is a mistake in how the ring was set up; `is_transient` asks whether
retrying the identical call could succeed with nothing else changing.

The pair exists because the two classes call for opposite responses — a
configuration error is a bug to fix and a traffic error is a state to handle —
and because neither is derivable from the other. **They are not complements.**
Their union covers six of nine variants; the remaining three are neither, and
nothing on the type says so.

### Algorithm

**Both predicates are a single `const fn` over an exhaustive `match`**, so the
algorithm is the membership list and nothing else:

```sh
cd "$(git rev-parse --show-toplevel)"
awk '/pub const fn is_configuration\( self \) -> bool/,/^  }$/; /pub const fn is_transient\( self \) -> bool/,/^  }$/' ring_types/src/error.rs
```

Live output:

```
  pub const fn is_configuration( self ) -> bool
  {
    match self
    {
      Self::CapacityZero
      | Self::CapacityNotPowerOfTwo( _ )
      | Self::BatchTooLarge { .. }
      | Self::PolicyUnsupported => true,
      Self::Full
      | Self::Empty
      | Self::Closed
      | Self::NameTaken
      | Self::NameUnknown => false,
    }
  }
  pub const fn is_transient( self ) -> bool
  {
    match self
    {
      Self::Full | Self::Empty => true,
      Self::CapacityZero
      | Self::CapacityNotPowerOfTwo( _ )
      | Self::Closed
      | Self::NameTaken
      | Self::NameUnknown
      | Self::BatchTooLarge { .. }
      | Self::PolicyUnsupported => false,
    }
  }
```

**Correction (2026-09-28):** the paragraph and the quoted output above
previously showed both predicates as a `matches!` set — a positive list
answering `true` for the named variants and falling through to `false` for
everything else, current or future. `Fix(ring_error_classification_not_exhaustive)`
(`ring_types/src/error.rs:98`) replaced both bodies with an exhaustive `match`
naming every variant on both arms, so a tenth `RingError` variant now fails to
compile here instead of silently joining the `false` side. The membership
itself — which variants land in which class — is unchanged; only the
mechanism that would catch a future variant changed, from silent to a compile
error.

**The full classification, variant by variant:**

| Variant | `is_configuration` | `is_transient` | Class | Caller's correct response |
|---------|:------------------:|:--------------:|-------|---------------------------|
| `CapacityZero` | ✅ | ❌ | Configuration | Fix the config; retrying is pointless |
| `CapacityNotPowerOfTwo` | ✅ | ❌ | Configuration | Same |
| `BatchTooLarge` | ✅ | ❌ | Configuration | Same — no amount of draining helps |
| `PolicyUnsupported` | ✅ | ❌ | Configuration | Same |
| `Full` | ❌ | ✅ | Traffic | Retry, back off, or drop per policy |
| `Empty` | ❌ | ✅ | Traffic | Retry or return nothing |
| `Closed` | ❌ | ❌ | **Neither** | Stop. Retrying never succeeds, and nothing is misconfigured |
| `NameTaken` | ❌ | ❌ | **Neither** | Pick another name |
| `NameUnknown` | ❌ | ❌ | **Neither** | The name was never registered |

**The three-variant gap is the finding.** `Closed`, `NameTaken` and
`NameUnknown` answer `false` to both predicates, and a caller writing

```rust
if e.is_configuration() { report_bug( e ) } else { retry_later( e ) }
```

sends all three down the retry branch. For `Closed` that is an infinite loop
against a ring that will never reopen — the worst of the three, and the only one
of them a real caller reaches, since the other two are constructed nowhere.

**`Closed` is genuinely neither, and that is correct.** It is not a
configuration mistake — the ring was fine and someone shut it down — and it is
not transient, because no peer's progress reverses a close. It is *terminal*, a
third class the type does not name. The classifier is not wrong; it is
incomplete, and its incompleteness is silent because `false` is a valid answer
to both questions.

**`is_transient`'s doc comment states the membership rule and it is exact.** It
says "true for exactly the two conditions a peer's progress clears" —
`Full` clears when a consumer drains, `Empty` clears when a producer publishes.
Nothing else in the enum has a peer whose ordinary operation resolves it.

**`is_configuration`'s membership is less obvious in one row.**
`BatchTooLarge { requested, capacity }` arises at runtime, from a call — which
makes it look like traffic. It is classified as configuration because the
condition is `requested > capacity`, and capacity is fixed at construction: the
request can never be served by this ring no matter what the peers do. **The test
that separates the two classes is not "when did it happen" but "does waiting
help".** By that test `BatchTooLarge` is configuration and the row is right.

**Both predicates are `const` and take `self` by value**, which is available
because `RingError` is `Copy`. The consequence is that a caller may classify in
a `const` context and, more usefully, may classify without borrowing — so an
error can be classified after being moved into a log line.

**Call sites, which are lopsided:**

```sh
cd "$(git rev-parse --show-toplevel)"
for m in is_configuration is_transient; do
  s=$( command grep -rn "\.$m(" ring_*/src 2>/dev/null | command grep -vc '^ring_types/' )
  t=$( command grep -rn "\.$m(" ring_*/tests 2>/dev/null | command grep -vc '^ring_types/' )
  printf '%-18s src=%-3s tests=%s\n' "$m" "$s" "$t"
done
```

Live output:

```
is_configuration   src=2   tests=11
is_transient       src=1   tests=6
```

**Three production call sites across a 33-crate family, against seventeen in
tests.** The predicates are exercised far more than they are used, which is the
signature of a classifier written for consumers who have not arrived — an
external caller behind the export Contract is the intended audience, and the
family's own crates mostly know which error they just produced.

### Algorithms

| File | Relationship |
|------|--------------|
| [001_validating_a_slot_count_to_a_power_of_two.md](001_validating_a_slot_count_to_a_power_of_two.md) | Produces the two variants that head the configuration class |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_five_classifier_predicates.md](../api/002_the_five_classifier_predicates.md) | These two among the crate's five predicates, and the two with no production caller at all |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_error_enum_as_a_closed_copy_set.md](../data_structure/002_the_error_enum_as_a_closed_copy_set.md) | The nine variants, their payloads, and the 24 bytes `BatchTooLarge` sets |

### Items

| File | Relationship |
|------|--------------|
| [../item/associated_function/004_ring_error_is_configuration.md](../item/associated_function/004_ring_error_is_configuration.md) | The four-variant set |
| [../item/associated_function/005_ring_error_is_transient.md](../item/associated_function/005_ring_error_is_transient.md) | The two-variant set |
| [../item/enum/002_ring_error.md](../item/enum/002_ring_error.md) | The enum both predicates partition |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_an_error_from_construction_to_display.md](../lifecycle/002_an_error_from_construction_to_display.md) | Where in an error's life classification happens, and what it is competing with |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_two_name_errors_nothing_constructs.md](../pitfall/002_two_name_errors_nothing_constructs.md) | Two of the three unclassified variants, and why they are unconstructed |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_ring_error.md](../type/002_ring_error.md) | The variant set and its validation rules |

### Sources

| File | Relationship |
|------|--------------|
| [`src/error.rs`](../../src/error.rs) | Lines 111–125 and 146–159, the two predicates; 88–90, the rationale for having both |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/types_test.rs`](../../tests/types_test.rs) | ✅ `errors_split_configuration_from_traffic` and `only_full_and_empty_are_transient` assert both memberships exhaustively. ⚠️ **Neither asserts the three-variant gap as a gap** — they assert `Closed` is not transient and not configuration, which is the same fact stated twice without naming the consequence. A test asserting that exactly three variants answer `false` to both would fail if a fourth were added, which is the change that makes the silent-retry failure worse |

### TY21 — Neither Classifier Has a Production Caller Outside This Crate

The classification this algorithm specifies is complete, tested, and — outside
`ring_types` — unexecuted.

```sh
cd "$(git rev-parse --show-toplevel)"
for m in is_configuration is_transient; do
  printf '%-18s all %2d   code %2d\n' "$m" \
    "$( grep -rn "\.$m()" ring_*/src/*.rs | grep -vc '^ring_types/' )" \
    "$( grep -rn "\.$m()" ring_*/src/*.rs | grep -v '^ring_types/' \
        | grep -vcE ':[0-9]+: *(//|///|//!)' )"
done
```

Live output:

```
is_configuration   all  2   code  0
is_transient       all  1   code  0
```

Both crates that *look* like adopters are demonstrating the method in a doc
comment. Strip comments and the count is zero for each.

### TY22 — Three of the Nine Variants Belong to Neither Class

The two predicates partition nine variants 4/2/3, and the third group has no
name:

| Class | Variants | Predicate |
|-------|----------|-----------|
| Configuration | `CapacityZero`, `CapacityNotPowerOfTwo`, `BatchTooLarge`, `PolicyUnsupported` | `is_configuration` |
| Transient | `Full`, `Empty` | `is_transient` |
| **Neither** | `Closed`, `NameTaken`, `NameUnknown` | **none** |

`is_configuration` returning `false` therefore means *either* "retry may work"
*or* "this is terminal and retrying will never work", and a caller branching on
it alone cannot tell `Full` from `Closed` — the exact distinction a producer
needs on a shutdown path. `ring_shutdown` is the crate that needs it, and it
matches its own `Refusal` type rather than asking either predicate.

The gap is a consequence of the classification being *total* over the enum while
the vocabulary is not: two predicates can name at most three classes if one is
the complement of the other, and here the complement is a union of two unlike
things.
