# API: The Check Surface

### Scope

- **Purpose**: Define the callable surface — three checks and the value they report — and record why it is three and not one.
- **Responsibility**: Signatures, guarantees, preconditions, and the reason each entry point exists separately from the others.
- **In Scope**: `check`, `Watch`, `check_ends`; what each costs the caller.
- **Out of Scope**: The invariants themselves (→ [`invariant/001`](../invariant/001_cursor_invariants_over_a_live_ring.md)); the reported value's shape (→ [`type/001`](../type/001_violation.md)).

### Abstract

Five entry points in three groups, split by *what the caller must supply* rather
than by what they check: one observation (`check`), a held baseline plus a new
observation (`Watch`), or a split ring whose cursors are not directly reachable
(`check_ends`). Every one returns `Result< _, Violation >`, and none of them
writes anything.

The split is not stylistic — it is the honest shape of the problem, since one of
the three defects is a property of a *pair* of readings and cannot be detected by
a function that sees only one.

### Operations

| Entry point | Signature | Needs | Catches |
|---|---|---|---|
| `check` | `fn( &CursorPair ) -> Result< (), Violation >` | One observation | D1, D2 |
| `Watch::new` | `fn( &CursorPair ) -> Result< Self, Violation >` | One observation | D1, D2 — refuses to baseline a broken pair |
| `Watch::observe` | `fn( &mut self, &CursorPair ) -> Result< (), Violation >` | A previous observation | D3, then D1, D2 |
| `Watch::last` | `fn( &self ) -> ( Seq, Seq )` | — | Nothing; exposes the baseline |
| `check_ends` | `fn< T : Send >( Capacity, &Producer<T>, &Consumer<T> ) -> Result< (), Violation >` | A split, quiescent ring | Two readings of one ring disagreeing |

**Everything returns `Result`, never `Option`.** A caller that finds a violation
usually wants to propagate it — `check( pair )?` inside a test helper or a tool
is the expected shape, and `Violation` implements `core::error::Error` for
exactly that (`a_violation_propagates_as_an_error`).

#### Why the surface is three things

**`check` and `Watch` are split because D3 costs the caller something the other
two do not.** D1 and D2 are properties of a single reading; D3 is a property of a
*pair* of readings, so detecting it requires the caller to hold state between
calls and to have taken a baseline before the corruption happened. Folding D3
into `check` would mean either a hidden global — unacceptable in a crate meant to
be callable from a test — or a `check` that silently did nothing about D3 while
appearing to check everything.

**`check_ends` is separate because it takes different arguments**, and that turns
out to be the ordinary case rather than the exceptional one. `ring_core` has no
`CursorPair` at all — it wraps a `ring_spsc` or `ring_mpsc` ring, each of which
holds one as a private field with no accessor — so a caller at the family's own
entry point cannot call `check` or `Watch`, only this.

**And this one cannot see D1**, because it is built from the derived readings the
saturating arithmetic masks. That is the crate's central limitation, measured and
pinned rather than implied:
→ [`integration/001`](../integration/001_reaching_the_cursors_of_a_live_ring.md).

### Error Handling

Every failure is one [`Violation`](../type/001_violation.md) value carrying the
numbers it was derived from. There is no error type of this crate's own beyond
that, and no error that means "the check could not run" — every input either
satisfies the invariants or does not.

### Compatibility Guarantees

| # | Guarantee | Evidence |
|---|---|---|
| A1 | No check writes to a cursor | `checking_leaves_both_cursors_where_they_were` — 100 checks, both cursors unmoved |
| A2 | Every check is `Acquire`-ordered | One `const OBSERVE` shared by all readers; a weaker ordering could report a violation of an invariant nothing violated |
| A3 | `Watch` reports a fault on every observation, not once | `a_failed_observation_leaves_the_baseline_alone` — the second look still fails |
| A4 | D3 is reported in preference to D1/D2 | A backwards cursor explains any ordering violation that came with it; reporting the consequence sends the reader to the wrong place |
| A5 | D1 is reported in preference to D2 | D1 is the one with no other symptom (→ [the pitfall](../pitfall/001_saturating_arithmetic_reports_health.md)); `a_pair_breaking_both_reports_the_invisible_one` |

**A3 is the guarantee that is easy to get wrong and expensive to lose.** The
natural implementation of `observe` — read, compare, store, return — adopts the
corrupt reading as the new baseline on its way out, so a permanent fault is
reported exactly once and treated as normal from the second observation onward.
The implementation stores only on success.

### Preconditions

| # | Precondition | On whom | If violated |
|---|---|---|---|
| B1 | `check_ends` sees a quiescent ring | The caller | False positives — the two readings are separate atomic loads of two different moments |
| B2 | `check_ends` is given the ring's real capacity | The caller | `ReadingsDisagree`, which is the right report for it (`a_ring_measured_against_the_wrong_capacity_disagrees`) |
| B3 | A `Watch` baseline predates the corruption | The caller | D3 goes undetected — a watch started after a reset sees only forward movement |

**B1 is not a defect to be worked around.** Making `check_ends` correct under
concurrent writes would need a consistent snapshot of both ends, which needs
either a lock or a seqlock retry — a synchronisation cost imposed on the ring by
its own diagnostic. The precondition is cheaper and honest, and the ordinary use
(a test, between phases) satisfies it without effort.

**B3 is the structural limit of `Watch` and cannot be removed.** No amount of
implementation makes an instrument see what happened before it was switched on.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
S=ring_debug/src/lib.rs
T=ring_debug/tests/debug_test.rs
echo '-- the five entry points --'
command grep -E '^  pub fn |^pub fn ' $S | sed 's/^/  /'
echo '-- what last() returns, beside the enum that exists to name an end --'
command grep 'fn last\|pub enum Cursor' $S | sed 's/^/  /'
echo '-- A4: the fixture that is its only guard, and what its docstring says it is for --'
command grep -B6 'fn a_backwards_consumer_is_named_as_the_consumer' $T | sed 's/^/  /'
python3 -c 'cap,p,c=8,20,4; print(f"  once the consumer moves to {c}: D1 {c>p}, D2 {p-c>cap} (distance {p-c} vs capacity {cap})")'
```

Live output:

```
-- the five entry points --
  pub fn check( pair : &CursorPair ) -> Result< (), Violation >
    pub fn new( pair : &CursorPair ) -> Result< Self, Violation >
    pub fn observe( &mut self, pair : &CursorPair ) -> Result< (), Violation >
    pub fn last( &self ) -> ( Seq, Seq )
  pub fn check_ends< T >
-- what last() returns, beside the enum that exists to name an end --
  pub enum Cursor
    pub fn last( &self ) -> ( Seq, Seq )
-- A4: the fixture that is its only guard, and what its docstring says it is for --
  ///
  /// Separate from the producer case because the `cursor` field is the entire
  /// diagnostic value of the variant — a report that says "a cursor moved
  /// backwards" without saying which one leaves the reader exactly where they
  /// started.
  #[ test ]
  fn a_backwards_consumer_is_named_as_the_consumer()
  once the consumer moves to 4: D1 False, D2 True (distance 16 vs capacity 8)
```

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_cursor_invariants_over_a_live_ring.md](../invariant/001_cursor_invariants_over_a_live_ring.md) | D1–D3 — what these entry points check |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_checking_a_pair_without_touching_it.md](../algorithm/001_checking_a_pair_without_touching_it.md) | A1 and A2 — how the reads are performed |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_reaching_the_cursors_of_a_live_ring.md](../integration/001_reaching_the_cursors_of_a_live_ring.md) | Why `check_ends` exists as a separate entry point |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/001_from_one_observation_to_a_sequence.md](../lifecycle/001_from_one_observation_to_a_sequence.md) | `Watch`'s own states — A3 and B3 as transitions |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_violation.md](../type/001_violation.md) | The returned value |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | The surface itself |

### Tests

| File | Relationship |
|------|--------------|
| `tests/debug_test.rs` | A1 — `checking_leaves_both_cursors_where_they_were`; A3 — `a_failed_observation_leaves_the_baseline_alone`; A5 — `a_pair_breaking_both_reports_the_invisible_one`; B2 — `a_ring_measured_against_the_wrong_capacity_disagrees` |

### DB25 — the crate that defines a type to name an end returns both ends unlabelled

`Cursor` exists for one reason: a report that says "a cursor moved backwards"
without saying which one leaves the reader where they started — the crate's own
words, in `a_backwards_consumer_is_named_as_the_consumer`'s docstring. It is
constructed at three sites, all of them building `CursorWentBackwards`.

`Watch::last` returns `( Seq, Seq )`. Two values of one type, distinguished by
position, in the crate that built an enum to avoid exactly that. The ordering is
carried by the doc comment — *"Producer first, then consumer"* — because the
signature cannot carry it, and a caller who destructures them backwards gets code
that compiles, runs, and asserts the wrong thing.

The exposure is small and the shape is worth recording anyway: `last` exists to let
a test check that a failed `observe` left the baseline alone (DB11), so its three
call sites are the tests most concerned with which cursor is which. **The accessor
that serves the crate's own precision requirement is the one place the crate stops
being precise.**

### DB26 — A4's only guard is a side effect of a fixture chosen for something else

A4 says a backwards cursor is reported in preference to an ordering violation that
came with it. Reversing `observe`'s two check blocks would break it, and one test
notices: `a_backwards_consumer_is_named_as_the_consumer`.

That test is not about precedence. Its docstring says it is separate from the
producer case because the `cursor` field is the entire diagnostic value of the
variant. Its fixture is `pair_at( 8, 20, 15 )` with the consumer then moved to 4 —
and 20 − 4 is 16, past a capacity of 8, so the observed pair violates D2 as well as
D3. The precedence is asserted by accident.

**Narrowing the fixture to any consumer between 12 and 14 preserves everything the
test is documented to check and silently deletes A4's coverage.** The move is still
backwards, the cursor is still named, the assertion still passes — and the
guarantee stops being pinned by anything. Recorded because the failure mode is a
tidy-up, not a bug: a fixture with numbers larger than they need to be is exactly
what a later reader simplifies.
