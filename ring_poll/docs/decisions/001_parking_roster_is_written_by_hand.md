# `PARKING_CRATES` stays a hand-written list that a test diffs against the manifests, not a generated one

Status: Accepted

## Context

`ring_poll::PARKING_CRATES` names the family crates that declare `ring_wait` as a direct dependency. Its doc
comment tables the crates that reach `ring_wait` only through an intermediate. No crate on the tick path may reach
any of them.

The set is derivable. `cargo metadata` or a build script could compute it from the manifests. Correctness does
not depend on the choice, because two tests already read the manifests off disk:

- `the_tick_path_cannot_reach_a_parking_operation` compares the crates whose manifests name `ring_wait` with the
  roster.
- `the_transitive_reach_is_wider_than_the_manifest_scan_can_see` walks `[dependencies]` to its closure, compares
  the result with a hand-written list of reaching crates, and asserts that no tick-path crate is in it.
  Dev-dependencies are excluded, because a dev-dependency is on no caller's tick path.

A stale roster fails the suite either way. The choice decides what the failure says.

## Decision

Keep the list hand-written. When a crate gains a path to `ring_wait`, a test fails and a person has to decide
whether the change was intended, then edit a public constant. A generated roster is always correct, so it would
regenerate to match, the suite would stay green, and nobody would be asked. The hand-written list is a tripwire,
and its value is that it goes off when the graph moves.

The roster names direct declarers, not every crate that reaches `ring_wait`. Its doc comment says so and lists the
transitive cases, and the second test pins them.

## Alternatives considered

- **Generate the roster from `cargo metadata` or a build script.** Always correct, and therefore silent at the
  moment a person should look. No crate in the family has a build script, so this would add new machinery. The
  closure walk in the second test already does most of what generation would need.
- **Widen the roster to every crate that reaches `ring_wait`.** This answers the question a scheduler author asks,
  which is which crates can end up sleeping. It also changes the array's public type, and the second test already
  pins the reach set.

## Consequences

- Hand-kept lists are a known weak spot in this family. `ring_types`' variant list has the same shape, and this
  crate now keeps two, the roster and the reach list in the transitive test.
- The tripwire works only while the `assert_eq!` diff stays readable. A long list would turn the failure from a
  signal into an obstacle.
- The original deferral expected `ring_bench` to bring a build-time metadata step that generation could reuse.
  `ring_bench` was written without one.
- Revisit when either hand-written list reaches six names. Then generate the set and move the tripwire to a gate
  that diffs the generated set against a declared-intent file, which keeps the human decision without the
  hand-typed list. Six is a guess at where a diff stops being readable, not a measurement.
