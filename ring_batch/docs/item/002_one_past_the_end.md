# Item: One Past the End

### Scope

**Purpose:** Record `end()` — the crate's only arithmetic, the three methods that
route through it, and the second clause of its doc comment, which is true for one
producer and false for several.

**Responsibility:** `BatchClaim::end`: its body, its consumers, and what it
claims about the cursor.

**In Scope:** `ring_batch/src/lib.rs:108-132`, `:147`, `:164`, `:188`.

**Out of Scope:** The overflow in that addition is
[`pitfall/002`](../pitfall/002_the_addition_with_no_panics_section.md). The
method whose comparisons depend on it is
[`item/001`](001_the_method_whose_reason_was_declined.md).

---

## One Operator, Three Callers

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the pre-BA28-fix doc clause, searched for in the current source (expect 0 -- see BA28 below) --'
printf '    hits: %s\n' "$( command grep -c -F '  /// One past the last sequence owned — the value the cell now holds.' ring_batch/src/lib.rs )"
echo '  -- every internal caller --'
command grep '\.end()' ring_batch/src/lib.rs | command grep -v '///'
echo '  -- every other arithmetic operator in the crate body --'
command grep -E '[a-z_)] [+*/-] |[+*/-]= ' ring_batch/src/lib.rs | command grep -v '///' | command grep -v '//!' || echo '    (none)'
```

Live output:

```
  -- the pre-BA28-fix doc clause, searched for in the current source (expect 0 -- see BA28 below) --
    hits: 0
  -- every internal caller --
    seq.0 >= self.start.0 && seq.0 < self.end().0
    ( self.start.0..self.end().0 ).map( Seq )
      && self.start.0 < other.end().0 && other.start.0 < self.end().0
  -- every other arithmetic operator in the crate body --
    (none)
```

---

### BA28 — "The Value the Cell Now Holds" Is a Single-Producer Statement

The doc's first clause — "one past the last sequence owned" — is a fact about the
claim and always true. The second — "the value the cell now holds" — is a fact
about the cursor, and holds only while nothing else advances it.

Measured, reading the cursor immediately after each `claim` and comparing:

```
--- "the value the cell now holds", checked immediately after each claim ---
   threads     claims         held     moved on        worst lag
         1      20000        20000            0            0 seq
         2      40000        40000            0            0 seq
         4      80000        79164          836        52280 seq
         8     160000       159541          459       596104 seq
        16     320000       318455         1545       378912 seq
```

**Finding.** At one and two threads the clause is exactly true — sixty thousand
claims, not one disagreement. From four threads it starts to fail: 836 of 80,000,
459 of 160,000, 1,545 of 320,000. The rate is low because the load follows the
`fetch_add` by a few instructions, and that is the trap — a reader who tests the
claim at four threads will most likely see it hold.

The lag when it does fail is not small. The worst single observation in this run
had the cell 596,104 sequences beyond the claim's `end()` by the time it was
read: seventy-four thousand foreign batches between one producer's `fetch_add`
and its own next load. A caller that treated `end()` as the cursor's current
value — to decide whether to publish, or how much room remains — would be wrong
by an unbounded amount, rarely, and never in a way a four-thread test would
reliably surface.

The clause is not needed for anything. Removing it leaves the method fully
documented, because what `end()` returns is a property of the claim and has
nothing to do with the cell.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'the cell itself will usually have moved past it' ring_batch/src/lib.rs
```

Live output:

```
  /// under contention the cell itself will usually have moved past it by
```

**Disposition:** applied — `end()`'s doc comment no longer says "the value the
cell now holds"; it states the true, always-correct property (a fact about the
claim alone) and names the contention gap directly instead of asserting
something only true at one or two threads.
Now prints: `the cell itself will usually have moved past it`

---

### BA29 — The Crate's Only Addition, With Three Methods and a Test Assertion Above It

`end()` performs the one arithmetic operation in `ring_batch`. Every other line
in the crate compares, copies, or calls; the census above finds no second
operator anywhere in the body.

Three of the eight `BatchClaim` methods route through it — `contains` for its
upper bound, `sequences` for its range end, `overlaps` for both of its
comparisons — and the contention suite's ordering assertion adds a fourth
consumer from outside:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- every consumer in the test suite --'
command grep '\.end()' ring_batch/tests/batch_test.rs
```

Live output:

```
  -- every consumer in the test suite --
  assert_eq!( batch.end(), Seq( 13 ) );
  assert_eq!( batch.start(), batch.end() );
        pair[ 0 ].end().0 <= pair[ 1 ].start().0,
```

**Finding.** Two of the three test call sites assert `end()`'s own value; the
third uses it as the ordering key under four threads. So the crate's single
addition sits beneath three of its eight methods and beneath the only assertion
protecting the per-thread ordering requirement — seven places in all, every one of
them a range comparison whose correctness begins with that `+`.

Concentration like this is usually a virtue — one place to get the arithmetic
right, and `Seq( start + count )` is right. It becomes a liability only because
the operation is a bare `+` on a `u64` with no `# Panics` section and no
saturating or checked variant, so every range operation the type offers inherits
whatever that addition does at the boundary. What it does is
[`pitfall/002`](../pitfall/002_the_addition_with_no_panics_section.md).

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`item/001`](001_the_method_whose_reason_was_declined.md) | The method with two of the four consumers |
| [`pitfall/002`](../pitfall/002_the_addition_with_no_panics_section.md) | What the `+` does at `u64::MAX` |
| [`invariant/002`](../invariant/002_ascending_not_contiguous.md) | The ordering assertion built on `end()` |
| [`api/001`](../api/001_twelve_items_seven_must_use.md) | The `#[ must_use ]` census this method sits in |

### Sources

| Fact | Where |
|------|-------|
| The doc and the body | `ring_batch/src/lib.rs:108-132` |
| The three internal callers | `ring_batch/src/lib.rs:147`, `:164`, `:188` |
| No other arithmetic in the crate | Census above |
| Cursor agreement by thread count | Release probe, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `a_claim_reports_its_own_extent` | That `end()` is `start + len` |
| `containment_includes_the_start_and_excludes_the_end` | The half-open convention `end()` defines |
| `a_threads_own_batches_stay_in_its_issue_order` | `end()` as an ordering key, under four threads |
| *(to create)* | That `end()` is not the cursor's value under contention — the clause the doc asserts and nothing checks |
