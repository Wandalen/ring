# Item: `counts`, the Method That Is Not a Snapshot — and the Two That Are Not on the Trait

### Scope

**Purpose:** Record the contract of the two methods that exist beside `SeqCell`
rather than on it, and what putting them there decides about which code in the
family can ever be measured.

**Responsibility:** `CountingSeq::counts`, `CountingSeq::reset_counts`, their
contracts and doctests, and the substitution points the family does and does not
offer them.

**In Scope:** `ring_atomic/src/lib.rs:297`, `:409-424`, `:426-452`;
`ring_claim/src/lib.rs:259`, `:432-445`; `ring_batch/src/lib.rs:221`,
`:306`; `ring_tls/src/lib.rs:280`.

**Out of Scope:** That a single `counts()` call is four reads and not one is
[`algorithm/002`](../algorithm/002_counts_is_four_reads_not_one.md) AT3. What the
shim costs per call is
[`non_functional_requirement/001`](../non_functional_requirement/001_what_the_instrument_costs.md).
The six constructors are [`item/001`](001_six_constructors_for_two_types.md).

---

## Four Methods on the Trait, Two Beside It

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- four methods on the trait, two beside it --'
command grep -E '^  fn [a-z_]+.*[;)]$' ring_atomic/src/lib.rs | head -4
command grep -E '^  pub fn (counts|reset_counts)' ring_atomic/src/lib.rs
echo '  -- what the two promise --'
sed -n '/^  \/\/\/ What this cell has been asked to do so far\.$/p;/^  \/\/\/ Return every count to zero, leaving the sequence itself untouched\.$/,/^  \/\/\/ only what the operation under test does\.$/p' ring_atomic/src/lib.rs
echo '  -- every generic SeqCell parameter in the family: a substitution point --'
command grep -rE '\b[A-Z] *: *SeqCell' --include=lib.rs */src/ | sed 's|ring/||;s|/src/lib.rs||'
echo '  -- every struct field typed PaddedCursor: no substitution point --'
command grep -rE '^ +[a-z_]+ *: *PaddedCursor,' --include=lib.rs */src/ | sed 's|ring/||;s|/src/lib.rs||'
echo '  -- and every crate that asserts an operation count --'
command grep -rl '\.counts()' --include=*.rs . | command grep -v ring_atomic/ | sed 's|ring/||;s|/.*||' | sort -u | tr '\n' ' '; echo
```

Live output:

```
  -- four methods on the trait, two beside it --
  fn load( &self, order : Ordering ) -> Seq;
  fn store( &self, value : Seq, order : Ordering );
  fn fetch_add( &self, n : u64, order : Ordering ) -> Seq;
  fn compare_exchange( &self, current : Seq, new : Seq, success : Ordering, failure : Ordering )
  pub fn counts( &self ) -> OpCounts
  pub fn reset_counts( &self )
  -- what the two promise --
  /// Return every count to zero, leaving the sequence itself untouched.
  ///
  /// Four independent `Relaxed` stores, not one atomic reset — safe to read
  /// as complete only while nothing else is touching the cell. For a test
  /// that sets up a state through the cell and then wants to count only what
  /// the operation under test does; confirming the sequence survived costs a
  /// count of its own, since the confirming read is itself counted.
  ///
  /// ```
  /// use core::sync::atomic::Ordering;
  /// use ring_atomic::{ CountingSeq, SeqCell };
  /// use ring_types::Seq;
  ///
  /// let cell = CountingSeq::default();
  /// cell.store( Seq( 5 ), Ordering::Release );
  /// cell.reset_counts();
  ///
  /// assert_eq!( cell.counts().total, 0 );
  /// assert_eq!( cell.load( Ordering::Acquire ), Seq( 5 ), "the sequence survives" );
  /// ```
  pub fn reset_counts( &self )
  {
    for counter in [ &self.loads, &self.stores, &self.fetch_adds, &self.compare_exchanges ]
    {
      counter.store( 0, Ordering::Relaxed );
    }
  }
}

/// Each method bumps its counter **before** delegating to the cell.
///
/// Fix(AT24): the order is deliberate and was undocumented. A concurrent
/// observer reading the counter and the cell while both are moving can
/// therefore see a counter that has already been incremented for an operation
/// the cell has not yet performed — a million-sample probe found 7,176 such
/// orderings. Bumping after the delegation would only move the window, not
/// close it; closing it needs a lock, which is the cost this type exists to
/// measure rather than pay.
///
/// The consequence is bounded and worth stating plainly: at rest the counts are
/// exact (`counts_are_exact_under_contention` asserts that), and in flight they
/// lead. Never assert a counter against the cell's value while anything is
/// still running.
///
/// Root cause: two pieces of state updated in sequence are observable between
/// the updates.
/// Pitfall: "the counts are exact" is a statement about quiescence, and every
/// test that establishes it is a test taken at rest.
impl SeqCell for CountingSeq
{
  fn load( &self, order : Ordering ) -> Seq
  {
    self.loads.fetch_add( 1, Ordering::Relaxed );
    self.cell.load( order )
  }

  fn store( &self, value : Seq, order : Ordering )
  {
    self.stores.fetch_add( 1, Ordering::Relaxed );
    self.cell.store( value, order );
  }

  fn fetch_add( &self, n : u64, order : Ordering ) -> Seq
  {
    self.fetch_adds.fetch_add( 1, Ordering::Relaxed );
    self.cell.fetch_add( n, order )
  }

  fn compare_exchange( &self, current : Seq, new : Seq, success : Ordering, failure : Ordering )
  -> Result< Seq, Seq >
  {
    self.compare_exchanges.fetch_add( 1, Ordering::Relaxed );
    self.cell.compare_exchange( current, new, success, failure )
  }
}
  -- every generic SeqCell parameter in the family: a substitution point --
ring_atomic:/// `&( dyn SeqCell + Sync )` at every site, and the three generic `C : SeqCell`
ring_batch:pub fn claim< C : SeqCell >( cursor : &C, count : usize, order : Ordering ) -> BatchClaim
ring_batch:pub fn claim_gated< P : SeqCell, C : SeqCell >
ring_tls:    C : SeqCell,
  -- every struct field typed PaddedCursor: no substitution point --
ring_claim:  cursor : PaddedCursor,
ring_cursor:  producer : PaddedCursor,
ring_cursor:  consumer : PaddedCursor,
ring_publish:  cursor : PaddedCursor,
  -- and every crate that asserts an operation count --
ring_batch ring_tls 
```

---

### AT27 — The Instrument Is Off the Trait, So It Reaches Exactly the Code That Happens to Be Generic

`counts` and `reset_counts` are inherent methods on `CountingSeq`. `SeqCell` has
four methods and neither of them is among them. That is the right call — a
production `AtomicSeq` has nothing to report, and a trait method returning counts
would have to be defaulted to zero or made to lie — but it has a consequence the
crate does not state.

A measurement is only possible where a `CountingSeq` can be substituted for the
cell the code would otherwise use, and substitution requires the code to be
generic. Written out generically, the measurement does not compile at all:

```
error[E0599]: no method named `counts` found for reference `&C` in the current scope
 --> src/bin/offtrait.rs:9:23
  |
9 |   let before = cursor.counts().fetch_adds;
  |                       ^^^^^^ method not found in `&C`
```

So the measurable surface of the family is a census, and it is a short one. Three
generic `SeqCell` parameters exist across all 33 crates — two in `ring_batch`, one
in `ring_tls` — and those two crates are, exactly, the two that assert an operation
count. Four struct fields are typed `PaddedCursor` outright, in `ring_claim`,
`ring_cursor`, and `ring_publish`; none of the three has a substitution point, and
none of the three asserts a count.

**Finding.** The boundary between what the family can measure and what it cannot is
not drawn where measurement is valuable. It is drawn where the code happens to have
been written generically, and the two sides came out backwards.

`ring_batch::claim` — which is measured — is one `fetch_add`, unconditionally, with
no loop and no retry. Its operation count is the constant `1` and can be derived by
reading it. `ring_claim::Claimer::claim` — which cannot be measured — is an
unbounded CAS retry loop (`:399-412`): one `load` to seed `current`, then a
`compare_exchange` per iteration, refreshing from the failure value and going
around again for as long as other producers keep winning. How many exchanges that
costs at eight producers is not derivable by reading it; it is the number an
operation counter exists to produce, and the field declaration at `:252` puts it
permanently out of reach.

The fix is one character wide — `Claimer` could take its cursor by generic
parameter as `ring_batch` does — and nothing records that the choice was ever made
or what it forecloses.

---

### AT28 — Both Contracts Describe an Instant, and the One Test That Knows Better Is the Crate's Own

`counts` is documented in eight words: "What this cell has been asked to do so
far." `reset_counts` promises to "Return every count to zero, leaving the sequence
itself untouched," for "a test that sets up a state through the cell and then wants
to count only what the operation under test does."

Both sentences describe a moment. Neither call is one. `counts` is four `Relaxed`
loads and a sum, so under concurrent traffic its four fields come from four
different instants and `total` from a fifth
([`algorithm/002`](../algorithm/002_counts_is_four_reads_not_one.md) AT3);
`reset_counts` is four `Relaxed` stores in a loop, so a cell being driven while it
is reset ends with counters zeroed at four different points and traffic recorded
between them. "So far" and "every count to zero" are both true only of a quiescent
cell, and the word *quiescent* appears in neither contract.

There is a second, smaller version of the same gap in `reset_counts` specifically,
and it bites immediately:

```
  measuring through the concrete type      : 1 fetch_add
  counters immediately after reset_counts  : 0
  the sequence it left alone               : Seq(40)
  counters after that one confirming read  : 1
```

Checking that the reset preserved the sequence costs a count, because the check is
a `load` and `load` is counted. Anyone following the documented recipe — reset,
confirm the setup survived, then run the operation under test — has already spent
one of the counts they were about to measure.

**Finding.** The crate's own suite knows this and the crate's own documentation
does not. `resetting_the_counts_leaves_the_sequence_alone` asserts the effect
deliberately, on purpose, with a message:

> ```
> assert_eq!( cell.counts(), OpCounts::default() );
> assert_eq!( cell.load( Ordering::Acquire ), Seq( 77 ), "the value survived the reset" );
> assert_eq!( cell.counts().loads, 1, "and counting resumed from zero" );
> ```

The `reset_counts` doctest four hundred lines away runs the same three steps in the
order that dodges it — total, *then* load — and says nothing. A reader who takes the
doctest as the recipe writes the two lines in the natural order and gets a count of
one where they expected zero.

`reset_counts` is also the least-exercised item in the crate: one call site across
all 33 crates, in the test that documents this effect. The stated use case — set up
through the cell, reset, then count the operation under test — has no user
anywhere.

Both contracts now name the quiescent requirement, and `reset_counts`'s also
names the confirming-read cost this instance measures:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A6 -F '/// Return every count to zero, leaving the sequence itself untouched.' ring_atomic/src/lib.rs
```

Live output:

```
    /// Return every count to zero, leaving the sequence itself untouched.
    ///
    /// Four independent `Relaxed` stores, not one atomic reset — safe to read
    /// as complete only while nothing else is touching the cell. For a test
    /// that sets up a state through the cell and then wants to count only what
    /// the operation under test does; confirming the sequence survived costs a
    /// count of its own, since the confirming read is itself counted.
```

**Disposition:** applied — `counts`'s contract in `src/lib.rs` now opens with
the quiescent qualifier, and `reset_counts`'s contract now states it is four
independent stores and that confirming the reset survived costs a count of
its own; the crate's 21 unit tests plus 8 doctests re-verified passing
(`cargo test --all-features`, 2026-09-03). Now prints:
`the operation under test does; confirming the sequence survived costs a`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`algorithm/002`](../algorithm/002_counts_is_four_reads_not_one.md) | The four reads inside one `counts()` call, and the tear between them |
| [`item/001`](001_six_constructors_for_two_types.md) | The six constructors, and the decisions behind them that *are* written down |
| [`invariant/002`](../invariant/002_every_increment_survives.md) | The same two-moments problem across `counts()` and `load()` |
| [`pattern/002`](../pattern/002_the_counting_cell_is_not_a_mock.md) | Why the counts can be trusted at all, and the substitution the pattern depends on |
| [`decisions/002`](../decisions/002_a_trait_because_the_criteria_needed_two.md) | The two criteria this instrument exists to serve, and where they are asserted |

### Sources

| Fact | Where |
|------|-------|
| The four trait methods | `ring_atomic/src/lib.rs:111`, `:114`, `:141`, `:149` |
| The two inherent ones | `ring_atomic/src/lib.rs:410`, `:446` |
| `counts`' eight-word contract | `ring_atomic/src/lib.rs:378-379` |
| `reset_counts`' contract and stated use case | `ring_atomic/src/lib.rs:426-432` |
| The three generic substitution points | `ring_batch/src/lib.rs:221`, `:306`; `ring_tls/src/lib.rs:280` |
| The four concrete cursor fields | `ring_claim/src/lib.rs:259`; `ring_cursor/src/lib.rs:249-250`; `ring_publish/src/lib.rs:87` |
| The unmeasurable CAS retry loop | `ring_claim/src/lib.rs:432-445` |
| The compile error, and the cost of a confirming read | Release probe, quoted above |
| The suite's deliberate assertion of that cost | `ring_atomic/tests/atomic_test.rs:226-227` |

### Tests

| Test | Covers |
|------|--------|
| `resetting_the_counts_leaves_the_sequence_alone` | That the reset zeroes the counters and spares the sequence — and, deliberately, that confirming it costs a count |
| `a_cell_never_touched_counts_zero` | That nothing counts an operation that did not happen |
| `total_is_the_sum_of_the_four_and_not_an_independent_counter` | What `counts()` computes, at rest |
| *(to create)* | Nothing resets a cell that is being driven, which is the case both contracts read as covered |
| *(to create)* | Nothing asserts an operation count for `ring_claim::Claimer::claim`, because its cursor field admits no substitution |
