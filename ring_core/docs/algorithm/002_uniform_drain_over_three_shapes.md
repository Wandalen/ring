# Algorithm: Uniform Drain Over Three Drain Shapes

### Scope

- **Purpose**: Specify how one `try_recv` and one `try_recv_batch` are served by three backends whose native drain shapes differ in kind, and record which of the three costs a copy.
- **Responsibility**: The per-backend drain procedure, the batch shape and why it appends, and the one place the uniform surface is measurably more expensive than the backend beneath it.
- **In Scope**: `Consumer::try_recv`, `Consumer::try_recv_batch`, `Consumer::len`, `Consumer::is_empty`.
- **Out of Scope**: Each backend's internal commit protocol (→ [`ring_spsc` algorithm/002](../../../ring_spsc/docs/algorithm/002_single_consumer_drain.md), [`ring_mpsc` algorithm/002](../../../ring_mpsc/docs/algorithm/002_batch_drain_by_cursor_swap.md)); the publish side (→ [`algorithm/001`](001_backend_dispatch_and_the_refusal_seam.md)).

### Abstract

Two drain entry points — one record, or many — served by three backends whose
native drain shapes differ in *unit of work*, not merely in spelling. Each
backend's cheapest operation is the other's awkward one, so the uniform surface
costs a copy in exactly one place, and that place is named rather than averaged
away.

### Algorithm

Dispatch on the private enum, then express the caller's requested shape in terms
of whatever the backend offers natively.

#### Three native shapes

The drain side is where the backends diverge most, and unlike the publish side
the divergence is not only in the API but in the *unit of work*:

| Backend | Native drain | Unit | Commit |
|---|---|---|---|
| SPSC | `drain()` → `Batch` | everything published since the last batch | the batch's `Drop` |
| MPSC | `drain()` → `Batch` | everything contiguously published | the batch's `Drop` |
| crossbeam | `pop()` → `Option< T >` | exactly one record | immediate |

So one backend's cheapest operation is the batch and another's is the single
pop, and each is awkward when expressed in terms of the other. The surface has
to offer both, and it does — `try_recv` and `try_recv_batch` — with the cost
falling differently on each backend.

#### Procedure — `try_recv`

**Output:** `Some( record )`, or `None` if nothing is available.

| Step | SPSC and MPSC | crossbeam |
|---|---|---|
| 1 | `drain_up_to( 1 )` | `pop()` |
| 2 | `get_mut( 0 )`, taking the record out of the slot; `None` if the batch is empty | return it directly |
| 3 | the batch's `Drop` commits one record, or none | — |

**`drain_up_to( 1 )` rather than `drain()` is what makes step 3 correct.** A
full `drain()` would take every published record into the batch, and the
batch's `Drop` commits *the whole batch* — so a `try_recv` that returned one
record would silently consume the rest. The bound is not an optimisation; it is
the difference between taking one record and discarding an unknown number.

**Step 2 is why [`ring_spsc::Batch::get_mut`](../../../ring_spsc/readme.md)
exists.** `get` yields a `&S`, which is enough to read a record but not to move
one out, and the uniform surface hands the caller a `T` rather than a `&T` —
because `ArrayQueue::pop` gives an owned value and no borrow can outlive the
pop. `ring_mpsc`'s batch always had `get_mut`; `ring_spsc`'s did not, and the
gap surfaced only here, at the composition point, which is the kind of gap a
composition point exists to surface.

#### Procedure — `try_recv_batch`

**Input:** `&mut Vec< T >`. **Output:** how many records were moved into it.

It **appends** rather than replacing, and reports only its own count — not the
buffer's length. Both choices are asserted rather than documented, because both
are the kind of thing a caller assumes:

```
try_recv_batch_appends_to_the_caller_s_buffer
```

Appending is what lets a caller accumulate across several calls without a
second buffer, which is exactly what the reached-test's drain loop does.

#### The one place this costs a copy

At crossbeam, `try_recv_batch` reads `len()`, then pops that many times,
collecting into a temporary before extending the caller's buffer.

**That temporary is a real cost, and it is not present at the other two
backends**, where the batch is already a contiguous view the records are moved
out of. It exists because `ArrayQueue` has no batch drain at all — the only way
to take *n* records is *n* pops, and the count must be sampled before the loop
since another consumer could otherwise make the loop unbounded.

Two consequences worth stating plainly:

- **The `len()` sample is advisory**, so the collected count may be lower than
  it. `filter_map` over `0..len` handles that: a pop returning `None`
  contributes nothing and the reported count is what was actually taken, never
  what was hoped for.
- **`try_recv_batch` at crossbeam is not cheaper than a loop of `try_recv`.**
  It is offered for surface uniformity, not for throughput. A caller
  benchmarking batch drain across backends is measuring this asymmetry as much
  as the backends — which is worth knowing before `ring_bench` attributes it
  to the queue.

### Occupancy Readings

`len` and `is_empty` are two computations of one fact — a cursor subtraction at
the in-house rings, a load at crossbeam — which is exactly the pair that can
drift apart. They are checked against each other at every point of a lap rather
than at the endpoints, because the interesting disagreement is mid-lap:

```
len_and_is_empty_agree_at_every_point_of_a_lap
```

Both are **advisory at every backend**, including SPSC — unlike
`free_capacity`, which is binding there. The asymmetry is real: the consumer's
peer is the producer, and a producer can always add records, so no occupancy
reading is ever a lower bound that stays true. A caller draining until
`is_empty` is racing, and should drain until `try_recv` returns `None` instead.

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_consumer_surface.md](../api/002_consumer_surface.md) | The contract this procedure implements |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_uniform_delivery_across_backends.md](../invariant/002_uniform_delivery_across_backends.md) | Exactly-once delivery, which the batch commit in step 4 is what preserves |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/003_occupancy_across_backends.md](../lifecycle/003_occupancy_across_backends.md) | The occupancy states `len` and `is_empty` read |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_spsc/readme.md`](../../../ring_spsc/readme.md) | `Batch::get_mut`, added for step 3; its own manual plan records the gap |
| [`ring_mpsc/docs/api/002_consumer_drain_surface.md`](../../../ring_mpsc/docs/api/002_consumer_drain_surface.md) | The batch shape both in-house backends share |

### Tests

| File | Relationship |
|------|--------------|
| `tests/core_test.rs` | `try_recv_batch_appends_to_the_caller_s_buffer` — the append and the count |
| `tests/core_test.rs` | `draining_an_empty_ring_is_zero_not_an_error` — the empty case at both entry points |
| `tests/core_test.rs` | `len_and_is_empty_agree_at_every_point_of_a_lap` — the occupancy pair |
| `tests/core_test.rs` | `a_heap_payload_round_trips_its_contents` — that step 3 moves the record rather than copying its bits |

### CO3 — The Two Methods Whose Ignored Return Destroys Data Now Say So

**What was found.** Eight of this crate's sixteen public functions carried
`#[ must_use ]`, and the split did not track how much ignoring the return cost:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_core
awk '/must_use/{m=1;next} /^    pub (const )?fn /{n=$0; sub(/^    pub (const )?fn /,"",n);
     sub(/\(.*/,"",n); printf "%-16s must_use %s\n", n, (m?"yes":"no"); m=0}' src/lib.rs
```

Live output:

```
new              must_use no
new_crossbeam    must_use no
backend          must_use yes
capacity         must_use yes
overflow         must_use yes
ends             must_use no
split            must_use no
try_push         must_use no
try_push_batch   must_use yes
try_clone        must_use yes
free_capacity    must_use yes
is_full          must_use yes
try_recv         must_use no
try_recv_batch   must_use yes
len              must_use yes
is_empty         must_use yes
```

Read the `no` rows. `new`, `ends`, `split`, `try_push` and `try_recv` return
`Result` or `Option`, which `core` already marks `#[ must_use ]` — the crate's
own attribute would be redundant, and its absence costs nothing.

**`try_push_batch` and `try_recv_batch` returned a plain `usize`, and that was
the only channel by which a caller learned the operation was partial.** `let _ =
producer.try_push_batch( &mut records );` compiled clean and silently discarded
however many records did not fit — and, because the batch push consumes one
extra record on refusal, one more besides. The two functions whose ignored
return actually destroys data were the two the compiler would not complain
about. They are the `must_use yes` rows above now.

**The attribute found twenty-four call sites the moment it was added, and every
one of them was a test.** Twelve in `ring_flush`, seven here, three in
`ring_poll`, one each in `ring_debug` and `ring_shutdown` — each staged a batch,
or drained one, and threw the count away. That is the finding's own claim
arriving as evidence rather than as an argument: the discards were not in some
hypothetical dependent, they were in the suites that exist to hold these crates
honest, written by people who had the invariant in mind and still had no reason
to type the count.

**More of them were in `ring_flush` than here**, which is the part worth
carrying forward. A hazard on a crate's own surface is normally found by reading
that crate; this one was concentrated one tier up, in the crate that drives
batches hardest, where reading `ring_core` would never have reached it. The
attribute found it in one build, in a crate nobody was looking at.

**Twenty-three of the twenty-four already knew the answer.** A test that pushes
`0..5` into a ring of eight is asserting something whether or not it says so,
and the fix is to say it —
`assert_eq!( producer.try_push_batch( &mut ( 0..5 ) ), 5 )`. The twenty-fourth
is the one worth having: `core_test.rs` drains the remainder after a loop of
unknown length, so the count is genuinely not known in advance, and what it
asserts instead is that the returned count equals the vector's growth. That
contract is the reason the return value is load-bearing at all, and nothing in
the suite had ever checked it.

**Disposition:** applied — both batch methods carry `#[ must_use ]`, so discarding the count is a build error under `-D warnings`; the twenty-four sites that were discarding it are twenty-four assertions instead, across six test files in five crates, and the family's 650 tests pass with the attribute in place. Now prints: `try_push_batch   must_use yes`

### CO4 — The Drain Loop Calls `try_recv` Once Per Record

The name suggests a bulk operation; the in-house arms are a `for` loop over
`try_recv`. That is not a defect — neither `ring_spsc` nor `ring_mpsc` offers a
bulk drain, so there is nothing to delegate to — but it bounds what a caller can
expect. Batching here amortizes the dispatch `match`, not the per-record work.

The crossbeam arm is the exception and pays for it in the other direction: it
`collect`s into an intermediate `Vec` before extending the caller's buffer
(→ [`../non_functional_requirement/002`](../non_functional_requirement/002_the_composition_adds_no_atomic_and_one_allocation.md)),
so the one arm that avoids the per-record loop is the one that allocates.

**Disposition:** declined — this instance's own text states plainly "that is
not a defect", since neither `ring_spsc` nor `ring_mpsc` offers a bulk drain
to delegate to; the finding records a measured architectural bound, not
something for this crate's own source or
`algorithm/002_uniform_drain_over_three_shapes.md` to fix.
