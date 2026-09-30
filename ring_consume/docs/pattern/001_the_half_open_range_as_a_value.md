# Pattern: The Half-Open Range as a Value

### Scope

**Purpose:** Record the shape `Available` shares with two sibling crates, what
each instance does the same, and where they diverge.

**Responsibility:** The "start plus length, `Copy`, sixteen bytes" range value as
a family pattern — its three instances, its invariants, and its one
inconsistency.

**In Scope:** `ring_consume::Available`, `ring_claim::Claim`,
`ring_batch::BatchClaim`; their fields, accessors, and derives.

**Out of Scope:** The layout numbers, which are
[`data_structure/001`](../data_structure/001_sixteen_and_twenty_four.md). The
read-then-report protocol, which is [`002`](002_read_then_report.md).

---

## Three Instances

```sh
cd "$(git rev-parse --show-toplevel)"
grep -A5 'pub struct Claim$'      ring_claim/src/lib.rs
grep -A5 'pub struct BatchClaim'  ring_batch/src/lib.rs
grep -A5 'pub struct Available'   ring_consume/src/lib.rs
```

Live output:

```
pub struct Claim
{
  start : Seq,
  len : usize,
}

pub struct BatchClaim
{
  start : Seq,
  count : usize,
}

pub struct Available
{
  start : Seq,
  len : u64,
}
```

| | `Claim` | `BatchClaim` | `Available` |
|--|--------|--------------|-------------|
| Crate | `ring_claim` | `ring_batch` | `ring_consume` |
| Start field | `start : Seq` | `start : Seq` | `start : Seq` |
| Length field | `len : usize` | `count : usize` | `len : u64` |
| Size | 16 | 16 | 16 |
| `Copy` | ✔ | ✔ | ✔ |
| `end()` derived | ✔ | ✔ | ✔ |
| Iterator over members | ✔ | ✔ | ✔ — `sequences()` |
| `must_use` on the struct | **severe message** | **none** | none |

### CN39 — The Pattern Is Consistent in Everything Except the Length Type

Three crates, written at different times for different halves of the handshake,
converged on the same six decisions: a `Seq` start, an integer length, a derived
end, `Copy`, a sixteen-byte footprint, and an iterator over the members. That is
a strong convergence and it means a reader who understands one understands all
three.

The divergence is the length type: `usize` twice, `u64` once. On any 64-bit
target the three are the same size, so nothing breaks and no test notices. What
it costs is at the seam:

```rust
let claim = claimer.claim( n )?;          // claim.len() -> usize
let run   = consumer.available();          // run.len()   -> u64
```

Code that compares or combines the two halves needs a cast, and there is no
family-wide statement of which type a sequence count is. Both choices are
defensible — `usize` matches slot indices and buffer lengths, `u64` matches
`Seq`'s own width and every `ring_seqno` return — and the two defences point in
opposite directions.

`Available`'s `u64` is arguably the more consistent of the two, since
`ring_seqno::pending` returns `u64`, `Seq::distance_to` returns `u64`, and
`Available::len` is exactly the value `pending` produced. A `usize` there would
require a cast *inside* `available()` rather than at the caller.

So the newest of the three instances is the one that got it right, and the
family now has two of three doing something else, with nothing recording that
the difference exists or which way it should be resolved.

**Cost:** reachable as a friction cost. Casts at every seam between the halves,
no stated convention, and the correct answer already implemented in the minority
instance.

---

### CN40 — `must_use` Is the Only Place the Pattern Deliberately Diverges, and It Is Undocumented

Of the six shared decisions, one carries a distinction rather than a
convergence:

| | Dropping it | `must_use` on the struct |
|--|-------------|--------------------------|
| `Claim` | strands a slot forever; the ring stalls | severe message |
| `BatchClaim` | **the same** | **none at all** |
| `Available` | costs nothing | none — correctly |

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
command grep -r 'must_use = ' ring_*/src/*.rs | sed 's|ring/||'
```

Live output:

```
ring_atomic/src/lib.rs:    #[must_use = "the returned sequence is the claim — dropping it claims a range nobody will use"]
ring_claim/src/lib.rs:#[must_use = "a claimed range that is never published strands its slots and stalls every consumer"]
ring_flush/src/lib.rs:#[must_use = "an ignored outcome is exactly how a misconfigured OnBarrier stays silent"]
ring_shutdown/src/lib.rs:    #[must_use = "a Stopped is the only route to a drain; bind it, or bind `_` to close and nothing else"]
ring_shutdown/src/lib.rs:    #[must_use = "this is the record itself, not a copy — dropping it loses it"]
ring_shutdown/src/lib.rs:#[must_use = "a Wake::Closed means stop, not publish"]
ring_slot/src/lib.rs:    #[must_use = "this is the only way a payload leaves the slot; dropping it here destroys the record"]
ring_spsc/src/lib.rs:#[must_use = "a reservation publishes on drop; dropping it immediately publishes an unwritten slot"]
ring_spsc/src/lib.rs:#[must_use = "a batch commits on drop; dropping it immediately discards the records it covers"]
ring_testkit/src/lib.rs:#[must_use = "nothing frees this — dropping the reference leaks the ring with no way to reach it again"]
ring_testkit/src/lib.rs:#[must_use = "nothing frees either allocation — dropping the pair leaks both with no way to reach them again"]
ring_testkit/src/lib.rs:    #[must_use = "the Outcome is the measurement — `script.run( &mut ring );` as a statement drives the ring and discards everything it observed"]
```

Live output — every messaged `must_use` in all 33 crates:

```
ring_atomic/src/lib.rs:  #[ must_use = "the returned sequence is the claim — dropping it claims a range nobody will use" ]
ring_claim/src/lib.rs:#[ must_use = "a claimed range that is never published strands its slots and stalls every consumer" ]
ring_flush/src/lib.rs:#[ must_use = "an ignored outcome is exactly how a misconfigured OnBarrier stays silent" ]
ring_shutdown/src/lib.rs:  #[ must_use = "a Stopped is the only route to a drain; bind it, or bind `_` to close and nothing else" ]
ring_shutdown/src/lib.rs:  #[ must_use = "this is the record itself, not a copy — dropping it loses it" ]
ring_shutdown/src/lib.rs:#[ must_use = "a Wake::Closed means stop, not publish" ]
ring_slot/src/lib.rs:  #[ must_use = "this is the only way a payload leaves the slot; dropping it here destroys the record" ]
ring_spsc/src/lib.rs:#[ must_use = "a reservation publishes on drop; dropping it immediately publishes an unwritten slot" ]
ring_spsc/src/lib.rs:#[ must_use = "a batch commits on drop; dropping it immediately discards the records it covers" ]
ring_testkit/src/lib.rs:#[ must_use = "nothing frees this — dropping the reference leaks the ring with no way to reach it again" ]
ring_testkit/src/lib.rs:#[ must_use = "nothing frees either allocation — dropping the pair leaks both with no way to reach them again" ]
ring_testkit/src/lib.rs:  #[ must_use = "the Outcome is the measurement — `script.run( &mut ring );` as a statement drives the ring and discards everything it observed" ]
```

Twelve messages in 33 crates, and all twelve name a consequence of *dropping*
the value. That is a coherent, well-applied convention.

`Available` correctly has none: it is a permission, not an obligation
([`data_structure/001`](../data_structure/001_sixteen_and_twenty_four.md) CN22),
and its five method-level bare `#[ must_use ]`s are the right weight for pure
accessors ([`api/001`](../api/001_sixteen_public_items.md) CN19).

`BatchClaim` is the outlier: it carries the same drop consequence as `Claim` —
a claimed range that is never published strands its slots — and has no
`must_use` at all, bare or messaged. That is a `ring_batch` finding rather than
a `ring_consume` one, and it is recorded here because the three-way comparison
is what makes it visible. Reading any one crate shows an annotation choice;
reading all three shows the annotation tracking *obligation* in one case,
correctly tracking its absence in another, and tracking nothing in the third.

The pattern would be complete if it stated the rule — *a range value whose drop
strands a slot gets a messaged `must_use`* — once, anywhere. The family follows
it in four places and writes it down in none.

**Cost:** reachable, and the reachable part is in `ring_batch`, not here.
Recorded because this crate is where the three-way comparison becomes possible.

---

## What All Three Get Right

| Decision | Why it matters |
|----------|----------------|
| length stored, end derived | an inconsistent range is unrepresentable |
| `Copy` | no lifetime, no borrow, no drop order to reason about |
| public constructor | testable in isolation — [`item/001`](../item/001_the_six_of_a_run.md) CN26 |
| an iterator over members | callers never hand-roll `start..start+len` |
| half-open | `end` is the next start; ranges compose without `+ 1` |

The last is the load-bearing one and the reason the pattern is worth naming.
Half-open ranges chain: one run's `end` is the next run's `start`, with no
adjustment. `an_available_run_is_half_open` asserts it for this crate, and
`commit`'s inclusive upper bound is the deliberate exception that makes
`commit( run.end() )` mean "all of it"
([`algorithm/002`](../algorithm/002_the_two_sided_guard.md)).

---

## Cross-References

| Definition | Instance | Relationship |
|------------|----------|--------------|
| pattern | [002](002_read_then_report.md) | the protocol shape rather than the value shape |
| data_structure | [001](../data_structure/001_sixteen_and_twenty_four.md) | the layout the three share |
| item | [001](../item/001_the_six_of_a_run.md) | `Available`'s six, in detail |
| api | [001](../api/001_sixteen_public_items.md) | the `must_use` accounting |
| algorithm | [002](../algorithm/002_the_two_sided_guard.md) | the inclusive exception to half-openness |

### Sources

| What | Where |
|------|-------|
| `Available` | `ring_consume/src/lib.rs:85-90` |
| `Claim` | `ring_claim/src/lib.rs:96-100` |
| `BatchClaim` | `ring_batch/src/lib.rs:55-60` |
| `pending`'s `u64` return | `ring_seqno/src/lib.rs:112` |

### Tests

| Claim | Verified by |
|-------|-------------|
| All three are 16 bytes | the `size_of` probe in [`data_structure/001`](../data_structure/001_sixteen_and_twenty_four.md) |
| The length types differ | the three `grep -A5` listings above |
| `Available` is half-open | `consume_test.rs:52`, `an_available_run_is_half_open` |
| An empty run has `start == end` | `consume_test.rs:57` |
