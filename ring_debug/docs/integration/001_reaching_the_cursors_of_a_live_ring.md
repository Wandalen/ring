# Integration: Reaching the Cursors of a Live Ring

### Scope

- **Purpose**: Record what this crate can and cannot be pointed at, and the measured consequence — the family exposes no path from a live `ring_core::Ring` to the cursors its strongest check needs.
- **Responsibility**: The reachability boundary, what survives it, and why the crate is still worth having on the wrong side of it.
- **In Scope**: Which types hold a `CursorPair`; which of them expose it; what a caller holding a `ring_core::Ring` can actually check.
- **Out of Scope**: The checks themselves (→ [`api/001`](../api/001_the_check_surface.md)); why the derived readings mask D1 (→ [`pitfall/001`](../pitfall/001_saturating_arithmetic_reports_health.md)).

### System Description

This crate checks cursors. The question that decides how useful it is, therefore,
is not what it checks but **what it can reach** — and the answer is measured
below rather than assumed, because it turned out to be narrower than the crate's
own purpose suggests.

Four family crates hold or define a `CursorPair`. Exactly one hands it out. The
distance between those two numbers is this document's subject and the crate's
central limitation.

### Integration Points

| Crate | Holds a `CursorPair`? | Exposes it? |
|---|---|---|
| `ring_cursor` | Defines it | **Yes** — `CursorPair::new` is public, as are `producer()`, `consumer()`, `capacity()` |
| `ring_spsc` | Yes, `Ring::cursors` | **No** — private field, no accessor |
| `ring_mpsc` | Yes | **No** — private field, no accessor |
| `ring_core` | **No.** It has no cursors of its own | — |

`ring_core::Ring` wraps a `ring_spsc::Ring` or a `ring_mpsc::Ring` in a private
`Storage` enum. So the chain from a live `ring_core::Ring` to a `CursorPair` is
broken twice: once at `Storage`, and again at the backend's own private field.

**Neither backend exposes the raw sequences either.** `ring_spsc`'s `Producer`
and `Consumer` do offer `position() -> Seq`, but `ring_core`'s do not — its
public surface is `free_capacity`, `is_full`, `len`, `is_empty`. Every one of
those is a *derived* reading.

#### What this costs, and it is the crate’s central limitation

`check` and `Watch` take a `CursorPair`. A caller holding a `ring_core::Ring`
cannot produce one, so **the two strongest checks are unreachable from the
family's own entry point.** What remains is `check_ends`, which compares
`Consumer::len()` against `Producer::free_capacity()`.

And those are exactly the readings that mask D1. The measurement in
[`pitfall/001`](../pitfall/001_saturating_arithmetic_reports_health.md) gives a
D1-corrupted pair `free_slots = 8`, `pending = 0` on a capacity of 8 — which sums
to 8, which is the capacity, which is a pass. Stated plainly:

> **`check_ends` cannot detect D1.** It is built from the same saturating
> arithmetic that hides it, so a D1-corrupted ring satisfies it exactly as a
> healthy one does.

This is pinned by `check_ends_cannot_see_the_corruption_check_can`, which asserts
both halves against the same pair: the derived sum passes, and `check` fails.
That test exists so the limitation is a fact in the suite rather than a caveat in
prose — if a future change to `distance_to` closed the gap, the test would fail
and the claim would be revisited rather than quietly outliving its truth.

#### Why the crate is still worth having

**Because the acceptance criterion is about cursors, not about `ring_core`.**
This crate's own invariant check must catch a deliberately corrupted cursor, and
the corruption is performed the only way the family allows one — `PaddedCursor::store`,
the same public call a producer publishes with. That is caught, on a real
`CursorPair`, in `a_consumer_ahead_of_its_producer_is_caught`.

**Because the crates that own their cursors are the ones that can corrupt them.**
`ring_spsc`, `ring_mpsc`, `ring_gating`, `ring_claim`, `ring_publish`,
`ring_barrier`, `ring_consume` and `ring_wait` all depend on `ring_cursor` and
all hold or manipulate cursors directly. Each of them can call `check` against
its own pair from its own tests — it is inside those crates that the check is
both reachable and useful, and it is inside those crates that a cursor bug would
be written.

**Because `check_ends` still catches something.** Two independently-computed
readings of one ring disagreeing means one of them is wrong, and that is a defect
class D1 masking does not cover. It is a weaker check than the other two and it
is the only one available at the top — both facts stated rather than one of them
implied.

#### What would close the gap, and why none of it is done here

| # | Option | Cost | Verdict |
|---|--------|------|---------|
| I1 | `ring_core::Ring` exposes its `CursorPair` | Publishes an internal that the crossbeam backend does not have at all — the accessor would return `Option` and be `None` for a third of the backends when that feature is enabled, but every backend this crate builds against is cursor-bearing since `ring_debug` never enables it | Rejected |
| I2 | `ring_core::Producer`/`Consumer` expose `position() -> Seq` | Small, and `ring_spsc` already does it | **The plausible one** — deferred, not dismissed |
| I3 | `ring_debug` reaches in via a `pub(crate)` or a feature flag | Couples a diagnostic to two backends' private layout | Rejected |
| I4 | Accept the boundary; check where the cursors are | Nothing | **Done** |

**I2 is deferred rather than taken because it is a change to `ring_core`'s public
surface made from inside a diagnostic crate.** Adding `position()` to
`ring_core`'s ends would make `check` reachable from the top, and it costs almost
nothing — `ring_spsc` already exposes exactly that method. But `ring_core` is one
of the family's five exported crates (gate G5), the change widens its API for a
consumer that is not itself exported, and the right place to decide it is
`ring_core`'s own docs rather than this instance. Recorded as deferred blocker
(hh) rather than made unilaterally.

Until then, the honest summary is: **this crate checks cursors, and the family's
top-level type does not let anyone hold one.**

### Error Handling

Nothing crosses this seam that *returns* a failure. One thing crossing it can
still abort the program, which is a different sentence and the one this table got
wrong for as long as it said the first (DB29 below).

| Edge | What crosses | Failure shape |
|---|---|---|
| `ring_cursor` | `producer()`, `consumer()`, `capacity()` | None — plain loads |
| `ring_core` | `is_full`, `len`, `is_empty` | None — comparisons over loads |
| `ring_core` | `free_capacity` | **Panics** on a lapped ring — `capacity − occupancy` is an unguarded `usize` subtraction, and `occupancy` is uncapped above |
| `ring_types` | `Seq`, `Capacity` | None — type aliases |

The distinction matters in one direction only. Every `Violation` this crate
returns is still a statement about the *ring* rather than about the check, and
there is still no variant meaning "could not read" — the only way to fail to read
a cursor is to be unable to name it, which is a compile error at the call site
rather than a runtime outcome (→ `check_ends`, which exists precisely because that
compile error is the common case). What the panic row adds is that on the input
`check_ends` is most worth calling on, the caller may not get a `Violation` back
at all, because the argument was computed by a subtraction that had already
failed.

### Compatibility Requirements

| # | Obligation | On whom |
|---|---|---|
| J1 | Provide a `CursorPair` to `check` or `Watch` | The caller — no family type will hand one over |
| J2 | Provide the ring's real capacity to `check_ends` | The caller — `ring_core::Ring::capacity()` is the source |
| J3 | Hold the ring quiescent across a `check_ends` call | The caller (→ [`api/001`](../api/001_the_check_surface.md)'s B1) |
| J4 | Not treat a `check_ends` pass as evidence against D1 | The caller — it is not, and cannot be |

**J4 is the obligation this instance exists to state.** A caller who reads
"runtime invariant checks over a live ring", calls the only check their ring
supports, and sees it pass, has learned strictly less than they think.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '-- a seam listed as having no failure shape --'
command grep -A2 'pub fn free_capacity( &self )' ring_spsc/src/lib.rs \
  | command grep -v '^ *$' | sed 's/^ */  /'
command grep -A5 'fn occupancy( &self )' ring_spsc/src/lib.rs \
  | command grep -v '^ *$' | tail -1 | sed 's/^ */  /'
echo '-- the saturating sibling, one crate over, for contrast --'
command grep -A4 'pub fn free_slots' ring_seqno/src/lib.rs | sed 's/^ */  /'
printf '  guarded siblings: ring_seqno free_slots %s, ring_spsc free_capacity %s\n' \
  "$( command grep -c 'capacity.get() as u64 ).saturating_sub' ring_seqno/src/lib.rs )" \
  "$( command grep -c 'capacity().get().\(saturating\|checked\)_sub' ring_spsc/src/lib.rs )"
echo '-- what the unguarded one does on the input check_ends exists to check --'
python3 -c 'cap, occ = 8, 11; print( f"  capacity {cap}, occupancy {occ}: debug build panics, release build answers {( cap - occ ) % 2**64}" )'
echo '-- I1 is rejected on the crossbeam backend. Is it ever compiled here? --'
sed -n '/^\[features\]/,/^\[dependencies\]/p' ring_core/Cargo.toml \
  | command grep -E '^default|^crossbeam' | sed 's/^/  /'
printf '  ring_debug requesting any ring_core feature: %s\n' \
  "$( command grep -c 'features' ring_debug/Cargo.toml || true )"
printf '  cfg sites gating that backend in ring_core: %s\n' \
  "$( command grep -c 'feature = "crossbeam"' ring_core/src/lib.rs || true )"
```

Live output:

```
-- a seam listed as having no failure shape --
  pub fn free_capacity( &self ) -> usize
  {
  self.ring.capacity().get().saturating_sub( self.occupancy() as usize )
  consumed.distance_to( produced )
-- the saturating sibling, one crate over, for contrast --
  pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity ) -> usize
  {
  let in_flight = consumer.distance_to( producer );
  ( capacity.get() as u64 ).saturating_sub( in_flight ) as usize
  }
  guarded siblings: ring_seqno free_slots 1, ring_spsc free_capacity 1
-- what the unguarded one does on the input check_ends exists to check --
  capacity 8, occupancy 11: debug build panics, release build answers 18446744073709551613
-- I1 is rejected on the crossbeam backend. Is it ever compiled here? --
  default = []
  crossbeam = [ "dep:crossbeam-queue" ]
  ring_debug requesting any ring_core feature: 0
  cfg sites gating that backend in ring_core: 16
```

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_check_surface.md](../api/001_the_check_surface.md) | The three entry points, and which of them this boundary reaches |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_cursor_invariants_over_a_live_ring.md](../invariant/001_cursor_invariants_over_a_live_ring.md) | D1 — the invariant the reachable check cannot see |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_saturating_arithmetic_reports_health.md](../pitfall/001_saturating_arithmetic_reports_health.md) | The measurement that makes J4 a fact rather than a caution |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_core/src/lib.rs`](../../../ring_core/src/lib.rs) | `Storage`, and the `Producer`/`Consumer` surface without `position()` |
| [`ring_spsc/src/lib.rs`](../../../ring_spsc/src/lib.rs) | `Ring::cursors` as a private field; `Producer::position` as the precedent for I2 |
| [`ring_mpsc/src/lib.rs`](../../../ring_mpsc/src/lib.rs) | The same private field in the multi-producer backend |

### Tests

| File | Relationship |
|------|--------------|
| `tests/debug_test.rs` | J4 — `check_ends_cannot_see_the_corruption_check_can` pins that the derived sum passes on a D1 pair and `check` does not; `the_two_ends_of_a_live_ring_agree` exercises the reachable path against a real `ring_core::Ring` |

### DB29 — a seam listed as having no failure shape is the one that can panic

The Error Handling table states that *"nothing crosses this seam that can fail"*
and prices the `ring_core` edge as *"None — derived readings, and the derivation is
the problem, not a failure"*.

The derivation is also a failure. `Producer::free_capacity` resolves to
`capacity.get() − occupancy()`, a plain `usize` subtraction, and `occupancy` is
uncapped above. A lapped ring makes it underflow: a panic in a debug or test build,
a wraparound in a release one
([`invariant/002`](../invariant/002_the_conservation_law_and_why_it_holds.md)'s V7).

So the one row promising infallibility named the one accessor that can abort the
program, and the sentence beneath it drew the distinction the wrong way round —
the derivation is *both* the problem and a failure. The consequence is not
hypothetical: `check_ends` is the only entry point a Contract caller can reach, and
on the input most worth checking it panics inside a crate this one does not name,
rather than returning the `Violation` its signature promises.

**The panic is not this crate's to remove, and the claim was.** `free_capacity`
lives in `ring_spsc`, which is not an edge in
[`decisions/001`](../decisions/001_four_edges_not_two.md)'s four and whose
`capacity − occupancy` is one of the sites
[`pattern/001`](../pattern/001_the_guard_that_makes_the_next_line_legal.md)'s
census counts — the same ruling DB35 reaches from the conservation law, arriving
at the same line from the other direction. What `ring_debug` owns is the sentence
that told a reader the seam could not fail, and the recipe that never measured
whether it could.

**Disposition:** applied — the Error Handling table splits the `ring_core` row in
two and prices `free_capacity` as **Panics** on a lapped ring, naming the
unguarded subtraction and the uncapped `occupancy` that reaches it; the paragraph
beneath now says which direction the infallibility claim survives in and which it
does not; and the recipe counts the guard on both siblings, so the day
`ring_spsc` adopts `ring_seqno`'s `saturating_sub` this document goes stale loudly
instead of staying wrong quietly — which it since has, per the correction below.
Now prints:
`guarded siblings: ring_seqno free_slots 1, ring_spsc free_capacity 1`

`ring_seqno`'s own `saturating_sub` moved to the far side of a widening cast
(`( capacity.get() as u64 ).saturating_sub( … )`) without dropping the guard, and
the recipe's literal pattern briefly went stale in exactly the way this section
warns against — reporting `0` for a still-guarded function, indistinguishable
from `ring_spsc` genuinely catching up. The pattern now matches the widened form.

**Correction (2026-09-28):** the Error Handling table's `free_capacity` row above,
and this finding, describe `ring_spsc::Producer::free_capacity` as it stood when
written. It has since been fixed
(`Fix(free_capacity_underflow_on_a_precondition_violation)` in
`ring_spsc/src/lib.rs`) to `saturating_sub` rather than subtract unguarded — the
`guarded siblings` count in the Regenerate output above went from
`ring_spsc free_capacity 0` to `1`, which is this section's own trap catching the
day it names. `free_capacity` no longer panics or wraps on a lapped ring; it now
returns `0`. The table row and the paragraph above are kept as the record of the
crate's prior behaviour rather than rewritten; the consequence for `check_ends`'s
D4 invariant is recorded in
[`invariant/002`](../invariant/002_the_conservation_law_and_why_it_holds.md)'s
matching correction under its DB35.

### DB30 — the rejected option is rejected on a backend that is never compiled here

I1 — exposing a `CursorPair` from `ring_core::Ring` — is rejected because the
accessor *"would return `Option` and be `None` for a third of the backends"*, the
crossbeam one having no cursors at all.

`ring_core`'s `default = []`, crossbeam is opt-in, and `ring_debug` requests no
features from it. In every build this crate participates in there are two backends,
both cursor-bearing, and the `Option` I1 objects to would be `Some` every time.

The rejection may still be right — `ring_core`'s public surface has to make sense
for consumers who *do* enable the feature, and an accessor that is sometimes `None`
is a poor thing to export. **What is recorded is that the reason given is measured
against a configuration this crate never builds in**, stated as a fraction of the
backends as though all three were always present. A reader weighing I1 against I2
is comparing a real cost to a conditional one, and the document presents both
flatly.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -F '| I1 |' ring_debug/docs/integration/001_reaching_the_cursors_of_a_live_ring.md
```

Live output:

```
| I1 | `ring_core::Ring` exposes its `CursorPair` | Publishes an internal that the crossbeam backend does not have at all — the accessor would return `Option` and be `None` for a third of the backends when that feature is enabled, but every backend this crate builds against is cursor-bearing since `ring_debug` never enables it | Rejected |
```

**Disposition:** applied — I1's Cost cell no longer states the `Option`/`None`
fraction as an unconditional property of the backend set; it now scopes that
cost to builds where the crossbeam feature is enabled and states that every
backend this crate itself builds against is cursor-bearing, since `ring_debug`
never enables it. Now prints: `every backend this crate builds against is cursor-bearing`
