# Data Structure Doc Definition

### Scope

- **Purpose**: Describe the two values this crate defines — the watcher and the report — field by field, and record what each field's presence or absence commits the crate to.
- **Responsibility**: Layout, derives, and the consequences of both for a caller holding one of these values.
- **In Scope**: `Watch`'s three fields and two derives; `Violation`'s four variants and eleven fields.
- **Out of Scope**: The traits `Violation` participates in (→ [`type/`](../type/readme.md)); the order in which fields are written (→ [`pattern/`](../pattern/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [A Watch Is Three Scalars and No Identity](001_a_watch_is_three_scalars_and_no_identity.md) | What `Watch` stores, and the two things it deliberately does not | 🔄 |
| 002 | [Four Variants and the Newtype They Drop](002_four_variants_and_the_newtype_they_drop.md) | What each report carries, and the one type-level concession at the boundary | 🔄 |

**Both instances turn on the same question asked twice: what is *not* in the
struct?** `Watch` holds no identity for the pair it watches, and `Violation`
holds no `Capacity` for the ring it describes. In each case the omission is
cheap, defensible, and invisible at the call site — which is what makes it worth
a document rather than a comment.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_debug/docs/data_structure
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### DB[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| DB[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| DB5 | `Watch::observe` | **latent hazard** | A stateful watch keeps no identity for the pair it watches, so observing a different ring produces a confident verdict about neither — declined, and pinned by a test. |
| DB6 | `Watch` | **latent hazard** | `Copy` was derived on a type whose value is being the single record of what was last seen; `Clone` alone now makes the forked baseline visible at the call site. |
| DB7 | `Violation` payloads | n/a — observation | A validated capacity newtype is unwrapped to `usize` at the error boundary, correctly in one variant and incidentally in the other, with the two cases never distinguished. |
| DB8 | `Violation` | n/a — observation | Every payload is a detection-time copy rather than a reference, which is what stops a held violation from reporting the ring as healthy when it is finally read. |
