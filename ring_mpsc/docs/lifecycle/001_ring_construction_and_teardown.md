# Lifecycle: Ring Construction and Teardown

### Scope

- **Purpose**: Decompose a ring's life into phases with stated entry conditions, and identify the one phase transition that carries a correctness obligation rather than a resource-release one.
- **Responsibility**: Name each phase, what establishes it, what it guarantees, and what must be true to leave it; state the drop-versus-drain question plainly.
- **In Scope**: Allocation, steady state, quiescence, and deallocation of one ring.
- **Out of Scope**: An individual producer's shorter cycle (→ [Producer Attachment and Detachment](002_producer_attachment_and_detachment.md)); the states within steady state (→ [Ring Occupancy Between the Cursors](../lifecycle/004_ring_occupancy_between_cursors.md)); how a shutdown signal reaches the producers, which is a sibling crate's (→ [Family Dependency Seam](../integration/001_family_dependency_seam.md)).

### Lifecycle Phases

| Phase | Established by | Guarantees while in it | Leaves via |
|-------|----------------|------------------------|------------|
| **P1 Constructed** | Allocation of the slot array, the stamp array, and both cursors; capacity validated | Both cursors read 0; every stamp holds a value below every lap-0 expected sequence; no thread holds a sequence | First claim, or teardown without ever being used |
| **P2 Steady** | The first claim | Producers may claim, write, and publish; the consumer may drain; occupancy moves freely within `0 ..= CAPACITY` | A shutdown signal reaching every producer |
| **P3 Quiescent** | The last producer has published or abandoned its final claim, and no new claims will occur | `producer_cursor` is final; every claimed sequence is either published or permanently unpublished | The consumer draining to the final watermark |
| **P4 Deallocated** | The ring's memory is released | Nothing; the ring no longer exists | — |

**P1's stamp precondition is the one initialization detail that matters.**
Every stamp must start below the lap-0 expected sequence for its slot, so
that the drain's `stamps[i] == expected` test reads *not published* on a
fresh ring rather than accidentally matching. Zeroed memory satisfies this
for slot 0 only by luck — slot 0's lap-0 expected sequence is 0, so a zeroed
stamp there reads as *published* and the consumer drains an uninitialized
payload cell. The fix is standard and must be stated rather than assumed:
initialize `stamps[i]` to a sentinel that is below every expected sequence
for that slot — conventionally `i.wrapping_sub(CAPACITY)`, i.e. the value
lap -1 would have left — or start the cursors at `CAPACITY` rather than 0.

**Neither of the two was taken; a third option is simpler and this instance's
framing was what hid it.** The framing assumed the sentinel must be *below*
the expected sequence, which is an ordering test's requirement. The drain's
test is not an ordering test — it is `stamps[ i ] == expected`, an equality.
Under equality, any value that is never a legitimate sequence works, and it
need not be ordered against anything.

So `UNSTAMPED = Seq( u64::MAX )` is the sentinel: a value a real sequence
reaches only after 2⁶⁴ publications, uniform across every slot, requiring no
per-slot arithmetic and no cursor offset. The instance's core warning was
exactly right and is why this is stated rather than assumed — a
`vec![ 0; capacity ]` stamp array *is* a bug on slot 0 — but the fix it
proposed was more machinery than the bug required.
`stamps_start_unstamped_and_there_is_exactly_one_per_slot` and
`draining_an_empty_ring_yields_an_empty_batch_and_moves_nothing` assert both
halves: the sentinel is what it should be, and a fresh ring drains nothing.

### Phase Transitions

| # | From → To | Trigger | Obligation |
|---|-----------|---------|------------|
| L1 | P1 → P2 | First successful claim | None |
| L2 | P2 → P3 | Last producer stops claiming | **External.** The ring has no mechanism to know this |
| L3 | P3 → P4 | Consumer drains to the final watermark, then the owner drops the ring | The correctness obligation — see below |
| L4 | P1 → P4 | Drop without ever being used | None; no payload was ever written |
| L5 | P2 → P4 | Drop while producers are live | **Forbidden** — see below |

**L2 is a transition this crate cannot detect, and that is a real gap rather
than a simplification.** Nothing in the ring observes producer count. The
consumer sees a stalled `producer_cursor`, which is indistinguishable from
producers that are merely idle, and no timeout can tell them apart correctly.
So P3 is established by a signal from outside — a shutdown flag, a producer
count reaching zero, a scope ending — and the ring is a passive participant.
[`ring_shutdown`](../../../ring_shutdown/readme.md) exists in the family for
exactly this, and the seam is
[Family Dependency Seam](../integration/001_family_dependency_seam.md)'s.

**L3 is where the correctness obligation lives, and it is easy to get wrong
by treating teardown as a memory problem.** Dropping the ring frees the slot
array. If any slot is Published-but-undrained at that moment, its payload's
destructor never runs — every element the consumer never got to is leaked,
silently, with no diagnostic. For a POD payload this is harmless. For a `T`
owning a heap allocation it is a leak proportional to the undrained backlog,
which is largest exactly when the system is under the load that made shutdown
necessary. So teardown has two possible contracts:

- **Drain-on-drop** — the ring's `Drop` walks from `consumer_cursor` to the
  published watermark and drops each element in place. Correct for any `T`,
  costs a scan at teardown, and requires `Drop` to be able to reason about
  publication state, which it can, since P3 guarantees no concurrent
  producer.
- **Leak-on-drop, documented** — release the memory and state that undrained
  elements are not dropped. Correct only if `T: Copy`, and this crate does
  not currently constrain `T`.

**Decided: drain-on-drop, and it cost neither a scan nor a `T: Copy` bound.**
This instance's contribution was to show the choice was not free — that
payload-agnosticism (→ [Sequence Number](../type/001_sequence_number.md)) makes
leak-on-drop unsafe as a default, so teardown and agnosticism trade against
each other. The trade turned out to be avoidable, because the framing had one
assumption too many: it assumed the *ring* must decide what happens to
undrained payloads.

It does not. Slots live in a `Buffer< S >`, and `Buffer`'s own `Drop` drops
every slot it owns — published, claimed, or never touched — because a slot is
just a value in an array. A `TypedSlot< T >` holding a `T` drops that `T`; an
empty one drops nothing. So undrained elements are destroyed exactly once at
teardown without this crate writing any teardown code at all, without a
publication-state scan, and for any `T`. The leak this instance warned about
would have required *going out of the way* to leak — a `ManuallyDrop`, or
`mem::forget` on the buffer.

`every_record_written_is_destroyed_exactly_once` asserts it with a
drop-counting payload: publish N, drain M < N, drop the ring, count N
destructor calls. That is exactly the test this instance asked for.

The POD coupling (→ [`ring_tls`'s POD payload requirement](../../../ring_tls/docs/non_functional_requirement/002_pod_pointer_free_payloads.md))
therefore **decouples**: it was "the same decision made twice" only under
leak-on-drop, where agnosticism was what made leaking unsafe. `ring_tls` may
still want POD payloads for its own reasons — thread-local storage has its
own constraints — but it no longer inherits a requirement from here.
`a_heap_payload_arrives_with_its_contents_rather_than_a_shallow_copy` is the
counter-case: a `String` round-trips intact, so agnosticism is exercised
rather than merely permitted.

**L5 is forbidden, and the borrow checker now enforces it.** Dropping the ring
while a producer may still claim is a use-after-free, not a leak. This
instance said Rust's ownership model handles it *if* producer handles borrow
the ring, and that is the shape chosen: `Producer< 'a, S >` and
`Consumer< 'a, S >` both borrow `&'a Ring< S >`, so the ring cannot be dropped
while either exists. There is no raw pointer and no `Arc`. L5 is not a
documented prohibition — it is a compile error, asserted by a `compile_fail`
doc test on `Reserved`, which cannot outlive the ring either.

### Dependencies

| Dependency | Phase | Why |
|------------|-------|-----|
| [`ring_config`](../../../ring_config/readme.md) | P1 | Supplies the capacity value and any policy parameters the constructor validates |
| [`ring_store`](../../../ring_store/readme.md) | P1, P4 | Owns the slot-array allocation and release this phase model treats as atomic steps |
| [`ring_cursor`](../../../ring_cursor/readme.md) | P1–P3 | Owns both cursors, including their initial values — which is where the stamp-sentinel-versus-cursor-offset choice above is actually made |
| [`ring_shutdown`](../../../ring_shutdown/readme.md) | L2 | The external signal this crate cannot generate for itself |
| [`ring_gating`](../../../ring_gating/readme.md) | P2, P3 | Owns the published-watermark computation the final drain in L3 needs |

### Cleanup Requirements

1. **The consumer drains to the final watermark before the ring is
   dropped** — or `Drop` does it, per the undecided contract above. One of
   the two must happen; neither happening is the leak.
2. **No producer holds an unpublished claim at P4.** An abandoned claim
   leaves a permanent gap: the drain stops at it and every later sequence,
   already published, becomes unreachable. Teardown after an abandoned claim
   therefore leaks *more* than the abandoned element — it leaks the entire
   tail behind it. This is the same wedge
   [Slot State Across One Lap](../lifecycle/003_slot_state_across_one_lap.md)'s
   Behavioral Invariant 3 names, seen from the teardown side.
3. **Both arrays are released together.** They are sized by the same capacity
   and have no independent lifetime; releasing one without the other is not a
   partial teardown, it is corruption.
4. **No cursor reset is required or meaningful.** P4 is terminal — the ring
   is not reused. A `clear()` that reset both cursors to 0 would produce a
   ring whose stamps all hold lap-`n` values against lap-0 expectations,
   which is P1's initialization bug reintroduced deliberately. If reuse is
   ever wanted, it is a fresh construction, not a reset.

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_sequence_stamped_ring.md](../data_structure/001_sequence_stamped_ring.md) | The fields allocated in P1 and released in P4; states that no operation in P2 allocates |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_family_dependency_seam.md](../integration/001_family_dependency_seam.md) | Where the L2 shutdown signal enters from, and which sibling crate owns each phase's machinery |

### Lifecycles

| File | Relationship |
|------|--------------|
| [002_producer_attachment_and_detachment.md](002_producer_attachment_and_detachment.md) | The shorter cycle nested inside P2; its handle shape is what makes L5 safe or unsafe |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/003_slot_state_across_one_lap.md](../lifecycle/003_slot_state_across_one_lap.md) | Its Behavioral Invariant 3 is Cleanup Requirement 2 seen from the per-slot side |
| [../lifecycle/004_ring_occupancy_between_cursors.md](../lifecycle/004_ring_occupancy_between_cursors.md) | Occupancy is 0 in P1 and must reach 0 for a clean P3 → P4 |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_capacity.md](../type/002_capacity.md) | Validated in P1; immutable for P2–P4, which is what makes the two arrays' shared lifetime sound |

### Sources

| File | Relationship |
|------|--------------|
| `src/lib.rs` | `Ring::new`/`Ring::with_config` are P1; `UNSTAMPED` is the sentinel this instance's construction section argues for; teardown is `Buffer`'s `Drop`, so P4 is code this crate does not write |

### Tests

| File | Relationship |
|------|--------------|
| `tests/mpsc_test.rs::draining_an_empty_ring_yields_an_empty_batch_and_moves_nothing` | A freshly constructed ring yields zero elements — the slot-0 stamp-sentinel bug, asserted rather than reasoned about |
| `tests/mpsc_test.rs::stamps_start_unstamped_and_there_is_exactly_one_per_slot` | The sentinel's value and the stamp array's shape, checked directly rather than inferred from the drain's behaviour |
| `tests/mpsc_test.rs::every_record_written_is_destroyed_exactly_once` | With a drop-counting payload: publish N, drain M < N, drop the ring, N destructors — the teardown contract this instance left open, resolved in the drain-on-drop direction |
| `tests/mpsc_test.rs::a_heap_payload_arrives_with_its_contents_rather_than_a_shallow_copy` | That payload-agnosticism survived the teardown decision rather than being traded for it |
| `tests/mpsc_test.rs::a_capacity_that_is_not_a_power_of_two_is_refused_before_a_ring_exists` | P1's validation, failing before any allocation |
| `src/lib.rs` `compile_fail` doc test | L5: a `Reserved` cannot outlive the ring, so drop-while-live is a compile error rather than a documented prohibition |

### MP31 — Construction Allocates Twice and Never Again

```sh
cd "$(git rev-parse --show-toplevel)"/ring_mpsc
grep -E 'Box<|Buffer::new|\.collect' src/lib.rs | grep -vE '^ *(//|///|//!)'
```

Live output:

```
  stamps : Box< [ AtomicSeq ] >,
      .collect::< Vec< _ > >()
      slots : Buffer::new( capacity ),
```

After `new` returns, no path allocates. That is what lets
`non_functional_requirement/002` describe backpressure as bounded — the ring
cannot grow to absorb a burst, so refusal is the only response, and refusal is
the property being measured.

### MP32 — `with_config` Reads One Field and Ignores the Rest

`a_config_supplies_the_capacity_and_its_other_fields_are_deliberately_unread` is
the test, and its name carries the intent. The policy and producer count are
`ring_core`'s to interpret — this crate is always multi-producer and always
refuses on full.

**The constructor has no production caller** (→ [`../decisions/002`](../decisions/002_the_observation_surface_kept_without_a_caller.md)):
`ring_core` builds its MPSC arm through `Ring::new` with a `Capacity` it has
already validated. So a constructor that exists to accept a `RingConfig` is not
the one the config-driven layer uses.
