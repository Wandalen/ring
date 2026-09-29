# Data Structure: Two Borrows and No Owned State

### Scope

**Purpose:** Establish what follows from `Consumer` owning nothing — for
construction, for thread-safety, and for the mistake the borrow is there to
prevent.

**Responsibility:** The consequences of `Consumer`'s two fields both being
borrows: auto-trait derivation, `const` construction, and the ownership error
the design forecloses.

**In Scope:** `Consumer< 'a >`'s field types; the auto-derived `Send`/`Sync`;
the module documentation's argument for borrowing; the tests that would fail to
compile without the auto traits.

**Out of Scope:** The lifetime's own consequences for callers — that is
[`type/002`](../type/002_the_lifetime_on_consumer.md). The size numbers, which
are [`001`](001_sixteen_and_twenty_four.md).

---

## Everything Is Borrowed

```rust
pub struct Consumer< 'a >
{
  cursor : &'a PaddedCursor,
  barrier : Barrier< 'a >,
}
```

`Barrier< 'a >` is itself `&'a [ PaddedCursor ]`. So a `Consumer` is two
references and a length, and it owns no heap memory, no cursor, and no lock. It
cannot be constructed without something else already owning the cursors, and it
becomes invalid when that something goes away.

### CN24 — The Borrow Is the Mechanism, and the Module Documentation Says So

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/## Why the cursor is borrowed rather than owned/,/overwrites unread slots/p' \
  ring_consume/src/lib.rs
```

Live output:

```
//! ## Why the cursor is borrowed rather than owned
//!
//! [`Consumer::new`] takes a `&PaddedCursor` from somewhere else, and the
//! somewhere else is almost always a producer's `ring_gating::GatingSet`. That
//! is the whole mechanism: the producer decides what it may overwrite by
//! reading the cursors in its set, so a consumer whose position lived in a
//! cursor it owned privately would be invisible to the producer and gate
//! nothing. A ring wired that way runs, passes every single-threaded test, and
//! overwrites unread slots on the first lap.
```

The argument, quoted from the source:

> [`Consumer::new`] takes a `&PaddedCursor` from somewhere else, and the
> somewhere else is almost always a producer's `ring_gating::GatingSet`. That
> is the whole mechanism: the producer decides what it may overwrite by
> reading the cursors in its set, so a consumer whose position lived in a
> cursor it owned privately would be invisible to the producer and gate
> nothing. A ring wired that way runs, passes every single-threaded test, and
> overwrites unread slots on the first lap.

This is the crate's best sentence and it is recorded here because it is a
positive finding of a kind the corpus rarely gets to make. The `&` in
`cursor : &'a PaddedCursor` is not a performance choice or a borrow-checker
concession — it is a correctness requirement, and the failure mode of getting
it wrong is named precisely: *runs, passes every single-threaded test, and
overwrites unread slots on the first lap.*

Compare what the corpus found in `ring_claim`: four disciplines followed
exceptionlessly and written down nowhere, two doc sentences that are measurably
false, and one accessor whose documentation points readers at a conflation a
sibling crate forbids. Here the load-bearing design constraint is stated at the
place a reader would look for it, with its failure mode, in five lines.

The same section covers the three other decisions a reader would question —
why `available` and `commit` are separate, why `commit` is clamped and
monotonic, and why the crate is single-consumer. All four are argued rather than
asserted.

**Cost:** none. Recorded as the family's counter-example to its own dominant
pattern.

---

### CN25 — `Send` and `Sync` Are Auto-Derived, and Nothing Says What They Cost

`Consumer` has no `unsafe impl Send` or `unsafe impl Sync`. Both are auto-derived
from its fields:

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'unsafe impl\|impl.*Send\|impl.*Sync' ring_consume/src/lib.rs
grep -c 'thread::scope' ring_consume/tests/consume_test.rs
```

Live output:

```
2
```

The first command produces no output — there are no manual impls. The second
returns `2`.

A `&T` is `Send` when `T : Sync`, and `Sync` when `T : Sync`. `PaddedCursor`
wraps an atomic, so it is `Sync`, so `&PaddedCursor` is both, so `Consumer` is
both. That derivation is correct and invisible.

Two of the crate's 21 tests use `std::thread::scope`, which requires the values
crossing the scope boundary to be `Send`. Neither test says so. If a future
change gave `Consumer` a non-`Sync` field — a `Cell` for a cached frontier, say,
which is exactly the kind of optimisation
[`non_functional_requirement/001`](../non_functional_requirement/001_what_the_read_path_costs.md)
CN34's allocation would motivate — the auto-derivation would silently stop, and
the failure would surface as a compile error in a test whose subject is
something else entirely.

This is the same finding `ring_claim` recorded as CL50 about `Claimer : Sync`,
and it landed harder here for two reasons. First, the motivation for adding
interior mutability was concrete and measured — one allocation per read call,
with an obvious cache as the fix. That motivation is gone: `b7e075ca` removed
the allocation, so the cache has nothing left to save and the most likely reason
anyone would have reached for a `Cell` has evaporated. The finding survives it
intact, because it was never about how likely the change was — an unasserted
property is unasserted whether or not anyone is currently tempted. Second,
`Consumer` is smaller and cheaper, so the temptation to give it a field is
correspondingly lower-friction.

The one-line guard, in either crate:

```rust
const _ : fn() = || { fn assert_sync< T : Sync >() {} assert_sync::< Consumer< '_ > >(); };
```

Nothing like it exists in any of the 33 crates.

**Cost:** reachable. The property holds, is relied on by two tests, and is
asserted nowhere; a plausible optimisation breaks it, and the diagnostic points
at the wrong file.

---

## What Owning Nothing Buys

| Consequence | Because |
|-------------|---------|
| `Consumer::new` is `const` | no allocation, no atomic init — just two pointer copies |
| A `Consumer` is cheap to build per-call | 24 bytes, no ownership transfer, no `Drop` |
| No `Drop` impl exists or is needed | nothing to release; contrast `Claim`, whose drop strands a slot |
| The producer can always see the cursor | it is the producer's own cursor, borrowed — CN24 |
| Two `Consumer`s over one cursor are possible | and wrong; the module doc says so, and nothing prevents it |

That last row is the cost side of the same coin, and
[`invariant/002`](../invariant/002_the_cursor_only_moves_forward.md) covers it:
because a `Consumer` owns nothing, constructing two over the same cursor is
free, compiles, and breaks the single-consumer assumption the whole design
rests on. An owned cursor would have made it impossible. The design chose the
borrow because the borrow is required for the producer to see the cursor at all
(CN24), and accepted that the borrow cannot enforce exclusivity.

---

## Cross-References

| Definition | Instance | Relationship |
|------------|----------|--------------|
| data_structure | [001](001_sixteen_and_twenty_four.md) | the sizes this shape produces |
| type | [002](../type/002_the_lifetime_on_consumer.md) | the single lifetime unifying both borrows |
| invariant | [002](../invariant/002_the_cursor_only_moves_forward.md) | the exclusivity the borrow cannot enforce |
| decisions | [001](../decisions/001_two_calls_not_one.md) | the other decision the module doc argues |
| non_functional_requirement | [001](../non_functional_requirement/001_what_the_read_path_costs.md) | the allocation that motivated the field CN25 warns about, and its removal |

### Sources

| What | Where |
|------|-------|
| The two borrowed fields | `ring_consume/src/lib.rs:209-214` |
| The borrowing argument | `ring_consume/src/lib.rs`, module doc |
| The two threaded tests | `ring_consume/tests/consume_test.rs` |
| `Claimer`'s equivalent finding | `ring_claim/docs/` CL50 |

### Tests

| Claim | Verified by |
|-------|-------------|
| No manual `Send`/`Sync` impls | `grep 'unsafe impl'` → no output |
| Two tests need `Send` | `grep -c 'thread::scope'` → 2 |
| Neither test asserts it | reading both; the requirement is implicit in `scope` |
| `Consumer::new` is `const` | `ring_consume/src/lib.rs:237` |
