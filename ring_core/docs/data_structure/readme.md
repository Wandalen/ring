# Data Structure Doc Definition

### Scope

- **Purpose**: Describe the state this crate owns — four parallel three-way enums whose arity is a compile-time property rather than a fixed number.
- **Responsibility**: Variant contents, where the uniform parts live versus the per-backend parts, the two structural asymmetries between the in-house arms and the crossbeam arm, and the ownership consequences of each.
- **In Scope**: `Ring`/`Storage`, `Ends`/`EndsInner`, `Producer`/`ProducerInner`, `Consumer`/`ConsumerInner`.
- **Out of Scope**: Each backend's own memory layout (→ its crate); the procedures dispatched over these enums (→ [`algorithm/`](../algorithm/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [The Three-Way Storage Enum](001_three_way_storage_enum.md) | Two builds rather than one type; slots versus values; and an exclusive split at two backends against a shared borrow at the third | 🔄 |



### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_core/docs/data_structure
printf 'instances:                '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### CO[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| CO[0-9]+ ' readme.md
# instances:                2
# finding headings inside:  4
# rows in the table below:  4
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CO9 | `Storage` | **measured cost** | The optional crossbeam backend costs sixteen `cfg` gates in one 653-line file. |
| CO10 | the five enums | n/a — observation | `Storage` is the only backend enum that owns its payload, which is why it is the only one without a lifetime parameter. |
| CO11 | the five enums | **measured cost** | Exhaustive matching converts a new backend into five compile errors rather than one silent fallthrough. |
| CO12 | the five enums | n/a — unenforced | The four private enums are kept variant-for-variant identical by convention, and no test or type asserts it. |
