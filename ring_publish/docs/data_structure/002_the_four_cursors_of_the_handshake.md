# Data Structure: The Four Cursors of the Handshake

### Scope

- **Purpose**: Place this crate's one cursor among the four that the claim → publish → available → commit handshake runs on, and record which are shared, which are private, and which are documented as shared but are not.
- **Responsibility**: Enumerate the four, show the wiring that connects them, name the wiring bug that passes every other test, and check `ring_claim`'s two documented seams into this crate against reality.
- **In Scope**: The cursor topology `tests/handshake_test.rs` assembles.
- **Out of Scope**: This crate's own cursor in isolation — see [`data_structure/001`](001_one_padded_cursor_and_nothing_else.md).

### The Four

`tests/handshake_test.rs:18-25` states the topology before the tests use it:

> Four crates, four cursors, and only two of them shared:
>
> - the **claimed** cursor is private to [`ring_claim::Claimer`]; nobody reads it
> - the **published** cursor belongs to [`Publisher`] and is what the consumer's
>   barrier waits on
> - the **consumer position** lives in the producer's `GatingSet`, which is what
>   makes the producer stop rather than lap it
> - the ring's own capacity is in that same set

| Cursor | Owner | Written by | Read by | Shape |
|--------|-------|-----------|---------|-------|
| claimed | `ring_claim::Claimer` | `claim` / `claim_up_to`, by compare-exchange | **nobody** | `PaddedCursor`, private field, public accessor |
| **published** | **`ring_publish::Publisher`** | **`try_publish`, by compare-exchange** | **the consumer's `Barrier`** | **`PaddedCursor`, private field, `cursor()`** |
| consumer position | `ring_gating::GatingSet` | `Consumer::commit` | `Claimer`, via `headroom` | one `PaddedCursor` per consumer |
| capacity | the same `GatingSet` | never — set at construction | `Claimer`, to bound the claim | `Capacity`, not a cursor |

The fourth is listed with the cursors because it is what closes the loop: the
producer stops when `claimed - slowest_consumer == capacity`, so the bound is a
cursor difference against a constant, and all three live in one place.

### The Wiring, in Two Lines

```rust
let barrier  = Barrier::over( core::slice::from_ref( publisher.cursor() ) );  // :390
let consumer = Consumer::new( consumers.cursor( 0 ).unwrap(), barrier );      // :391
```

Read outward from the consumer: it *reports into* a cursor borrowed from the
producer's own `GatingSet`, and it *waits on* a barrier built over the
publisher's cursor. Two shared cursors, in opposite directions, and the consumer
touches both.

`Barrier::over` taking a bare `&[ PaddedCursor ]` rather than a `GatingSet` is
what makes the first half possible at all, and it was changed for this test.
`ring_barrier/src/lib.rs:40-44`:

> Taking the aggregate instead would have meant a publisher's cursor could never
> be depended on at all, since there is no way to move an existing cursor into a
> set that owns its own. That is not a hypothetical: it is what made the
> four-operation handshake in `ring_publish/tests/handshake_test.rs` unwireable
> until this signature changed.

### PB15 — The Wiring Bug That Passes Every Other Test

`tests/handshake_test.rs:383-407` exists only to assert the topology, and
`:386-387` states what it is guarding against:

> The wiring assertion. A consumer holding a cursor the producer does not read
> passes every other test in this file and gates nothing.

The failure is exact and silent. Give the consumer its own fresh
`PaddedCursor` instead of `consumers.cursor( 0 )` and everything still compiles;
`commit` still advances something; `available` still returns the right ranges;
items still arrive in order at any scale a single-lap test reaches. What stops
working is backpressure — the producer gates on a cursor nobody advances, so it
laps the ring and overwrites slots the consumer has not read.

The assertion is an address comparison, not a value comparison:

```rust
assert!(
  core::ptr::eq( consumer.cursor(), consumers.cursor( 0 ).unwrap() ),
  "the consumer is reporting into a cursor the producer never reads"
);
```

Two cursors holding the same `Seq` are indistinguishable by value at every
moment before the first commit, which is exactly when a value assertion would
run. That is why this and
[`data_structure/001`](001_one_padded_cursor_and_nothing_else.md)'s
`cursor()`-identity check are the suite's only two `core::ptr::eq` assertions:
both guard against a *copy where a share was intended*, and a copy is
value-equal until it diverges.

The rest of the test then proves the wiring end to end behaviourally: claim a
full lap of 4, publish it, confirm the next claim is `Err( RingError::Full )`,
commit through `Seq( 2 )`, and confirm exactly two slots were released
(`:402-406`).

### PB16 — `ring_claim` Documents Two Seams Into This Crate and Neither Exists

The claimed cursor is described by `handshake_test.rs:20` as *"private to
`ring_claim::Claimer`; nobody reads it"*. `ring_claim`'s own documentation says
otherwise, in two places:

`ring_claim/src/lib.rs:281-282`, on `Claimer::cursor`:

> The producer cursor, **for `ring_publish` to read** and for a gating set on the
> other side of the ring to be built against.

`ring_claim/src/lib.rs:106-108`, on `Claim::new`:

> Public because **`ring_publish`** and the test suites of both crates need to
> construct one directly; a producer obtains real claims from
> [`Claimer::claim`], which is the only path that establishes exclusivity.

Checked:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r 'Claim::new\|claimer\.cursor()' */src/*.rs */tests/*.rs | grep -v '///'
# documentation excluded — these pages quote both names, so leaving docs/ in
# would make this recipe match its own quoted output and never settle
grep -r 'Claim::new\|claimer\.cursor()' ring_publish/ | grep -v '/docs/' \
  || echo '(no matches outside the documentation)'
grep -rhoE 'claim(er)?\.[a-z_]+\(' ring_publish/tests/handshake_test.rs | sort | uniq -c
```

Live output:

```
ring_batch/src/lib.rs:  BatchClaim::new( cursor.fetch_add( count as u64, order ), count )
ring_claim/src/lib.rs:        Ok( _ ) => return Ok( Claim::new( current, count ) ),
ring_claim/src/lib.rs:        Ok( _ ) => return Ok( Claim::new( current, granted ) ),
ring_mpsc/src/lib.rs:    let claim = self.claimer.cursor().addr();
ring_batch/tests/batch_test.rs:  let batch = BatchClaim::new( Seq( 14 ), 6 );
ring_batch/tests/batch_test.rs:  let batch = BatchClaim::new( Seq( 3 ), 20 );
ring_batch/tests/batch_test.rs:  let batch = BatchClaim::new( Seq( 5 ), 0 );
ring_batch/tests/batch_test.rs:  let batch = BatchClaim::new( Seq( 10 ), 3 );
ring_batch/tests/batch_test.rs:  let batch = BatchClaim::new( Seq( 4 ), 0 );
ring_batch/tests/batch_test.rs:  let batch = BatchClaim::new( Seq( 10 ), 3 );
ring_batch/tests/batch_test.rs:  let batch = BatchClaim::new( Seq( 10 ), 0 );
ring_batch/tests/batch_test.rs:  let a = BatchClaim::new( Seq( 4 ), 4 ); // 4..8
ring_batch/tests/batch_test.rs:  assert!( !a.overlaps( &BatchClaim::new( Seq( 0 ), 4 ) ), "abutting below" );
ring_batch/tests/batch_test.rs:  assert!( !a.overlaps( &BatchClaim::new( Seq( 8 ), 4 ) ), "abutting above" );
ring_batch/tests/batch_test.rs:  assert!( a.overlaps( &BatchClaim::new( Seq( 7 ), 4 ) ), "one sequence shared" );
ring_batch/tests/batch_test.rs:  assert!( a.overlaps( &BatchClaim::new( Seq( 1 ), 4 ) ), "one sequence shared, below" );
ring_batch/tests/batch_test.rs:  assert!( a.overlaps( &BatchClaim::new( Seq( 0 ), 16 ) ), "fully contained" );
ring_batch/tests/batch_test.rs:  let empty = BatchClaim::new( Seq( 5 ), 0 );
ring_batch/tests/batch_test.rs:  let wide = BatchClaim::new( Seq( 0 ), 16 );
ring_batch/tests/batch_test.rs:  let a = BatchClaim::new( Seq( 1 ), 2 );
ring_batch/tests/batch_test.rs:  assert_ne!( a, BatchClaim::new( Seq( 1 ), 3 ) );
ring_batch/tests/batch_test.rs:  assert_eq!( batch, BatchClaim::new( Seq( 4 ), 2 ) );
ring_claim/tests/claim_test.rs:  let claim = Claim::new( Seq( 4 ), 3 );
ring_claim/tests/claim_test.rs:  let empty = Claim::new( Seq( 5 ), 0 );
ring_claim/tests/claim_test.rs:  assert!( !empty.overlaps( Claim::new( Seq( 0 ), 100 ) ), "a zero-width range covers no slot" );
ring_claim/tests/claim_test.rs:  assert!( !Claim::new( Seq( 0 ), 100 ).overlaps( empty ), "and the check is symmetric" );
ring_claim/tests/claim_test.rs:      let claim = Claim::new( Seq( start ), len );
ring_claim/tests/claim_test.rs:  let first = Claim::new( Seq( 0 ), 4 );
ring_claim/tests/claim_test.rs:  let second = Claim::new( Seq( 4 ), 4 );
ring_claim/tests/claim_test.rs:          let a = Claim::new( Seq( a_start ), a_len );
ring_claim/tests/claim_test.rs:          let b = Claim::new( Seq( b_start ), b_len );
ring_claim/tests/claim_test.rs:  assert_eq!( claimer.cursor().load( Ordering::Acquire ), Seq::ZERO );
ring_claim/tests/claim_test.rs:  assert_eq!( claimer.cursor().load( Ordering::Acquire ), Seq( 3 ) );
ring_claim/tests/claim_test.rs:  let stolen = claimer.cursor().fetch_add( 4, Ordering::AcqRel );
ring_claim/tests/claim_test.rs:  claimer.cursor().store( Seq::ZERO, Ordering::Release );
ring_claim/tests/claim_test.rs:  const A : Claim = Claim::new( Seq( 4 ), 4 );
ring_claim/tests/claim_test.rs:  const B : Claim = Claim::new( Seq( 6 ), 4 );
ring_claim/tests/claim_test.rs:  const C : Claim = Claim::new( Seq( 8 ), 4 );
ring_claim/tests/claim_test.rs:  let a = Claim::new( start, 4 );
ring_tls/tests/tls_test.rs:  assert_eq!( claim, BatchClaim::new( Seq::ZERO, 0 ) );
(no matches outside the documentation)
     21 claimer.claim(
      1 claimer.claimed(
      2 claimer.headroom(
      7 claim.len(
     10 claim.start(
```

| Documented seam | Real call sites | In `ring_publish` |
|-----------------|-----------------|-------------------|
| `Claimer::cursor` — "for `ring_publish` to read" | `ring_mpsc:862` (as `.addr()`, a cache-line check — not a read of the value); `ring_claim/tests/claim_test.rs:290,292` | **none** |
| `Claim::new` — "because `ring_publish` … need[s] to construct one directly" | `ring_claim/src:409,452` (internal); `ring_claim/tests/claim_test.rs` ×8 | **none** |

`ring_publish` uses exactly three things from `ring_claim`, all in
`handshake_test.rs`: `claimer.claim()` (17 sites), `claim.start()` (10),
`claim.len()` (7). It never constructs a `Claim` and never touches the claimed
cursor — and cannot, in the library, because `ring_claim` is a dev-dependency
([`integration/001`](../integration/001_ten_crates_name_it_and_none_depends_on_it.md) § PB3).
The `BatchClaim::new` hits the grep also returns are a different type in
`ring_batch` and are not this seam.

Neither is a defect in behaviour. `Claimer::cursor` has a real second
justification in the same sentence — *"for a gating set on the other side of the
ring to be built against"* — and `Claim::new`'s public constructor is genuinely
needed by `ring_claim`'s own suite. What is wrong is the named consumer: two
public items justify their visibility by pointing at a crate that does not use
them and, given the dependency direction, never will.

This is the same shape as
[`integration/002`](../integration/002_the_two_crates_that_declined.md) § PB5 —
a document naming a future that did not arrive — and it is worth recording
together with it, because the two point in opposite directions. `ring_claim`
expected `ring_publish` to reach *into* it; the family expected something to
reach into `ring_publish`. Neither happened, and the handshake works anyway,
which is the actual evidence that the four-cursor split is correct: the two
halves needed less of each other than either expected.

### Why Four and Not Two

A design with one producer cursor (claimed *and* published) is smaller, faster,
and wrong. `src/lib.rs:18-27` states it:

> `ring_claim` advances a cursor when a producer *takes* a range; this crate
> advances a different one when the producer has *finished writing* it. Between
> the two, the slot is claimed and unwritten — and the feature's central
> requirement is that a consumer never sees it.
>
> Conflating the two cursors is not a subtle bug. It publishes uninitialised
> memory, it passes every single-threaded test (where the write completes before
> anything can read), and it fails only under load.

`ring_claim` says the same from its side (`:310-315`), naming the feature
document that forbids it. Two crates, two module docs, one requirement — and
[`workaround/001`](../workaround/001_the_loom_seam_and_its_only_user.md)'s loom
model is the only thing in the repository that actually checks it, because the
failure it produces is invisible to a threaded test on x86.

The consumer-position and capacity cursors are the mirror requirement: without
them the producer laps the consumer, which is the same corruption from the other
end. All four exist because the ring has two boundaries and each needs two
cursors — one for *taken*, one for *done*.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_the_compare_exchange_that_refuses.md](../algorithm/001_the_compare_exchange_that_refuses.md) | The operation that moves the second of the four |

### Data Structures

| File | Relationship |
|------|--------------|
| [001_one_padded_cursor_and_nothing_else.md](001_one_padded_cursor_and_nothing_else.md) | This crate's own cursor, layer by layer |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_ten_crates_name_it_and_none_depends_on_it.md](../integration/001_ten_crates_name_it_and_none_depends_on_it.md) | The four dev-dependencies this topology needs |
| [../integration/002_the_two_crates_that_declined.md](../integration/002_the_two_crates_that_declined.md) | The other document naming a future that did not arrive |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_the_four_operation_handshake.md](../lifecycle/002_the_four_operation_handshake.md) | The four operations these four cursors carry |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_conflating_the_two_cursors.md](../pitfall/002_conflating_the_two_cursors.md) | What merging cursors one and two produces |

### Workarounds

| File | Relationship |
|------|--------------|
| [../workaround/001_the_loom_seam_and_its_only_user.md](../workaround/001_the_loom_seam_and_its_only_user.md) | The only check that the claimed/published split actually holds |

### Sources

| File | Relationship |
|------|--------------|
| `ring_publish/src/lib.rs:18-27` | Why the two producer cursors are separate |
| `ring_claim/src/lib.rs:106-108,281-282,336-341` | The two documented seams, and the same requirement from the other side |
| `ring_barrier/src/lib.rs:30-44` | Why `Barrier` takes a bare slice, and which test forced it |
| `ring_gating/src/lib.rs` | The set holding cursors three and four |
| `ring_consume/src/lib.rs:12-19` | Why `available` and `commit` are separate calls on cursor three |

### Tests

| File | Relationship |
|------|--------------|
| `tests/handshake_test.rs:16-30` | The topology, stated before it is used |
| `tests/handshake_test.rs:383-407` | The wiring assertion — `core::ptr::eq`, then the behavioural proof |
| `tests/handshake_test.rs:409-432` | A stalled consumer stops the producer after exactly one lap |
| `tests/handshake_test.rs:563-588` | A consumer that never commits leaves the producer exactly one lap ahead |
