# Decisions

### Scope

- **Purpose**: Record the architecture decisions for `ring_bench` that passed the Decision Gate — genuine open trade-offs with real switching cost — rather than being fixed in place directly.
- **Responsibility**: Index this crate's Architecture Decision Records (ADRs) and the questions it raised for other crates without the standing to rule them.
- **In Scope**: Proposed, accepted, and superseded ADRs for this crate's design; open questions this crate's measurements created for its dependencies.
- **Out of Scope**: The five-crate Contract itself, which is decided at the family level rather than by this crate; `ring_types`' choice of `DropNewest` as the default policy, which is that crate's and already made; `ring_handle`'s omission of `try_clone`, which is a question for `ring_handle`.

Not a doc-definition collection — per `doc_des.rulebook.md`'s classification of
`docs/decisions/` as a non-doc-definition directory, ADRs here use the format at
`doc_des.rulebook.md § Architecture Documentation : Architecture Decision
Records` and are indexed only in this file, not in `definition/readme.md` or
`graph.yml`.

### Index

| ADR | Rules | Status |
|---|---|---|
| [001 — Five Candidates for Four Named Paths](001_five_candidates_for_four_named_paths.md) | Pending 2 | accepted 2026-08-28 |
| [002 — No Test Asserts an Ordering](002_no_test_asserts_an_ordering.md) | Pending 3 | accepted 2026-08-28 |

**Both ADRs here were forced by a measurement rather than by an argument**, and
that is the pattern worth recording. `ring_factory`'s decisions register closed
eight of ten pendings "mostly from below" — its dependencies shipping shapes
that answered questions it had written down. This crate has no dependents, so
nothing can arrive from below; every question it closed, it closed by running
something and reading a number that contradicted the plan.

**The asymmetry that follows: this crate cannot receive answers, only send
them.** Both open pendings below are questions for *other* crates, raised by
measurements only this one could take, and neither is this crate's to rule.

### Open

**Pending 1 — should the Contract compose `ring_factory` with `ring_flush`?**

The staged write path is one of the four named candidates, and a
consumer bound to the five-name export surface **cannot build it**:
`build` produces a `ring_handle::Split`, `Flusher::new` requires a
`ring_core::Producer`, and nothing on the Contract bridges them.
→ [`integration/001`](../integration/001_declared_edges_and_the_three_that_were_missing.md).

Three candidate closures, costed there. All three modify a crate this one does
not own, and two modify a Contract crate's public surface — which is a family
decision, not a consumer's. Filed rather than ruled.

**What makes this a pending and not a bug report:** the gap has a legitimate
reading. `ring_flush` may be intended for consumers who already hold a
`ring_core::Producer` — inside the family — and the Contract's job may be to
expose the *policy* vocabulary rather than the composition. Nothing written down
says which, and this crate's need does not settle it.

**Pending 2 — should `Ring::with_config` exist?**

`ring_factory`'s own decisions register raised this
([Pending 8](../../../ring_factory/docs/decisions/readme.md)) on the grounds
that "its *name* claims more than it does, and it is the one construction path
that reads as compliance while bypassing the factory." It recorded three
candidate answers and no evidence, because the crate that raised it could not
produce any — it reaches one door.

**This crate reaches both with one config, and the divergence is now
measured.** `ring_spsc::Ring::with_config` and `ring_mpsc::Ring::with_config`
read capacity and ignore `overflow`, so the identical `RingConfig` that
`ring_factory::build` rejects outright builds a working ring one level down:

```rust
let evicting = RingConfig::new( 64 ).unwrap().with_overflow( OverflowPolicy::DropOldest );
assert!( Factory.build::< u64 >( evicting ).is_err() );          // the door refuses
assert!( run( Candidate::DirectSpsc, &workload ).is_ok() );      // the backend does not
```

Asserted by `the_direct_doors_ignore_the_policy_the_contract_door_refuses`.
**The pending stays open and is no longer evidence-free** — which is the whole
value this crate could add to it. Ruling it is `ring_factory`'s and the two
backends', jointly.

### Closed

**Closed 1 — how many candidates?** Four write paths are named; five ship
(six with `crossbeam`). Ruled by
[ADR 001](001_five_candidates_for_four_named_paths.md), forced by the producer
ceiling measurement.

**Closed 2 — does the suite assert a ranking?** No, never, not once. Ruled by
[ADR 002](002_no_test_asserts_an_ordering.md).

**Closed 3 — is `conserved` a correctness assertion?** It was written as one and
demoted to a report during implementation: under the default policy it is
legitimately `false`, so asserting it would have made the family's own
documented idiom untestable. Fixed in place rather than filed — the correction
took one afternoon and had no switching cost once the distinction between "a
property the run must have" and "a property of the configuration the run is
reporting on" was written down.
→ [`pitfall/003`](../pitfall/003_ok_is_not_kept_and_the_verdict_inverts.md).

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench/docs/decisions
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### BN[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| BN[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BN13 | `decisions/002`'s licensed ordering verdict | **wrong doc** | "The lock-free paths beat the mutex baseline by 3–5x with no overlap in any round" is the single ordering claim this decision says is safe to state, and its own evidence table disagrees |
| BN14 | `decisions/002`'s B3 variance rounds | **misleading doc** | Every recorded round names `off_the_shelf`, which exists only under `--features crossbeam`, so the variance evidence was taken in a build the suite does not run |
| BN15 | `decisions/001`'s *What forced it* block | n/a — unenforced | This decision's entire evidentiary base is fenced ```bash and the corpus recipe checker reads only ```sh, so the premise is re-run by nothing |
| BN16 | `decisions/001`'s grounds for `DirectSpsc` | **wrong doc** | `DirectSpsc` exists on the stated grounds that it and `ContractRing` differ only in the layers between the caller and the same data structure, and this crate's own algorithm document records that they do not |
