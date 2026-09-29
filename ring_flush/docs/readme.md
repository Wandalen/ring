# docs

Design documentation for `ring_flush`, as typed doc definitions.

| Directory | Responsibility |
|------|-----------------|
| `algorithm/` | The per-append decision that must cost nothing, and the five-step sequence that reordering collapsed to three |
| `api/` | A policy surface and a driver surface, kept apart because one is configuration and the other is a schedule |
| `data_structure/` | Two pointer widths with no indirection, and the log that exists only because the acceptance criterion is negative |
| `decisions/` | Six trade-offs, four now closed against the implementation, and the two still open filed as ADRs that rule the evidence rather than the question |
| `definition/` | Module Index — every definition, instance, ADR and finding in this crate, in one place |
| `integration/` | Three declared dependencies, eighteen reachable crates, and a position on the export surface that no other exported crate shares |
| `invariant/` | Trigger exclusivity, and the restriction the crate exists to impose: a publication point is designed, not inherited |
| `item/` | Seven nouns and seventeen verbs, catalogued against the four documents that describe the same declarations by role |
| `lifecycle/` | The consolidation cycle and a policy's own arc, plus the two state machines a flush spans as orthogonal axes |
| `non_functional_requirement/` | This crate's binary Reached condition, and the cost ceiling the acceptance table does not state |
| `pattern/` | Policy as a value; driven rather than self-firing — the two choices everything else follows from |
| `pitfall/` | A policy that cannot see its own trigger, and two batch sizes that must not diverge |
| `type/` | Three variants and four outcomes, two of which exist to separate "did not fire" from "fired, found nothing" |
| `workaround/` | External constraints this crate absorbs, with costs and deletion conditions |

**This crate is a decision, not a mechanism, and that is the fact every
directory above is arranged around.** It owns no buffer and no ring. It supplies
one thing `ring_tls` deliberately withheld — a trigger — and one thing the
family needed a place for: the ruling that publication happens at a moment
somebody chose rather than at whatever moment a buffer happens to fill.

The instances sit at three grains. `invariant/` and
`non_functional_requirement/` document the **contract grain** — trigger
exclusivity, the designed publication point, and the binary Reached condition
[the acceptance table](../../bench_harness/docs/acceptance/001_feature_reached_tests.md)
grades them by. `algorithm/`, `data_structure/`, `pattern/` and `pitfall/`
document the **mechanism grain**: how a sixteen-byte value consulted per append
decides, and what the resulting flush must sequence. `api/`, `lifecycle/`,
`type/` and `integration/` document the **surface grain** —
what a caller configures, calls, and must remember to call again.

**Three findings run through the whole set.**

First, **the crate states three obligations it cannot enforce**, and they arrive
from three unrelated directions. A consumer must drive
([`pattern/002`](pattern/002_driven_not_self_firing.md)); must announce barriers
truthfully if it chose `OnBarrier` ([`api/002`](api/002_the_driver_surface.md));
must retry a rejected final drain
([`lifecycle/002`](lifecycle/002_from_configuration_to_the_final_drain.md)'s W3).
None is checkable here, and the third loses data when it is neglected. That
accumulation is not sloppiness — it is what being a policy rather than a thing
costs.

Second, **`OnBarrier` cannot observe a barrier**, and a dependency edge would
not help if it had one. `ring_barrier` is not in this crate's closure — it was,
four edges away through the consumer's side of the ring, until `ring_spsc` and
`ring_mpsc` were implemented and dropped their `ring_consume` edges. The trap
survived the topology change unchanged, because what is missing is the barrier
*instance* the consumer is gated on and a *notification* when it advances, and
no arrangement of edges delivers either
(→ [`pitfall/001`](pitfall/001_on_barrier_cannot_see_the_barrier.md)). That is
why the policy takes the fact as an argument and why the acceptance criterion's
negative case is the one that matters.

**A corollary worth carrying to the other 32 crates:** a finding resting on a
dependency closure is perishable while the family is being implemented, so
every count in these instances ships with the command that regenerates it.

Third, **the acceptance criterion is a runtime negative and therefore needs
evidence the criterion itself does not mandate.** "Fires at exactly its stated
trigger and at no other point" cannot be shown by any positive assertion; it
needs a recorded flush log, and it needs that log to be complete. Nothing in
this crate's acceptance row requires completeness
(→ [`non_functional_requirement/001`](non_functional_requirement/001_three_triggers_proven_by_a_flush_log.md)'s
B4). The mitigation is structural — entries derive from
[`FlushOutcome`](type/002_flush_outcome.md) rather than being written beside it,
so a flush with no log entry is unrepresentable.

**This crate is one of the family's five exported names** — and the only one
that is a decision rather than a thing a consumer holds
([`integration/002`](integration/002_a_decision_on_the_export_surface.md)).
`ring_factory` makes things, `ring_handle` and `ring_tls` are things,
`ring_types` is vocabulary. Importing this one acquires no capability; it
accepts the three obligations above.

No `format/` directory exists: this crate defines no byte layout. `FlushPolicy`
is two pointer widths because of alignment and a discriminant, not because any
external format requires it — and it is written that way rather than as `16`
because the shipped assertion is the one that holds on every target
(→ [`data_structure/001`](data_structure/001_the_policy_enum.md)'s FL10).

`item/` exists, **and this readme argued twice that it should not**. The first
argument was an ordering constraint — `item_des.rulebook.md` reads its required
sections off real declarations and a skeleton had none — and it expired when the
crate was written. The second argument was about grain: every declaration is
already the subject of [`type/001`](type/001_flush_policy.md),
[`type/002`](type/002_flush_outcome.md),
[`data_structure/002`](data_structure/002_the_flush_log.md),
[`api/001`](api/001_the_policy_surface.md) and
[`api/002`](api/002_the_driver_surface.md), so an `item/` instance would be a
fifth listing of the same names.

**The second argument was wrong about what a catalog is for, and the two
instances demonstrate it rather than asserting it.** Those five documents each
describe a *subset* chosen by role — the policy, the outcome, the log, the
configuration surface, the driver surface — and none of them can see a property
that only appears when the whole set is laid out at once.
[`item/001`](item/001_seven_nouns_thirteen_variants.md) finds that five of the
seven nouns share one derive list and the two that differ are the two with
state; [`item/002`](item/002_seventeen_verbs_that_never_touch_the_producer.md)
finds that no verb on the surface reaches the producer, which is the only thing
the crate publishes through. Neither is visible from any single role document,
and neither is a fifth listing of anything.

The general shape, recorded because it will recur across the other 32 crates:
**an anti-duplication argument compares what two documents contain and cannot
see what a document would find.**

### Related Crates

Three declared dependencies, one guarantee depended on without being named, one
crate whose absence leaves an unanswered question, and one sibling on the same
surface with no edge in either direction.

| Crate | Relationship |
|-------|--------------|
| [`ring_tls/readme.md`](../../ring_tls/readme.md) | One of three declared dependencies. Owns seal, drain and reset, and names this crate as where the trigger belongs — the seam this crate exists to complete |
| [`ring_core/readme.md`](../../ring_core/readme.md) | The second. The ring a flush publishes into; its fullness is `OnFull`'s entire question |
| [`ring_types/readme.md`](../../ring_types/readme.md) | The third, and the one this readme long listed as *not* declared. `RingError` is `append`'s failure type, so the edge arrived with D4 rather than with the original design |
| [`ring_batch/readme.md`](../../ring_batch/readme.md) | **Depended on and not declared.** The contiguous claim is what makes a flush of `N` cost one fence; reached through `ring_core`, tested against by nothing here |
| [`ring_barrier/readme.md`](../../ring_barrier/readme.md) | **Unreachable, and briefly was not.** `OnBarrier` is named after it and could not use it either way. The whole subject of `pitfall/001` |
| [`ring_overflow/readme.md`](../../ring_overflow/readme.md) | Reachable and unused. Owns what should happen to an append with nowhere to go — narrowed from two questions to one when reordering removed the stranded-buffer case |
| [`ring_shutdown/readme.md`](../../ring_shutdown/readme.md) | **Unreachable, and the question it raised is closed.** Its `drain_all` is consumer-side (ring → `Vec`); `drain_final` is producer-side (buffer → ring). Opposite ends of the ring, and the shared verb was the whole of the resemblance |
| [`ring_handle/readme.md`](../../ring_handle/readme.md) | The other crate on the export surface, with no edge in either direction. Two of five exported names are independent decisions, which is a correctness property |
| [`ring_testkit/readme.md`](../../ring_testkit/readme.md) | **Unreachable, and now the only possible host.** A counting allocator needs `unsafe`, which gate G6 confines to four declared crates — so the measurement cannot be taken here or in `ring_tls` at all, and the long-term home became the only home |
