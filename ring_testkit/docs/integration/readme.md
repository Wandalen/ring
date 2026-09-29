# Integration Doc Definition

### Scope

- **Purpose**: Account for every dependency edge this crate declares — what each is used for, how much of it is reachable, and which of them an ordinary build cannot see.
- **Responsibility**: The edge list, the surface each edge contributes, the reasons the unused parts are unused, and the evidence each account rests on.
- **In Scope**: `ring_core`, `ring_tls`, `ring_shutdown`, `ring_config`, `ring_types`, `loom`; the `cfg(loom)` seam as this crate meets it.
- **Out of Scope**: What the fixture does with the edges (→ [`algorithm/`](../algorithm/readme.md)); whether the missing edge should be added (→ [`decisions/readme.md`](../decisions/readme.md) Pending 4).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [The Three Edges And The One That Is Missing](001_the_three_edges_and_the_one_that_is_missing.md) | The five edges present in every build, and the crate this one needs and was not given | 🔄 |
| 002 | [The Edge That Only Exists Under A Cfg](002_the_edge_that_only_exists_under_a_cfg.md) | The sixth entry, conditional, and the three populations that "uses the cfg" turns out to name | 🔄 |

**The split is by whether the edge is there when you look.** `001`'s five edges
are in every build: a reader can see them, a compiler checks them, and
`cargo +nightly udeps` reports on them. `002`'s single edge is in none of those
places unless `RUSTFLAGS` says so, which is why it needs its own account rather
than a sixth row in `001`'s table — the questions worth asking about it are
different questions.

They are apart for a second reason. `001` is about a crate's own dependency
list. `002` is about a *cfg* — a workspace-wide switch this crate happens to be
the only current consumer of — and its subject keeps escaping the crate boundary
in a way `001`'s does not.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit/docs/integration
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### TK[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| TK[0-9]+ ' readme.md )"
printf 'edges accounted for:      %s\n' "$( command grep -cE '^(ring_[a-z_]+|loom) = ' ../../Cargo.toml || true )"
```

Live output:

```
instances:                2
finding headings inside:  4
rows in the table below:  4
edges accounted for:      6
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TK17 | the unused waiting surface | **misleading doc** | The `ring_shutdown` row explains `wait_for_close`, `for_space_or_close` and `Wake` being unused with "those need a second thread, and a script has one" — true of `Script::run` and not of the crate, which calls `thread::spawn` ten times across three files, twice inside `leak_ends`' own doc example. |
| TK18 | the citation behind "no fourth edge" | **misleading doc** | Evidence G2 cites a clean `cargo +nightly udeps` run, and M4's command carries no `RUSTFLAGS`, so the one dependency declared under `[target.'cfg(loom)']` is absent from the graph the tool walks — it can neither report `loom` unused nor confirm it used. |
| TK19 | the loom dependency's five declarers | n/a — unadopted | `tests/exhaustive_test.rs` is the only file in all thirty-three crates opening `#![ cfg( loom ) ]`, while five manifests declare `loom = "0.7"`; three of the other four never write `loom::` at all, and `001` presents them as four precedents for the shape this crate is in fact the first to use. |
| TK20 | the workspace's record of the cfg's readers | n/a — drift | The root `Cargo.toml` comment justifying a workspace-wide `check-cfg` says "Only ring_atomic, ring_cursor and ring_publish read the cfg"; the measured populations are 2, 5, 20 and 1, no one of which is that trio, and the crate holding the family's only loom model is named in none of them. |
