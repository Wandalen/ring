# Integration Doc Definition

### Scope

- **Purpose**: Record both halves of this crate's place in the family — what it can reach, and what reaches it — with the measurement rather than the intention.
- **Responsibility**: The reachability boundary in each direction, what crosses each seam, and what the crate's other documents assume about a caller.
- **In Scope**: Which types hold a `CursorPair` and which expose one; the four inbound edges; the outbound edges and the manifest sections they would occupy.
- **Out of Scope**: The checks themselves (→ [`api/`](../api/readme.md)); the door built to route around the boundary (→ [`workaround/`](../workaround/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Reaching the Cursors of a Live Ring](001_reaching_the_cursors_of_a_live_ring.md) | Four crates hold a `CursorPair`, one hands it out, and the family's entry point is not that one | 🔄 |
| 002 | [The Edges That Were Never Drawn](002_the_edges_that_were_never_drawn.md) | The outbound direction, measured against the eight callers the inbound argument names | 🔄 |

**The two directions fail differently and only one of them is a design problem.**
Inbound, the crate cannot reach what it wants to check, and closing that needs a
decision in a crate on the export Contract. Outbound, nothing has taken an edge
that costs one line in a `[dev-dependencies]` section and requires no decision from
anyone.

Splitting them keeps the second from being read as a consequence of the first. It
is not: the eight crates that can already reach this check are the eight that have
not called it, and their reachability was never the obstacle.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_debug/docs/integration
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### DB[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| DB[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| DB29 | the Error Handling table | **latent hazard** | The seam documented as having no failure shape routes through a plain subtraction that underflows on a lapped ring, so the crate's only Contract-reachable entry point panics inside a crate it does not name on the input most worth checking. |
| DB30 | rejected option I1 | **misleading doc** | An option is rejected as breaking for a third of the backends, on a backend that is opt-in, not requested by this crate, and never compiled in any build it participates in. |
| DB31 | the eight named callers | n/a — unadopted | Every crate named as a natural caller takes the prerequisite dependency the argument identifies and none takes the edge itself, leaving a complete and correct case for the crate unacted-on in eight manifests. |
| DB32 | `ring_testkit` and `ring_debug` | n/a — observation | The family built a testkit and a cursor checker for the same reader and neither names the other, so the one place the check would be idiomatic is the one place it has to be remembered. |
