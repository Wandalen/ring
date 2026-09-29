# Integration: Prospective Consumer Adoption

### Scope

- **Purpose**: State what adopting this crate would require of a consumer, including the four orchestration capabilities the crate does not supply, so the cost is known before the benchmark verdict rather than after.
- **Responsibility**: State what adoption replaces, the obligations it transfers, and why the crate's continued existence is conditional on a consumer actually adopting it.
- **In Scope**: The adoption seam above the export boundary; the consumer-side obligations; the measured baseline.
- **Out of Scope**: Whether adoption happens, which is a future benchmark verdict; the family-internal seams (→ [Family Dependency Seam](001_family_dependency_seam.md)); the composition adoption implies (→ [Staging Then Merge](../pattern/002_staging_then_merge.md)).

### System Description

This crate is not a dependency of anything today. It exists because two
consumers were found to need the same per-thread append discipline, so the
mechanism was factored out once rather than built twice.

**The crate's own justification is weaker without an adopter**, and that
should be recorded rather than glossed. `ring_tls` was factored out to serve
prospective consumers, and neither has adopted it yet. The sibling crate
records the same situation
(→ [`ring_mpsc`](../../../ring_mpsc/docs/integration/002_prospective_consumer_adoption.md)),
and the honest conclusion is the same: a shared crate with no adopters is
duplication with extra steps, and removal is the correct response rather than
indefinite skeleton maintenance.

**The benchmark still needs a baseline.**
[Measured Before Adopted](../non_functional_requirement/001_measured_before_adopted.md)
measures this crate's implementation against alternative write-path designs;
a benchmark with no baseline establishes only that the code runs.

### Integration Points

| # | Seam | What crosses it | What it requires of the consumer |
|---|------|-----------------|----------------------------------|
| W1 | Append | The consumer's records into per-thread regions | That its record type satisfies the POD and alignment preconditions (→ [POD and Pointer-Free Payloads](../non_functional_requirement/002_pod_pointer_free_payloads.md)) |
| W2 | Tag vocabulary | The consumer's own record kinds, as opaque tags | That the consumer keeps its vocabulary — this crate supplies none (→ [Record Tag](../type/001_record_tag.md)) |
| W3 | Registration | Thread setup ordering | **That the consumer registers each thread before its first append** (→ [Register Before First Append](../pattern/001_register_before_first_append.md)) |
| W4 | Consolidation trigger | The decision of when to consolidate | That the consumer owns the period, and understands it as a correctness parameter (→ [Consolidation Cycle](../lifecycle/002_consolidation_cycle.md)) |
| W5 | Thread teardown | Notice that an appending thread is exiting | That the consumer joins threads before final consolidation, or accepts the loss window (→ [Thread Registration and Teardown](../lifecycle/001_thread_registration_and_teardown.md)) |
| W6 | Merge | Staged records into an ordered structure | That the consumer supplies the merge — this crate provides no cross-thread ordering |

**W3 through W6 are what
[Family Dependency Seam](001_family_dependency_seam.md) leaves unowned,
restated as consumer obligations.** That restatement is the point of this
instance: from the family's side these are gaps in the crate graph; from the
consumer's side they are four things it must do that a bare ring would not
require. Adopting this crate is not swapping one data structure for another —
it is taking on registration ordering, a consolidation schedule,
thread-lifetime management, and a merge stage.

**W4 is the one whose family-side story has since changed, and the consumer
obligation survives the change intact.** `ring_flush` is wired — it depends on
this crate and owns the seal/drain/reset sequencing. What it does not own is
*when* the cycle runs: the policy still has to be chosen, and on the barrier
variant something outside must announce the barrier. So the crate graph gained
an owner for the mechanism while the consumer kept the obligation for the
schedule. A consumer reading only the dependency graph would conclude W4 is
handled; it is not.

**W1's payload constraint is likely to be the binding one for a prospective
consumer whose own records are not already POD.** POD and pointer-free is a
real restriction, and whether a given consumer's records satisfy it is not
established by anything read here. If
they do not, adoption is blocked on either changing the record
representation or relaxing the constraint — and relaxing it would mean
records with destructors in a region that is reset rather than dropped
(→ [Buffer Epoch Cycle](../lifecycle/003_buffer_epoch_cycle.md)'s
Behavioral Invariant 4), which is a leak by construction.

**W2 is the seam that stays clean and is worth naming as a success rather
than a cost.** The consumer keeps its own `OpCode`-style vocabulary
unchanged; this crate never learns it. That is what made the crate shareable
between two consumers with different vocabularies in the first place, and it
survives the loss of one of them.

### Error Handling

Adoption failure modes are decisions, not runtime errors:

| Failure | Meaning | Consequence |
|---------|---------|-------------|
| The verdict favours another pattern | Staging does not beat the alternatives when measured | The crate stays unadopted; [Measured Before Adopted](../non_functional_requirement/001_measured_before_adopted.md) exists to permit this |
| W1's constraint excludes the consumer's record type | Records cannot be staged as POD | Adoption blocked on the record representation, not on this crate |
| The consumer cannot meet W3 or W5 | Thread lifetimes are outside its control — a third-party pool | Silent data loss if adopted anyway. **The failure that must not be discovered in production** |
| The consumer's latency budget is below the cycle period | Batching is unacceptable | A bare ring is the correct answer (→ [Staging Then Merge](../pattern/002_staging_then_merge.md)'s applicability table) |
| No consumer ever adopts | Both prospective consumers picked something else | The crate is deleted |

**The third row is the one adoption review must actually check**, because it
is the only one that fails silently after adoption rather than blocking it
before. A consumer that cannot control its threads' lifetimes will adopt
successfully, run correctly in testing, and lose data in production
proportional to thread churn.

### Compatibility Requirements

1. **Consumers depend on this crate directly.** Unlike `ring_mpsc`, there is
   no absorbing indirection — `ring_tls` is on the family's export list, so
   this seam is the real boundary and its signatures are the real contract.
2. **The four obligations W3–W6 are stated at adoption, not discovered.**
   A consumer told only "per-thread staging, no atomics" has been given the
   benefit without the cost.
3. **The measured baseline must be real.** Adoption is gated on a benchmark
   verdict, not on inspection;
   [Measured Before Adopted](../non_functional_requirement/001_measured_before_adopted.md)
   exists so that gate has a concrete comparison to run.
4. **Adoption is per-consumer.** No coordinated migration, no version
   negotiation — path dependencies at one workspace version.
5. **The crate's continued existence is conditional on adoption.** With no
   consumer having adopted yet, "nobody adopts" is a live outcome with a
   defined response, not an indefinite hold.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_writer_append_surface.md](../api/001_writer_append_surface.md) | The surface W1 and W2 cross; its Compatibility Guarantee 2 is Requirement 1 |
| [../api/002_consolidator_read_surface.md](../api/002_consolidator_read_surface.md) | The surface W4 and W6 cross |

### Integrations

| File | Relationship |
|------|--------------|
| [001_family_dependency_seam.md](001_family_dependency_seam.md) | The four absences this instance restates as W3–W6 |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_single_writer_append.md](../invariant/001_single_writer_append.md) | What a consumer's threading model must not violate |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_thread_registration_and_teardown.md](../lifecycle/001_thread_registration_and_teardown.md) | W3 and W5's obligations, and the loss window the third error row describes |
| [../lifecycle/002_consolidation_cycle.md](../lifecycle/002_consolidation_cycle.md) | W4's obligation, and why the period is a correctness parameter |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_measured_before_adopted.md](../non_functional_requirement/001_measured_before_adopted.md) | The gate this seam waits behind, and the instance permitting the not-adopted outcome |
| [../non_functional_requirement/002_pod_pointer_free_payloads.md](../non_functional_requirement/002_pod_pointer_free_payloads.md) | W1's constraint |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_register_before_first_append.md](../pattern/001_register_before_first_append.md) | W3's rule |
| [../pattern/002_staging_then_merge.md](../pattern/002_staging_then_merge.md) | W6's composition, and the applicability table the fourth error row cites |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_record_tag.md](../type/001_record_tag.md) | W2's clean seam — the consumer's vocabulary, opaque to this crate |

### Sources

| File | Relationship |
|------|--------------|
| `src/lib.rs` | Names both prospective consumers and the agnosticism W2 preserves |

### Tests

| File | Relationship |
|------|--------------|
| `tests/adoption_test.rs` (to create) | A consumer-shaped record type with a non-trivial destructor is rejected at compile time rather than staged and leaked — W1's constraint, expressed as a bound rather than as prose |

### TL26 — The Prospective Consumers Arrived and Are Not the Ones Named

```sh
cd "$(git rev-parse --show-toplevel)"
for c in $( ls -d ring_*/ | tr -d / ); do
  [ "$c" = ring_tls ] && continue
  dep=$( grep -vE '^\\s*#' $c/Cargo.toml | grep -c '^ring_tls = ' )
  code=$( find $c -name '*.rs' -exec cat {} + 2>/dev/null \
          | grep -vE '^\\s*(//|///|//!)' | grep -c 'ring_tls' )
  [ "$dep$code" = 00 ] && continue
  printf '%-14s declares=%-3s uses=%s\n' "$c" "$dep" "$code"
done
```

Live output:

```
ring_atomic    declares=0   uses=3
ring_bench     declares=1   uses=2
ring_factory   declares=0   uses=1
ring_flush     declares=1   uses=12
ring_testkit   declares=1   uses=1
```

Three consumers, all in-repo, all declaring the dependency they use — none of
which were anticipated when this crate was factored out, and the byte region
they want is still unwritten (→ [`../data_structure/002`](../data_structure/002_a_vec_and_a_limit.md)).

### TL27 — No Undeclared Consumer and No Prose-Only Consumer

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'ring_factory: grep -c ring_tls Cargo.toml  = %s\n' \
  "$( grep -c ring_tls ring_factory/Cargo.toml )"
printf 'ring_factory: the line that matched        =%s\n' \
  "$( grep ring_tls ring_factory/Cargo.toml | cut -c1-58 )"
printf 'ring_factory: comment-filtered dependency  = %s\n' \
  "$( grep -vE '^\\s*#' ring_factory/Cargo.toml | grep -c '^ring_tls = ' )"
```

Live output:

```
ring_factory: grep -c ring_tls Cargo.toml  = 1
ring_factory: the line that matched        =# Message 960 also assigned `ring_stats` and `ring_tls`; b
ring_factory: comment-filtered dependency  = 0
```

A count of `ring_tls` in `ring_factory/Cargo.toml` returns 1 and means zero. The
Rust comment filter used everywhere else in this corpus, `grep -vE '^\s*(//|///|//!)'`,
does nothing for TOML — the manifest comment marker is `#`, and a reach scan
that forgets it reports a dependency that was explicitly removed.
