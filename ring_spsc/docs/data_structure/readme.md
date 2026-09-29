# Data Structure Doc Definition

### Scope

- **Purpose**: Document the ring's field-level shape — and, as informatively, the per-slot publication state it does not carry.
- **Responsibility**: Name the fields, their single writers, their cache-line placement, and the operations over them with costs.
- **In Scope**: The slot storage; the two padded cursors; the derived quantities.
- **Out of Scope**: The slot's internal byte layout, which is `ring_slot`'s; the procedures over the structure (→ [`algorithm/`](../algorithm/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Two-Cursor Ring Without Per-Slot State](001_two_cursor_ring.md) | Three fields where the multi-producer sibling needs four, and why cursor separation is a contract rather than an optimization | 🔄 |


### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_spsc/docs/data_structure
printf 'instances:                '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### SP[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| SP[0-9]+ ' readme.md
# instances:                2
# finding headings inside:  4
# rows in the table below:  4
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SP9 | `CursorPair` | n/a — observation | A single imported type owns both cursors and their cache-line separation, so this crate declares no padding of its own. |
| SP10 | `slots` | n/a — duplication | Both crates wrap `Buffer< UnsafeCell< S > >` per slot, for the same reason and with the same Miri history behind it. |
| SP11 | the absent stamp | n/a — observation | Two of the three conditions that make a stamp necessary hold here; only out-of-order publication fails, and it fails because of producer count. |
| SP12 | the absent stamp | n/a — observation | `ring_mpsc`'s stale-stamp pitfall has no analogue here, so this crate's `pitfall/002` documents an entirely different failure. |
