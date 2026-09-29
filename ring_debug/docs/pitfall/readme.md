# Pitfall Doc Definition

### Scope

- **Purpose**: Record the traps around this crate's subject — the arithmetic that hides a corruption, and the check that is the only one a family caller can reach.
- **Responsibility**: What each trap is, what it presents as, and which of them are aimed at a reader of the document rather than at the family.
- **In Scope**: The saturating readings under both cursor violations; the clamps that produce them; `check_ends`'s capacity argument and the one variant it reports through.
- **Out of Scope**: The checks that catch these (→ [`invariant/`](../invariant/readme.md), [`api/`](../api/readme.md)); the memory ordering (→ [`algorithm/`](../algorithm/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Saturating Arithmetic Reports Health](001_saturating_arithmetic_reports_health.md) | The measurement that gives the crate its reason to exist, and why the fix it suggests is the wrong one | 🔄 |
| 002 | [The Number the Borrow Checker Hides](002_the_number_the_borrow_checker_hides.md) | The capacity `check_ends` cannot derive, and the one report two unrelated causes share | 🔄 |

**The split is the trap in the thing being checked against the trap in the
checker.** `001` is about `ring_seqno` and `ring_types`: a corrupted ring reports the
readings of a healthy one, which is why an opt-in checker exists at all. `002` is
about this crate: the only entry point that checker exposes to a real
`ring_core::Ring` takes a number the caller has to supply by hand, and reports a
wrong one in the same words it reports a broken ring.

They are kept apart because each ends in a *"and the obvious fix is worse"*
paragraph aimed at a different reader — P4 at someone who would harden the family's
arithmetic, P8 at someone who would harden this crate's signature. Read together
the second reads as a caveat on the first; it is a separate trade with a separate
answer.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_debug/docs/pitfall
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### DB[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| DB[0-9]+ ' readme.md )"
printf 'failure IDs across both:  %s\n' "$( command grep -hoE '^\| P[0-9]+ ' [0-9][0-9][0-9]_*.md | tr -d '| ' | sort -V | tr '\n' ' ' )"
```

Live output:

```
instances:                2
finding headings inside:  4
rows in the table below:  4
failure IDs across both:  P1 P2 P3 P4 P4 P5 P6 P7 P8 
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| DB49 | the cost P4 defends against | **misleading doc** | The branch a defensive `distance_to` would add is rejected as a cost on "the gating read that every claim performs", and neither of the family's two in-house backends takes a `ring_seqno` edge at all — each computes its own headroom in a crate that never reaches the line being defended. |
| DB50 | the asymmetry's second clamp | **misleading doc** | The masking is attributed to one `saturating_sub` in another crate; a second one sits in `ring_seqno::free_slots` itself and is the clamp that fires on the other corruption, making two of that corruption's three readings identical to a legitimately full ring. |
| DB51 | `check_ends`'s capacity argument | **latent hazard** | The only check reachable from a live ring compares against a capacity the caller cannot derive from either value it passes — the object holding it is mutably borrowed for as long as those values exist — so it has to be captured before the split, which the rustdoc now shows and three of the suite's four call sites now do. |
| DB52 | `ReadingsDisagree`'s two causes | n/a — coverage | One construction site reports both a genuinely inconsistent ring and a mistyped third argument in the same sentence about the ring, and the only cause the suite ever produces is the second one. |
