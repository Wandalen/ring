# API: Producer Surface

### Scope

- **Purpose**: Fix the publishing contract a caller gets regardless of backend, and mark which of its promises are uniform and which are per-backend.
- **Responsibility**: Method set, signatures, error shape, compatibility guarantees, and the two divergences from [`ring_handle`](../../../ring_handle/docs/api/001_producer_surface.md)'s specified surface.
- **In Scope**: `Producer::try_push`, `try_push_batch`, `try_clone`, `free_capacity`, `is_full`.
- **Out of Scope**: The procedures behind them (→ [`algorithm/001`](../algorithm/001_backend_dispatch_and_the_refusal_seam.md)); the consumer half (→ [`api/002`](002_consumer_surface.md)).

### Abstract

Five publishing operations, identical in shape at every backend, none of which
allocates. Two of the five carry a promise that is not uniform: `free_capacity`
is binding at SPSC and advisory elsewhere, and `is_full` inherits exactly that
split because it is defined in terms of it.

### Operations

| Operation | Signature | Returns | Allocates | Notes |
|---|---|---|---|---|
| Publish one | `try_push( &mut self, record : T ) -> Result< (), T >` | unit, or the record back | **Never** | The refusal carries the record, which is what makes recovery free |
| Publish a batch | `try_push_batch( &mut self, records : &mut impl Iterator< Item = T > ) -> usize` | how many were accepted | Never | Partial acceptance is the normal case |
| Second producer | `try_clone( &self ) -> Option< Producer< '_, T > >` | `None` at SPSC | Never | The cardinality discriminator |
| Ask for room | `free_capacity( &self ) -> usize` | a lower bound | Never | **Binding at SPSC, advisory elsewhere** |
| Ask if full | `is_full( &self ) -> bool` | `free_capacity() == 0`, literally | Never | **Not a second reading** — the same contract in boolean form |

### Error Handling

**There is no error type on this surface.** A full ring is not a failure — it is
the condition the ring's bounded capacity exists to produce — so refusal is
reported in the return value's own shape rather than through an `Err` variant
carrying a message:

| Operation | How a refusal appears | What the caller has afterwards |
|---|---|---|
| `try_push` | `Err( record )` | The record, intact and owned — recovery costs nothing |
| `try_push_batch` | A count below the number offered | The iterator, positioned after the refused record |
| `try_clone` | `None` | Certainty that this backend is single-producer, not a transient failure |

**Nothing here panics, and nothing returns a `Result< _, E >` with an error
type.** `Err( T )` is the record itself, so there is no allocation, no formatting,
and nothing to render. The one hazard is that the batch form's count and the
iterator's position have to agree — guarantee 4 below is what pins that.

### Compatibility Guarantees

1. **A refusal never destroys the record.** `Err( record )` hands it back intact
   at every backend. This required work at MPSC, whose native `push` consumes
   the value — see [`algorithm/001`](../algorithm/001_backend_dispatch_and_the_refusal_seam.md)'s
   refusal seam, which is where the crate's one shipped bug was.

2. **Nothing on this surface allocates.** A publish path that allocates has a
   failure mode the ring's own bounded capacity was chosen to avoid.
   `try_push_batch` takes `&mut impl Iterator` rather than a collection for the
   same reason: the caller's records need never be gathered first.

3. **`free_capacity` means something strictly stronger at SPSC**, and the
   signature does not say so. This is the surface's sharpest hazard and it has
   its own instance (→ [`pitfall/001`](../pitfall/001_free_capacity_carries_two_contracts.md)).
   `try_clone` is the only machine-checkable way to tell which contract applies.

   **`is_full` is inside this guarantee, not beside it.** It is defined as
   `self.free_capacity() == 0` with no `match` and no per-backend arm, so a
   caller who avoids `free_capacity` and asks `is_full` instead has changed the
   spelling and nothing else — `!is_full()` is exactly as advisory at MPSC and
   crossbeam as the number it wraps. This crate never calls `ring_spsc`'s own
   `is_full`; there is only ever the one definition.

4. **`try_push_batch` consumes the refused record from the iterator.** A caller
   resuming from the same iterator resumes *after* the refusal. Asserted, not
   merely stated.

### Divergence From `ring_handle`'s Specified Surface

[`ring_handle` api/001](../../../ring_handle/docs/api/001_producer_surface.md)
specifies the same operations one layer up, and three things differ. The
divergence is real and unresolved; reconciling it belongs to `ring_handle`,
once it is implemented.

| Point | `ring_handle` api/001 | here | |
|---|---|---|---|
| `try_push` receiver | `&self` | `&mut self` | diverges |
| `try_push_batch` receiver | `&self` | `&mut self` | diverges |
| `free_capacity` receiver | `&self` | `&self` | **agrees** |
| Refusal type | `Full< T >` | `T` | diverges |
| `is_closed` | present | **absent** | settled |

**The receiver divergence is exactly the two mutating methods, and that is not
arbitrary.** The two that differ reach a backend producer; the one that agrees
only reads. Honouring `&self` needs a producer that can be used without
exclusive access at every backend — and measured, that is already true at two
of the three:

| Backend producer | Usable behind `&self`? | |
|---|---|---|
| `ring_mpsc::Producer` | yes — `impl Copy` (`ring_mpsc/src/lib.rs:738`) | ready |
| crossbeam arm | yes — it is a `&ArrayQueue`, and every operation takes `&self` | ready |
| `ring_spsc::Producer` | **no** — no `Copy`, no `Clone` | the blocker |

So the divergence reduces to one crate, not three. That is a much smaller
question than the table alone suggests, and it is the reason `try_clone`'s MPSC
arm can already write `*producer`.

`is_closed` is settled: liveness belongs to `ring_shutdown`, and
a handle-local copy of that flag is the failure that crate exists to prevent,
so this crate has no flag to read (→ [`invariant/001`](../invariant/001_no_atomic_of_its_own.md)).

The refusal type is the cheapest of the three to change and the least urgent:
`Full< T >` carries the same record `T` does, with a name attached.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_backend_dispatch_and_the_refusal_seam.md](../algorithm/001_backend_dispatch_and_the_refusal_seam.md) | How guarantee 1 is achieved over three differently-shaped backends |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_handle_surface_divergence.md](../integration/002_handle_surface_divergence.md) | The divergence table above, as an integration obligation on `ring_handle` |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_free_capacity_carries_two_contracts.md](../pitfall/001_free_capacity_carries_two_contracts.md) | Guarantee 3, worked out as a caller-facing trap |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_producer_cardinality.md](../type/002_producer_cardinality.md) | `try_clone`'s return as the discriminator guarantee 3 relies on |

### Tests

| File | Relationship |
|------|--------------|
| `tests/core_test.rs` | `a_refused_record_comes_back_on_every_backend` — guarantee 1 |
| `tests/core_test.rs` | `try_clone_refuses_at_spsc_and_permits_elsewhere`, `cloned_producers_share_one_ring` — the cardinality half |
| `tests/core_test.rs` | `free_capacity_never_overstates_the_room_available` — the testable half of guarantee 3 |
| `tests/core_test.rs` | `a_partial_batch_push_reports_its_count_and_consumes_the_refused_record` — guarantee 4 |
| `tests/manual/readme.md` | C6 — the divergence table, checked by running a command rather than by review |

### CO5 — Five of Seven Producer Methods Have Family Callers

```sh
cd "$(git rev-parse --show-toplevel)"
DEP="ring_bench ring_debug ring_factory ring_flush ring_handle ring_poll ring_shutdown ring_testkit"
for m in '\.try_push(' '\.try_push_batch(' '\.free_capacity()' '\.is_full()' '\.try_clone()'; do
  N=0; L=0
  for c in $DEP; do
    k=$( cat $c/src/*.rs | grep -vE '^\s*(//|///|//!)' | grep -c "$m" )
    L=$(( L + k )); [ "$k" -gt 0 ] && N=$(( N + 1 ))
  done
  printf '%-20s lines %d  crates %d\n' "$m" "$L" "$N"
done
```

Live output:

```
\.try_push(          lines 6  crates 5
\.try_push_batch(    lines 6  crates 5
\.free_capacity()    lines 4  crates 4
\.is_full()          lines 3  crates 3
\.try_clone()        lines 0  crates 0
```

The surface this instance documents is used, unevenly, and one method is not
used at all. That last zero is the subject of its own decision
(→ [`../decisions/001`](../decisions/001_the_backend_discrimination_surface_has_no_caller.md)).

### CO6 — The Producer Is Not `Send` at Crossbeam and the Docs Do Not Say So

`Producer< 'a, T >` borrows the ring. Whether two of them can cross a thread
boundary is decided by the inner handle's own auto-traits, which differ per
backend — and the API documentation for this type discusses cardinality
(`try_clone`) without ever discussing `Send`.

A caller reading only this crate learns that multiple producers may exist and not
whether they may exist on different threads. The information is recoverable —
`four_threads_publishing_through_clones_lose_nothing` in `tests/core_test.rs`
demonstrates it for the MPSC backend — but a test is a poor place to discover a
type's thread-safety contract.

Recorded as a gap rather than a hazard: nothing here is stated wrongly, and the
compiler enforces the truth regardless of what the prose omits.
