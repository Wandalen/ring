# API Doc Definition

### Scope

- **Purpose**: Define the crate's public surface twice — once as signatures a caller reads, once as an edge a manifest would have to declare.
- **Responsibility**: What each entry point guarantees and requires; what a consumer must import alongside it; who has taken the edge.
- **In Scope**: `Script`, `Step`, `Outcome`, `Anomaly`, `audit_received`, `audit_received_unordered`, `leak`, `leak_ends`, and the crate's re-export policy.
- **Out of Scope**: The bodies behind the signatures (→ [`algorithm/`](../algorithm/readme.md)); the field-by-field definitions (→ [`type/`](../type/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [The Script Surface](001_the_script_surface.md) | Every signature, what it guarantees, and the three choices that are not obvious | 🔄 |
| 002 | [The Surface No Crate Has Taken](002_the_surface_no_crate_has_taken.md) | The same surface as a dependency edge — imports, re-exports, and an adoption count of zero | 🔄 |

**The split is the surface a caller reads against the surface a manifest
declares, and they were written apart because only one of them has ever been
exercised.** `001` is complete on its own terms: eleven operations, their
preconditions, and the three signature decisions worth defending. `002` asks the
question `001` cannot — who calls this — and the answer is nobody, which is not a
caveat on a signature but a fact about the crate.

Folding them together would let the adoption count read as a footnote to the
guarantees. It is the other way round: every guarantee in `001` is currently a
promise to a hypothetical consumer, and `002` is where that is said plainly.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit/docs/api
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### TK[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| TK[0-9]+ ' readme.md )"
```

Live output:

```
instances:                2
finding headings inside:  4
rows in the table below:  4
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TK5 | `Script`'s two accessors | n/a — unadopted | `steps()` and `stage_limit()` have four call sites across every crate in `ring/`, `any crate root` and `the spike root`, all four inside the two tests that assert them; `Script::run` reads both fields directly, so the encapsulation they provide has no consumer inside the crate or outside it. |
| TK6 | two encapsulation policies | n/a — inconsistency | `Outcome` exposes ten bare `pub` fields with no accessors and `Script`, 424 lines later in the same file, exposes none and reaches its two fields through methods; both are defensible and neither is written down, so a type added later has two conventions to choose between. |
| TK7 | a testkit with no consumer | n/a — unadopted | Zero manifests in `ring/` declare `ring_testkit` and there are zero `use ring_testkit` statements outside it, while eighteen other crates name it in prose — so the fixtures, the audit and the loom bridge are exercised only by this crate's own 33 tests. |
| TK8 | `ring_flush`'s routed allocation measurement | n/a — doc gap | `ring_flush` defers its C2 allocation constraint to this crate as "the one place a counting allocator could be justified", and there is no `#[ global_allocator ]` here — four sibling crates each built their own instead — and no mention of allocation in the feature the routing cites. |
