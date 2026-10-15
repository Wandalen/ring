# `Script::run` stays on `u32` records until a consumer needs to drive its own record type

Status: Deferred

## Context

`ring_testkit::Script::run` takes `&mut Ring<u32>` and mints its own records, consecutive `u32`s from `0`, one per
pushed or staged record. A caller whose real records are some `MyEvent` can drive the same shape of sequence and read
the `Outcome`, but cannot push its own records through a script.

The crate's claim is that one script run against two equivalent rings produces equal `Outcome`s. Anything that lets
two runs mint different records would break that claim without the ring being at fault.

The other half of the crate is already generic. `ring_testkit::leak` and `ring_testkit::leak_ends` accept a `Ring<T>`
for any `T: Send`, because they mint nothing. They take a ring the caller built and hand back what a loom thread
needs. The open question is therefore about `Script::run` alone.

## Decision

Keep `Script::run` on `Ring<u32>`, and defer a generic record type until a consumer needs one. Minting consecutive
integers makes "two runs mint the same values" true by construction, with no caller input that could make it false.
Until a record has behaviour the fixture must exercise, it is a label, and a `u32` is the cheapest label that is also
ordered. The audit functions rely on that order.

## Alternatives considered

- **A caller-supplied closure that produces records.** Every call is a chance for two runs to differ, so determinism
  would rest on the caller's care instead of on the fixture.
- **A seed or an iterator.** The same objection in a different form.
- **A generic `Script::run<T>` that keeps the audit whole.** `Outcome::received` and `Outcome::published` would become
  `Vec<T>` with `T: PartialEq + Debug`, and minting would need one of the two sources above. `Anomaly::Unminted` and
  `Anomaly::OutOfOrder` carry record values, so `Anomaly` would become generic or switch to positions. The provenance
  check is the part with no mechanical answer. `Outcome::audit`, `ring_testkit::audit_received` and
  `ring_testkit::audit_received_unordered` all test `value >= minted`, which asks whether this run could have minted
  the value. No standard trait answers that for an arbitrary `T`, and `PartialOrd` would type-check while meaning
  nothing for a `MyEvent`.

## Consequences

- A consumer cannot use a script to count `Drop` runs of its own record type or to check payload bytes on the way
  out. It can call `leak_ends` and write the loom model by hand.
- Going generic needs a design decision about provenance, not only new trait bounds. The candidates are a minting
  trait the fixture defines, dropping the provenance check for non-`u32` records, or a caller-supplied predicate,
  which brings back the closure alternative.
- Revisit when a consumer's record type has behaviour the fixture needs to exercise, such as a `Drop` impl whose runs
  should be counted or a payload whose bytes should be checked on exit.
