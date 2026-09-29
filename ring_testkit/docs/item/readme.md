# Item Doc Definition

### Scope

- **Purpose**: Catalogue the public surface as a set — and the absences no single declaration can carry — so that cross-declaration properties become checkable.
- **Responsibility**: Enumerate every declaration and every load-bearing omission, with the command that regenerates each enumeration.
- **In Scope**: Eight public declarations, four `impl` blocks, seven public methods, eight `#[ must_use ]` attributes; twelve absences.
- **Out of Scope**: What each signature guarantees (→ [`api/`](../api/readme.md)); the layout of the values themselves (→ [`data_structure/`](../data_structure/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Two Components That Meet In No Line Of `src/`](001_two_components_that_meet_in_no_line_of_src.md) | The declarations that exist, and the reachability relation over them | 🔄 |
| 002 | [What The Crate Does Not Declare](002_what_the_crate_does_not_declare.md) | Twelve absences, ten correct and two exposed | 🔄 |

**Why a catalogue of absences is a doc instance and not a note.** An absent
declaration has no rustdoc page, so `#![ deny( missing_docs ) ]` — which this
crate carries — cannot require anything of it. Both exposures in `002` are
absences, and neither is visible from any page that lint guarantees exists.

The two instances answer opposite questions about the same set. `001` asks what
the declarations *are* to each other and finds two disconnected components; `002`
asks what is missing from the set and finds that the gaps are all at the crate's
edges. Neither question can be answered by reading any single declaration's page,
which is what makes them a catalogue rather than an index.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit/docs/item
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### TK[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| TK[0-9]+ ' readme.md )"
printf 'declarations catalogued:  %s\n' "$( command grep -cE '^\| [0-9] \| ' 001_two_components_that_meet_in_no_line_of_src.md )"
printf 'absences catalogued:      %s\n' "$( command grep -cE '^\| A[0-9]+ \| ' 002_what_the_crate_does_not_declare.md )"
```

Live output:

```
instances:                2
finding headings inside:  4
rows in the table below:  4
declarations catalogued:  8
absences catalogued:      12
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TK25 | the reachability relation over the public surface | n/a — observation | The call graph over the eight public declarations has two disconnected components inside `src/lib.rs` — the scripted fixture and the loom bridge — joined by no expression in the library and used together only in the two test files. |
| TK26 | where `#[ must_use ]` was placed | **latent hazard** | **Was exposed, now closed** — five attributes sat on four accessors and a builder pair whose results a caller can recompute for free, while `Script::run`, `leak` and `leak_ends` carried none and returned plain values, so `script.run( &mut ring );` and `leak( ring );` both compiled as statements; all three now carry the attribute with a message naming what is lost, which is what brings the count to eight. |
| TK27 | `Anomaly`'s room attribute | **latent hazard** | **Was exposed, now closed** — the only type this crate returns for a caller to match had three variants and no room attribute, while `invariant/001` weighed adding a fourth as a live design option; `Anomaly` now carries `#[ non_exhaustive ]`, and the fourth variant arrived additively behind it. |
| TK28 | the import cost of the entry point | n/a — observation | `Script::run` takes a `ring_core::Ring< u32 >` and the crate declares zero `pub use`, so a consumer needs `ring_core` and `ring_config` in its own manifest before it can call the one function the fixture exists for — the same three-crate cost this crate's own dev-dependencies pay. |
