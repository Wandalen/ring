# Pattern Doc Definition

### Scope

- **Purpose**: Document recurring structural solutions applicable to this crate's ring, so a future manifest or codegen step recognises the shape rather than re-deriving it.
- **Responsibility**: Named patterns bridging a declarative channel description to this crate's own ring instance.
- **In Scope**: How a declared channel maps to a ring instance, and how a ring's compile-time slot type is bound from data.
- **Out of Scope**: The ring mechanism itself (→ [`algorithm/`](../algorithm/readme.md), [`data_structure/`](../data_structure/readme.md)); which concrete ring pattern ships, which stays this family's own open verdict.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Channel-to-Ring Binding](001_channel_to_ring_binding.md) | A declared channel is one ring regardless of reader count, and a ring's compile-time slot type is either shared across channels or generated per channel via build-time codegen | 🔄 |


### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_mpsc/docs/pattern
printf 'instances:                '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### MP[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| MP[0-9]+ ' readme.md
# instances:                2
# finding headings inside:  3
# rows in the table below:  3
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| MP41 | channel-to-ring binding | n/a — unadopted | The pattern describes how a channel-shaped API binds onto this ring; exactly one crate does it. |
| MP42 | the stamp pattern | n/a — observation | The sibling crate wraps the same way and reuses slots the same way, and uses cursor comparison instead — which is the pattern's applicability condition made concrete. |
| MP43 | the stamp pattern | n/a — observation | Nothing in the test reads a cursor, which is what lets the drain scan stop at the first gap without coordination. |
