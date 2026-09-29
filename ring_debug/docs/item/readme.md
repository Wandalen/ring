# Item Doc Definition

### Scope

- **Purpose**: Catalogue the public surface as a set — and the absences that no single declaration can carry — so that cross-declaration properties become checkable.
- **Responsibility**: Enumerate every declaration and every load-bearing omission, with the command that regenerates each enumeration.
- **In Scope**: The eight public declarations and two private ones; the four attributes; the six things the crate never declares.
- **Out of Scope**: What each signature guarantees (→ [`api/`](../api/readme.md)); the layout of the values themselves (→ [`data_structure/`](../data_structure/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Three Nouns, Five Verbs, and the Enum Two Doors Cannot Reach](001_three_nouns_five_verbs_and_the_enum_two_doors_cannot_reach.md) | The declarations that exist, and the reachability relation between them | 🔄 |
| 002 | [What the Crate Does Not Declare](002_what_the_crate_does_not_declare.md) | The six omissions, four correct and two exposed | 🔄 |

**Why a catalogue of absences is a doc instance and not a note.** An absent
declaration has no rustdoc page, so `#![ deny( missing_docs ) ]` — which this
crate carries — cannot require anything of it. The two exposures in `002` are
both absences, and neither is visible from any page the lint guarantees exists.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_debug/docs/item
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### DB[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| DB[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| DB1 | `Cursor` | n/a — observation | A public enum is reachable from one of three entry points and the type system does not carry the condition. |
| DB2 | attribute set | n/a — observation | The crate's four attributes are exactly the complement of what `core` already provides, with no overlap and no gap. |
| DB3 | `impl Error for Violation` | n/a — inconsistency | The `Error` impl is empty for a worked-out reason and six sibling crates are empty for unexamined ones, so the unanimity carries no information. |
| DB4 | `Violation` | **latent hazard** | A four-variant defect enum is closed to extension by default rather than by decision, in the one crate whose subject is unanticipated defects. |
