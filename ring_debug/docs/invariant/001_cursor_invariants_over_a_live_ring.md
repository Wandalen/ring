# Invariant: Cursor Invariants Over a Live Ring

### Scope

- **Purpose**: State the properties a ring's cursors must satisfy at every moment, which the family's arithmetic assumes and never verifies.
- **Responsibility**: Enumerate the invariants, what each excludes, what detects a breach, and what a breach costs.
- **In Scope**: The producer/consumer cursor pair; ordering, distance, and monotonicity.
- **Out of Scope**: Why the arithmetic hides a breach (→ [Saturating Arithmetic Reports Health](../pitfall/001_saturating_arithmetic_reports_health.md)); the sequence of observations monotonicity needs (→ [`lifecycle/001`](../lifecycle/001_from_one_observation_to_a_sequence.md)).

### Invariant Statement

**At every moment, for a ring of `capacity` slots with cursors `p` (producer)
and `c` (consumer):**

| # | Invariant | Excludes |
|---|-----------|----------|
| D1 | `c <= p` | A consumer reading slots the producer has not published |
| D2 | `p - c <= capacity` | A producer overwriting slots the consumer has not read |
| D3 | Neither cursor ever decreases | A cursor reset or rollback, which makes every prior reading a lie |

**D1 and D2 are two halves of one statement** — `0 <= p - c <= capacity` — and
they are split because they fail for different reasons and cost different
things. D1 fails when a consumer advances without a corresponding publish; D2
fails when a producer publishes without a corresponding gating check. Only D2
has a name in the family's existing docs — "the lap bug."
[`ring_gating`](../../../ring_gating/docs/lifecycle/002_the_producer_walking_a_lap_against_a_stall.md)
documents the same scenario, a producer lapping a stalled consumer.

**D3 is not implied by D1 and D2 and is not checkable in the same way.** Every
pair `(p, c)` satisfying `0 <= p - c <= capacity` is a valid *snapshot*; D3 is a
statement about two snapshots. A pair that reads `(0, 0)` is either a new ring
or a catastrophically corrupted one, and no property of that single reading
distinguishes them.

### Enforcement Mechanism

| # | Mechanism | Covers | Gap |
|---|-----------|--------|-----|
| E1 | No publish path in the family computes a smaller `Seq` — a caller convention, not a property `Seq` itself enforces | Accidental arithmetic rollback | Nothing prevents `store`ing an arbitrary `Seq` — [`PaddedCursor::store`](../../../ring_cursor/src/lib.rs) takes any value |
| E2 | This crate's `check` | D1 and D2, at the moment it is called | Only when called. It is a diagnostic, not a barrier |
| E3 | This crate's `Watch` | D3, across the observations it was shown | Corruption between two observations that restores a valid-looking pair |
| E4 | The family's own correctness | All three, in principle | Unverified. That is the whole reason for E2 and E3 |

**E1 is the gap that makes this crate necessary rather than paranoid.**
`ring_cursor::PaddedCursor` exposes `store`, because a producer publishing *is*
a store. Every claim path in the family therefore has a legitimate,
type-checked way to write any sequence value it likes into a cursor, and the
difference between publishing and corrupting is arithmetic the compiler cannot
see.

**E2's "only when called" is a deliberate shape, not a shortfall.** Making the
check automatic means putting it on the claim path — the family's hottest —
where it would cost every correct program to catch a state no correct program
reaches (→ [the pitfall](../pitfall/001_saturating_arithmetic_reports_health.md)'s
P4). The check is a thing a test, a debug build, or an investigation runs.

### Violation Consequences

| # | Violation | Detected by | Consequence |
|---|-----------|-------------|-------------|
| V1 | `c > p` (D1) | E2 only | **Silent and unsafe.** The ring reports empty-and-healthy and the producer overwrites unread slots (the pitfall's measurement) |
| V2 | `p - c > capacity` (D2) | E2; also visible in `pending` exceeding capacity | Unread records are overwritten, but `may_claim` returns `false`, so the family declines to make it worse |
| V3 | A cursor decreases (D3) | E3 only | Every reading taken before the decrease described a ring that no longer exists |
| V4 | Both cursors advance by the same amount, wrongly | **Nothing here.** D1–D3 all hold | The ring is internally consistent and disagrees with the records actually stored |

**V1 is the violation this crate is for.** It is the only one of the four that
is both undetectable by any existing reading and actively harmful — V2 leaves
evidence, V3 is caught by `Watch`, and V4 is out of reach of any cursor-only
check.

**V4 is the honest boundary of what a cursor checker can do.** These invariants
are about cursors, not about records. A ring whose cursors are perfectly
consistent and whose slots hold the wrong data satisfies D1, D2 and D3
completely. Checking that would need to compare cursors against slot contents,
which needs the slot type, which this crate does not have and should not
acquire — that is `ring_slot`'s and the tests' territory.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
D=ring_debug/docs/invariant/001_cursor_invariants_over_a_live_ring.md
echo '-- the crate has three entry points. Which does the E-table name? --'
E=$( awk '/^### Enforcement Mechanism/{f=1;next} /^### /{f=0} f' $D )
for n in 'check' 'Watch' 'check_ends' ; do
  printf '  %-11s in the enforcement section: %s\n' "$n" \
    "$( printf '%s' "$E" | command grep -oF -- "\`$n\`" | wc -l )"
done
echo '-- E1 credits the type. What does the type actually offer? --'
command grep 'pub struct Seq' ring_types/src/id.rs | sed 's/^/  /'
printf '  impl Sub or SubAssign for Seq:              %s\n' \
  "$( command grep -c 'impl Sub' ring_types/src/id.rs || true )"
printf '  sites in this crate storing an arbitrary Seq: %s\n' \
  "$( command grep -c 'store( Seq(' ring_debug/tests/debug_test.rs || true )"
```

Live output:

```
-- the crate has three entry points. Which does the E-table name? --
  check       in the enforcement section: 1
  Watch       in the enforcement section: 1
  check_ends  in the enforcement section: 0
-- E1 credits the type. What does the type actually offer? --
  pub struct Seq( pub u64 );
  impl Sub or SubAssign for Seq:              0
  sites in this crate storing an arbitrary Seq: 16
```

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_checking_a_pair_without_touching_it.md](../algorithm/001_checking_a_pair_without_touching_it.md) | How D1 and D2 are evaluated without perturbing what they measure |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_check_surface.md](../api/001_the_check_surface.md) | E2 and E3 as callable operations |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_saturating_arithmetic_reports_health.md](../pitfall/001_saturating_arithmetic_reports_health.md) | V1's measurement, and why V1 and V2 differ so much |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/001_from_one_observation_to_a_sequence.md](../lifecycle/001_from_one_observation_to_a_sequence.md) | D3 — why it needs a second observation and what that costs |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_violation.md](../type/001_violation.md) | V1–V3 as values a caller can match on |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_cursor/src/lib.rs`](../../../ring_cursor/src/lib.rs) | `PaddedCursor::store` — the same public call a producer publishes with, and the reason a corrupted cursor is reachable at all |
| [`ring_gating/docs/lifecycle/002`](../../../ring_gating/docs/lifecycle/002_the_producer_walking_a_lap_against_a_stall.md) | The same scenario this crate calls "the lap bug" — a producer lapping a stalled consumer |

### Tests

| File | Relationship |
|------|--------------|
| `tests/debug_test.rs` | V1 — `a_consumer_ahead_of_its_producer_is_caught`; V2 — `a_producer_more_than_a_lap_ahead_is_caught`; V3 — `a_cursor_that_goes_backwards_is_caught`. Each corrupts a real `CursorPair` through `ring_cursor`'s own public `store` rather than a fixture, which is what makes them the acceptance criterion's "deliberately corrupted cursor" |

### DB33 — the invariant document does not know about the crate's third entry point

E1–E4 enumerate what enforces D1, D2 and D3. `check_ends` is not among them, and
neither is the property it checks. The document names `check` and `Watch`, states
that E2 and E3 are the crate's mechanisms, and stops.

That omission is not cosmetic. `check_ends` is the only entry point reachable from
the family's own Contract
([`workaround/001`](../workaround/001_the_door_ring_core_does_not_open.md)), so the
enforcement table describes the two doors a caller cannot open and skips the one
they can. A reader arriving here to learn what protects a live ring is given the
complete answer for a caller who has gone below `ring_core`, and no answer at all
for the caller the family documents.

The invariant it checks is a genuinely different statement — over derived readings
rather than cursors, with a term the ring does not own — which is why the fix is
[`invariant/002`](002_the_conservation_law_and_why_it_holds.md) rather than a
fifth row here. Recorded because a table that reads as exhaustive and is not is the
form of gap that gets propagated: three later documents cite this E-table as the
crate's enforcement inventory.

### DB34 — the first enforcement mechanism is a naming convention presented as a type property

E1 credits D3's protection to *"`ring_types::Seq` having no public decrement"*.
`Seq` is `pub struct Seq( pub u64 )` with no `Sub` or `SubAssign` impl — so the
claim is true as stated and misleading as read. There is no decrementing *method*,
and there does not need to be: the field is public, so `Seq( n )` for any smaller
`n` is one expression, and this crate's own suite writes one into a live cursor
eleven times.

The row's Gap column gets halfway there — it notes that `store` accepts any value —
but it attributes the residual safety to the type, and the type contributes
nothing. What actually keeps a sequence from going backwards is that no publish
path in the family computes a smaller one; the convention is in the callers, not in
`Seq`.

This matters more here than it would elsewhere. **D3 is the one invariant with no
snapshot test, so `Watch` exists entirely because E1 is not real** — and an
enforcement table that overstates E1 understates the reason for the crate's most
expensive entry point.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -F '| E1 |' ring_debug/docs/invariant/001_cursor_invariants_over_a_live_ring.md
```

Live output:

```
| E1 | No publish path in the family computes a smaller `Seq` — a caller convention, not a property `Seq` itself enforces | Accidental arithmetic rollback | Nothing prevents `store`ing an arbitrary `Seq` — [`PaddedCursor::store`](../../../ring_cursor/src/lib.rs) takes any value |
```

**Disposition:** applied — E1's Mechanism cell no longer credits `ring_types::Seq`
itself with having no public decrement; it now states the actual mechanism — no
publish path in the family computes a smaller `Seq` — and names it a caller
convention rather than a property the type enforces. Now prints: `No publish path in the family computes a smaller`
