# Type Doc Definition

### Scope

- **Purpose**: Document the two values this crate exists to define, whose trait implementations — especially the absent ones — are as much a part of their definition as their fields.
- **Responsibility**: Define each value, its representation, its auto-trait status, and the validation rules governing it.
- **In Scope**: `Producer< T >` and `Consumer< T >`.
- **Out of Scope**: Their methods (→ [`api/`](../api/readme.md)); the backend's own layout, which is `ring_core`'s.

### Overview Table

| ID | Name | Purpose | domain | ddd | Status |
|----|------|---------|--------|-----|--------|
| 001 | [Producer](001_producer.md) | The exclusive right to publish, with three of its seven validation rules currently undetected | ring | value object | 🔄 |
| 002 | [Consumer](002_consumer.md) | The exclusive right to drain — and therefore the consume point itself, whose placement no rule here can constrain | ring | value object | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_handle/docs/type
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### HD[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| HD[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| HD41 | the Definition table | n/a — inconsistency | The Definition table gives the handle's size as one pointer and the Validation table gives 24 bytes for the producer and 16 for the consumer; the suite asserts an equality with the wrapped type and pins no absolute figure at all |
| HD42 | the Exported row | **wrong doc** | The row places `Producer` on the family Contract, which is a list of five crate names rather than type names — the coincidence that this crate exports exactly five types is what makes the error read as plausible |
| HD43 | the Enforced by column | n/a — drift | Four rows in the crate declare a rule undetectable and two are contradicted by a Tests row in the same file; the two that are still accurate are the two nobody wrote a test for |
| HD44 | the `Debug` derive | **latent hazard** | Both stated `Debug` constraints are decided four crates down by a hand-written impl in `ring_spsc` that gives a different reason, and the local test would pass a `Debug` that printed every pending record |
