# Lifecycle Doc Definition

### Scope

- **Purpose**: Trace the four values that move — two across crate boundaries on their way somewhere (a slot count toward a mask, an error toward a message), and two through the states the design permits them to occupy (a capacity through validation, a policy through refusal).
- **Responsibility**: For each, state the phases or states, the transitions between them, the crates each phase depends on, and the cleanup required.
- **In Scope**: Six phases for a capacity across four crates; five for an error across six; four reachable capacity states and one unreachable fifth; six policy states across five crates.
- **Out of Scope**: The two validation tests read as an algorithm (→ [`../algorithm/`](../algorithm/)); the internal structure of each type (→ [`../type/`](../type/)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [A Slot Count From Request to Mask](001_a_slot_count_from_request_to_mask.md) | Six phases, two fallible transitions, and fifteen crates that hold the result without ever re-checking it | 🔄 |
| 002 | [An Error From Construction to Display](002_an_error_from_construction_to_display.md) | Five phases, and the finding that two of the nine variants have no first phase at all | 🔄 |
| 003 | [A Capacity Request Through Validation](003_a_capacity_request_through_validation.md) | Four reachable states and one unreachable fifth — and it is the fifth's emptiness, not the validation, that fifteen crates rely on | 🔄 |
| 004 | [An Overflow Policy From Declaration to Refusal](004_an_overflow_policy_from_declaration_to_refusal.md) | Six states across five crates, one variant that is configurable, refused and supported in that order, and a handler nothing calls | 🔄 |

**Both arcs end in "no cleanup required", and the two reasons are different.**
001's is that a `Capacity` is a `usize` behind a newtype. 002's is that
`RingError` is `Copy`, which forbids `Drop` outright — the same constraint that
forbids `NameTaken` from carrying a name, which is why the family has two error
types. **The cleanup requirement being "none" and the error surface being split
are the same fact**, and 002 is where that is spelled out.

**Two questions live here, and the second is the one that carries weight.** 001
and 002 ask who *owns* each phase, which is how they can record that seven of
nine error variants are constructed somewhere else and two are constructed
nowhere. 003 and 004 ask which states are *reachable* — a different question
with a different answer. 003's entire subject is a state nothing ever enters,
and the emptiness of that state is what fifteen crates are built on.

**The two reachability arcs are opposites in ownership.** 003's every state
belongs to this crate, so it can be verified by reading one file. 004 has
exactly one state here — the declaration — and the other five belong to
`ring_config`, `ring_core`, `ring_overflow` and `ring_stats`, so its invariants
are claims about crates this one must not depend on.

**Modelling 004 from here is what made its two divergences visible.**
`ring_core` took `ring_overflow`'s pure `would_resolve` and left the recording
`resolve` behind, so no drop is counted where it happens; `ring_bench` populates
the counters afterwards from the offered-minus-received difference instead.
Neither crate is wrong on its own terms, and neither crate's documentation is
the place the mismatch shows up.


### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_types/docs/lifecycle
printf 'instances:               '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### TY[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| TY[0-9]+ ' readme.md
# instances:               4
# finding headings inside:  5
# rows in the table below:  5
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TY39 | ring family | n/a — coverage | Only the request and validation phases run here; the mask, index, store and read phases run in crates holding a `Capacity` they cannot have constructed wrongly |
| TY40 | ring family | n/a — observation | Only the two capacity variants are constructed inside `ring_types`; five are constructed by dependents and two by nothing at all |
| TY41 | `ring_types` | n/a — observation | No test asserts the fifth state is unreachable and none can; its emptiness follows structurally from `Capacity` having no public constructor other than `new` |
| TY42 | `ring_core` | n/a — coverage | `ring_core:154` is the only place a policy is refused; `ring_factory` re-wraps that error and constructs none of its own |
| TY43 | ring family | n/a — observation | `ring_wait` matches all four `WaitKind` variants and `ring_overflow` all three `OverflowPolicy` variants with no wildcard, so a new variant is a compile error where it matters |
