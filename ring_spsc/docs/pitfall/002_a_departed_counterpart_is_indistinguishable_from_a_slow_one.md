# Pitfall: A Departed Counterpart Is Indistinguishable From a Slow One

### Scope

- **Purpose**: Record that neither end can observe the other's departure, so a dropped counterpart is reported as ordinary backpressure forever.
- **Responsibility**: The trap, the two tests that pin the behaviour as intended, and where the remedy lives.
- **In Scope**: What a `Producer` sees after its `Consumer` is dropped, and the reverse.
- **Out of Scope**: Shutdown signalling, which is `ring_shutdown`'s; the pairing lifecycle itself (→ [`../lifecycle/002`](../lifecycle/002_producer_consumer_pairing.md)).

### The Trap

A `Consumer` that is dropped stops advancing its cursor. The producer's
free-capacity check reads that cursor and finds no headroom, so after `capacity`
pushes every subsequent `try_push` returns the record and every `claim` returns
`RingError::Full` — permanently, with no error distinguishable from a consumer
that is merely busy.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_spsc
grep -oE 'fn a_departed_(consumer|producer)[a-z_]*' tests/spsc_test.rs
```

Live output:

```
fn a_departed_producer_leaves_the_published_tail_drainable
fn a_departed_consumer_leaves_the_producer_reporting_full_forever
```

### It Is Deliberate, and the Test Says Why

`a_departed_consumer_leaves_the_producer_reporting_full_forever` carries the
reasoning inline: the error is `Full` and **not** a hypothetical `Closed`,
because

> a ring that guessed at counterpart death here would be reporting a condition
> it cannot actually observe: a consumer that has stopped for a millisecond and
> one that has stopped for good look identical from this end.

That is the correct call. A `Closed` variant would be a lie in every case where
the consumer comes back, and nothing in the type system distinguishes the two.

The symmetric case is gentler: `a_departed_producer_leaves_the_published_tail_drainable`
— a dropped producer leaves everything it published still available, so the
consumer drains to the end and then reads empty. Losing the producer costs
nothing; losing the consumer costs everything.

### What It Costs

A caller that treats `Err( record )` as retry-later will spin forever against a
dead consumer, at full CPU, with no diagnostic. The ring cannot help — the
signal it would need does not exist at this layer.

**The remedy is a layer up.** `ring_shutdown` exists to carry exactly this
signal, and a composition that uses this crate directly does not get it.

### Sources

| File | Relationship |
|------|-----------------|
| `tests/spsc_test.rs` | Both departure tests, and the reasoning quoted above |
| `src/lib.rs` | `free_capacity`, whose reading of the consumer cursor is the mechanism |
| `001_spsc_correctness_does_not_transfer.md` | The sibling pitfall — this crate's guarantees under a different cardinality |

### SP46 — The Asymmetry Is Total: Losing the Producer Costs Nothing

Both cases are tested and only one is a trap. `a_departed_producer_leaves_the_published_tail_drainable`
is the benign half — the consumer drains what exists and then reads empty, which
is indistinguishable from a slow producer and harmless.

The consumer's departure is the harmful half for a structural reason: the
producer's headroom check *reads the consumer's cursor*, so a cursor that stops
moving is a permanent refusal. Nothing in the reverse direction has that
shape.

### SP47 — The Refusal Is Correct and the Caller Has No Way to Act on It

The test's own comment defends the choice and the defence is right: a consumer
stopped for a millisecond and one stopped for good are indistinguishable from the
producer's end, so a `Closed` variant would be a guess.

**But the error's name carries a temporal claim it cannot support.** `Full`
means "no headroom now"; every reasonable caller reads that as "try again". In
the departed-consumer case every retry will fail, forever, at whatever rate the
caller retries.

The remedy is `ring_shutdown`, one layer up. What is recorded here is that
neither the error type nor its documentation points a reader toward it.

**Disposition:** declined — "the error type" and "its documentation" both
name `RingError::Full`'s own doc comment in
`ring_types/src/error.rs`, which carries no pointer to
`ring_shutdown` today; this instance already supplies the pointer this
crate's own docs need, and the remaining fix belongs to `ring_types`, not
this crate.
