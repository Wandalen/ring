# Algorithm Doc Definition

### Scope

- **Purpose**: Give the two procedures this crate performs — selecting a backend from one boolean, and assembling the parts a selected backend needs.
- **Responsibility**: For each, state the abstract and the steps, including the steps that cannot be taken from here.
- **In Scope**: Backend selection; assembly and handle production.
- **Out of Scope**: What each backend does once built, which is `ring_spsc`'s and `ring_mpsc`'s.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Selecting a Backend From One Boolean](001_selecting_a_backend_from_one_boolean.md) | `is_multi_producer()` is the whole branch, and the reason the other four fields cannot influence it | 🔄 |
| 002 | [Assembling a Ring From a Validated Record](002_assembling_a_ring_from_a_validated_record.md) | Five fields consumed in a fixed order, two of which this crate cannot fully honour | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_factory/docs/algorithm
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### FC[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| FC[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| FC1 | `ring_factory` | n/a — observation | Three backends, two selection mechanisms, and neither can see the other: `is_multi_producer()` chooses between the in-house rings, a Cargo feature chooses crossbeam, and the crossbeam path reads only `capacity()` and `overflow()` — `is_multi_producer` is named zero times in this crate |
| FC2 | `ring_factory` | n/a — coverage | The crate's only algorithm is a branch made one crate down, and no test at this crate's grain asserts which backend a build selected, because nothing on the return path reports it |
| FC3 | `ring_factory` | n/a — observation | The assembly has no control flow at all — zero `if`, zero `return`, zero loops outside doc comments, and both `match` arms belong to error translation rather than assembly |
| FC4 | `ring_factory` | n/a — unenforced | Two of the five fields the assembly consumes — `wait` and `batch` — reach nothing, and the only crate that reads either builds no rings |
