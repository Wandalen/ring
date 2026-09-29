# Data Structure Doc Definition

### Scope

- **Purpose**: Turn this crate's storage — "a cache-aligned ring buffer" — into a field-level structure, so that what is decided and what remains open can be told apart.
- **Responsibility**: Document `ring_mpsc`'s own structure, its field grain, the operation set the algorithm instances execute against it, and the hardware contracts its layout carries.
- **In Scope**: Slot array, per-slot sequence stamp, the two cursors, and the false-sharing separation between them.
- **Out of Scope**: The procedures operating on these fields (→ [`algorithm/`](../algorithm/readme.md)); the orderings each access carries (→ [Publication Ordering](../invariant/002_publication_ordering.md)); what a slot's payload bytes mean, which belongs to whichever consumer wrote them.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Sequence-Stamped Ring](001_sequence_stamped_ring.md) | Fixed-capacity slots plus a per-slot sequence stamp that distinguishes empty from stale from published without a separate flag, and two cursors on separate cache lines | 🔄 |


### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_mpsc/docs/data_structure
printf 'instances:                '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### MP[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| MP[0-9]+ ' readme.md
# instances:                2
# finding headings inside:  4
# rows in the table below:  4
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| MP9 | `slots` | n/a — observation | Wrapping the whole `Buffer` materialised `&mut Buffer< S >` per write, aliasing the entire allocation; Miri caught it as a retag conflict. |
| MP10 | `ring_store` | n/a — observation | `Buffer< UnsafeCell< S > >` was unconstructible until `Buffer::new` was bounded on `Default` rather than `Slot`. |
| MP11 | `padding` | **measured cost** | The two cursors get `PaddedCursor`; the per-slot stamps are deliberately unpadded, on an argument about which contention is structural. |
| MP12 | `UNSTAMPED` | n/a — observation | The sentinel is a legal `Seq` value, made safe by a wrap bound rather than by the type. |
