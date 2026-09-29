# Algorithm: The Two-Sided Guard

### Scope

**Purpose:** Set out what `commit` checks before it stores, establish that both
of its refusals are documented and tested, and identify what the check cannot
see.

**Responsibility:** `Consumer::commit`'s guard — its two bounds, its inclusive
endpoints, its error contract, and its cost.

**In Scope:** `ring_consume/src/lib.rs:379-431`; the `# Errors` section;
the tests covering each bound.

**Out of Scope:** The store itself and its ordering — that is
[`decisions/002`](../decisions/002_plain_stores_rather_than_compare_exchange.md).
The sibling that skips the guard, which is
[`pitfall/001`](../pitfall/001_commit_available_does_not_call_commit.md).

---

## Both Ends, Inclusive

```rust
pub fn commit( &self, through : Seq ) -> Result< Seq, RingError >
{
  let run = self.available();
  if through < run.start() || through > run.end()
  {
    return Err( RingError::Empty );
  }

  self.cursor.store( through, COMMIT );
  Ok( through )
}
```

The accepted set is `[ run.start(), run.end() ]` — inclusive at both ends, which
is deliberate and slightly unusual given that `Available` itself is half-open:

| `through` | Accepted | Meaning |
|-----------|:--------:|---------|
| `< start` | ✘ | backwards — re-reads slots the producer may already have reused |
| `== start` | ✔ | commit nothing; a legal no-op |
| in between | ✔ | commit part of the run |
| `== end` | ✔ | commit the whole run |
| `> end` | ✘ | frees slots the consumer has not read |

`end` is the first sequence *not* in the run, and committing to it is correct —
`commit( end )` means "everything before `end` has been read", which is exactly
the whole run. The inclusive upper bound and the half-open range agree because
`commit` names an exclusive boundary while `Available` names a span. That is
easy to get wrong in either direction and the test suite pins it:
`the_accepted_commits_are_exactly_the_available_range_inclusive_of_both_ends`
asserts the whole set by name.

### CN16 — The `# Errors` Section Documents Both Conditions, Which the Write Half Does Not

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/# Errors/,/^  \/\/\/ ```$/p' ring_consume/src/lib.rs | head -12
```

Live output:

```
  /// # Errors
  ///
  /// [`RingError::Empty`] when `through` is past what is available: the
  /// consumer would be freeing slots it has not read. [`RingError::Empty`] also
  /// when `through` is behind the current position, which would re-read slots
  /// the producer has already been cleared to reuse.
  ///
  /// ```
```

Quoted from the source:

> [`RingError::Empty`] when `through` is past what is available: the
> consumer would be freeing slots it has not read. [`RingError::Empty`] also
> when `through` is behind the current position, which would re-read slots
> the producer has already been cleared to reuse.

One error value, two distinct causes, both named, each with its consequence
spelled out. And the doctest immediately below asserts both:

```rust
assert_eq!( consumer.commit( Seq( 5 ) ), Err( RingError::Empty ), "not read yet" );
assert_eq!( consumer.commit( Seq( 3 ) ), Ok( Seq( 3 ) ) );
assert_eq!( consumer.commit( Seq( 2 ) ), Err( RingError::Empty ), "backwards" );
```

This is the direct contrast with the write half, and the reason it is recorded
here. `ring_claim`'s corpus found that `RingError::Full` is returned from two
sites reporting three distinct situations, and that the documented advice is
correct for exactly one of them — a caller who reads the docs and retries is
right in one case out of three. `ring_consume` returns one error value from one
site for two situations, documents both, and tests both.

Same family, same error type, same "one value, several causes" problem, opposite
outcomes. The difference is not subtlety of the code — `commit`'s two cases are
if anything harder to tell apart from the outside than `claim`'s three, since
both produce the identical `Empty`. The difference is that someone wrote the
second sentence.

**Cost:** none. Recorded as the family's model for documenting an overloaded
error, and as evidence that the `Full` gap in `ring_claim` is an omission rather
than a house style.

---

### CN17 — A Refused Commit Costs Exactly As Much As an Accepted One

The guard's first statement is `let run = self.available();`. So every call to
`commit` — including every call that will be refused a line later — performs the
full available-range computation: one load of its own cursor, one load per
barrier dependency, and — until commit `b7e075ca` — one heap allocation inside
`ring_cursor::slowest`
([`non_functional_requirement/001`](../non_functional_requirement/001_what_the_read_path_costs.md)
CN34).

There is still no cheap rejection path. `commit( Seq( u64::MAX ) )` reads every
dependency before it refuses. What changed is the size of what it wastes, not
whether it wastes it: the finding is that the guard runs after the work, and
that is a property of the source line, not of what the line costs this month.

For the intended usage this is irrelevant — a caller commits what `available`
just told it, so the error branch is never taken. It matters for the two shapes
that do hit it:

**A caller validating input.** Code that accepts a sequence from elsewhere and
uses `commit`'s `Err` as the validity check pays the full read-path cost per
rejection. Nothing in the documentation suggests `commit` is expensive, and its
signature — a comparison against a range — reads as though it should be nearly
free.

**A caller retrying.** `Empty` from the "past what is available" branch is a
transient condition: the producer may publish more at any moment. A caller
looping on `commit` until it succeeds is doing exactly what the error invites,
and re-deriving on every attempt a range it could have asked for directly with
`available()`. That used to cost an allocation per attempt; it now costs one
load per dependency per attempt, which is cheaper and just as unnecessary.

The second is the one worth naming, because the documented advice implicitly
encourages it. `Empty` means "not yet" in one of its two cases and "never" in
the other, and the `# Errors` text distinguishes them clearly — but a caller
cannot distinguish them *at runtime*, since both produce the identical value.
So a caller that wants to retry the transient case has no way to avoid also
retrying the permanent one, and the retry loop it writes will spin forever on a
backwards commit that can never succeed — allocating on every turn, before
`b7e075ca`.

The fix is not in this crate: distinguishing the two would need a second
`RingError` variant, which is `ring_types`' decision. What this crate could do
is say so — one sentence noting that the two conditions are indistinguishable
to a caller, so `Err` should be treated as a caller bug rather than a retry
signal.

**Cost:** reachable. The guard is correct and well documented; what is
undocumented is that its error is not safely retryable and that each attempt is
not free.

---

## What Is Correctly Absent

| Not present | Correctly so |
|-------------|--------------|
| a separate `RingError` per branch | that is `ring_types`' surface, not this crate's; and the two share a caller response |
| a `debug_assert!` instead of a check | the condition is caller-supplied, so it must be checked in release too |
| a compare-exchange on the store | one writer — see [`decisions/002`](../decisions/002_plain_stores_rather_than_compare_exchange.md) |
| a fast path skipping `available()` | the guard's bounds *are* the available range; there is nothing to skip |

---

## Cross-References

| Definition | Instance | Relationship |
|------------|----------|--------------|
| algorithm | [001](001_position_frontier_pending.md) | the computation this guard runs first |
| api | [002](../api/002_the_two_commits.md) | the contract both commit forms share |
| pitfall | [001](../pitfall/001_commit_available_does_not_call_commit.md) | the sibling that skips this guard entirely |
| decisions | [002](../decisions/002_plain_stores_rather_than_compare_exchange.md) | what the store after the guard commits to |
| invariant | [002](../invariant/002_the_cursor_only_moves_forward.md) | the property the lower bound enforces |

### Sources

| What | Where |
|------|-------|
| The guard | `ring_consume/src/lib.rs:427-431` |
| The `# Errors` section | `ring_consume/src/lib.rs:379-384` |
| The doctest asserting both | `ring_consume/src/lib.rs:399-401` |
| `ring_claim`'s contrasting `Full` | `ring_claim/docs/pitfall/002` |

### Tests

| Claim | Verified by |
|-------|-------------|
| Both ends inclusive | `consume_test.rs`, `the_accepted_commits_are_exactly_the_available_range_inclusive_of_both_ends` |
| Past-available is refused, cursor unmoved | `consume_test.rs:209-217` |
| Backwards is refused, cursor unmoved | `consume_test.rs:223-232` |
| Committing where you stand is a no-op | `consume_test.rs`, `committing_where_you_already_are_is_allowed_and_is_a_no_op` |
| A refused commit still pays the full read | `commit` calls `available()` before the guard, which loads every dependency; it also allocated, at 1 per call, until `b7e075ca` |
