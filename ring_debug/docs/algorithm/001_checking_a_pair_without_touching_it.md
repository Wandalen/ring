# Algorithm: Checking a Pair Without Touching It

### Scope

- **Purpose**: Specify how D1 and D2 are evaluated against a live ring without perturbing what they measure or reporting violations that never occurred.
- **Responsibility**: The read ordering, the comparison, the check order, and the one subtraction that must not saturate.
- **In Scope**: `check`, `check_seqs`, the `Acquire` choice, the D1-before-D2 order.
- **Out of Scope**: The stateful half (→ [`lifecycle/001`](../lifecycle/001_from_one_observation_to_a_sequence.md)); why the family's own arithmetic cannot do this (→ [`pitfall/001`](../pitfall/001_saturating_arithmetic_reports_health.md)).

### Abstract

Two cursor invariants checked against a ring that may be running, using nothing
but loads. Four reads, two comparisons, one plain subtraction — and the plainness
is load-bearing, since the saturating arithmetic the ring itself uses is exactly
what hides the fault this crate is looking for.

The procedure never writes, never allocates, and never takes a lock, so a caller
can run it against a live pair without changing the behaviour they were trying to
observe.

### Algorithm

```
check( pair ):
  p  <- pair.producer().load( Acquire )
  c  <- pair.consumer().load( Acquire )
  k  <- pair.capacity()
  return check_seqs( p, c, k )

check_seqs( p, c, k ):
  if c > p:            return ConsumerAheadOfProducer { p, c }
  if p - c > k:        return ProducerLappedConsumer  { p, c, k }
  return Ok
```

Four reads, two comparisons, one subtraction. Nothing else, deliberately: this
runs against a ring that may be in use, and every additional operation is another
chance to disturb it or to observe it inconsistently.

#### The subtraction is plain, and that is the point

`p - c` here is ordinary `u64` subtraction, not
[`Seq::distance_to`](../../../ring_types/src/id.rs)'s `saturating_sub`. Using the
family's own helper would inherit the exact masking this crate exists to defeat —
a D1 pair would saturate to zero, `0 > k` would be false, and `check` would agree
with `free_slots` that a corrupted ring is empty and healthy.

**The plain subtraction is safe only because D1 is checked first.** `c > p` is
the entire precondition for `p - c` not to underflow, and the two lines are
therefore ordered, not merely both present. Reversing them would panic in a debug
build and wrap in a release one, on precisely the input this crate is for.

#### The check order is a diagnostic decision, not an arithmetic one

Beyond the underflow constraint, D1 is reported first because it is the violation
with no other symptom. A pair breaking both — `producer 0, consumer 40` on a
capacity of 8 — is reported as D1, because D2 is visible in `pending` to anyone
who looks and D1 is visible nowhere
(`a_pair_breaking_both_reports_the_invisible_one`).

The same reasoning orders `Watch::observe`: D3 first, then D1 and D2. A cursor
that went backwards explains any ordering violation that arrived with it, and
reporting the consequence in place of the cause sends an investigation to the
wrong end of the ring.

#### `Acquire`, and why not `Relaxed`

| Ordering | Cost | Risk |
|---|---|---|
| `Relaxed` | Cheapest | The two loads may be reordered relative to the publishing stores, so the check can observe a `(p, c)` pair no thread ever held |
| **`Acquire`** | A fence on some targets, free on x86 — this workspace builds on `aarch64`, where it is `ldar` rather than a plain `ldr` and is not free | None of the above; matches the gating reads in `ring_cursor` this crate is checking the results of |
| `SeqCst` | A full barrier | Buys nothing here — there is no other ordered operation for the checks to be ordered against |

**A false positive in a diagnostic is worse than no diagnostic**, which is what
decides this. A `Relaxed` check that occasionally reports a violation of an
invariant nothing violated sends someone hunting for a corruption that does not
exist, and — worse — trains the next reader to disbelieve the instrument.

`Acquire` does not make the two loads atomic *together*. Under concurrent
writing, `check` still observes two moments, and a pair that was legal at each of
them is not guaranteed legal as a pair. This is accepted rather than solved: the
alternative is a seqlock retry or a lock, imposing synchronisation on the ring
for the benefit of its own diagnostic. See
[`api/001`](../api/001_the_check_surface.md)'s B1.

### Complexity

| Operation | Time | Space | Allocations |
|---|---|---|---|
| `check` | O(1) — two atomic loads, two comparisons | O(1) | None |
| `Watch::observe` | O(1) — same, plus two comparisons against the baseline | O(1), inline in the `Watch` | None |
| `check_ends` | O(1) — two derived reads | O(1) | None |

Nothing here scales with capacity or with occupancy, which is what makes the
checks callable in a loop from a test without changing the timing of what they
observe more than an ordinary claim would.

### Failure

| # | Failure | Consequence |
|---|---|---|
| N1 | D2 checked before D1 | `p - c` underflows on the exact input the crate is for — panic in debug, wraparound in release |
| N2 | `Seq::distance_to` used for `p - c` | D1 saturates to zero and passes; the crate agrees with the reading it exists to contradict |
| N3 | `Relaxed` loads | False positives; the instrument becomes untrustworthy, which is worse than absent |
| N4 | A `store` added to the check path | The diagnostic perturbs the ring it measures |

**N2 is the one that would look like a cleanup.** `distance_to` is the family's
own helper for exactly this subtraction, and replacing two lines of hand-rolled
arithmetic with it is the kind of tidying that reads as obviously correct. It
would silently disable D1, and four tests across three entry points would stop
catching it: `a_consumer_ahead_of_its_producer_is_caught`,
`a_pair_breaking_both_reports_the_invisible_one`,
`a_watch_refuses_to_baseline_a_broken_pair`, and
`a_watch_still_catches_the_stateless_violations`.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
S=ring_debug/src/lib.rs
T=ring_debug/tests/debug_test.rs
echo '-- N2 names one test that would fail. Which tests actually depend on D1 firing? --'
awk '/^fn [a-z_]+\(\)/{f=$2} /ConsumerAheadOfProducer/ && !seen[f]++ {print "  " f}' $T
echo '-- the helper N2 warns against, and the direction it subtracts in --'
command grep -A3 'fn distance_to' ring_types/src/id.rs | sed 's/^/  /'
echo '-- the ordering table names one architecture. This is the one we build on --'
printf '  host: %s\n' "$( uname -m )"
printf '  targets configured in the root manifest: %s\n' \
  "$( command grep -c 'target\.' Cargo.toml || true )"
```

Live output:

```
-- N2 names one test that would fail. Which tests actually depend on D1 firing? --
  a_consumer_ahead_of_its_producer_is_caught()
  a_pair_breaking_both_reports_the_invisible_one()
  a_watch_refuses_to_baseline_a_broken_pair()
  a_watch_still_catches_the_stateless_violations()
  a_violation_reports_the_numbers_it_was_derived_from()
  a_lap_report_that_contradicts_itself_does_not_fabricate_a_distance()
-- the helper N2 warns against, and the direction it subtracts in --
    pub const fn distance_to( self, later : Self ) -> u64
    {
      later.0.saturating_sub( self.0 )
    }
-- the ordering table names one architecture. This is the one we build on --
  host: aarch64
  targets configured in the root manifest: 0
```

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_cursor_invariants_over_a_live_ring.md](../invariant/001_cursor_invariants_over_a_live_ring.md) | D1 and D2 — what this evaluates |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_check_surface.md](../api/001_the_check_surface.md) | A1, A2, A5 and B1 as guarantees rather than as steps |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_saturating_arithmetic_reports_health.md](../pitfall/001_saturating_arithmetic_reports_health.md) | N2 — the exact line that must not be reused here |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/001_from_one_observation_to_a_sequence.md](../lifecycle/001_from_one_observation_to_a_sequence.md) | Where this algorithm runs inside `Watch`, and what is checked before it |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | `check`, `check_seqs`, `OBSERVE` |
| [`ring_cursor/src/lib.rs`](../../../ring_cursor/src/lib.rs) | `CursorPair::producer`/`consumer`/`capacity` — the reads |
| [`ring_atomic/src/lib.rs`](../../../ring_atomic/src/lib.rs) | `SeqCell::load` — the ordering parameter |

### Tests

| File | Relationship |
|------|--------------|
| `tests/debug_test.rs` | N1's absence — `a_consumer_ahead_of_its_producer_is_caught` runs the underflowing input; N4's absence — `checking_leaves_both_cursors_where_they_were`; the check order — `a_pair_breaking_both_reports_the_invisible_one`; the boundaries — `a_consumer_level_with_its_producer_is_not_a_violation`, `a_producer_exactly_a_lap_ahead_is_full_not_lapped` |

### DB21 — the failure table names one test that would fail, and four would

N2 warns that swapping the plain subtraction for `Seq::distance_to` would silently
disable D1, and quantifies the exposure: *"every test in the file except
`a_consumer_ahead_of_its_producer_is_caught` would still pass."*

Four tests assert a `ConsumerAheadOfProducer` produced by a check, not one:
`a_consumer_ahead_of_its_producer_is_caught`,
`a_pair_breaking_both_reports_the_invisible_one`,
`a_watch_refuses_to_baseline_a_broken_pair`, and
`a_watch_still_catches_the_stateless_violations`. A fifth site constructs the
variant by hand for rendering and would be unaffected.

The finding is not the arithmetic — it is what the sentence was for. N2's whole
rhetorical weight is *this change is nearly invisible to the suite*, and the true
figure is four independent tests across three different entry points, which is not
nearly invisible at all. **A stated coverage figure that overstates a gap is the
same defect as one that understates it**: both are a number a reader will act on
without re-measuring, in a document whose other numbers are all measured.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A4 -F 'N2 is the one that would look like a cleanup' ring_debug/docs/algorithm/001_checking_a_pair_without_touching_it.md
```

Live output:

```
**N2 is the one that would look like a cleanup.** `distance_to` is the family's
own helper for exactly this subtraction, and replacing two lines of hand-rolled
arithmetic with it is the kind of tidying that reads as obviously correct. It
would silently disable D1, and four tests across three entry points would stop
catching it: `a_consumer_ahead_of_its_producer_is_caught`,
```

**Disposition:** applied — N2's stated exposure in this same file no longer
reads "every test in the file except one would still pass"; it now names all
four tests that assert a `ConsumerAheadOfProducer` produced by a check and
would stop catching D1. Now prints: `four tests across three entry points`

### DB22 — the one concrete platform in the cost table is the one this workspace does not build for

The `Acquire` row of the ordering table prices the choice as *"a fence on some
targets, free on x86"*. This workspace builds on `aarch64`, where an acquire load
is `ldar` rather than a plain `ldr` and is not free — and the root manifest
configures no target that would make x86 the relevant case.

The decision the table supports is entirely correct and does not depend on the
price: `Acquire` is chosen because a `Relaxed` check can report a violation nothing
committed, and [`api/001`](../api/001_the_check_surface.md)'s reasoning is that a
false positive in a diagnostic is worse than no diagnostic. The cost column is
context, not argument.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -F "Acquire\`** | A fence on some targets" ring_debug/docs/algorithm/001_checking_a_pair_without_touching_it.md
```

Live output:

```
| **`Acquire`** | A fence on some targets, free on x86 — this workspace builds on `aarch64`, where it is `ldar` rather than a plain `ldr` and is not free | None of the above; matches the gating reads in `ring_cursor` this crate is checking the results of |
```

**Disposition:** applied — the ordering table's `Acquire` row no longer prices
the choice by an unqualified "free on x86"; the cell now states this workspace
builds on `aarch64`, where an acquire load is `ldar` rather than a plain `ldr`
and is not free. Now prints: `this workspace builds on`

Recorded because it is the crate's only claim about machine behaviour and it
describes a machine nobody here runs. **A parenthetical is exactly where a
platform assumption survives longest** — nothing rereads a cost column once the
decision above it is settled, and the same sentence would read as authoritative to
someone deciding whether the check is cheap enough to call in a hot loop, which is
the one question this crate's [`non_functional_requirement/`](../non_functional_requirement/readme.md) documents exist to answer.
