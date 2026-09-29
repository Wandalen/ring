# Algorithm Doc Definition

### Scope

- **Purpose**: Document the two procedures this crate performs — the one that creates every guarantee it has, and the one that must add nothing to the hot path.
- **Responsibility**: Name each step, what it costs, and which steps are load-bearing rather than bookkeeping.
- **In Scope**: The split; the per-operation delegation path.
- **Out of Scope**: The ring operations themselves, which are `ring_core`'s and its backends'; ring construction, which is `ring_factory`'s.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Splitting a Ring Into Two Ends](001_splitting_a_ring_into_two_ends.md) | Six steps, three of them load-bearing, and one that is an absence listed as a step | 🔄 |
| 002 | [Delegating an Operation to the Backend](002_delegating_to_the_backend.md) | A three-step procedure with no fourth step, and five tempting additions that each cost the family | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_handle/docs/algorithm
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### HD[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| HD[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| HD1 | the split procedure | n/a — inconsistency | The instance documents one procedure in six steps and the surface implements it as three separate calls, so no step boundary corresponds to a call boundary a caller can observe |
| HD2 | step 2 | n/a — observation | The allocation the procedure's second step describes happens in `ring_core`, which this crate depends on but whose allocating call it cannot name — the step describes work no line of this crate performs |
| HD3 | step 6 | n/a — unenforced | The delegation procedure's final step is that nothing is added on the way back, which is an absence: no assertion, no test and no gate checks it, and the only thing standing there is review |
| HD4 | the two procedures | n/a — observation | Neither documented procedure has a failure path, and `ring_config` accepts a configuration that would give one — the procedures are total because the constructor already rejected everything that would make them partial |
