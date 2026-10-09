# Overflow policies are decided per backend

Status: Accepted

## Context

`OverflowPolicy` has three variants (`Fail`, `DropNewest`, `DropOldest`) and
`ring_core` serves them over three backends (SPSC, MPSC, interim crossbeam).
Not every combination can exist: evicting an unread record contradicts the
exactly-once delivery both in-house rings guarantee, while the crossbeam
queue does it natively with `force_push`. An undecided matrix invites two
failure shapes: a policy silently behaving as another (a `DropOldest` that
discards the newcomer), or a policy nobody exercises rotting until a caller
depends on it.

## Decision

- `Fail` and `DropNewest` are wired on every backend through the one refusal
  rule in `Producer::try_push`: a refused claim resolves through
  `ring_overflow::would_resolve`, `Fail` hands the record back as `Err` and
  `DropNewest` discards it as `Ok`. Both are covered by tests
  (`drop_newest_discards_the_incoming_record_without_an_error`,
  `the_same_program_behaves_identically_on_every_backend`) and by the
  `overflow` benchmark suite (`fail_retry`, `drop_newest`).
- `DropOldest` is refused at construction on the in-house backends with
  `RingError::PolicyUnsupported` (`drop_oldest_is_rejected_by_the_in_house_backends`).
  A construction error beats a push that quietly behaves as `DropNewest`: the
  caller fixes their config instead of losing records they believe kept.
- `DropOldest` is honoured only on the interim crossbeam backend, via
  `force_push` (`crossbeam_honours_drop_oldest_by_evicting`). When the interim
  backend leaves, the policy needs a native implementation or a second refusal.
- `Block` and overwrite-on-full do not exist and are not planned: both are
  per-priority-class decisions the family deliberately leaves unresolved, and
  overwrite additionally violates exactly-once. A fourth variant must arrive
  with its own decision record, not as an enum addition.

## Alternatives considered

- **Implement eviction natively.** Requires taking an unread record out from
  under a consumer that may be reading it — a second ownership protocol over
  the one the claim/publish guards already enforce. Rejected until a caller
  needs it enough to fund that protocol.
- **Accept `DropOldest` natively and behave as `DropNewest`.** Same signature,
  opposite data loss. Rejected: silent misbehaviour is worse than a loud refusal.

## Consequences

- `Resolution::EvictedOldest` is unconstructible through `ring_core`'s native
  backends. It stays because it is the only value for which
  `accepted_incoming` is true; deleting it would collapse that predicate.
- Every `OverflowPolicy` variant is now either exercised (tests plus the
  `overflow` benches) or refused with a reason. A new variant compiles
  against nothing silently: the wildcard-free matches in `ring_overflow` and
  the `ALL` length assertions stop it.
