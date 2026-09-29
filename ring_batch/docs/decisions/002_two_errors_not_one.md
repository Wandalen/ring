# Decisions: Two Errors, Not One

### Scope

**Purpose:** Record why `claim_gated` distinguishes an impossible request from a
currently-blocked one, what the split buys a caller's retry loop, and what the
blocked variant does not carry.

**Responsibility:** `RingError::BatchTooLarge` against `RingError::Full` at the
two return sites, and the classification predicates a caller branches on.

**In Scope:** `ring_batch/src/lib.rs:266-277`, `:316-326`;
`ring_types/src/error.rs:52-54, 63-71`.

**Out of Scope:** The ordering decision is
[`decisions/001`](001_ordering_is_the_callers_except_where_it_is_not.md). The
step order that puts the two checks in this sequence is
[`algorithm/002`](../algorithm/002_check_then_advance.md).

---

## Two Returns, Two Variants

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the two error returns --'
command grep -E 'RingError::(BatchTooLarge|Full)' ring_batch/src/lib.rs | command grep -v '///'
echo '  -- the stated reason --'
command grep -m1 -A2 -F '/// Separating the two is what lets a caller loop on one and give up on the' ring_batch/src/lib.rs
echo '  -- declared, classified, displayed --'
command grep 'BatchTooLarge' ring_types/src/error.rs
```

Live output:

```
  -- the two error returns --
    return Err( RingError::BatchTooLarge { requested : count, capacity : capacity.get() } );
    return Err( RingError::Full );
  -- the stated reason --
/// Separating the two is what lets a caller loop on one and give up on the
/// other. A single `Full` for both would make an impossible request look like a
/// transient one, and a retry loop would spin forever.
  -- declared, classified, displayed --
  BatchTooLarge
      | Self::BatchTooLarge { .. }
      | Self::BatchTooLarge { .. }
      Self::BatchTooLarge { requested, capacity } =>
```

Three lines for the oversized case, one for the blocked one. Line 88 is the
whole decision: `BatchTooLarge` sits inside `is_configuration`'s `matches!` arm,
and `Full` does not.

**Correction (2026-09-28):** the census above now prints four lines for
`BatchTooLarge`, not three. The `Fix(ring_error_classification_not_exhaustive)`
comment at `ring_types/src/error.rs:98-109` rewrote `is_configuration` and
`is_transient` from `matches!()` macros naming only their positive cases into
exhaustive `match` blocks naming every variant, so `is_transient` now also
names `Self::BatchTooLarge { .. }` explicitly, in its `false` arm
(`ring_types/src/error.rs:156`), alongside the pre-existing
`is_configuration` arm. `is_configuration` is no longer a `matches!` call at
all, so "Line 88" and "`matches!` arm" are both stale — superseded by the
Sources table's own `ring_types/src/error.rs:115-118`. The classification
itself is unchanged: `BatchTooLarge` is still a configuration error, `Full`
still is not.

---

### BA15 — The Split Is the One Decision in the Crate With a Test That Names Its Reason

A single `Full` for both cases would be defensible on its own terms — the ring
is not going to serve the request either way. It fails on the caller's side: a
retry loop cannot distinguish "wait for a consumer" from "you asked for more
than the ring holds," and the second never clears.

The crate does not just document that. `ring_types::RingError` carries
`is_transient` and `is_configuration`, and the test asserts against both:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A7 -F '  let outcome = claim_gated( &producer, &consumer, 9, capacity, Ordering::AcqRel );' ring_batch/tests/batch_test.rs
```

Live output:

```
  let outcome = claim_gated( &producer, &consumer, 9, capacity, Ordering::AcqRel );

  assert_eq!( outcome, Err( RingError::BatchTooLarge { requested : 9, capacity : 8 } ) );
  let error = outcome.unwrap_err();
  assert!( error.is_configuration(), "a caller must not retry this" );
  assert!( !error.is_transient() );
  assert!( RingError::Full.is_transient(), "and must retry the other" );
}
```

**Finding.** The assertion is on the *classification*, not the variant. A test
that only checked `Err( BatchTooLarge { .. } )` would pass if the variant were
later reclassified as transient, which is the change that would actually break a
caller's loop. Asserting `is_configuration()` and `!is_transient()` pins the
property the split exists for rather than the name it happens to have.

This is the crate's only decision with that shape — `decisions/001`'s ordering
argument has no test at all, and the size-check ordering in
[`algorithm/002`](../algorithm/002_check_then_advance.md) BA3 is pinned by
operation counts rather than by a named property.

---

### BA16 — `Full` Carries Nothing, and the Crate That Rebuilt This One Grew a Function For It

`BatchTooLarge { requested, capacity }` tells a caller exactly how it was wrong.
`Full` is a unit variant: no free-slot count, no cursor positions, no hint. A
caller that wanted 64 and could have used 12 has no way to learn that from the
error, and must re-read both cursors and recompute `free_slots` itself — the
same two loads `claim_gated` just performed and discarded.

One tier up, `ring_claim` solved that by adding an operation rather than a
payload:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A9 -F '  /// Claim as many of `max` sequences as are available, down to one.' ring_claim/src/lib.rs
```

Live output:

```
  /// Claim as many of `max` sequences as are available, down to one.
  ///
  /// For a batching producer that would rather write four items now than wait
  /// for room for eight.
  ///
  /// # Errors
  ///
  /// [`RingError::Full`] when not even one slot is free. Never
  /// `BatchTooLarge` — a `max` wider than the ring is not an error here, it is
  /// simply more than will be granted.
```

**Finding.** `claim_up_to` is the answer to "how much could I have had", asked
as a request rather than read out of an error. Its doc comment goes further and
retires `BatchTooLarge` for that entry point entirely — a `max` wider than the
ring stops being an error and becomes a cap.

`ring_batch` has no counterpart. Its two entry points are all-or-nothing, so a
producer that would rather write four now than wait for eight has to implement
the retry-and-halve loop itself, on top of an error that told it nothing. The
crate that rebuilt this one's range object (see
[`data_structure/002`](../data_structure/002_the_struct_ring_claim_wrote_again.md))
also rebuilt its error contract, and neither crate records the difference.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`algorithm/002`](../algorithm/002_check_then_advance.md) | Where each error is returned in the three-step body |
| [`decisions/001`](001_ordering_is_the_callers_except_where_it_is_not.md) | The crate's other decision, which has no test |
| [`data_structure/002`](../data_structure/002_the_struct_ring_claim_wrote_again.md) | The other half of what `ring_claim` rebuilt |
| [`lifecycle/002`](../lifecycle/002_the_empty_claim_as_a_first_class_state.md) | The zero-count request, which never produces either error |

### Sources

| Fact | Where |
|------|-------|
| The two returns | `ring_batch/src/lib.rs:318`, `:325` |
| The stated reason | `ring_batch/src/lib.rs:275-277` |
| `is_configuration`'s arm, and `Full`'s absence from it | `ring_types/src/error.rs:115-118` |
| `is_transient` | `ring_types/src/error.rs:146-159` |
| `claim_up_to`'s contract | `ring_claim/src/lib.rs:450-459` |

### Tests

| Test | Covers |
|------|--------|
| `an_oversized_request_is_a_configuration_error_not_back_pressure` | Both classifications, by property rather than variant |
| `a_full_ring_refuses_with_full_and_advances_nothing` | That `Full` leaves the cursor where it was |
| `a_consumer_advancing_reopens_the_gate` | That `Full` really is transient |
| *(to create)* | Nothing covers the partial-grant case, because the crate has no operation for it |
