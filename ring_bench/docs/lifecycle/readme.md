# Lifecycle Doc Definition

### Scope

- **Purpose**: Record the phases a comparison passes through from a description to a verdict, and the states one candidate occupies inside them — marking which belong to this crate and which to a dependency.
- **Responsibility**: State the phases and the states, their ordering constraints and guards, and where each can fail.
- **In Scope**: Description, admission, construction, publication, drain, accounting, verdict; one candidate, one workload, from admission to outcome or refusal.
- **Out of Scope**: The candidates' internal lifecycles; the internal state of a ring, which is `ring_core`'s.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [From a Description to a Verdict](001_from_a_description_to_a_verdict.md) | Seven phases across four crates, of which exactly one is timed and exactly one is this crate's judgement | 🔄 |
| 002 | [One Candidate Through One Run](002_one_candidate_through_one_run.md) | Six states, two terminal, and the guard that decides which of the two a run ends in | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench/docs/lifecycle
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### BN[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| BN[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BN29 | `lifecycle/001`'s Phase 3 | **misleading doc** | Construction carries a single ✅ with two error variants beside it, which reads as a property every candidate passes through, and half the runners cannot refuse at all — their signatures say so |
| BN30 | `Comparison::run`'s candidate loop | n/a — observation | Phase 7 is described as comparative, and what feeds it is a plain sequential loop that measures every candidate exactly once, in declaration order, with no warm-up |
| BN31 | `lifecycle/002`'s States table and diagram | n/a — drift | The table and the transition diagram describe a seven-state, five-edge machine, and six of the seven state names appear nowhere in the crate |
| BN32 | `lifecycle/002`'s `Building → Unbuildable` edge | n/a — inconsistency | Four lines apart, this document gives the same edge two different refusal-variant counts |
