# Pattern: Read Then Report

### Scope

**Purpose:** Record the two-phase protocol shape this crate shares with the
write half, and establish what the two instances have in common and where the
symmetry stops.

**Responsibility:** The "ask what you may do, do it, then say you did" protocol
as a family pattern — its two instances, its shared window, and its asymmetric
enforcement.

**In Scope:** `available` / `commit` against `claim` / `publish`; the window each
opens; the mechanisms guarding each.

**Out of Scope:** The range value both phases exchange — that is
[`001`](001_the_half_open_range_as_a_value.md). Why the split exists, which is
[`decisions/001`](../decisions/001_two_calls_not_one.md).

---

## Two Instances of One Shape

| Phase | Write half | Read half |
|-------|-----------|-----------|
| Ask | `Claimer::claim( count ) -> Result< Claim, RingError >` | `Consumer::available() -> Available` |
| Act | write the slots | read the slots |
| Report | `Publisher::publish( start, len ) -> Seq` | `Consumer::commit( through ) -> Result< Seq, RingError >` |
| Window | between claim and publish | between available and commit |
| What is borrowed in the window | slots nobody else may claim | slots the producer may not overwrite |

The same protocol, run in opposite directions by the two halves of one
handshake. `ring_publish/tests/handshake_test.rs` runs all four operations
against each other and is the only test in the family that exercises the whole
shape.

### CN41 — Neither Report Operation Takes the Value Its Ask Produced

```sh
cd "$(git rev-parse --show-toplevel)"
grep -E '^\s*pub (const )?fn' ring_publish/src/lib.rs
grep -E '^\s*pub fn (claim|claim_up_to)' ring_claim/src/lib.rs
grep -E '^\s*pub fn (available|commit)' ring_consume/src/lib.rs
```

Live output:

```
  pub fn new() -> Self
  pub const fn cursor( &self ) -> &PaddedCursor
  pub fn published( &self ) -> Seq
  pub fn try_publish( &self, start : Seq, len : usize ) -> Result< Seq, Seq >
  pub fn publish( &self, start : Seq, len : usize ) -> Seq
  pub fn is_published( &self, seq : Seq ) -> bool
  pub fn claimed( &self ) -> Seq
  pub fn claim( &self, count : usize ) -> Result< Claim, RingError >
  pub fn claim_up_to( &self, max : usize ) -> Result< Claim, RingError >
  pub fn available( &self ) -> Available
  pub fn available_up_to( &self, max : u64 ) -> Available
  pub fn commit( &self, through : Seq ) -> Result< Seq, RingError >
  pub fn commit_available( &self ) -> Seq
```

The signatures that matter:

```
ring_publish:161  pub fn try_publish( &self, start : Seq, len : usize ) -> Result< Seq, Seq >
ring_publish:200  pub fn publish( &self, start : Seq, len : usize ) -> Seq
ring_claim:421    pub fn claim( &self, count : usize ) -> Result< Claim, RingError >
ring_consume:425  pub fn commit( &self, through : Seq ) -> Result< Seq, RingError >
```

`claim` produces a `Claim`. `publish` does **not** take one — it takes a
decomposed `( Seq, usize )` pair. `available` produces an `Available`. `commit`
does not take one either — it takes a bare `Seq`.

So on both sides the protocol is *asked* in terms of a range value and
*reported* in terms of loose integers, and the value the ask produced has no
type-level connection to the report. Four consequences, and the first is the one
that reframes a finding in a sibling crate:

**`Claim`'s severe `must_use` is not a supplement to a type-level guarantee —
it is the entire mechanism.** `ring_claim`'s corpus already recorded (CL21) that
the family's most severe `must_use` message has no runtime mechanism behind it.
The protocol-level view shows why: a `publish` that consumed the `Claim` would
have made dropping one a compile-time-visible mistake, and `publish`'s signature
forecloses that. The annotation is carrying the whole obligation because the
signature declined to.

**Publishing a range nobody claimed compiles.** `publish( Seq( 900 ), 12 )` is a
legal call against any `Publisher`. Nothing links it to a grant.

**Committing a range nobody was offered compiles.** Identically, and this crate's
guard catches only the out-of-range subset
([`decisions/001`](../decisions/001_two_calls_not_one.md) CN7).

**Both halves could have been shaped the other way, and the family knows how.**
`ring_spsc` does it, twice, with drop-guards whose messages name the cost:

```
ring_spsc/src/lib.rs:731: "a reservation publishes on drop; dropping it immediately publishes an unwritten slot"
ring_spsc/src/lib.rs:960: "a batch commits on drop; dropping it immediately discards the records it covers"
```

That is the tradeoff a consuming signature or a drop-guard would face here, and
it is real — a guard that reports on drop reports whether or not the work was
done. Neither Tier 5 crate records that the choice was considered.

**Cost:** reachable, and symmetric across both halves. The decomposed report
signature is what leaves `Claim`'s obligation resting entirely on a lint and
`Available`'s ordering hazard resting on nothing.

---

### CN42 — The Two Halves Have Opposite Fallibility, in Both Phases

| | Ask | Report |
|--|-----|--------|
| Write half | `claim` — **fallible** (`Result< Claim, RingError >`) | `publish` — **infallible** (`-> Seq`) |
| Read half | `available` — **infallible** (`-> Available`) | `commit` — **fallible** (`Result< Seq, RingError >`) |

An exact inversion, and every cell of it is correct:

| Cell | Why |
|------|-----|
| `claim` fallible | the ring can genuinely be full; the caller must back off |
| `available` infallible | "nothing to read" is an empty run, not an error — an idle ring is the steady state |
| `publish` infallible | `try_publish` exists for the checked form; `publish` is the one that asserts |
| `commit` fallible | the argument is caller-supplied and can be out of range in two directions |

So the fallibility tracks *where a caller can be wrong*, which is the right
criterion, and it lands on opposite phases in the two halves because the two
halves can be wrong at opposite moments. A producer can fail to get space; a
consumer cannot fail to find out how much there is. A consumer can name a bad
sequence; a producer using `publish` has already been given the range and is
asserting it.

What the inversion costs is that the four operations do not look like one
protocol from the outside. A caller writing the loop sees `?` on `claim`, no `?`
on `publish`, no `?` on `available`, and `?` on `commit` — and there is no
document anywhere stating that the pattern is deliberate rather than four
independent decisions.

The remaining asymmetry is `try_publish`, which returns `Result< Seq, Seq >` —
an error type that is a sequence rather than a `RingError`. `ring_consume` has
no equivalent checked/unchecked pair; `commit` is the checked form and
`commit_available` is the unchecked one, differing in argument rather than in
error handling ([`api/002`](../api/002_the_two_commits.md)). Three crates, three
different shapes for "the checked and unchecked versions of the report."

**Cost:** reachable as a documentation gap. Every cell is individually right;
nothing states that the pattern is a pattern, and the checked/unchecked
convention differs in all three crates that have one.

---

## Cross-References

| Definition | Instance | Relationship |
|------------|----------|--------------|
| pattern | [001](001_the_half_open_range_as_a_value.md) | the value the two phases exchange |
| decisions | [001](../decisions/001_two_calls_not_one.md) | why the split exists at all |
| lifecycle | [001](../lifecycle/001_a_sequence_from_published_to_committed.md) | the unobservable transition both windows contain |
| api | [002](../api/002_the_two_commits.md) | the report phase's two forms |
| integration | [001](../integration/001_four_edges_in_and_none_out.md) | the handshake test that exercises all four |

### Sources

| What | Where |
|------|-------|
| The read half | `ring_consume/src/lib.rs:336,425` |
| The write half | `ring_claim/src/lib.rs`, `ring_publish/src/lib.rs` |
| The four-operation test | `ring_publish/tests/handshake_test.rs` |
| The drop-guard precedent | `ring_spsc/src/lib.rs:731, 960` |

### Tests

| Claim | Verified by |
|-------|-------------|
| `publish` does not take a `Claim` | `ring_publish/src/lib.rs:200` — `( start : Seq, len : usize )` |
| `commit` takes a bare `Seq` | `ring_consume/src/lib.rs:425` |
| `available` cannot fail | its return type is `Available`, not `Result` |
| `publish` cannot fail | `ring_publish/src/lib.rs:200` returns `Seq` |
| Drop-guards exist in the family | the two messaged `must_use`s in `ring_spsc` |
