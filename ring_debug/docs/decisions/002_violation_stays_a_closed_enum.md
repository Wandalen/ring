# `Violation` stays a closed enum, without `#[non_exhaustive]`

Status: Accepted

## Context

`ring_debug::Violation` has one variant per defect the crate checks for, and each variant carries the numbers it was
derived from. A caller that wants to react differently to a consumer ahead of its producer than to a cursor that
went backwards matches on the variants.

The crate's subject is defects nobody anticipated. The variants are the failures someone thought of, and the crate
exists on the premise that a ring can break in ways its own code does not check. A new variant is therefore a
likely change, not a remote one.

Rust offers two shapes for that change:

- A closed enum. A new variant fails to compile at every exhaustive `match` outside the crate.
- `#[non_exhaustive]`. A new variant is additive, and every `match` outside the crate must carry a `_ =>` arm.

The family has no stated rule. `ring_types::RingError` and `ring_testkit::Anomaly` carry the attribute.

## Decision

`Violation` is closed. A caller's `match` names every variant, and a new variant is a compile error at each site
that has to decide what to do with it. The comment in `impl Display for Violation` records that the enum is closed
on purpose.

The `_ =>` arm is the reason. In a diagnostic crate that arm is where an unanticipated defect would be routed, and
the type system would be telling every caller that ignoring it is fine. The premise that makes a new variant likely
is the same premise that makes a default arm expensive. The new variant is by construction the one nobody
anticipated, and so the one least safe to send to a default. A compile break happens once, at a version boundary,
with the compiler pointing at every site. A default arm is silent on every call.

## Alternatives considered

- **Mark `Violation` `#[non_exhaustive]`.** Adding a defect stops being a breaking change, at the price of turning
  "handled all of them" into "handled some and ignored the rest" in every caller. A diagnostic that ignores a
  violation is worse than one that fails to compile when a new violation appears.
- **Follow `ring_types::RingError`.** `RingError` is a public error type crossing crate boundaries, for callers who
  mostly act on its classification (`RingError::is_configuration`, `RingError::is_transient`) rather than on each
  variant. `Violation` callers are the opposite case, and the individual variant is the information they want.

## Consequences

- Adding a variant is a breaking change for every exhaustive matcher. No crate in the family depends on
  `ring_debug`, so today that break lands on the crate's own tests and on code outside the workspace.
- `ring_testkit::Anomaly`, also an enum of properties that did not hold, made the opposite choice so that its next
  variant is additive. The two diagnostic enums disagree, and nothing in the family decides between them.
- Revisit when `ring_debug` is published, so that the break reaches callers outside the workspace, or when the
  family adopts a rule for `#[non_exhaustive]` that covers diagnostic enums.
