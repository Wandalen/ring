# API: Producer Surface

### Scope

- **Purpose**: State the single producer's caller-facing surface, and record which of its shape questions are genuinely cheap to leave open because this crate sits behind the family's export boundary.
- **Responsibility**: The operations, their costs, the error cases, and the compatibility position a non-exported crate occupies.
- **In Scope**: The producer end's operations; the three candidate shapes; the error surface.
- **Out of Scope**: The consumer end (→ [Consumer Surface](002_consumer_surface.md)); the procedure behind the surface (→ [Uncontended Claim and Publish](../algorithm/001_uncontended_claim_and_publish.md)); the handle type a consumer actually holds, which is [`ring_handle`](../../../ring_handle/readme.md)'s.

### Abstract

One producer, one method's worth of work: put a record into the ring, or find
out that it is full. Every operation below is **wait-free** — bounded steps,
no loop that can spin on another thread's progress — because there is no other
producer to contend with and the consumer never blocks a publish, only bounds
it.

**This surface is not what a consumer of the family sees.** `ring_spsc` is
not one of the family's externally-visible crates: a consumer obtains SPSC
behaviour by asking
[`ring_factory`](../../../ring_factory/readme.md) for it and receiving
[`ring_handle`](../../../ring_handle/readme.md) values. Gate `G5` fails the
build if any crate outside the family names `ring_spsc` as a dependency.

That is not a footnote — **it is what makes the open questions below cheap.**
A signature change here reaches `ring_core` and `ring_handle` and stops. The
same question left open in [`ring_tls`](../../../ring_tls/docs/api/001_writer_append_surface.md),
which *is* exported, is a public-contract question instead.

### Operations

| Operation | Shape | Cost | Wait-free |
|-----------|-------|------|-----------|
| `try_push( record )` | `Result< (), Full >` | Load, compare, memcpy, release store | Yes |
| `claim()` | `Result< Slot, Full >` | Load, compare | Yes |
| `publish( slot )` | `()` | Release store | Yes |
| `free_capacity()` | `usize` | Acquire load, subtract | Yes |
| `is_full()` | `bool` | Acquire load, compare | Yes |

**Three candidate shapes — shape 3 chosen, with shape 1 layered over it.**

The implementation's `claim()` returns a `Reservation` that publishes on drop,
and `try_push( record )` is built on top of that guard rather than beside it.
Shape 2 was rejected on the abandonment case below; shape 1 was rejected as a
*primitive* because it forces a move of the record into the slot, but adding it
as a convenience over the guard costs nothing and cannot reintroduce the case
the guard rules out — the same "over, never under" rule the consumer surface
applies to `pop()`.

One wrinkle the table did not anticipate: `try_push( record )` is not writable
generically. `Slot` has no by-value setter — `TypedSlot::set` and
`BytesSlot::write` are inherent, with different signatures — so the generic
fused form takes a closure (`push_with`), and the by-value `try_push` exists
only on `Producer< '_, TypedSlot< T > >`. It also returns `Result< (), T >`
rather than `Result< (), RingError >`: a push that fails must hand the record
back, or a caller applying backpressure has nothing to retry with.

The candidates as originally stated:

1. **Fused `try_push`.** One call, record passed by value. Simplest to use
   and impossible to misuse; forces a move of the record into the slot, which
   for a large payload is a copy the other shapes avoid.
2. **`claim` / write / `publish`.** Three calls; the caller writes the payload
   in place inside the slot. Zero-copy for large records; a caller that
   claims and then returns early without publishing stalls the ring
   permanently (→ [Producer and Consumer Pairing](../lifecycle/002_producer_consumer_pairing.md)).
3. **Claim returning a guard.** The guard publishes on drop, making shape 2's
   abandonment case unreachable at the cost of a `Drop` impl in the hot path
   and a lifetime tying the caller to the ring.

Shape 2's hazard is milder here than in the multi-producer ring. An
abandoned claim in `ring_mpsc` wedges *every* producer and loses the entire
published tail behind the gap; in SPSC the abandoning thread is the only
producer, so it wedges only itself — still fatal to the ring, but with no
other thread's data lost alongside it. "Milder" was not enough to keep it: the
cost of shape 3 turned out to be one `Drop` impl containing a single store,
and the lifetime it introduces is one the borrowed drain needs anyway.

**`free_capacity()` is actionable here, and this is the sharp difference from
the multi-producer surface.** In `ring_mpsc` the same call is advisory: another
producer may consume the reported space between the check and the claim, so a
caller can only treat it as a hint. With one producer nothing can take that
space — a reported `free_capacity()` of 8 guarantees the next 8 pushes
succeed. It is the same signature with a materially stronger contract
(→ [Free Capacity](../type/002_free_capacity.md)).

### Error Handling

| Condition | Surfaces as | Caller's options |
|-----------|-------------|------------------|
| Ring full | `Err( Full )` or `RingError` per `ring_types` | Retry, drop per the configured [`ring_overflow`](../../../ring_overflow/readme.md) policy, or apply backpressure upstream |
| A second producer exists | **Nothing** — silent data race | None at runtime. The invariant must be enforced structurally (→ [Exactly One Producer, Exactly One Consumer](../invariant/001_exactly_one_producer_one_consumer.md)) |
| Claim abandoned without publish | **Unrepresentable** — the guard publishes on drop, including on unwind | None needed. This is why shape 3 was chosen |

**The second row is the surface's one genuine danger.** Every operation above
is sound for exactly one caller thread and unsound for two, and nothing in the
signatures says so. `ring_handle`'s `Producer`, which is what an actual
consumer holds, is where non-`Clone`-ness or a `!Sync` marker can make the
second-producer case unrepresentable — `ring_handle`'s own reached-test
asserts the handle split, and it is the enforcement point this surface relies
on but does not itself provide.

**No operation blocks.** Nothing here parks, sleeps, or spins on the
consumer's progress; a full ring is reported, not waited on. That is
the family's try-only requirement satisfied at the source, though it is
enforced at `ring_poll` and `ring_handle` rather than here.

### Compatibility Guarantees

1. **This surface is internal.** `ring_spsc` stays off the family's export
   list; changes reach `ring_core` and `ring_handle` and go no further. The
   open shape questions above cost a family-internal refactor to close, not a
   breaking change.
2. **The surface must remain swappable.** An optional
   crossbeam backend sits behind `ring_core`, and its reached-test requires the
   identical suite to pass against both backends. Anything exposed here that
   crossbeam cannot express is a constraint on that swap
   (→ [Family Dependency Seam](../integration/001_family_dependency_seam.md)).
3. **`free_capacity()`'s stronger contract is SPSC-specific and must not be
   assumed family-wide.** Code written against it that later runs on an MPSC
   ring is silently wrong (→ [SPSC Correctness Does Not Transfer](../pitfall/001_spsc_correctness_does_not_transfer.md)).
4. **Version movement is lockstep** — all 33 crates are path dependencies at
   one workspace version.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_uncontended_claim_and_publish.md](../algorithm/001_uncontended_claim_and_publish.md) | The procedure every operation above runs, identical under all three shapes |

### APIs

| File | Relationship |
|------|--------------|
| [002_consumer_surface.md](002_consumer_surface.md) | The other end; its commit is what frees the capacity this end reports |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_family_dependency_seam.md](../integration/001_family_dependency_seam.md) | Guarantee 2's swap constraint, and the crates a change here reaches |
| [../integration/002_reached_through_the_export_surface.md](../integration/002_reached_through_the_export_surface.md) | Why this surface is not the one a consumer sees |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_exactly_one_producer_one_consumer.md](../invariant/001_exactly_one_producer_one_consumer.md) | The unenforced precondition the Error Handling table's second row describes |
| [../invariant/002_no_lock_in_the_path.md](../invariant/002_no_lock_in_the_path.md) | The wait-free property the Operations table claims |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_producer_consumer_pairing.md](../lifecycle/002_producer_consumer_pairing.md) | Shape 2's abandonment case, and the pairing that makes a second producer unrepresentable |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_spsc_correctness_does_not_transfer.md](../pitfall/001_spsc_correctness_does_not_transfer.md) | Guarantee 3's hazard |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_free_capacity.md](../type/002_free_capacity.md) | The value `free_capacity()` returns, and why it binds here |

### Sources

| File | Relationship |
|------|--------------|
| [`bench_harness/docs/acceptance/001_feature_reached_tests.md`](../../../bench_harness/docs/acceptance/001_feature_reached_tests.md) | This crate's own binary Reached condition, and `ring_handle`'s handle-split assertion this surface leans on |

### Tests

| File | Relationship |
|------|--------------|
| `tests/spsc_test.rs` | `one_producer_and_one_consumer_exchange_one_hundred_thousand_items` — this crate's own claiming test |
| `tests/spsc_test.rs` | `free_capacity_is_actionable_rather_than_advisory` — a reported `n` is followed by exactly `n` successful pushes and then one failure |
| `tests/spsc_test.rs` | `a_failed_push_returns_the_record_rather_than_swallowing_it`, and `push_with_does_not_call_the_writer_when_the_ring_is_full` |
| `tests/spsc_test.rs` | `a_reservation_publishes_on_drop_even_unwritten` — the guard shape's defining behaviour |
| `tests/manual/readme.md` | S2 — the no-lock clause, which no test run can demonstrate |

### SP5 — The Producer Is Neither `Clone` nor `Sync`, and Both Are Asserted

```sh
cd "$(git rev-parse --show-toplevel)"/ring_spsc
printf 'compile_fail doc tests:  '; grep -c 'compile_fail' src/lib.rs
printf 'the send/sync test:      '; grep -oE 'fn both_ends[a-z_]*' tests/spsc_test.rs
```

Live output:

```
compile_fail doc tests:  6
the send/sync test:      fn both_ends_are_send_and_neither_is_sync
```

**The contrast with the sibling is total.** `ring_mpsc::Producer` is
`Send + Sync + Copy` — duplicating it is what multi-producer *means*. Here the
same-named type must not be duplicable at all, and the property is a negative,
so only a compile-fail test can check it.

A reader moving code between the two crates sees one name, one shape, and
opposite intent.

### SP6 — Three Publish Shapes for One Protocol

`claim` hands back a guard and lets the caller write through it; `push_with`
takes a closure and runs it against the slot; `try_push` takes an owned record
and moves it in. All three end at the same cursor advance.

**`push_with` is the one that does not allocate a decision.**
`push_with_does_not_call_the_writer_when_the_ring_is_full` pins the property
that makes it useful — the closure is not invoked on refusal, so a caller can
put expensive record construction inside it and pay nothing when the ring is
full. Neither of the other two can offer that: `claim`'s caller writes after the
check, and `try_push`'s caller has already built the record.
