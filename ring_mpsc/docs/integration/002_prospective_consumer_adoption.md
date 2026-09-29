# Integration: Prospective Consumer Adoption

### Scope

- **Purpose**: State what adopting this crate would actually require of a consumer, so the adoption cost is known before the benchmark verdict arrives rather than discovered while acting on it.
- **Responsibility**: Name the prospective consumers, what each would replace, the constraints adoption imposes on the consumer's own code, and the one consumer that no longer exists but still defines the bar.
- **In Scope**: The adoption seam above the family's export surface; the consumer-side changes adoption implies; the measured baseline adoption must beat.
- **Out of Scope**: Whether adoption happens, which is the family's own verdict; the family-internal seams (→ [Family Dependency Seam](001_family_dependency_seam.md)); the measurement itself (→ [Measured Before Adopted](../non_functional_requirement/001_measured_before_adopted.md)).

### System Description

This crate is not a dependency of anything today. It exists because two
independent consumers were found to need the same mechanism, and building it
twice was an outcome the family acted to prevent. Adoption is
future work in both cases and gated on a measured verdict.

> **This instance cites a deleted predecessor crate on purpose.** That
> predecessor was deleted on 2026-08-26 and carried no surviving mechanism,
> so it has **no successor crate**. Its working mailbox mechanism remains
> the measured baseline this crate's adoption gate is written against — the
> thing a winning ring has to beat — so the references below are deliberate
> citations of a predecessor, not stale names awaiting a rename. Repointing
> them at `ring_mpsc` would make the crate contrast against itself. This is
> the same reading note [`../readme.md`](../readme.md) carries, applied to
> the adoption seam specifically.

| Prospective consumer | Status | What it would replace | What it gains |
|----------------------|--------|-----------------------|---------------|
| the prospective consumer | Live crate, has not adopted | Its documented single-threaded intent-submission serialisation point | Parallel submission — the serialisation point becomes a lock-free compare-exchange rather than a lock |
| ~~the deleted predecessor crate~~ | **Deleted 2026-08-26**, no successor | The predecessor's internal CAS-stack mailbox mechanism | Would have gained bounded footprint and no per-message allocation |

**The second row is why the first row's measurement is not self-referential.**
With the predecessor deleted, the prospective consumer is the only live one,
and a benchmark comparing this crate against *nothing* would establish only
that it works. The deleted predecessor's mailbox supplies a real,
previously-working implementation with a known shape — a CAS stack with
per-message heap nodes — to measure against. Deleting the crate did not
delete the comparison, and that is the whole reason these citations are
preserved rather than pruned.

### Integration Points

| # | Seam | What crosses it | What it requires of the consumer |
|---|------|-----------------|----------------------------------|
| J1 | The prospective consumer's submission intake | Intents from many gameplay threads to one merge point | That its intent type becomes the ring's `T` — see the payload constraint below |
| J2 | The prospective consumer's merge ordering | The ring's total order becomes the log's order | **That the log accepts hardware-arrival order as its canonical order.** This is the substantive one |
| J3 | Export surface | The consumer names `ring_handle`/`ring_factory`, never `ring_mpsc` | That it takes the family's public crates as dependencies, keeping [Family Dependency Seam](001_family_dependency_seam.md)'s Compatibility Requirement 1 intact |
| J4 | Shutdown | The consumer signals quiescence; the ring cannot detect it | That it owns producer-count or scope tracking itself (→ [Ring Construction and Teardown](../lifecycle/001_ring_construction_and_teardown.md)'s L2) |

**J2 is the adoption cost that is not about performance at all.** The
prospective consumer's determinism thesis — log-is-truth, replay reproduces
state — requires that the log's order be reproducible. A ring's total order
is the order producers won the claim race in, which is reproducible *within
one run* and not across runs: replaying the same workload on the same
machine yields a different interleaving. (The race is a compare-exchange loop
rather than a fetch-add, a choice already ruled on at the family level.
This makes J2 *worse*, not better, and the direction is worth noting: a
fetch-add's winner is decided by one hardware arbitration, while a CAS loop's
winner is decided by arbitration plus however many retries each contender
happened to take. Non-reproducibility across runs was already total, so
nothing changes operationally — but any future argument that the interleaving
is "nearly" stable has one more reason to be wrong.) So adopting this crate for the prospective consumer means either
(a) the log's order is defined as whatever the ring produced and determinism
is recovered by replaying the *log* rather than the workload, or (b) a
deterministic ordering key is carried in the payload and the merge sorts by
it after draining, which reintroduces a sort the ring's ordering was supposed
to have removed.

Option (a) is almost certainly the intended reading — replay-from-log is
the prospective consumer's stated model, and it does not require the
*production* order to be reproducible. But the distinction is load-bearing
enough that adopting
without stating which reading applies would leave the determinism guarantee
resting on an unexamined assumption. **This instance does not decide it**;
it names it as a question that belongs to the adoption decision rather than
to this crate's mechanism.

**J1's payload constraint was the one expected to bite, and it does not.**
This instance recorded that nothing bounds `T`, that `Send` is certainly
required, and that the teardown contract *may* force `Copy`
(→ [Ring Construction and Teardown](../lifecycle/001_ring_construction_and_teardown.md)'s
L3) — which would exclude an intent type owning heap data, and route adoption
through [`ring_tls`](../../../ring_tls/readme.md)'s staging half rather than
directly into this ring.

**No `Copy` bound exists.** Teardown is drain-on-drop: `Ring`'s `Drop` walks
the undrained range and clears each slot, so owned payloads are destroyed
exactly once with no `Copy` requirement anywhere.
`a_heap_payload_arrives_with_its_contents_rather_than_a_shallow_copy` publishes
a `String` and reads its contents back on the other side;
`every_record_written_is_destroyed_exactly_once` counts destructor calls
against a drop-counting payload across drained and undrained slots alike.

So J1 costs a consumer nothing, the `ring_tls` routing is not needed for this
reason, and the Error Handling row below that assumed it is now unreachable.

**The prediction was sound and the mitigation was unnecessary, which is a
distinction worth keeping.** The `Copy` risk was real when written: it followed
from a teardown design that was genuinely under consideration. What retired it
was choosing a different teardown, not discovering the risk had been
overstated. A reader should take from this that the constraint moved rather
than evaporated — and specifically that
[`ring_tls`](../../../ring_tls/readme.md)'s own POD requirement, which cited
this coupling, no longer inherits a justification from here.

### Error Handling

Adoption failure modes are decisions, not runtime errors:

| Failure | Meaning | Consequence |
|---------|---------|-------------|
| The verdict favours another pattern | This crate is not the winner | The crate stays unadopted; the family's other write-path crates carry the winner. Not a defect — [Measured Before Adopted](../non_functional_requirement/001_measured_before_adopted.md) exists to permit this outcome |
| The verdict favours the ring but J2 is unresolved | Mechanism wins, ordering semantics unsettled | Adoption blocked on a determinism ruling, not on code |
| ~~J1's payload constraint excludes the consumer's type~~ | ~~The ring cannot carry the intent directly~~ | **Unreachable since a later revision.** There is no `Copy` bound and no POD requirement; an owned payload round-trips and is dropped exactly once. Kept struck through rather than deleted, because the row records a risk that was correctly identified and then designed away, not one that was wrong |
| No consumer ever adopts | Both prospective consumers pick something else | The crate is deleted. A shared crate with no sharers is duplication with extra steps |

**The last row is a real possible outcome and stating it is the point.** This
crate's justification is that two consumers needed the same thing; one of
those two has since been deleted. If the prospective consumer also declines, the
sharing argument is empty and the honest response is removal rather than
keeping a skeleton indefinitely. Recording that here means the decision is
visible when the verdict lands rather than being quietly deferred forever.

### Compatibility Requirements

1. **Consumers integrate through the export surface, never directly.** J3.
   A direct `ring_mpsc` dependency from outside the family forfeits the
   freedom that makes every open question in this crate non-breaking.
2. **The measured baseline must remain citable.** The deleted `Mailbox`'s
   documented mechanism is the comparison point; the citations above are
   deliberate and must not be repointed at a successor, because there is no
   successor.
3. **Adoption is per-consumer, not family-wide.** The prospective consumer
   adopting would not commit anything else. There is no coordinated migration and no
   version negotiation — the family's crates are path dependencies at one
   workspace version (→ [Family Dependency Seam](001_family_dependency_seam.md)'s
   Compatibility Requirement 3).
4. **The ordering semantics in J2 must be stated by the consumer, not
   assumed by this crate.** This crate guarantees a total order; it does not
   and cannot guarantee that the order is reproducible across runs, and no
   consumer should read the total-order invariant as promising that.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_producer_publish_surface.md](../api/001_producer_publish_surface.md) | The surface J1 crosses; its `T` stayed unbounded, so the payload constraint narrowed nothing |
| [../api/002_consumer_drain_surface.md](../api/002_consumer_drain_surface.md) | The surface the consumer's merge point calls; its Compatibility Guarantee 1 is what J2 interrogates |

### Integrations

| File | Relationship |
|------|--------------|
| [001_family_dependency_seam.md](001_family_dependency_seam.md) | The seam below; its export-surface boundary is what J3 requires consumers to respect |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_single_consumer_total_order.md](../invariant/001_single_consumer_total_order.md) | Guarantees a total order; J2 shows that is not the same as a reproducible one |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_ring_construction_and_teardown.md](../lifecycle/001_ring_construction_and_teardown.md) | Its L2 is J4's obligation and its L3 is J1's payload constraint |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_measured_before_adopted.md](../non_functional_requirement/001_measured_before_adopted.md) | The gate this whole seam waits behind, and the instance that permits the not-adopted outcome |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_channel_to_ring_binding.md](../pattern/001_channel_to_ring_binding.md) | How a consumer's declared channel would bind to a ring instance — the mechanical half of what J1 requires |

### Sources

| File | Relationship |
|------|--------------|
| `src/lib.rs` | Implemented; still adopted by nothing. `Ring`'s `Drop` is what retires J1's payload constraint, and the absence of any `Copy` or POD bound on `S` is the checkable form of that |

### Tests

| File | Relationship |
|------|--------------|
| `tests/mpsc_test.rs::a_heap_payload_arrives_with_its_contents_rather_than_a_shallow_copy` | J1's payload constraint, answered: a `String` crosses the ring with its contents, so the unbounded-`T` claim holds and the `ring_tls` routing is not needed for this reason |
| `tests/mpsc_test.rs::every_record_written_is_destroyed_exactly_once` | The other half of the same answer — destructor accounting across drained and undrained slots, which is what a `Copy` bound would have existed to avoid |
| `tests/mpsc_test.rs::a_bytes_payload_round_trips_its_written_length` | The POD-shaped case still works, so retiring the constraint widened the admissible payloads rather than trading one set for another |

### MP20 — The Only Live Consumer Is `ring_core`

The instance discusses prospective consumers. The measured present is two
crates, one of which uses a single name.

**That makes `ring_core` the whole of this crate's contract in practice.** A
change to `Producer`'s signature affects one caller; a change to `Ring::new`'s
affects two. The prospective consumers named here would each roughly double the
surface actually under contract.

### MP21 — A Prospective Consumer Would Reach the Unreached Methods

`committed`, `published_through` and `claimed` are the three numbers a
backpressure layer reads to decide whether to admit work; `available` is the one
it reads to decide whether to drain. Only the last has a caller.

So [`../decisions/002`](../decisions/002_the_observation_surface_kept_without_a_caller.md)'s
open question resolves differently depending on this instance's prospective
consumers arriving or not — which is a dependency between two open items worth
recording in both.
