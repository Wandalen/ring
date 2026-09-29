# API: Consumer Surface

### Scope

- **Purpose**: Specify the draining end's operations, its absences, and the one property that distinguishes it from the producer end — that where this value *lives* is a correctness question, not just a design one.
- **Responsibility**: The operations, their error handling, and the compatibility guarantees the export list imposes.
- **In Scope**: `Consumer`'s methods and non-methods; the drain shape.
- **Out of Scope**: The publishing end (→ [Producer Surface](001_producer_surface.md)); where the value should be held (→ [The Barrier Holds the Consumer](../lifecycle/002_the_barrier_holds_the_consumer.md)).

### Abstract

**`Consumer` is the value whose holder is allowed to drain — and, because there
is exactly one of it, whoever holds it *is* the consume point.**

That is the asymmetry with the producer end. A `Producer` in the wrong place
publishes from the wrong thread, which is a cardinality question the ring
crates already speak to. A `Consumer` in the wrong place makes "what had been
published when the consumer looked" depend on thread timing — the one input a
replay cannot reproduce.

So the surface below is ordinary, and the value it belongs to is not.

### Operations

| Operation | Shape | Returns | Blocks | Notes |
|-----------|-------|---------|--------|-------|
| Drain one item | `try_recv( &mut self ) -> Option< T >` | The item, or nothing available | **Never** | `Option`, not `Result` — "nothing available" is not an error, and modelling it as one makes every caller unwrap a non-failure |
| Drain a batch | `try_recv_batch( &mut self, out: &mut Vec< T > ) -> usize` | How many were moved | **Never** | The amortizing shape. Writing into a caller-owned buffer avoids allocating per drain |
| Drain everything available | `drain( &mut self ) -> Drain< '_, '_, T >` | An iterator over the published prefix | Never | Bounded at call time by what was published then, not by what arrives during iteration — otherwise it never terminates under a live producer |
| Ask how much is waiting | `len( &self ) -> usize` | A lower bound | Never | May grow between the read and the drain; never shrinks, since this is the only consumer |
| Ask whether nothing is waiting | `is_empty( &self ) -> bool` | — | Never | `len() == 0` |

**`drain` returns a named `Drain` type rather than `impl Iterator`.** The bound
is the guarantee, and `size_hint()` is how a caller reads it — an opaque
`impl Iterator` would hide the one property the operation exists for.

**The `is_closed` row is absent, and the receivers are `&mut self`.** Both are
corrections this instance's producer-side twin explains in full
(→ [`api/001`](001_producer_surface.md)'s "Three shapes this instance specified
that the implementation changed", and [`decisions/002`](../decisions/002_why_is_closed_is_absent.md)).

**The third row's bound is the one subtle operation on this surface.** A
`drain()` that yields whatever is available *as it iterates* has no termination
guarantee while a producer is running — it is a live-lock dressed as an
iterator, and it satisfies "never blocks" at every individual step. Fixing the
bound at call time is what makes a full drain a bounded operation, and it is
also what makes the drain deterministic given a fixed publication history,
which is what a deterministic replay requires.

**`len()` is a lower bound in the opposite direction from
[`free_capacity`](001_producer_surface.md).** Because this is the only consumer,
nothing else removes items, so the count can only grow — a caller that drains
`len()` items will succeed. The producer's `free_capacity` at MPSC cardinality
has no such guarantee, and the asymmetry is worth stating because the two
methods read as duals and are not.

#### Absent operations, and why

| Absent | Why | Consequence if added |
|--------|-----|----------------------|
| Any publish / `try_push` | The capability belongs to `Producer` | [Capability Follows the Handle](../invariant/001_capability_follows_the_handle.md)'s V2 — caught by the compile-fail case |
| A blocking `recv` | Nothing reachable from a handle may park | W1. And this one is *more* tempting than its producer counterpart, because "wait for work" is what a consumer normally does |
| `recv_timeout( d )` | Bounded parking is parking | W2 |
| `Clone` | Two consumers would each see a partial stream, and the consume point stops being a point | The determinism this arrangement buys is gone; **not covered by the compile-fail cases as specified** |
| `Deref`, `inner()`, a public field | Restores the full backend surface | V4 |
| `peek()` returning a reference | Would borrow from the ring across a slot that the producer may overwrite | A use-after-overwrite with no unsafe block in sight at the call site |

**The `Clone` row is a sharper problem here than on the producer end.** A
cloned `Producer` is a data race — bad, and at least a known category. A cloned
`Consumer` is not a race at all: two consumers each drain a well-formed subset,
nothing corrupts, every test passes, and the system is no longer replayable
because which subset each got depends on timing.

**The `peek` row is worth stating even though it is not obviously wanted**,
because it is the natural way to write "look before you commit" and it cannot
be sound here: the slot backing the reference is reusable the moment the
consumer cursor advances, and nothing in the borrow's type ties it to that
cursor.

### Error Handling

| Condition | Result | Meaning |
|-----------|--------|---------|
| Nothing published since the last drain | `None` | **Not an error.** The normal case at a barrier that ran with no work |
| The ring is closed and drained | `None` | Indistinguishable from the row above by return value alone. Nothing on *this* surface separates them — the distinction lives in [`ring_shutdown`](../../../ring_shutdown/readme.md), for the reason in [`decisions/002`](../decisions/002_why_is_closed_is_absent.md) |
| The ring is closed with items outstanding | The items, then `None` | Close does not discard; `drain_all()`, `ring_shutdown`'s operation, is the operation that guarantees this |
| No consumer capability | — | **Not a runtime condition.** The call does not compile |

**Rows one and two collapsing is a deliberate simplification with a cost.** A
consumer that must distinguish "empty now" from "empty forever" needs a second
call, and a consumer that does not check will spin at a barrier that no longer
has a producer. The alternative — a `Result< Option< T >, Closed >` — makes the
common case, which is a plain drain at a barrier, carry two layers of unwrapping
for a condition that occurs once per ring.

### Compatibility Guarantees

1. **`try_recv` returns `Option`, not `Result`.** Empty is not a failure, and
   changing this reshapes every call site.
2. **`drain()`'s bound is fixed at call time.** An implementation that keeps
   yielding as items arrive is not a compatible change, it is an unbounded loop
   with the same signature.
3. **`len()` is a lower bound that only grows.** This holds because there is one
   consumer, and it stops holding the moment a `Clone` is added — so the
   guarantee and the absent-`Clone` row are the same commitment stated twice.
4. **The absent list is part of the contract** — additions to it break other
   crates' guarantees without breaking any caller.
5. **No error or return type names a crate outside the five-name export list.**
6. **Version movement is lockstep.**

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_delegating_to_the_backend.md](../algorithm/002_delegating_to_the_backend.md) | What each operation does between call and return |

### APIs

| File | Relationship |
|------|--------------|
| [001_producer_surface.md](001_producer_surface.md) | The complementary half, and `free_capacity`'s opposite-direction bound |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_capability_follows_the_handle.md](../invariant/001_capability_follows_the_handle.md) | The absent-publish row, stated as an invariant |
| [../invariant/002_no_parking_operation_is_reachable.md](../invariant/002_no_parking_operation_is_reachable.md) | Why the blocking `recv` row is absent despite being the natural consumer idiom |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_the_barrier_holds_the_consumer.md](../lifecycle/002_the_barrier_holds_the_consumer.md) | Why where this value lives matters more than what its methods are |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_a_convenience_method_undoes_the_crate.md](../pitfall/001_a_convenience_method_undoes_the_crate.md) | The absent rows as edits with plausible justifications |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/004_ring_liveness_through_a_handle.md](../lifecycle/004_ring_liveness_through_a_handle.md) | The closed/drained states rows one to three of Error Handling distinguish |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_consumer.md](../type/002_consumer.md) | The value this surface belongs to |

### Sources

| File | Relationship |
|------|--------------|
| [`../invariant/001_capability_follows_the_handle.md`](../invariant/001_capability_follows_the_handle.md) | "A consumer handle can drain and cannot publish" |
| [`../lifecycle/002_the_barrier_holds_the_consumer.md`](../lifecycle/002_the_barrier_holds_the_consumer.md) | Why the `Clone` row and the `drain()` bound are correctness questions rather than API taste |
| [`ring_shutdown/readme.md`](../../../ring_shutdown/readme.md) | The close semantics Error Handling's third row depends on |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/handle_test.rs`](../../tests/handle_test.rs) · `drain_is_bounded_at_the_call_that_made_it` | `drain()` terminates while a producer is actively publishing — guarantee 2, the one that distinguishes a bounded drain from a live-lock |
| [`tests/handle_test.rs`](../../tests/handle_test.rs) · `an_empty_ring_answers_promptly`, `drain_of_an_empty_ring_yields_nothing` | `try_recv` on an empty ring returns `None` and does not spin — guarantee 1. The timing half is a threshold, not a proof: 10 000 empty drains under 500 ms, chosen because a 50 µs park on each would land right at it |
| [`tests/ui/consumer_publishes.rs`](../../tests/ui/consumer_publishes.rs) | The absent-operations table's first row |

### HD7 — Every Refusal on This Surface Is a Zero-Information `Option`

The draining side never says *why* nothing came back:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the consuming signatures --'
command grep -E '^  pub fn (try_recv|try_recv_batch|len|is_empty|drain)\(' ring_handle/src/lib.rs
echo '  -- Result vs Option across the whole public surface --'
printf '  returning Result: %s\n' \
  "$( command grep -cE '^  pub (const )?fn .*-> Result<' ring_handle/src/lib.rs )"
printf '  returning Option: %s\n' \
  "$( command grep -cE '^  pub (const )?fn .*-> Option<' ring_handle/src/lib.rs )"
```

Live output:

```
  -- the consuming signatures --
  pub fn try_recv( &mut self ) -> Option< T >
  pub fn try_recv_batch( &mut self, out : &mut Vec< T > ) -> usize
  pub fn drain( &mut self ) -> Drain< '_, 'a, T >
  pub fn len( &self ) -> usize
  pub fn is_empty( &self ) -> bool
  -- Result vs Option across the whole public surface --
  returning Result: 1
  returning Option: 1
```

`try_recv` returns `Option< T >`. One method on the whole crate returns a
`Result`, and it is on the producing side.

**"Empty" and "closed and will never fill" are the same value here.** The
producing surface makes exactly this distinction load-bearing — api/001's own
Error Handling section argues at length that collapsing `Full` and `Closed` into
one refusal makes a caller spin against a dead ring — and the consuming surface
collapses the analogous pair without comment.

The asymmetry is defensible: a consumer that keeps polling an empty ring wastes
a tick, while a producer that keeps retrying a closed one loses records. It is
also undocumented, and a reader who takes api/001's argument seriously will
expect its mirror here and find an `Option`.

Closing the gap needs `ring_shutdown`, which
[`decisions/002`](../decisions/002_why_is_closed_is_absent.md) rules out of
reach for this crate — so the asymmetry is forced, and stating that is the
whole fix available today.

### HD8 — The Only `Vec` in the Public Surface Is on the Draining Side

`try_recv_batch` takes a `&mut Vec< T >`, and nothing else here names a
container:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- container types in public signatures --'
command grep -E '^  pub (const )?fn ' ring_handle/src/lib.rs \
  | command grep -E 'Vec<|Box<|\[|impl Iterator'
echo '  -- and what the publishing side takes instead --'
command grep -E '^  pub fn try_push_batch\(' ring_handle/src/lib.rs
```

Live output:

```
  -- container types in public signatures --
  pub fn try_push_batch( &mut self, records : &mut impl Iterator< Item = T > ) -> usize
  pub fn try_recv_batch( &mut self, out : &mut Vec< T > ) -> usize
  -- and what the publishing side takes instead --
  pub fn try_push_batch( &mut self, records : &mut impl Iterator< Item = T > ) -> usize
```

`try_push_batch` takes `&mut impl Iterator< Item = T >` — no container at all.
`try_recv_batch` takes a caller-owned `Vec` to append into.

**The two batch methods are mirror operations with non-mirror signatures**, and
the reason is real: an iterator cannot be filled, only drained, so the receiving
side needs somewhere to put things. But `Vec` is a choice among several — a
slice, a `SmallVec`, an `Extend` bound — and it is the one that forces an
allocation the caller may not want on a tick path.

Nothing here is wrong. What is missing is that this is the crate's single point
of contact with the allocator, on a surface whose whole argument
(→ [`algorithm/002`](../algorithm/002_delegating_to_the_backend.md)) is that it
adds nothing to the hot path. The choice deserves a line in the instance and has
none.
