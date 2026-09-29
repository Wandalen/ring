# Lifecycle: The Validity Window of an Answer

### Scope

- **Purpose**: Establish how long each reading stays true after it is returned, show that one answer of one function decays in the unsafe direction, and record the three different mechanisms consumers use to survive that.
- **Responsibility**: Give the decay direction per function per answer, name the condition under which the unsafe case is reachable, and show that the crate itself says nothing about any of it.
- **In Scope**: What happens to a returned value while the caller holds it.
- **Out of Scope**: The values themselves — see [`001`](001_one_pair_across_one_lap.md).

### The Crate Is Timeless and Its Callers Are Not

`ring_seqno` takes `Seq` by value, computes, and returns. Nothing it touches can
change during a call, so within the crate the question does not arise. Its tests
agree: across all 11 tests in `tests/seq_test.rs` no value changes between two
calls — every assertion is over literals.

One tier up the same values live in `AtomicU64`. By the time `may_claim` returns
`true`, the positions it was computed from may already be gone.

**And the crate never mentions it.** The whole staleness vocabulary appears once
in 136 lines, in boilerplate:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -iE 'stale|race|concurren|atomic|thread' ring_seqno/src/lib.rs
# 3://! Tier 1 of the ring family's 33 crates — the concurrency write-path implementation.
```

Live output:

```
//! Tier 1 of the ring family's 33 crates — the concurrency write-path implementation.
```

That single hit is the word "concurrency," appearing only inside that one
boilerplate line above. Every one of the four consumers below has at least one
real mention.

### What Makes the Question Tractable

Cursors are monotonic — a producer and a consumer both only advance, never
retreat. So each reading's error has a **known sign**, and the whole analysis
reduces to: does that sign point toward permissiveness or toward caution?

Write `d = producer − consumer`. Then:

| Event | Effect on `d` |
|-------|:-------------:|
| The consumer advances | `d` shrinks |
| A producer advances | `d` grows |

### The Decay Table

| Reading | Answer | Stale answer becomes | Direction | Safe? |
|---------|--------|----------------------|:---------:|:-----:|
| `may_claim` | `false` | may become `true` | over-cautious — refuse a claim that is now legal | ✅ costs throughput only |
| `may_claim` | **`true`** | **may become `false`** | **over-permissive — grant a claim that is now illegal** | ❌ |
| `free_slots` | any | can only shrink if a producer advances | a lower bound | ✅ |
| `pending` | any | can only grow as more is published | a lower bound | ✅ |
| `slowest` | any | the true minimum can only have advanced | a lower bound | ✅ |
| `laps_between` | any | — | no caller outside this crate (**Finding SQ27**) | n/a |

**Four of the five decay safely; the exception is one of `may_claim`'s two
answers.** Every other reading is a quantity, and a quantity that under-reports
makes its caller do less than it could. `may_claim` is a permission, and a
permission that over-reports makes its caller do something it must not.

### The Unsafe Case Needs a Second Producer

The `true`→`false` flip requires `d` to grow, which requires the producer cursor
to advance. A caller that just read `may_claim( p, c ) == true` holds `p` — so if
it is the *only* producer, nothing can advance `p` but itself, and the `true`
stays true until it acts on it.

| Ring shape | Can `may_claim`'s `true` go stale? |
|------------|:----------------------------------:|
| `ring_spsc` — one producer | ❌ The producer owns the only thing that could invalidate it |
| `ring_mpsc` — many producers | ✅ Another producer's claim moves the cursor between the read and the act |

So the one unsafe decay in the crate is exactly the multi-producer case, and it
is the reason `ring_claim` cannot simply ask and then write.

### Three Consumers, Three Mechanisms, None Named Here

Each tier-4/5 consumer solved this independently:

**1. `ring_claim` — commit, do not trust.** The gate is the loop condition of a
compare-exchange, and the crate says why in its own comment:

```rust
// ring_claim/src/lib.rs:432-445
// The gate is the loop condition, and is therefore re-read on every
// iteration: on a failed exchange another producer moved the cursor, so
// the headroom computed against the old value is stale and granting on it
// would overlap that producer's range.
let mut current = self.claimed();
while count <= self.consumers.headroom( current )
{
  let next = current.advanced_by( count as u64 );
  match self.cursor.compare_exchange( current, next, CLAIM_SUCCESS, GATING )
  {
    Ok( _ ) => return Ok( Claim::new( current, count ) ),
    Err( actual ) => current = actual,
  }
}
```

The answer is never acted on — it is *committed*, and a failed exchange discards
it and re-derives. This comment is the only place in all 33 crates where the
unsafe decay is written down, and it is four tiers away from the function it
describes.

**2. `ring_wait` — make the stale value unrepresentable.**

```rust
// ring_wait/src/lib.rs:179-182
pub fn wait_until< F >( kind : WaitKind, spins : usize, mut ready : F ) -> Result< usize, RingError >
where
  F : FnMut() -> bool,
```

The parameter is a closure, not a `bool`. There is no way to pass a value read
once and re-tested; the type forces re-evaluation on every spin. The staleness bug
is not documented here because it cannot be written.

**3. `ring_barrier` — re-read after the wait, do not reuse.**

```rust
// ring_barrier/src/lib.rs:270-274
ring_wait::wait_until( kind, spins, || self.admits( from, count ) )?;
self.frontier().ok_or( RingError::Empty )
```

`admits` already computed a frontier; `wait_for` throws it away and reads again.
The crate states the reason, and it is the *safe* decay being deliberately
exploited:

> The returned sequence is the frontier at the moment the wait succeeded, not
> `from + count` — a consumer that waited for one item and found six should drain
> six, and returning the requested count instead would throw away the batch that
> waiting just discovered.

That is the lower-bound property of `pending`/`slowest` turned into a feature: the
stale answer under-reports, so re-reading can only find *more*, and the API is
shaped to collect it.

### The Gap

| What | Where it is stated | Where it should also be |
|------|--------------------|-------------------------|
| `may_claim`'s `true` is not durable under multiple producers | a comment in `ring_claim` | `may_claim`'s own doc comment |
| The other four readings are lower bounds | nowhere; `ring_barrier` exploits it without naming it | the module doc |
| Nothing here is atomic or synchronised | nowhere | the module doc |

The third is the one a new caller is most likely to get wrong. `ring_seqno`'s
functions look like ring operations — they take a producer and a consumer and
answer questions about a live buffer — and they are pure functions over two
integers. A reader who assumes otherwise writes `if may_claim( … ) { write() }`
and has a correct-looking single-producer program that corrupts under two.

Not fixed here: adding a `# Concurrency` paragraph to five doc comments is a
source change to the crate under documentation, which belongs to a run with its
own verification. Recorded as the reason the crate's silence is a real gap rather
than an acceptable division of labour — the tier that *could* stay silent is the
one whose type system forbids the mistake, and that is `ring_wait`, not this one.

### Why the Tests Cannot Catch It

Nothing in `tests/seq_test.rs` could: the functions are pure, so a test can only
demonstrate that a given input maps to a given output. The bug lives entirely in
the interval between two calls, which this crate has no way to create.

That is not a coverage gap — it is a correct division. The interval is
`ring_claim`'s, and `ring_claim/tests/` is where a two-producer race belongs.
Worth stating explicitly, because "add a concurrency test to `ring_seqno`" is the
natural wrong response to this document.

### SQ33 — Both Answers Go Stale, in Opposite Directions

The verdict is a snapshot, and which way it rots depends on who moves:

```
may_claim == true    can become false  (a producer advanced)
may_claim == false   can become true   (a consumer advanced)

ring_seqno/src/lib.rs   says neither
ring_claim/src/lib.rs says it, in a comment
```

**Finding.** `may_claim`'s `true` and `false` decay in opposite directions, and the crate documents neither — the warning exists only as a comment in `ring_claim`.

---

### SQ34 — One Doc States the Convention All Five Assume

The interval convention decides every answer in the crate, and appears once:

```
pending      "published up to but not including `producer`"   states it
laps_between  —                                                assumes it
may_claim     —                                                assumes it
free_slots    —                                                assumes it
slowest       —                                                assumes it
```

**Finding.** `pending`'s doc is the only one of five that states the half-open interval convention ("up to but not including"), and the other four depend on it silently.

---

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_the_slowest_fold.md](../algorithm/002_the_slowest_fold.md) | The fold whose input is read one atomic at a time |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_slice_that_slowest_reads.md](../data_structure/002_the_slice_that_slowest_reads.md) | R6 — the slice's elements never coexisted |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_every_reading_is_total.md](../invariant/002_every_reading_is_total.md) | Totality holds regardless of staleness; the two properties are independent |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_three_capacity_readings.md](../item/001_the_three_capacity_readings.md) | `may_claim`'s contract, which omits this |

### Lifecycles

| File | Relationship |
|------|--------------|
| [001_one_pair_across_one_lap.md](001_one_pair_across_one_lap.md) | The same readings with time held still |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_every_reading_is_allocation_free.md](../non_functional_requirement/001_every_reading_is_allocation_free.md) | Why re-deriving on every spin is affordable at all |

### Sources

| File | Relationship |
|------|--------------|
| `ring_seqno/src/lib.rs:1-136` | The crate, and its one mention of concurrency |
| `ring_claim/src/lib.rs:432-445` | The CAS loop, and the only written statement of the unsafe decay |
| `ring_wait/src/lib.rs:179-182` | The closure parameter that forbids a stale bool |
| `ring_barrier/src/lib.rs:246-252` | Re-reading the frontier, and the reason |
| `ring_gating/src/lib.rs:217-222` | `headroom`, the value `ring_claim` re-derives per iteration |

### Tests

| File | Relationship |
|------|--------------|
| `tests/seq_test.rs:15-171` | All 11 tests, none of which lets a value change between calls |
| `ring_claim/src/lib.rs:405-420` | The doctest that exercises a claim, a refusal, and a release in sequence |
