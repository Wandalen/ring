# Algorithm: The Third Ordering

### Scope

- **Purpose**: Specify which cursor is read first, why that is a third ordering decision distinct from the two [`algorithm/001`](001_checking_a_pair_without_touching_it.md) documents, and what it selects.
- **Responsibility**: The temporal order of the two loads, the skew it admits, and which of the three defects that skew can manufacture.
- **In Scope**: The producer-then-consumer sequence at all three read sites; the false positives available to each order.
- **Out of Scope**: The memory ordering (→ [`algorithm/001`](001_checking_a_pair_without_touching_it.md)'s `Acquire` table); the quiescence precondition as a caller obligation (→ [`api/001`](../api/001_the_check_surface.md)'s B1).

### Abstract

`check` performs two loads. They cannot happen at the same instant, so one of them
is first, and on a running ring the second one sees a later world than the first.
Which cursor gets the earlier read is a decision — and it was, when this document
was written, the one ordering decision in the crate that no document stated and no
comment marked.

The crate used to make it three times, identically, and say so nowhere. It now
makes it once, in a function whose rustdoc names the false positive the choice
selects — which is the whole of what DB23 asked for.

### Algorithm

```
read_pair( pair ):            # every read site in the crate, in source order
  p <- pair.producer().load( OBSERVE )     # first
  c <- pair.consumer().load( OBSERVE )     # second, at a strictly later moment
  return ( p, c )
```

The three read sites used to spell that sequence three times. At two of them it
was a pair of `let` bindings; at the third, inside `check`, the two loads were
*arguments to a call*, so the ordering was Rust's left-to-right argument
evaluation rather than statement sequence — same result, one degree less visible,
and the degree that mattered. DB23 below is what became of that; the three sites
now call one `observe_pair` and the pseudocode above is the whole of it.

Three orderings meet in this function and only two of them are documented:

| # | Ordering | Decides | Documented in |
|---|---|---|---|
| 1 | Memory (`Acquire`) | Whether the loads can be reordered against the publishing stores | [`algorithm/001`](001_checking_a_pair_without_touching_it.md) |
| 2 | Check (D1 before D2, D3 before both) | Which violation is reported when a pair breaks several | [`algorithm/001`](001_checking_a_pair_without_touching_it.md), [`api/001`](../api/001_the_check_surface.md)'s A4/A5 |
| 3 | **Load (producer before consumer)** | Which false positive skew can manufacture | Here, and `observe_pair`'s rustdoc |

#### What the order selects

Both cursors on a live ring only ever advance, and the consumer never truly passes
the producer. Read them at two different moments and one of those two facts stops
protecting the comparison.

| Read order | Pair the check sees | Can manufacture |
|---|---|---|
| Producer, then consumer (current) | An **early** producer against a **late** consumer | D1 — the consumer appears to have overtaken |
| Consumer, then producer | A **late** producer against an **early** consumer | D2 — the gap appears wider than a lap |

Neither order is skew-free; the choice is only ever *which* false positive is
available. D3 is immune to both, because a backwards report needs a reading lower
than the baseline and skew can only ever inflate the later of the two.

**The current order makes D1 the reachable false positive, and D1 is the violation
this crate treats as most important.** [`api/001`](../api/001_the_check_surface.md)'s
A5 reports D1 in preference to D2 precisely because D1 is the one with no other
symptom; [`pitfall/001`](../pitfall/001_saturating_arithmetic_reports_health.md) is
the measurement of why. A spurious D2 would be checked against `pending` and
dismissed in a minute. A spurious D1 says the ring is silently losing records.

### Complexity

| Operation | Skew window | Widened by |
|---|---|---|
| `check` | Between the two loads — a handful of instructions | Nothing; there is no work between them |
| `Watch::observe` | The same, plus the interval since the baseline | The caller's polling interval, which is unbounded |
| `check_ends` | Between two *derived* reads on two different objects | The same, and the readings are already lossy ([`workaround/001`](../workaround/001_the_door_ring_core_does_not_open.md)'s W1) |

The window in `check` is as narrow as it can be made without a snapshot
primitive — the two loads are adjacent statements with nothing between them, which
is the cheapest available mitigation and the only one applied.

### Failure

| # | Failure | Consequence |
|---|---|---|
| N5 | A statement inserted between the two loads | The skew window widens for no stated benefit, and nothing marks the lines as adjacent by requirement |
| N6 | The two loads swapped | The reachable false positive silently changes from D1 to D2; every test still passes |
| N7 | The order read as arbitrary and "tidied" for symmetry with `Violation`'s field order | Same as N6, arrived at by a change that looks like formatting |

**N6 and N7 are the same edit reached two ways, and no test in the crate would
report either.** The suite is single-threaded, so nothing can observe skew at all.
What stands against them now is one rustdoc paragraph saying which false positive
the order picks — which is a reader stopping, not a build failing, and worth
naming as the weaker of the two things a finding can produce.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
S=ring_debug/src/lib.rs
T=ring_debug/tests/debug_test.rs
echo '-- every load site, in the order the source performs them --'
command grep 'load( OBSERVE )' $S | sed 's/^ */  /'
echo '-- how many places make that ordering decision, and how many merely spend it --'
printf '  sites performing the two loads: %s\n' "$( command grep -c 'load( OBSERVE )' $S )"
printf '  sites reading a pair through observe_pair: %s\n' \
  "$( command grep -c '= observe_pair( pair )' $S )"
echo '-- is any of them marked, commented, or otherwise held in place? --'
printf '  comments mentioning read order: %s\n' \
  "$( command grep -ci 'read.*first\|order of the.*load\|load.*order' $S || true )"
echo '-- can any test observe skew at all? --'
printf '  thread::spawn or thread::scope in the suite: %s\n' "$( command grep -c 'spawn\|thread::scope' $T || true )"
echo '-- what each order can manufacture on a ring that was valid throughout --'
python3 - << 'EOF'
cap = 8
p0, c0 = 10, 8      # t0: valid, gap 2
p1, c1 = 20, 15     # t1: valid, gap 5 — both cursors advanced, consumer never passed
print( f"  true ring: t0 ( producer {p0}, consumer {c0} ), t1 ( producer {p1}, consumer {c1} ), capacity {cap} — valid at both" )
print( f"  producer-first sees ( {p0}, {c1} ): D1 fires {c1 > p0}" )
print( f"  consumer-first sees ( {p1}, {c0} ): D1 fires {c0 > p1}, D2 fires {p1 - c0 > cap} ( gap {p1 - c0} )" )
EOF
```

Live output:

```
-- every load site, in the order the source performs them --
  let producer = pair.producer().load( OBSERVE );
  let consumer = pair.consumer().load( OBSERVE );
-- how many places make that ordering decision, and how many merely spend it --
  sites performing the two loads: 2
  sites reading a pair through observe_pair: 3
-- is any of them marked, commented, or otherwise held in place? --
  comments mentioning read order: 1
-- can any test observe skew at all? --
  thread::spawn or thread::scope in the suite: 0
-- what each order can manufacture on a ring that was valid throughout --
  true ring: t0 ( producer 10, consumer 8 ), t1 ( producer 20, consumer 15 ), capacity 8 — valid at both
  producer-first sees ( 10, 15 ): D1 fires True
  consumer-first sees ( 20, 8 ): D1 fires False, D2 fires True ( gap 12 )
```

### Algorithms

| File | Relationship |
|------|--------------|
| [001_checking_a_pair_without_touching_it.md](001_checking_a_pair_without_touching_it.md) | The other two orderings, and the comparison these reads feed |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_check_surface.md](../api/001_the_check_surface.md) | B1 — the quiescence precondition this is the mechanism behind; A5 — the priority the order argues against |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_cursor_invariants_over_a_live_ring.md](../invariant/001_cursor_invariants_over_a_live_ring.md) | D1, D2, D3 — which of the three skew can fabricate |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | The three load pairs |

### Tests

| Test | Relationship |
|------|--------------|
| `checking_leaves_both_cursors_where_they_were` | The closest the suite comes to a repeated read — 100 checks, all quiescent |
| `the_two_ends_of_a_live_ring_agree` | The one test whose name claims a live ring, and which is also single-threaded |

### DB23 — the one ordering decision nothing records is the one that picks which false positive is reachable

The crate documents its memory ordering at length and its check order twice. Its
load order — producer first, at all three read sites — appears in no document, no
comment, and no test.

That ordering is not neutral. Reading the producer early and the consumer late
means the pair handed to `check_seqs` is an old producer against a fresh consumer,
which is exactly the shape that fabricates **D1**. The other order fabricates D2
instead. Since the cursors are monotonic, one of the two is always the reachable
one and there is no third option short of a snapshot primitive.

**So the crate has, silently, chosen to make its most alarming false positive the
available one** — the violation
[`api/001`](../api/001_the_check_surface.md)'s A5 prioritises *because* it has no
other symptom, and therefore the one a reader has no second source to check it
against. A spurious D2 is refuted by glancing at `pending`. A spurious D1 says
records are being lost silently, and the crate's own documents explain at length
why nothing else would show it.

**The exposure was never the part worth fixing.** The window is a couple of
instructions wide, `check` is documented as needing a quiescent ring, and
swapping the loads only moves which false positive is reachable — there is no
order that has none. What was actually wrong is the last sentence of the previous
paragraph: the decision was made three times without being made once, so no
reader could tell a choice from an accident, and a fourth read site would have
made it a fourth time by copying whichever neighbour it was pasted from.

**Disposition:** applied — the three open-coded producer-then-consumer load pairs
in `check_seqs`, `Watch::observe` and `Watch::new` now call one private
`observe_pair`, whose rustdoc states producer-first as a choice and names D1 as
the false positive that choice makes the reachable one; the crate performs the
two loads in one place instead of three, and the next read site inherits the
ruling instead of re-deciding it. Now prints:
`sites reading a pair through observe_pair: 3`

### DB24 — the concurrency the crate is built for is the one condition its suite never creates

Every failure mode this crate guards against is a concurrency artefact, and every
mitigation it applies is a concurrency mitigation: `Acquire` rather than `Relaxed`,
loads kept adjacent, a precondition that the ring be quiescent. The suite contains
no threads.

All twenty-two tests drive cursors by storing into them from the test thread, so
`OBSERVE` could be `Relaxed` and every one would pass —
[`api/001`](../api/001_the_check_surface.md)'s A2 is argued, never asserted. The
same is true of B1: no test violates the quiescence precondition, so the skew
window has never been observed, and the false positive DB23 describes has never
been produced by anything but reasoning.

This is a reasonable place to end up. A test that reliably reproduces a
two-instruction skew window is a hard test to write and a worse test to own, and
the family's `loom` configuration exists for questions of this shape. **What is
worth recording is the gap between how much of this crate's design is decided by
concurrency and how much of its evidence comes from anywhere near it** — the
documents reason about racing threads on every page, and the executable part of
the crate has never seen two.
