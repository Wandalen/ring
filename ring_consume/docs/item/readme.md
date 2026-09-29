# item

Fourteen methods across two types, taken one at a time. `Available`'s six are
pure arithmetic on two integers and five of them are `const`. `Consumer`'s eight
split exactly where the atomics start — three `const` accessors, five that touch
the cursor — and the two boundaries are the same line without exception.

The findings here are about individual items rather than the surface as a whole
(which is [`api/`](../api/readme.md)): a public constructor that makes the type
guarantee nothing, an accessor that hands out store access and documents itself
as an accessor, and a return-by-value shape that looks like a micro-optimisation
and is actually a constraint a test in a fourth crate imposed.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [The Six of a Run](001_the_six_of_a_run.md) | CN26, CN27 — a public constructor and what it costs the type's guarantees, and an `end` computed rather than stored |
| 002 | [The Eight of a Consumer](002_the_eight_of_a_consumer.md) | CN28, CN29 — `cursor()` as the crate's widest hole, and `barrier()`'s by-value return forced by a downstream test |

### The Fourteen, With Their Boundaries

| Type | Item | `const` | Atomics |
|------|------|:-------:|:-------:|
| `Available` | `new`, `start`, `end`, `len`, `is_empty` | ✔ | ✘ |
| `Available` | `sequences` | ✘ — iterator | ✘ |
| `Consumer` | `new`, `cursor`, `barrier` | ✔ | ✘ |
| `Consumer` | `position` | ✘ | 1 load |
| `Consumer` | `available`, `available_up_to` | ✘ | 1 + `n` loads |
| `Consumer` | `commit`, `commit_available` | ✘ | 1 + `n` loads, 1 store |

The `const` boundary and the atomic boundary coincide everywhere except
`sequences`, whose exception is the iterator rather than an atomic
([`type/001`](../type/001_availables_const_surface.md) CN47).

### Where Each Item's Shape Was Decided Elsewhere

Three of the fourteen have a shape imposed from outside this crate, and none says
so:

| Item | Shaped by | How |
|------|-----------|-----|
| `barrier()` | `ring_publish/tests/handshake_test.rs` | `Barrier` was made `Copy` so a caller could hold both a `Consumer` and its barrier; `ring_barrier`'s module doc records the change and this crate does not |
| `cursor()` | `ring_gating` | must return the borrow at full `'a` so a gating set can be built against it |
| `sequences()` | `ring_types` | `Seq` is not `Step` and has no conversion, so the body must escape through `.0` — [`workaround/001`](../workaround/001_five_functions_none_const.md) CN52 |

### The Widest Hole

`cursor()` returns `&'a PaddedCursor`. `PaddedCursor` implements `SeqCell`, whose
`store` takes `&self`. So the return value permits writes that bypass both
commits and both halves of `commit`'s guard, in safe code, in one line.

That is not fixable — the cursor is shared state the producer reads directly, and
a `Consumer` cannot hold exclusive write access without breaking the mechanism
([`data_structure/002`](../data_structure/002_two_borrows_and_no_owned_state.md)
CN24). The finding is that the doc comment describes what the function returns
and never that writing through it is possible and always wrong.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order

# the fourteen, with attributes, comment lines stripped
grep -vE "^[[:space:]]*//" ring_consume/src/lib.rs | grep -E 'pub (const )?fn'

# receivers — nothing takes &mut self
grep -E 'fn [a-z_]+\( *&mut self' ring_consume/src/lib.rs || echo "  none"

# what a &PaddedCursor can do
grep -E '^\s*fn (store|load|fetch_add|compare_exchange)' ring_cursor/src/lib.rs

# the two bodies whose shape came from elsewhere
grep -A4 'pub const fn cursor\|pub const fn barrier' ring_consume/src/lib.rs
grep 'handshake_test' ring_barrier/src/lib.rs

# every external call into the Consumer
command grep -r 'consumer\.\|Consumer::' */tests/*.rs | command grep -v ring_consume/ | head
```

Live output:

```
  pub const fn new( start : Seq, len : u64 ) -> Self
  pub const fn start( self ) -> Seq
  pub const fn end( self ) -> Seq
  pub const fn len( self ) -> u64
  pub const fn is_empty( self ) -> bool
  pub fn sequences( self ) -> impl Iterator< Item = Seq >
  pub const fn new( cursor : &'a PaddedCursor, barrier : Barrier< 'a > ) -> Self
  pub const fn cursor( &self ) -> &'a PaddedCursor
  pub const fn barrier( &self ) -> Barrier< 'a >
  pub fn position( &self ) -> Seq
  pub fn available( &self ) -> Available
  pub fn available_up_to( &self, max : u64 ) -> Available
  pub fn commit( &self, through : Seq ) -> Result< Seq, RingError >
  pub fn commit_available( &self ) -> Seq
  none
  fn load( &self, order : Ordering ) -> Seq
  fn store( &self, value : Seq, order : Ordering )
  fn fetch_add( &self, n : u64, order : Ordering ) -> Seq
  fn compare_exchange( &self, current : Seq, new : Seq, success : Ordering, failure : Ordering )
  pub const fn cursor( &self ) -> &'a PaddedCursor
  {
    self.cursor
  }

--
  pub const fn barrier( &self ) -> Barrier< 'a >
  {
    self.barrier
  }

//! made the four-operation handshake in `ring_publish/tests/handshake_test.rs`
ring_barrier/tests/barrier_test.rs://! for a consumer. The two crates give opposite answers to the same-shaped
ring_barrier/tests/barrier_test.rs:  // consumer. Both are "no constraint from dependencies" resolved to what a
ring_batch/tests/batch_test.rs:  assert_eq!( consumer.counts().total, 0 );
ring_batch/tests/batch_test.rs:  consumer.store( Seq( 2 ), Ordering::Release );
ring_core/tests/core_test.rs:    assert!( consumer.is_empty(), "{backend:?} started non-empty" );
ring_core/tests/core_test.rs:    assert_eq!( consumer.try_recv(), None, "{backend:?} yielded a record from an empty ring" );
ring_core/tests/core_test.rs:          consumer.try_recv_batch( &mut received ) > 0,
ring_core/tests/core_test.rs:    let drained = consumer.try_recv_batch( &mut received );
ring_core/tests/core_test.rs:    assert!( consumer.is_empty(), "{backend:?} still reports records after a full drain" );
ring_core/tests/core_test.rs:      consumer.try_recv_batch( &mut received ),
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CN26 | `ring_consume` | n/a — observation | `Available::new` is public, so the type guarantees nothing about provenance — correct here, and the opposite of what "public constructor on a concurrency type" usually implies |
| CN27 | `ring_types` | n/a — observation | `end` is computed by `advanced_by`, which is plain `+`, so the range type inherits `ring_types`' unchecked arithmetic — unreachable at 585 years |
| CN28 | `ring_consume` | **latent hazard** | `cursor()` returns a reference through which a caller may store, bypassing every guarantee `commit` provides; the doc comment describes it as an accessor |
| CN29 | `ring_barrier` | n/a — observation | `barrier()` returns by value because `Barrier` is `Copy`, a signature `ring_publish`'s handshake test forced and which `ring_barrier` alone records |
