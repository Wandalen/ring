# Pitfall: SPSC Correctness Does Not Transfer to MPSC

### Scope

- **Purpose**: Name the trap created by this crate's own role as the family's correctness floor — that "validated against SPSC" reads as evidence about the ring design when it is evidence about one configuration of it.
- **Responsibility**: The trap's shape, the specific properties that silently fail to carry, the failure it produces, and what actually mitigates it.
- **In Scope**: Reasoning and code carried from the SPSC configuration to the multi-producer one.
- **Out of Scope**: The MPSC ring's own pitfalls (→ [`ring_mpsc`](../../../ring_mpsc/docs/pitfall/readme.md)); whether either wins the benchmark.

### Trap

This crate is "the shape the rest of the family is validated against," on the
argument that getting it right first "means the multi-producer path is an
extension of something already known good rather than the first thing tried."

That is a sound development strategy and it invites an unsound inference:
**that properties established here hold in the general case.** They do not, and
the ones that fail are precisely the ones that make this crate fast.

| Property established here | Holds in MPSC? | Why not |
|---------------------------|----------------|---------|
| A plain load of the producer cursor is sound | **No** | The cursor has N writers; a plain load is a data race |
| `free_capacity()` binds — `n` reported means `n` publishes succeed | **No** | Another producer may take the space first (→ [Free Capacity](../type/002_free_capacity.md)) |
| The producer cursor is the available bound | **No** | It is a *claim* bound; sequences below it may be unpublished |
| The published region is a contiguous prefix | **No** | A slower producer leaves a hole (→ [Slot State Without Holes](../lifecycle/003_slot_state_without_holes.md)) |
| Publish and drain are wait-free | **No** | The claim becomes a contended RMW — lock-free, not wait-free |
| No per-slot state is needed | **No** | Stamps are how the hole is detected |
| Full is actionable rather than racy | **No** | The observation can be invalidated before it is acted on |

**Seven properties, and every one of them fails.** That is not a caveat on an
otherwise-transferable design; it is closer to saying the two rings share a
name and a slot array.

The trap has teeth because the *code* often does transfer — the same call
sites, the same method names, the same shapes compile against both. The
reasoning that justified the code is what does not transfer, and reasoning
leaves no compile error behind.

The family's own structure makes it easier to fall into.
[`ring_core`](../../../ring_core/readme.md) puts both rings behind one
interchangeable-backend API, and the family requires "the identical test
suite" to pass against a third one. Interchangeable at the surface is exactly
the condition under which a caller stops tracking which is underneath.

### Failure

| Carried assumption | Failure under MPSC | Visibility |
|--------------------|--------------------|------------|
| Trusting `free_capacity()` before a batch | Some publishes return `Full` mid-batch; a caller that treated the check as a guarantee drops records or panics | **Load-dependent.** Correct at one producer, degrades as producer count rises — passes CI, fails in production |
| Assuming publication is contiguous | A consumer written to drain to the producer cursor reads an unpublished slot | **Silent** — garbage or a torn value, indistinguishable from a payload bug |
| Assuming wait-freedom | A latency budget derived from SPSC measurements is exceeded under contention | Visible as a timing regression whose cause is attributed to the workload |
| Reusing a plain cursor load | Data race; UB | **Silent on x86-64**, reproducible on AArch64 |

**Every row degrades with producer count**, which is the worst possible shape:
the configuration used during development (one producer, for simplicity) is
the one where the bug does not appear.

**This pitfall is more likely to bite the family's own authors than its
consumers.** A consumer reaches the rings through
[`ring_handle`](../../../ring_handle/readme.md) and never names either crate,
by design. The person carrying an SPSC assumption into MPSC code is whoever
implements `ring_core`, `ring_flush`, or the benchmark — people who have read
this crate closely and correctly, which is exactly what makes the inference
feel safe.

### Mitigation

1. **State the properties as SPSC-specific where they are stated at all.**
   Every instance in this crate that records one of the seven names the
   multi-producer contrast in the same table. That is why those contrasts are
   present throughout rather than gathered here — a reader meets the warning
   where the property is, not in a document they may not open.

2. **Make the reached-tests configuration-specific.** This crate's own test
   asserts one producer; `ring_mpsc`'s asserts four. Neither is a rename of
   the other, and the acceptance table keeps them as separate rows with
   separate claiming tests — `ring_spsc/tests/spsc_test.rs` and
   `ring_mpsc/tests/mpsc_test.rs`. A single shared suite would encode exactly
   the assumption this pitfall warns against.

3. **Do not let `ring_core`'s interchangeability imply interchangeable
   reasoning.** "Identical test suite against both backends" is
   a claim about the *surface*, and it is correct as stated. It is not a claim
   that a caller may reason identically about the two — that is the inference
   to block, and `ring_core`'s own docs are where to block it.

4. **Prefer the properties that do transfer when writing shared code.**
   "Publish returns `Result`", "drain is batch-shaped", "nothing blocks" hold
   in both. `free_capacity()` binding, contiguous publication, and wait-freedom
   do not. Shared code written against only the first set is portable by
   construction.

**What does not mitigate it: a comment.** The seven properties are load-bearing
in the code that uses them, and a note saying "SPSC only" next to a call site
does not survive the refactor that moves the call.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_producer_surface.md](../api/001_producer_surface.md) | Its Compatibility Guarantee 3 is this pitfall stated as a contract |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_two_cursor_ring.md](../data_structure/001_two_cursor_ring.md) | The absent per-slot state, row 6 of the trap table |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_reached_through_the_export_surface.md](../integration/002_reached_through_the_export_surface.md) | Why the family's own authors are the exposed population, not consumers |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_no_lock_in_the_path.md](../invariant/002_no_lock_in_the_path.md) | The wait-freedom row, and why it degrades rather than breaks |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_correctness_floor_for_the_family.md](../non_functional_requirement/001_correctness_floor_for_the_family.md) | The role that creates this trap in the first place |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/003_slot_state_without_holes.md](../lifecycle/003_slot_state_without_holes.md) | The contiguity row |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_free_capacity.md](../type/002_free_capacity.md) | The binding-versus-advisory row, worked out in full |

### Sources

| File | Relationship |
|------|--------------|
| [`bench_harness/docs/acceptance/001_feature_reached_tests.md`](../../../bench_harness/docs/acceptance/001_feature_reached_tests.md) | This crate and `ring_mpsc` as separate rows with separate claiming tests — mitigation 2 |

### Tests

| File | Relationship |
|------|--------------|
| `tests/spsc_test.rs` | `with_config_takes_the_capacity_and_ignores_the_rest` — mitigation 2, expressed structurally. There is no producer-count parameter to default: `split` yields exactly one of each end, and a `RingConfig` carrying `producer_count = 8` is not honoured here |

### SP44 — The Non-Transfer Is Enforced by the Type System, Not by Documentation

The pitfall warns that SPSC correctness does not transfer. The signatures
enforce it: `Ring::split` here yields `( Producer, Consumer )` directly, while
`ring_mpsc::Ring::ends` yields an `Ends` that must then be split.

**A migration therefore fails to compile rather than failing at runtime**, which
is the strongest available form of this warning and is not what the pitfall's
prose claims — it argues from reasoning about cardinality rather than from the
type difference that already blocks the mistake.

### SP45 — The Reverse Direction Is Unguarded

Reasoning built for many producers is conservative at one — that direction is
safe. What is unsafe is a habit formed here and carried there: assuming the
producer cursor is a frontier, or that a claim cannot be overtaken.

`ring_core` holds both backends behind one API, so a contributor reads both
crates in one sitting. The pitfall names the risk in the safe direction and the
type system covers that one; the risky direction is covered by neither.

```sh
cd "$(git rev-parse --show-toplevel)"
grep -c 'Reasoning honed on this crate is conservative' ring_spsc/src/lib.rs
```

Live output:

```
1
```

**Disposition:** applied — `ring_spsc/src/lib.rs`'s crate-level doc now states
the reverse-direction risk explicitly, naming the two specific habits (cursor
as frontier, claim as impossible to overtake) that are safe here and unsafe
carried into `ring_mpsc`, at the point a contributor reads before going to look
at the other crate. Now prints: `1`
