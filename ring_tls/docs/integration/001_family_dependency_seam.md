# Integration: Family Dependency Seam

### Scope

- **Purpose**: State what this crate composes from — a notably short list — and account for the family crates whose absence from that list leaves real capabilities unowned.
- **Responsibility**: Name each declared dependency's contribution, each conspicuous absence and what it leaves open, and what sitting *on* the export boundary rather than behind it costs in design freedom.
- **In Scope**: `ring_types`, `ring_atomic` and `ring_batch`; the dropped `ring_slot`; the inbound-only `ring_flush` edge; the absent `ring_config`, `ring_registry` and `ring_shutdown`; the external export boundary.
- **Out of Scope**: The consumer above the boundary (→ [Prospective Consumer Adoption](002_prospective_consumer_adoption.md)); each dependency's own mechanism.

### System Description

`ring_tls` declares **three** dependencies where its sibling
[`ring_mpsc`](../../../ring_mpsc/docs/integration/001_family_dependency_seam.md)
declares seven:

```toml
[dependencies]
ring_types  = { path = "../ring_types" }
ring_atomic = { path = "../ring_atomic" }
ring_batch  = { path = "../ring_batch" }
```

That is consistent with the mechanism being genuinely simpler — a bump
pointer and a region, against a ring's slots, stamps, two cursors, claim,
publish, gating, and consume. But it is not only simplicity. Three family
crates that would plausibly serve this one are absent, and each absence
leaves a capability this crate's own instances describe as needed.

| Layer | Crates | This crate's relationship |
|-------|--------|---------------------------|
| **Above** — external surface | `ring_factory`, `ring_handle`, **`ring_tls`**, `ring_flush`, `ring_types` | This crate **is one of them.** Its surface is a public contract, not an internal seam |
| **This crate** | `ring_tls` | Owns the per-thread bump-append discipline, and nothing else |
| **Below** — composed from | `ring_types`, `ring_atomic`, `ring_batch` | Three crates. `ring_slot` was here and was removed during implementation — see S2 |
| **Absent but implicated** | `ring_registry`, `ring_flush`, `ring_config`, `ring_shutdown` | Each named by an instance here as owning something this crate needs |

**Being on the export boundary rather than behind it is the structural
difference from `ring_mpsc`, and it inverts that crate's main freedom.**
`ring_mpsc` can leave every open question open indefinitely, because
`ring_handle` absorbs any change before it reaches a consumer. `ring_tls` has
no such absorber: its signatures are the contract. So the open questions in
[Writer Append Surface](../api/001_writer_append_surface.md) and
[Consolidator Read Surface](../api/002_consolidator_read_surface.md) are more
expensive to leave open here than the equivalent ones are there — the same
question, different cost, purely because of which side of the boundary the
crate sits on.

### Integration Points

| # | Seam | Direction | What crosses it | Assumption made |
|---|------|-----------|-----------------|-----------------|
| S1 | [`ring_types`](../../../ring_types/readme.md) | This crate → | The shared record vocabulary tags are drawn from | That `ring_types` supplies a *tag representation* without supplying tag *meanings* — otherwise this crate's payload-agnosticism (→ [Record Tag](../type/001_record_tag.md)'s V2) is broken by its own dependency |
| S2 | [`ring_slot`](../../../ring_slot/readme.md) | This crate → (dev only) | Slot addressing, in tests | **Resolved by demotion.** This instance recorded the fit as questionable; implementation moved it from `[dependencies]` to `[dev-dependencies]`. It is no longer part of the mechanism, only of testing it — see below |
| S2a | [`ring_atomic`](../../../ring_atomic/readme.md) | This crate → | The ordering primitives the seal/publish handshake is built from | That the epoch swap this crate's sealing depends on is expressible in `ring_atomic`'s vocabulary rather than needing raw intrinsics |
| S2b | [`ring_batch`](../../../ring_batch/readme.md) | This crate → | Contiguous multi-slot claim, so a drained region lands as one claim rather than N | That this crate's "single contiguous claim" need and `ring_batch`'s own batch claim are the *same* mechanism — if they diverge, this crate's central throughput property depends on a crate that is optimising for a different caller |
| S3 | Export boundary | → Consumers | The writer and consolidator surfaces | That consumers depend on this crate directly, with no absorbing indirection |
| S4 | [`ring_flush`](../../../ring_flush/readme.md) | → This crate | Sequences seal/drain/reset into a consolidation cycle | **Declared, and inbound only** — `ring_flush/Cargo.toml` names this crate; this crate names nothing back. It cannot reach, notify, or even identify its own consolidator |

**S2 was recorded here as an open question and implementation answered it.**
The question was that `ring_slot` is named for fixed-size indexed slots — the
ring's shape — while a bump-allocated append log has variable-length records
addressed by offset, not by index. Two explanations were offered: either
`ring_slot` is more general than its name suggests, or the dependency is a
leftover from the family-wide decomposition being applied uniformly rather than
per-crate. **It was the second, and the resolution is more precise than
removal.** `ring_slot` moved from `[dependencies]` to `[dev-dependencies]`;
`ring_atomic` and `ring_batch` took its place among the real ones. That is the
shape the mechanism actually needs — ordering primitives for the seal
handshake, a contiguous batch claim for the drain — while slot addressing
survives exactly where it was always meaningful: in tests, which exercise this
crate against a ring and therefore do speak in slots.

**The demotion is the interesting part, and it generalises.** A dependency
that belongs in the tests but not the implementation is invisible to
`cargo tree --depth 1`'s first stanza and to any reading of `[dependencies]`
alone, yet it is exactly the shape "this abstraction is about how we *check*
the thing, not what the thing *is*" takes in a manifest. S2b is the row that
most resembles the original S2 question: `ring_batch` is depended on for a
property — one contiguous claim — that its own feature describes from a
different caller's point of view, and nothing yet guarantees the two readings
stay the same.

**Verify the current dependency set rather than trusting this table:**

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/\[dependencies\]/,/^\[/p' ring_tls/Cargo.toml
```

Live output:

```
[dependencies]
ring_types = { path = "../ring_types" }
ring_atomic = { path = "../ring_atomic" }
ring_batch = { path = "../ring_batch" }

[dev-dependencies]
```

**The three absences, and what each leaves unowned:**

| Absent crate | What this crate's instances say it would own | Left as |
|--------------|---------------------------------------------|---------|
| [`ring_config`](../../../ring_config/readme.md) | Region size and consolidation period | Unowned; both are correctness parameters with no configuration home |
| [`ring_registry`](../../../ring_registry/readme.md) | The global buffer set the consolidator iterates, and the orphan list for dead threads | Open — registry ownership is unresolved (→ [Registration State](../lifecycle/004_registration_state.md)) |
| [`ring_shutdown`](../../../ring_shutdown/readme.md) | The thread-exit signal this crate cannot detect | Unowned (→ [Thread Registration and Teardown](../lifecycle/001_thread_registration_and_teardown.md)'s N2) |

**`ring_flush` is deliberately not in that table, and the distinction it draws
is the useful one.** It is the one collaborator that *is* wired: it declares
this crate as a dependency and owns the consolidation trigger
(→ [Consolidation Cycle](../lifecycle/002_consolidation_cycle.md)'s K1). What
makes it invisible from here is direction, not absence — the edge points
inward only, so this crate cannot name its consolidator, cannot signal it, and
cannot tell whether one exists. A seam being *declared* and a seam being
*reachable from this side* are different properties, and only the first holds.

**Taken together the three absences have a pattern: everything this crate
cannot do alone is unowned.** Registration, configuration, and shutdown
detection are each named by an instance here as belonging elsewhere, and none
of the elsewheres is connected. The crate is a correct and complete
*mechanism* with most of its *orchestration* unattached. That is a defensible
skeleton state — the mechanism is what a future benchmark must measure
first — but it means the crate as it stands cannot be adopted without a consumer supplying
three things it may not know are missing, on top of driving the consolidation
cycle that `ring_flush` sequences but nothing yet schedules.

### Error Handling

Nothing crosses these seams as an error value. The failure classes mirror the
sibling crate's, with one addition specific to sitting on the export
boundary:

| Failure class | Example | How it surfaces |
|---------------|---------|-----------------|
| **Compile-time** | `ring_types` changes its tag representation | Build failure — loud |
| **Silent correctness** | `ring_types` grows tag *semantics* this crate starts relying on | Nothing fails; the agnosticism erodes gradually and is noticed only when a second consumer with a different vocabulary tries to adopt |
| **Silent design drift** | `ring_batch`'s contiguous-claim guarantee is tuned for a different caller's needs and quietly stops matching this crate's | Nothing fails; this crate's central throughput property depends on another crate's optimisation target, and the divergence has no test that spans both |
| **External breakage** | Any signature change here | **Reaches consumers directly** — no absorbing layer, unlike every internal family crate |

**The second row is the one worth guarding against actively**, because it is
how a deliberately-neutral crate stops being neutral. This crate's stated
contribution is "the append discipline alone, never the payload vocabulary
either consumer encodes into it" (`src/lib.rs`), and that boundary lives
entirely at S1. A `ring_types` that supplies vocabulary rather than
representation would let this crate silently acquire opinions it was factored
out to avoid.

### Compatibility Requirements

1. **This crate is externally visible and its signatures are a contract.**
   Unlike `ring_mpsc`, changes here are breaking changes for consumers with
   no intermediary. This is a requirement on how the open questions are
   closed, not merely an observation.
2. **S1 carries representation, never vocabulary.** The payload-agnosticism
   this crate is built on has exactly one place it can leak, and this is it.
3. **S2's fit is resolved before implementation.** A dependency that cannot
   be justified from the two crates' own descriptions should be either
   explained or dropped, not carried forward into code.
4. **The four absences are the consumer's to supply until wired.** A consumer
   adopting today must provide registration, triggering, configuration, and
   shutdown detection itself, and must be told so
   (→ [Prospective Consumer Adoption](002_prospective_consumer_adoption.md)).
5. **Version movement within the family is lockstep** — all 33 crates are
   path dependencies at one workspace version, with no mixed-version support
   intended.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_writer_append_surface.md](../api/001_writer_append_surface.md) | Its Compatibility Guarantee 2 is Requirement 1 stated from the surface side |
| [../api/002_consolidator_read_surface.md](../api/002_consolidator_read_surface.md) | Its three-primitive shape exists to preserve S4's seam |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_thread_local_append_log.md](../data_structure/001_thread_local_append_log.md) | The region S2's slot addressing would apply to, and the fit question that raises |

### Integrations

| File | Relationship |
|------|--------------|
| [002_prospective_consumer_adoption.md](002_prospective_consumer_adoption.md) | The seam above S3, where the four absences become the consumer's problem |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_thread_registration_and_teardown.md](../lifecycle/001_thread_registration_and_teardown.md) | Names `ring_registry` and `ring_shutdown` as owners of what its N1 and N2 need |
| [../lifecycle/002_consolidation_cycle.md](../lifecycle/002_consolidation_cycle.md) | Names `ring_flush` as owner of its K1 trigger |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_pod_pointer_free_payloads.md](../non_functional_requirement/002_pod_pointer_free_payloads.md) | The payload constraints S1 must express as representation rather than vocabulary |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/004_registration_state.md](../lifecycle/004_registration_state.md) | Owns the `ring_registry`-or-consumer question this instance records as an absence |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_record_tag.md](../type/001_record_tag.md) | Its V2 is the agnosticism S1 must not break |

### Sources

| File | Relationship |
|------|--------------|
| `Cargo.toml` | Declares S1 and S2; the four absences are read from it |
| `src/lib.rs` | States the agnosticism Requirement 2 protects |

### Tests

| File | Relationship |
|------|--------------|
| `tests/seam_test.rs` (to create) | This crate compiles and functions with a `ring_types` tag type it has no knowledge of — S1's representation-not-vocabulary boundary, asserted by using an opaque newtype the crate never matches on |

### TL24 — The Dependency Seam Described Here Is Two Crates Short

```sh
cd "$(git rev-parse --show-toplevel)"/ring_tls
printf '[dependencies]:\n'; sed -n '/^\[dependencies\]/,/^$/p' Cargo.toml | grep '^ring_'
printf '[dev-dependencies]:\n'; sed -n '/^\[dev-dependencies\]/,/^$/p' Cargo.toml | grep '^ring_'
printf 'and what src/lib.rs imports:\n'
grep -vE '^\\s*(//|///|//!)' src/lib.rs | grep '^use '
```

Live output:

```
[dependencies]:
ring_types = { path = "../ring_types" }
ring_atomic = { path = "../ring_atomic" }
ring_batch = { path = "../ring_batch" }
[dev-dependencies]:
ring_store = { path = "../ring_store" }
ring_event = { path = "../ring_event" }
ring_slot = { path = "../ring_slot" }
and what src/lib.rs imports:
use core::sync::atomic::Ordering;
use ring_atomic::SeqCell;
use ring_batch::{ claim, BatchClaim };
use ring_types::{ RingError, Seq };
```

`ring_atomic` arrives directly and is imported for `SeqCell`; `ring_batch`
supplies `claim` and `BatchClaim`. The three dev-dependencies exist for the
integration tests, which stage into a real `ring_store`.

### TL25 — The Four Conspicuous Absences Are Still Absent, for a Different Reason

The instance's argument is that a per-thread buffer touches no shared state
and therefore needs none of the four. That reasoning does not survive: the built
crate *does* touch shared state, in `flush_into`, through a `SeqCell` cursor.

It reaches it through `ring_batch::claim` instead — one crate, one call — which
is a stronger version of the same conclusion than the one written here.
