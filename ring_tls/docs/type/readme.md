# Type Doc Definition

### Scope

- **Purpose**: Define the two small values the append discipline is written in — the per-record tag and the consolidation epoch — so that "tagged record" and "one consolidation" name fixed things rather than being re-derived per instance.
- **Responsibility**: Give each a definition, validation rules, and the size trade it carries; keep the payload vocabulary out, since that is deliberately not this crate's.
- **In Scope**: Record tag and epoch as value types.
- **Out of Scope**: The payload a tag discriminates, which belongs to the consumer (→ [`../readme.md`](../readme.md)); the region layout that stores them, undecided; the ordering primitives the seal handshake uses, which are [`ring_atomic`](../../../ring_atomic/readme.md)'s.

### Overview Table

| ID | Name | Purpose | domain | ddd | Status |
|----|------|---------|--------|-----|--------|
| 001 | [Record Tag](001_record_tag.md) | The per-record discriminator that makes a byte region self-describing — and the per-record overhead that buys it | staging | value object | 🔄 |
| 002 | [Epoch](002_epoch.md) | The consolidation-cycle counter, and the only mechanism that could recover a consistent global snapshot across per-thread buffers | staging | value object | 🔄 |




### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_tls/docs/type
printf 'instances:                '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### TL[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| TL[0-9]+ ' readme.md
# instances:                2
# finding headings inside:  2
# rows in the table below:  2
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TL48 | the record tag | **wrong doc** | A one-byte discriminant preceding each record is defined here and appears nowhere in the family's code. |
| TL49 | the epoch | **wrong doc** | The epoch that dates a consolidation snapshot has no declaration, and there is no consolidation to date. |
