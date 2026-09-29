# Invariant Doc Definition

### Scope

- **Purpose**: Document the two restrictions this crate's surface carries, both enforced by the same mechanism — absence — along two independent axes.
- **Responsibility**: State each invariant, where enforcement lives, and what each violation actually costs.
- **In Scope**: The capability split; the non-parking restriction.
- **Out of Scope**: The wait strategies themselves, which are `ring_wait`'s; the poll loop, which is `ring_poll`'s.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Capability Follows the Handle](001_capability_follows_the_handle.md) | Five violations, of which the two that actually get made compile and pass every test | 🔄 |
| 002 | [Nothing Reachable From a Handle Can Park](002_no_parking_operation_is_reachable.md) | A constraint this crate must satisfy and does not claim — so nothing in its own test run goes red when it breaks | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_handle/docs/invariant
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### HD[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| HD[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| HD49 | the Enforcement table | **misleading doc** | The Enforcement table records compile-time detection for the property V4 says nothing detects — the Detected when column means "when a violation is caught" in two rows and "when the property is currently true" in a third |
| HD50 | `try_clone` | n/a — doc gap | The family answers the cardinality question twice — at runtime by `ring_core::Producer::try_clone`, which grants a second producer on an MPSC ring, and by absence here, which refuses for every backend — and only the absence is written down |
| HD51 | `PARKING_CRATES` | **latent hazard** | The graph guard reads every sibling manifest and classifies on the single substring `ring_wait`, so an edge to `ring_shutdown` or `ring_barrier` — both on its own roster — leaves the measured set unchanged and the suite green |
| HD52 | the invariant statement | **wrong doc** | "Every method reachable from a handle returns a `Result` or an `Option`" is true of two of the twelve methods, and the file's own Tests row names the counterexamples and explains why the shape was the wrong thing to ask for |
