# Item: The Method Whose Reason Was Declined

### Scope

**Purpose:** Record `overlaps` — the one public method whose doc states why it
exists, the test that reason names, and the fact that the test declined it and
said so.

**Responsibility:** `BatchClaim::overlaps`: its stated purpose, its call sites,
and its `is_empty` guard.

**In Scope:** `BatchClaim::overlaps` and its doc comment in
`ring_batch/src/lib.rs`; `overlap_is_exactly_range_intersection` and the
opening comment of `concurrent_batch_claims_never_overlap` in
`ring_batch/tests/batch_test.rs`.

**Out of Scope:** The `end()` every one of these comparisons routes through is
[`item/002`](002_one_past_the_end.md). The disjointness invariant the method was
meant to witness is [`invariant/001`](../invariant/001_disjointness_is_free.md).

---

## A Method With a Reason Attached

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the stated reason --'
command grep -m1 -A4 -F '  /// Whether this claim and `other` share any sequence.' ring_batch/src/lib.rs
echo '  -- the body --'
command grep -m1 -A7 -F '  /// assert!( a.overlaps( &BatchClaim::new( Seq( 3 ), 4 ) ) );' ring_batch/src/lib.rs | tail -n 6
echo '  -- the test that reason names, and what its own comment claims --'
awk '/^fn concurrent_batch_claims_never_overlap\(\)$/{ f = 1 } f && /^  \/\//{ print } f && /^  const /{ exit }' ring_batch/tests/batch_test.rs
```

Live output:

```
  -- the stated reason --
  /// Whether this claim and `other` share any sequence.
  ///
  /// Two overlapping claims mean two producers writing one slot. The
  /// whole-run contention test does not call this — it uses a `HashSet`
  /// over individual sequences instead, a stronger check that also catches
  -- the body --
  #[ must_use ]
  pub const fn overlaps( &self, other : &Self ) -> bool
  {
    !self.is_empty() && !other.is_empty()
      && self.start.0 < other.end().0 && other.start.0 < self.end().0
  }
  -- the test that reason names, and what its own comment claims --
  // Disjointness of returned sequences — real, and free of charge from
  // fetch_add's own atomicity, but blind to this crate's actual race: a
  // claim can be disjoint from every other claim and still point at a slot
  // the consumer has not released yet (-> BA23, pitfall/001). Every claim
  // taken by every thread is collected and checked pairwise-disjoint by
  // sequence, which is stronger than checking the claims' ranges: it
  // catches an off-by-one at either end.
```

The same sentence opens both. The second one goes on to explain why it is not
using the method the first one was written for.

---

### BA26 — The Whole-Run Test Exists, Names the Same Property, and Chose Not to Use It

`overlaps` is documented as "Exposed so a test can assert it over a whole run
rather than trusting the protocol." That test exists 150 lines away in the
integration suite. It collects every claim from every thread and checks them
pairwise-disjoint — and it does so with a `HashSet` over individual sequences,
because, in its own words, that "is stronger than checking the claims' ranges: it
catches an off-by-one at either end."

**Finding.** The method's stated reason for existing was considered and rejected
by the test it names, on a technical ground the test states explicitly. Both
comments are correct in isolation. Read together they say: this method exists for
a job, and the job was done better without it.

Neither document knows about the other. The doc comment was written when
`overlaps` was added and has not been revisited; the test comment was written
when the whole-run check was designed and does not mention that a purpose-built
method was available and passed over. The reader who follows the doc comment to
the test finds the method absent and no explanation of why.

The shared opening sentence is a second problem in its own right — see
[`invariant/001`](../invariant/001_disjointness_is_free.md) BA23. "The property a
claim protocol must never violate" is true of protocols generally and false of
this one, which cannot violate it: the sequences come from `fetch_add` and are
disjoint whatever the gate does.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'whole-run contention test does not call this' ring_batch/src/lib.rs
```

Live output:

```
  /// whole-run contention test does not call this — it uses a `HashSet`
```

**Disposition:** applied — `overlaps`'s own doc comment no longer claims it was
"Exposed so a test can assert it over a whole run"; it now states directly that
the whole-run test does not call it and names the stronger check that test uses
instead, closing the gap the reader who followed the doc comment would have hit.
The shared overclaiming opening sentence itself was already corrected on the
test side under BA23.
Now prints: `whole-run contention test does not call this`

---

### BA27 — The Most-Exercised Method in the Crate, Called Nowhere Outside It

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- call sites per method, in the test suite --'
for m in new start len is_empty end contains sequences overlaps
do
  printf '    %-10s %s\n' "$m" "$( command grep -c "\.$m(" ring_batch/tests/batch_test.rs || true )"
done
echo '  -- and outside the crate --'
echo "    ring_tls files calling .overlaps( : $( command grep -rl '\.overlaps(' --include=*.rs ring_tls/ 2>/dev/null | wc -l )"
echo "    the two tests that hold all nine   :"
command grep 'fn overlap_is_exactly_range_intersection\|fn an_empty_claim_overlaps_nothing_even_inside_another' ring_batch/tests/batch_test.rs | sed 's/^/      /'
```

Live output:

```
  -- call sites per method, in the test suite --
    new        0
    start      5
    len        8
    is_empty   3
    end        3
    contains   7
    sequences  3
    overlaps   9
  -- and outside the crate --
    ring_tls files calling .overlaps( : 0
    the two tests that hold all nine   :
      fn overlap_is_exactly_range_intersection()
      fn an_empty_claim_overlaps_nothing_even_inside_another()
```

**Finding.** Nine call sites, more than any other method, and all nine are inside
the two tests of `overlaps` itself. Zero elsewhere in the suite, zero in
`ring_tls`, zero anywhere in the family. A method exhaustively tested and never
used — the tests cover abutting below, abutting above, one sequence shared at
each end, self-overlap, full containment, and three empty-range cases.

That coverage is not wasted. The `!self.is_empty() && !other.is_empty()` guard is
load-bearing and non-obvious: without it, an empty claim at sequence 5 compared
against `[0, 16)` satisfies both `5 < 16` and `0 < 5`, and reports an overlap it
does not have. `contains`, three lines above, needs no such guard — a point
compared against a half-open range that has collapsed to nothing is simply
outside it. The same edge case, handled two different ways in adjacent methods,
both correct, and only one of the two has a comment saying why.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`item/002`](002_one_past_the_end.md) | The `end()` all three comparisons route through |
| [`invariant/001`](../invariant/001_disjointness_is_free.md) | The property both comments open with, and why this protocol cannot violate it |
| [`api/001`](../api/001_twelve_items_seven_must_use.md) | The `#[ must_use ]` this method carries and `claim` does not |
| [`lifecycle/002`](../lifecycle/002_the_empty_claim_as_a_first_class_state.md) | The empty claim the guard exists for |

### Sources

| Fact | Where |
|------|-------|
| The stated reason | `ring_batch/src/lib.rs:167-171` |
| The body and its guard | `ring_batch/src/lib.rs:184-189` |
| The test that declined it | `ring_batch/tests/batch_test.rs:318-321` |
| Nine call sites, all in two tests | Census above |

### Tests

| Test | Covers |
|------|--------|
| `overlap_is_exactly_range_intersection` | Both abutting cases, both one-shared cases, self, containment |
| `an_empty_claim_overlaps_nothing_even_inside_another` | The three empty-range cases the guard exists for |
| `concurrent_batch_claims_never_overlap` | The whole-run property, without calling this method |
