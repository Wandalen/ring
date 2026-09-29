# Invariant: The Cursor Only Moves Forward

### Scope

**Purpose:** State the consumer's own half of the handshake discipline — that
its cursor is monotonic — identify what enforces it, and record a defect in the
verification that covers it.

**Responsibility:** The monotonicity of the consumer cursor: the guard that
holds it, the exhaustive test that verifies it, and the manual check that names
that test.

**In Scope:** `commit`'s lower bound; the exhaustive sweep
`the_accepted_commits_are_exactly_the_available_range_inclusive_of_both_ends`;
`tests/manual/readme.md § N6`.

**Out of Scope:** The upper bound and what it protects — that is
[`001`](001_never_reads_past_what_was_published.md). The single-writer premise,
which is
[`decisions/002`](../decisions/002_plain_stores_rather_than_compare_exchange.md).

---

## The Property

> A consumer's cursor never takes a value less than one it has already held.

It is the mirror of [`001`](001_never_reads_past_what_was_published.md): that one
stops the consumer running ahead of the producer, this one stops it falling
back. Both refusals live in the same `if`, and the module documentation argues
them together:

> Moving backwards re-reads slots the producer has already been told it may
> reuse.

Once the cursor passes a sequence, the producer is free to overwrite that slot.
A cursor that moved backwards would report those slots as unread, and a consumer
acting on the report would read whatever the producer has since written there.

### CN12 — The Guard Is the Only Enforcement, and It Guards Only One of Two Doors

```sh
cd "$(git rev-parse --show-toplevel)"
# unnumbered deliberately: the filter runs first, so any `-nE` here would number
# the *filtered* stream — offsets that look like source addresses and are not
command grep -vE "^[[:space:]]*//" ring_consume/src/lib.rs \
  | command grep -E '\.store\(|run\.start'
```

Live output:

```
    Available::new( run.start(), run.len().min( max ) )
    if through < run.start() || through > run.end()
    self.cursor.store( through, COMMIT );
    debug_assert!( end >= run.start(), "available() must never return end < start" );
    if end != run.start()
      self.cursor.store( end, COMMIT );
```

The lower bound `through < run.start()` is the whole mechanism, and `run.start()`
is the cursor's current value — so the guard is literally "refuse to go
backwards from where we are." It is correct, it is checked in release, and
`committing_backwards_is_refused_and_moves_nothing` asserts both halves of its
name.

The second door is `commit_available`, which does not run that guard at all
([`pitfall/001`](../pitfall/001_commit_available_does_not_call_commit.md) CN43).
It cannot move the cursor backwards — its value is `available().end()`, which is
`start + len` and therefore never less than `start`. So monotonicity held
through both doors even before this finding, but for two different reasons: one
was checked in release, one was arithmetic.

That distinction is what makes the duplication in CN43 matter for this
invariant specifically. If `available()` ever returned a run whose `end` could
precede its `start` — a wraparound bug in `ring_seqno::pending`, an underflow in a
future `available_from`-style addition — `commit`'s guard would catch it and
`commit_available` would store it. The invariant would fail through the
unguarded door while the guard that exists for it watched the other one.

The one-line strengthening this finding asked for is a `debug_assert!` on the
run's own two ends inside `commit_available`, or the delegation CN43 discusses.
The first of the two has since been written — it is the line the recipe below
prints, and the Disposition records it. CN43's delegation still has not.

**Cost:** was reachable, and conditional on a defect elsewhere. The invariant
held either way; one of its two entry points held it by construction rather
than by check, and nothing recorded which. The `debug_assert!` closes that in
debug builds; in release the second door is still arithmetic, which is what
keeps CN43 live.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -F 'debug_assert!( end >= run.start()' ring_consume/src/lib.rs
```

Live output:

```
    debug_assert!( end >= run.start(), "available() must never return end < start" );
```

**Disposition:** applied — `commit_available` now carries exactly the
one-line strengthening this finding names, `debug_assert!( end >= run.start()
… )`, so the second door is checked rather than relying solely on
`available()`'s arithmetic. The crate's 22 tests (1 `allocation_test.rs` + 21
`consume_test.rs`) plus 17 doctests re-verified passing (`cargo test
--all-features`, 2026-09-04). Now prints:
`debug_assert!( end >= run.start(), "available() must never return end < start" );`

---

### CN13 — The Manual Check Named a Test That Did Not Exist

`tests/manual/readme.md § N6` was a good check with a broken reference.

```sh
cd "$(git rev-parse --show-toplevel)"

# the name the check used to use. Counted, not listed, so the answer prints as
# a zero rather than as nothing: a stage that emits no output at all is
# indistinguishable from a stage whose pattern is simply wrong.
command grep -c 'commit_accepts_exactly_the_reachable_range' \
  ring_consume/tests/manual/readme.md ring_consume/tests/consume_test.rs \
  | sed 's|ring_consume/tests/|  hits for the old name in |'

# every test name that does exist
command grep -A1 '^#\[ test \]' ring_consume/tests/consume_test.rs \
  | command grep '^fn ' | sed 's/fn /  /;s/()$//'
```

Live output:

```
  hits for the old name in manual/readme.md:0
  hits for the old name in consume_test.rs:0
  an_available_run_is_half_open
  an_empty_run_starts_and_ends_in_the_same_place
  sequences_yields_exactly_the_run
  nothing_is_available_before_anything_is_published
  available_grows_as_publication_advances
  available_shrinks_as_the_consumer_commits
  available_starts_where_the_consumer_stands
  a_consumer_with_no_dependencies_never_has_anything_available
  available_is_bounded_by_the_slowest_of_several_dependencies
  available_up_to_caps_without_changing_the_start
  available_up_to_zero_is_empty_not_everything
  committing_exactly_what_is_available_succeeds
  committing_past_what_is_available_is_refused_and_moves_nothing
  committing_backwards_is_refused_and_moves_nothing
  committing_where_you_already_are_is_allowed_and_is_a_no_op
  the_accepted_commits_are_exactly_the_available_range_inclusive_of_both_ends
  commit_available_takes_everything_and_reports_where_it_reached
  the_consumer_cursor_is_what_the_producer_would_gate_on
  the_consumer_exposes_the_barrier_it_was_built_over
  a_consumer_never_reads_past_what_was_published
  every_sequence_is_offered_exactly_once_across_a_full_drain
```

The old name now reads zero in both files, which is the fix landing rather than
the finding evaporating: when this was written it read one in the manual plan
and zero in the test file. The test the check was describing existed the whole
time, under the name
`the_accepted_commits_are_exactly_the_available_range_inclusive_of_both_ends`
listed above. Neither name is a substring of the other, so no search a reader
performed could connect them.

What worked and what did not:

| `§ N6` | Status, as found |
|--------|------------------|
| its `grep` command | ✔ worked — it searches for `let accepted = consumer.commit`, which is in the sweep's inner loop |
| its expected shape | ✔ correct — a `Consumer::new` inside that inner loop, under its explanatory comment |
| its Run Record entry | ✔ honest — N6 was run and confirmed |
| the test name in its prose | ✘ **did not exist** |

So the check passed and would have kept passing, while the sentence introducing
it — *"The sweep in `commit_accepts_exactly_the_reachable_range` is the crate's
strongest test"* — sent a reader looking for something that was not there. The
failure was invisible to the check's own procedure, because the procedure never
uses the name.

This is the corpus's recurring shape arriving in the verification machinery
itself: *the code is right and the prose is not.* `ring_claim`'s corpus recorded
eleven of thirteen definitions reaching that conclusion independently, including
two doc sentences that are measurably false and one accessor naming a caller
that does not exist. Here the false sentence is inside the document whose job is
to catch exactly this class of drift.

It is also the cheapest possible fix — one identifier, in one line, in a file
that is already maintained. It is recorded because the run record dated
2026-08-28 says `N1–N6, 6/6 as expected`, and a check whose prose names a
non-existent test can report 6/6 forever without anyone noticing.

**Cost:** reachable. The check is sound and the reference is stale; a reader
following `§ N6` to the crate's strongest test finds nothing, and the check's
own pass/fail signal cannot detect it.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'the_accepted_commits_are_exactly_the_available_range_inclusive_of_both_ends' \
  ring_consume/tests/manual/readme.md ring_consume/tests/consume_test.rs
```

Live output:

```
ring_consume/tests/manual/readme.md:The sweep in `the_accepted_commits_are_exactly_the_available_range_inclusive_of_both_ends`
ring_consume/tests/consume_test.rs:fn the_accepted_commits_are_exactly_the_available_range_inclusive_of_both_ends()
```

**Disposition:** applied — `tests/manual/readme.md § N6` now names the test
that actually exists in `consume_test.rs`, instead of a name that is a
substring of neither; the recipe itself is corrected from bare `grep` to
`command grep`, so the shim cannot reorder the two files' hits and the evidence
is which file each name appears in — the file is what the finding turns on, and
a line number would only go stale (re-run 2026-09-04). Now prints:
`consume_test.rs:fn the_accepted_commits_are_exactly_the_available_range_inclusive_of_both_ends`

---

## What Holds It Up

| Layer | Contribution |
|-------|--------------|
| `commit`'s lower bound | refuses any `through < run.start()` |
| `commit_available`'s arithmetic | `end = start + len`, so never backwards — unchecked |
| single writer | no other thread can store a stale value — [`decisions/002`](../decisions/002_plain_stores_rather_than_compare_exchange.md) |
| `COMMIT` = `Release` | the store is visible to the producer with the reads that preceded it |
| the exhaustive sweep | every (position, candidate) pair up to the frontier, fresh consumer each |

---

## Cross-References

| Definition | Instance | Relationship |
|------------|----------|--------------|
| invariant | [001](001_never_reads_past_what_was_published.md) | the upper bound, sharing the same `if` |
| pitfall | [001](../pitfall/001_commit_available_does_not_call_commit.md) | the unguarded second door |
| algorithm | [002](../algorithm/002_the_two_sided_guard.md) | the guard's mechanics |
| decisions | [002](../decisions/002_plain_stores_rather_than_compare_exchange.md) | why one writer makes a plain store safe |
| lifecycle | [001](../lifecycle/001_a_sequence_from_published_to_committed.md) | monotonicity as a property of the state machine |

### Sources

| What | Where |
|------|-------|
| The lower bound | `ring_consume/src/lib.rs`, `if through < run.start()` in `commit` |
| The exhaustive sweep | `ring_consume/tests/consume_test.rs`, `the_accepted_commits_are_exactly_the_available_range_inclusive_of_both_ends` |
| The fresh-consumer construction | the same test, the `Consumer::new` in its inner loop |
| The reference that was stale | `ring_consume/tests/manual/readme.md § N6`, the sentence naming the sweep |
| The monotonicity argument | `ring_consume/src/lib.rs`, module doc |

### Tests

| Claim | Verified by |
|-------|-------------|
| Backwards commits are refused, cursor unmoved | `committing_backwards_is_refused_and_moves_nothing` |
| Every (position, candidate) pair is swept | `the_accepted_commits_are_exactly_the_available_range_inclusive_of_both_ends` |
| Each sweep case gets a fresh consumer | the same test's inner loop, which constructs a `Consumer` per case |
| `§ N6`'s test name did not exist | `command grep -c` for the old name over both files → 0 and 0, above |
| `§ N6`'s grep still works | `let accepted = consumer.commit` is still in that sweep, with the `Consumer::new` above it |
