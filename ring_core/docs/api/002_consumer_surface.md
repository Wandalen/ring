# API: Consumer Surface

### Scope

- **Purpose**: Fix the drain contract a caller gets regardless of backend, and mark the one operation whose cost differs by backend rather than only its shape.
- **Responsibility**: Method set, signatures, buffer ownership, occupancy semantics, and compatibility guarantees.
- **In Scope**: `Consumer::try_recv`, `try_recv_batch`, `len`, `is_empty`.
- **Out of Scope**: The procedures behind them (→ [`algorithm/002`](../algorithm/002_uniform_drain_over_three_shapes.md)); the producer half (→ [`api/001`](001_producer_surface.md)).

### Abstract

Four drain operations, identical in shape at every backend. The consumer never
allocates for itself; the only growth is the caller's own buffer. One operation —
the batch drain — differs in cost rather than in contract, because the crossbeam
arm has no native batch form.

### Operations

| Operation | Signature | Returns | Allocates | Notes |
|---|---|---|---|---|
| Take one | `try_recv( &mut self ) -> Option< T >` | the record, or nothing | Never | `None` means empty, not an error |
| Take many | `try_recv_batch( &mut self, out : &mut Vec< T > ) -> usize` | how many were moved | **Caller's buffer only** | **Appends**; may reallocate the caller's `Vec` |
| Occupancy | `len( &self ) -> usize` | a lower bound | Never | Only ever grows — the reader is the only party that can subtract |
| Emptiness | `is_empty( &self ) -> bool` | — | Never | Always agrees with `len() == 0` |

### Error Handling

**This surface has no failure modes at all** — not an error type, not a panic
path, not a fallible constructor. Emptiness is the only condition either entry
point can report, and it is reported in the value:

| Operation | Empty ring yields | Not an error because |
|---|---|---|
| `try_recv` | `None` | An empty ring is the steady state of a consumer that is keeping up |
| `try_recv_batch` | `0`, with the caller's buffer untouched | The same, in count form |

**A `Result` here would make the common case look exceptional** and force every
caller to discriminate on something that carries no information. The asymmetry
with the producer surface is deliberate: a refused *push* has a record that must
go somewhere, so it needs a return path; a refused *pop* has nothing to hand
back.

### Compatibility Guarantees

1. **`None` and `0` are the empty ring, not a failure.** Neither entry point has
   an error path for emptiness — a `Result` here would make the common case look
   exceptional and force every caller to discriminate. Asserted at both entry
   points by `draining_an_empty_ring_is_zero_not_an_error`.

2. **`try_recv_batch` appends and reports only its own count.** It never clears
   the buffer, and the returned `usize` is what *this call* moved — not
   `out.len()`. Both are the kind of thing a caller assumes rather than reads,
   so both are asserted.

3. **The consumer never allocates for itself.** The only allocation on this
   surface is the caller's own `Vec` growing, which the caller controls by
   reserving. The crossbeam arm builds a temporary internally — a real cost,
   scoped to that backend and documented where it happens
   (→ [`algorithm/002`](../algorithm/002_uniform_drain_over_three_shapes.md)).

4. **`len` is a lower bound that only ever grows — at every backend.** There is
   exactly one `Consumer` per `split()`, and `Consumer` has no `try_clone`, so
   the only party that can *remove* records is the caller reading the number.
   A producer may add between the read and the drain; nothing may subtract.

   So `len()` items are guaranteed available, and *more* may be. This matches
   [`ring_handle` api/002](../../../ring_handle/docs/api/002_consumer_surface.md)
   exactly, and it is the mirror of `free_capacity`, which bounds in the
   opposite direction and holds only at SPSC.

   The practical consequence is unchanged: **drain until `try_recv` returns
   `None`, never until `is_empty()`.** `is_empty()` is a lower bound of zero,
   which is true of every ring and therefore tells a live drain loop nothing —
   the loop terminates only because the producer stopped, not because the
   reading said so.

5. **`len` and `is_empty` never disagree with each other.** Two computations of
   one fact — a cursor subtraction in-house, a load at crossbeam — checked at
   every point of a lap rather than only at the ends, because mid-lap is where
   the interesting disagreement would be.

### Divergence From `ring_handle`'s Specified Surface

Same shape as the producer half: the mutating operations take `&mut self` here
and `&self` there, because they reach a backend consumer that requires exclusive
access. `len`/`is_empty` agree at `&self`. The full table and the `ring_handle`
obligation live in [`integration/002`](../integration/002_handle_surface_divergence.md).

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_uniform_drain_over_three_shapes.md](../algorithm/002_uniform_drain_over_three_shapes.md) | How three native drain shapes serve these two entry points |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_handle_surface_divergence.md](../integration/002_handle_surface_divergence.md) | The receiver divergence, as an obligation on `ring_handle` |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_uniform_delivery_across_backends.md](../invariant/002_uniform_delivery_across_backends.md) | Exactly-once delivery, which guarantee 2's append semantics must not disturb |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/003_occupancy_across_backends.md](../lifecycle/003_occupancy_across_backends.md) | The states guarantees 4 and 5 read |

### Tests

| File | Relationship |
|------|--------------|
| `tests/core_test.rs` | `draining_an_empty_ring_is_zero_not_an_error` — guarantee 1 |
| `tests/core_test.rs` | `try_recv_batch_appends_to_the_caller_s_buffer` — guarantee 2 |
| `tests/core_test.rs` | `len_and_is_empty_agree_at_every_point_of_a_lap` — guarantee 5 |
| `tests/core_test.rs` | `a_heap_payload_round_trips_its_contents` — that a record is moved out, not copied |

### CO7 — The Consumer Side Now Carries the Warning the Producer Side Had

**What was found.** `pitfall/001` states that `free_capacity` is binding at SPSC
and advisory elsewhere, and `free_capacity`'s own rustdoc said so in bold.
`Consumer::len` and `Consumer::is_empty` had exactly the same property in mirror
image — a reported `0` can become non-zero the instant a producer on another
thread publishes — and neither said anything about it. Both now do:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_core
command grep -m1 -B11 -A19 -F '  pub fn len( &self ) -> usize' src/lib.rs
```

Live output:

```
  /// How many records are waiting, at least.
  ///
  /// A lower bound: it may grow between this read and the next drain. It never
  /// shrinks on its own, since this is the only consumer.
  ///
  /// **Binding at SPSC, advisory at MPSC and crossbeam** — the mirror image
  /// of [`Producer::free_capacity`](Producer::free_capacity), and asymmetric
  /// for the same reason: against one producer the reading cannot move under
  /// you, against several it can. Let [`try_recv`](Self::try_recv) be the
  /// authority, which is always correct.
  #[ must_use ]
  pub fn len( &self ) -> usize
  {
    match &self.inner
    {
      ConsumerInner::Spsc( consumer ) => consumer.available(),
      ConsumerInner::Mpsc( consumer ) => consumer.available(),
      #[ cfg( feature = "crossbeam" ) ]
      ConsumerInner::Crossbeam( queue ) => queue.len(),
    }
  }

  /// Whether nothing is waiting, by the same reading as [`len`](Self::len).
  ///
  /// **Binding at SPSC, advisory at MPSC and crossbeam.** A reported `true`
  /// can be false the instant a producer on another thread publishes.
  #[ must_use ]
  pub fn is_empty( &self ) -> bool
  {
    self.len() == 0
  }
```

**The producer side got a documented hazard and the consumer side got the same
hazard undocumented.** A caller who read `free_capacity`'s warning and
reasonably concluded that occupancy readings are annotated where they race would
read `is_empty` as reliable. It is reliable at SPSC only, and now says so.

The gap was cheap to make and expensive to notice, which is the shape worth
recording. Nothing was wrong on either side taken alone: `free_capacity` had its
warning, `len` had an accurate lower-bound sentence, and both were written by
somebody who understood the asymmetry. What made the pair misleading was that
one of them was annotated and the other was not — a difference that only exists
across two functions, so no read of either one on its own can surface it. The
same is true of `is_full`, which CO17 covers.

**Disposition:** applied — `len` and `is_empty` each carry the SPSC/MPSC asymmetry in bold and name `try_recv` as the authority, so the annotation is symmetric across the two ends and a caller inferring the rule from `free_capacity` now infers it correctly. Now prints: `**Binding at SPSC, advisory at MPSC and crossbeam** — the mirror image`

### CO8 — The Consumer Is Singular on Every Backend and Nothing Enforces It

`Consumer` has no `try_clone` and no `Clone`, so the singular-consumer property
looks type-enforced. It is actually enforced one level up: `Ends::split` takes
`&'a mut self` and returns the pair, so a second call needs a second mutable
borrow of the same `Ends` and does not compile.

The distinction matters for anyone extending the crate. Adding a `Consumer`
constructor that does not route through `split` would silently break the
invariant with no compile error inside `Consumer` itself, because `Consumer` was
never where the rule lived.
