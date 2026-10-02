# `Publisher::publish` takes a bare `(start, len)`, so publishing exactly the claimed range is the caller's job

Status: Accepted

## Context

`ring_claim::Claimer::claim` hands a producer a `ring_claim::Claim`, a start and a length. `ring_publish::Publisher`
advances a separate published cursor once the producer has written that range. The protocol is correct only if each
producer follows one rule. It publishes exactly the range `Claimer::claim` returned to it, once, after writing every
slot in it.

Breaking the rule has four forms, and none is detected where the mistake is made:

- Publishing a range that was never claimed. `try_publish` returns `Err(frontier)` forever, and `publish` spins
  forever.
- Publishing the same claim twice. The second call hangs.
- Publishing a `len` other than the claimed one. `try_publish` succeeds for any `len` once `start` is the frontier. A
  shorter `len` strands the rest of the claim, so the next producer hangs in a thread that did nothing wrong. A longer
  one lets consumers read slots nobody wrote.
- Two threads publishing copies of one `Claim`. One of them hangs.

`Publisher::publish` waits on a bare `spin_loop` hint, so each hang also holds a core. Its `# Panics` section adds a
fifth way in, and that one is no caller's fault. A predecessor that drops its claim without publishing stalls every
producer after it.

## Decision

`Publisher::publish(&self, start: Seq, len: usize) -> Seq` and `Publisher::try_publish(&self, start: Seq, len: usize)
-> Result<Seq, Seq>` take two scalars. They take no `Claim`, and no guard publishes on drop. The rule lives in prose,
in the `# Panics` section of `Publisher::publish`, and `try_publish` is the variant for a caller that wants to bound
the wait itself.

The reason is that `ring_publish` stays testable alone. `ring_claim` is a dev-dependency only, and
`tests/publish_test.rs` never names it.

## Alternatives considered

- **Take a `ring_claim::Claim`.** Start and length would arrive from one value, so the wrong-`len` form becomes
  unrepresentable. A cycle is not the obstacle, because `ring_claim` does not depend on `ring_publish`. The cost is
  promoting `ring_claim` to a normal dependency, which ends single-crate testability. It also closes only that one
  form. `Claim` is `Copy`, so publishing one claim twice still compiles. `Claim::new` is public, so
  `publisher.publish(Claim::new(Seq(99), 4))` still compiles and hangs.
- **Make `Claim` linear, consumed by publication.** That would close the double-publish and shared-copy forms. Every
  `Claim` accessor takes `self` by value, and callers write `publish(claim.start(), claim.len())`, so dropping `Copy`
  means changing those accessors to `&self` and touching every caller of `Claimer::claim`, `ring_mpsc` included.
- **A guard that publishes on drop.** This closes every form, including an early return or an unwind between claim
  and publish. It needs a ring to publish into, and `Publisher` holds only a `ring_cursor::PaddedCursor`. Staying
  free of a ring is what keeps it testable alone. The crates that own a ring built the guard. `ring_spsc::Reservation`
  and `ring_mpsc::Reserved` are non-`Copy` and publish in `drop`. The doc on `ring_spsc::Producer::claim` rejects the bare
  claim/publish pair in writing, because no runtime check can tell "claimed and about to publish" from "claimed and
  abandoned".

## Consequences

- The crate's one precondition is a doc comment. Three of the four forms show up as a hung thread with no message.
  The wrong-`len` form shows up one publication later, in another producer, or as unwritten data at the consumer.
- A test can only observe a misuse through `try_publish`. The same call through `publish` hangs the suite.
- No crate depends on `ring_publish`, as a normal or a dev dependency. `ring_spsc` has no use for it at one producer,
  where the frontier is always already at the next range. `ring_mpsc` rejects making one producer wait on another and
  publishes through per-slot stamps instead. So far the obligation has no caller to burden.
- Revisit if `ring_publish` needs `ring_claim` as a normal dependency for some other reason. The testability argument
  is then gone, and taking a `Claim` closes the wrong-`len` form for free.
- Revisit when a claim becomes retractable. A spin budget on `publish` would then have a correct handling for
  exhaustion, which today it lacks.
- Revisit when payloads grow large enough that waiting on a predecessor's slot write dominates, or `Yield` beats
  `Spin`. Nothing has measured this. The only multi-producer test, in `tests/handshake_test.rs`, carries a one-word
  payload.
- Revisit when a tick-path caller appears. A tick has a deadline, and an unbounded spin can miss it.
- Revisit when a consumer needs contention-tolerant publication and cannot own a stamp array the way `ring_mpsc` does.
  That reopens the exact-frontier refusal argued in [`src/lib.rs`](../../src/lib.rs).
