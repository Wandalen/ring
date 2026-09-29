# Invariant: No Resolution Overwrites Unread Data Silently

### Scope

**Purpose:** Record the crate's safety invariant — nothing may keep the incoming
item and destroy unread data without reporting the loss — and the shape of the
test that enforces it.

**Responsibility:** The implication the test asserts, the single arm that reaches
its antecedent, and what happens to the assertion if that arm stops reaching it.

**In Scope:** `ring_overflow/tests/overflow_test.rs:89-124`;
`ring_overflow/src/lib.rs:142`, `:234`.

**Out of Scope:** The variant-count guard is
[`invariant/001`](001_the_mapping_is_total_and_injective.md) § OV22. The variant no
production build reaches is
[`pitfall/001`](../pitfall/001_the_variant_a_default_build_cannot_reach.md).

---

## The Criterion, Written and Executed

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the acceptance criterion, as written --'
command grep -m1 -A2 -F '/// No resolution both keeps the incoming item and destroys unread data without' ring_overflow/tests/overflow_test.rs
echo '  -- and as executed --'
command grep -m1 -A15 -F 'fn no_resolution_overwrites_unread_data_silently()' ring_overflow/tests/overflow_test.rs | tail -n 14
echo '  -- the guard that the antecedent was ever satisfied --'
command grep -m1 -B2 -A4 -F '    checked, 1,' ring_overflow/tests/overflow_test.rs
echo '  -- the antecedent that gates the assertion --'
command grep -m1 -F '      Self::EvictedOldest => true,' ring_overflow/src/lib.rs
echo '  -- and the single arm that can satisfy it --'
command grep 'DropOldest => Resolution::EvictedOldest' ring_overflow/src/lib.rs
```

Live output:

```
  -- the acceptance criterion, as written --
/// No resolution both keeps the incoming item and destroys unread data without
/// saying so — the invariant an overwrite variant would break. Evicting is
/// permitted, but only because it is *reported* as a loss.
  -- and as executed --
  let mut checked = 0;
  for policy in OverflowPolicy::ALL
  {
    let resolution = would_resolve( policy );
    if resolution.accepted_incoming()
    {
      checked += 1;
      assert!
      (
        resolution.lost_an_item(),
        "{resolution:?} accepted an item into a full ring without reporting a loss"
      );
    }
  }
  -- the guard that the antecedent was ever satisfied --
  assert_eq!
  (
    checked, 1,
    "no policy in OverflowPolicy::ALL produces an accepting resolution — \
     this test proved nothing"
  );
}
  -- the antecedent that gates the assertion --
      Self::EvictedOldest => true,
  -- and the single arm that can satisfy it --
    OverflowPolicy::DropOldest => Resolution::EvictedOldest,
```

---

### OV23 — The Invariant Permits the Dangerous Outcome and Requires Only That It Be Reported

The criterion is not "nothing destroys unread data." Eviction is allowed. What is
forbidden is destroying unread data *without saying so* — the test's own comment
puts the emphasis on the word *reported*.

Formally: `accepted_incoming() ⟹ lost_an_item()`. An outcome that takes the
incoming item into a full ring must also be one that reports a loss, because
something had to leave to make room. `EvictedOldest` satisfies both. `Refused`
satisfies neither. `DroppedIncoming` satisfies only the consequent, which the
implication permits.

**Finding.** This is the right invariant and the honest one — it names what the
crate actually guarantees rather than a stronger claim it cannot keep. A ring with
`DropOldest` configured does lose unread data, and the design position is that
losing it visibly is acceptable while losing it silently is not.

What is missing is that this position appears only in a test's doc comment. The
crate's module documentation, `Resolution`'s declaration, and both predicates'
doc comments state what each value means; none of them states the rule that
relates them, which is the crate's single safety property and is enforced
nowhere else.

---

### OV24 — The Assertion Is Reached by One Arm and Would Pass Vacuously Without It

The test is an implication guarded by an `if`. The assertion executes only for
resolutions where `accepted_incoming()` is true, and `accepted_incoming()` is
`true` for `Self::EvictedOldest` alone — one variant, produced by one arm of
`would_resolve`, reached by one policy.

If that arm ever changed, or if `accepted_incoming` were narrowed, the loop would
run three times, enter the body zero times, assert nothing, and report success.

**Finding.** The suite had no guard that the antecedent is ever satisfied. Nothing
asserted that at least one policy reaches the assertion, so the difference between
"the invariant holds" and "the invariant was never tested" was invisible in the
test output — both were a green `no_resolution_overwrites_unread_data_silently`.

This mattered more here than it usually would, because the one variant holding the
antecedent up is also the one no production build can reach: `DropOldest` is
rejected at construction by `Ring::new`, and the `crossbeam` path returns before
resolving
([`pitfall/001`](../pitfall/001_the_variant_a_default_build_cannot_reach.md)). So
the crate's central safety invariant was carried, in the suite, entirely by a
configuration that no shipping build accepts — and a future change removing
`DropOldest` from `OverflowPolicy::ALL` would have turned the test vacuous without
turning it red.

The loop now counts its own satisfactions and asserts the count afterwards, so an
implication with no witness fails instead of passing. The census above shows both
halves.

**Disposition:** applied — `no_resolution_overwrites_unread_data_silently` in
`ring_overflow/tests/overflow_test.rs` now increments `checked` inside the
guarded branch and asserts `checked == 1` after the loop, with the reasoning
recorded above the assertion. The exact future edit this finding names was run to
prove the guard bites rather than assuming it: temporarily narrowing
`OverflowPolicy::ALL` to `[ DropNewest, Fail ]` — the change a reader could
justify on the grounds that no default backend supports `DropOldest` — turns the
test red with "no policy in OverflowPolicy::ALL produces an accepting resolution —
this test proved nothing", where before it stayed green. `OverflowPolicy::ALL` was
restored and the full 73-test suite passes. Now prints: `    checked, 1,`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`invariant/001`](001_the_mapping_is_total_and_injective.md) | The structural properties beneath this one |
| [`pitfall/001`](../pitfall/001_the_variant_a_default_build_cannot_reach.md) | Why the sole witness is unreachable in production |
| [`decisions/001`](../decisions/001_the_fourth_variant_that_is_not_there.md) | What "accepted" is scoped to mean |
| [`type/002`](../type/002_three_variants_and_two_questions.md) | The partition the implication is stated over |

### Sources

| Fact | Where |
|------|-------|
| The criterion's own statement | `ring_overflow/tests/overflow_test.rs:89-91` |
| The guarded assertion and its witness count | Census above |
| `accepted_incoming`'s single true arm | Census above |
| The one arm producing that variant | Census above |
| `DropOldest` rejected at construction | `ring_core/src/lib.rs:165`, `:348` |

### Tests

| Test | Covers |
|------|--------|
| `no_resolution_overwrites_unread_data_silently` | The implication, over the published policy set, and that the antecedent is satisfied at least once |
| `the_two_readings_partition_the_outcomes` | Both predicates on all three variants, on literals |
| `a_refusal_loses_nothing` | That `Refused` satisfies neither side |
