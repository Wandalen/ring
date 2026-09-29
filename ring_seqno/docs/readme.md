# docs

Design documentation for `ring_seqno`, as typed doc definitions.

| Directory | Responsibility |
|------|-----------------|
| `algorithm/` | The computations, and the equivalences between them |
| `api/` | The public surface and what its shape commits to |
| `data_structure/` | What the crate operates on, all of it owned elsewhere |
| `decisions/` | Choices with live alternatives, recorded with their arguments |
| `definition/` | Module Index — every definition and every instance in this crate, in one place |
| `integration/` | The one dependency, and how the readings reached four tiers |
| `invariant/` | Properties that must hold for every input |
| `item/` | Per-function contracts and coverage |
| `lifecycle/` | The readings across a lap, and how long each stays true |
| `non_functional_requirement/` | Cost and portability requirements |
| `pattern/` | Reusable shapes this crate instantiates |
| `pitfall/` | Wrong uses that compile and look right |
| `type/` | What the readings are measured in |
| `workaround/` | Compensations the family applies around this crate's shape |

Scope of this crate: sequence numbers and their comparison across laps.

Start at [`definition/readme.md`](definition/readme.md) — it carries the full
instance table and the eleven findings this corpus recorded.

### What a Tier-1 Crate's Documentation Is For

`ring_seqno` is 136 lines with five functions, one import, no types and no state.
Documenting it at the same depth as a crate ten times its size is only worth it
because of where it sits: everything above it computes with these four
subtractions, so this is the one place where the family's shared arithmetic
decisions can be examined together.

That shows in the findings. Eleven were recorded and **six belong to other
crates** — an unused dependency edge in `ring_atomic`, a no-op cast in
`ring_batch`, an inlined duplicate in `ring_consume`, a wrong doc comment in
`ring_types`, and two structural couplings in `ring_debug`. None of them is
visible from inside its own crate; each becomes visible when the reading it
concerns is traced outward from here.

### The Two Threads Running Through the Corpus

**Totality bought by erasure.** Every reading is total for every input
([`invariant/002`](invariant/002_every_reading_is_total.md)), achieved by
saturating subtraction and a guaranteed non-zero capacity. The cost is that a
corrupted ring — a consumer ahead of its producer — reads exactly like a healthy
empty one, which is why `ring_debug` cannot use these functions at all
([`workaround/001`](workaround/001_the_diagnostic_that_reimplements_the_readings.md)).
The decision is right and the consequence is real; both are recorded.

**Decisions kept in, and decisions pushed out.** `may_claim` centralises the
boundary comparison so no caller re-derives it
([`pattern/001`](pattern/001_the_predicate_beside_its_quantity.md)); `slowest`
refuses to centralise the empty-set identity, because four consumers resolve it
four different ways — maximum, minimum, propagate, and error
([`pattern/002`](pattern/002_the_shared_fold_that_declines_an_identity.md)). The
pair is the crate's whole design philosophy, and picking the mathematically
correct identity would have been the dangerous choice.
