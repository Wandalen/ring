# Invariant: Tier Zero Depends on Nothing

### Scope

- **Purpose**: State the empty-dependency-list rule that makes the family's 33-crate forest acyclic by construction rather than by audit, and record what it costs and what would break it.
- **Responsibility**: State the invariant, its enforcement mechanism, and the consequences of violation.
- **In Scope**: The empty `[dependencies]`; the acyclicity argument; the `error_tools` exception; what nothing enforces.
- **Out of Scope**: The 31 crates that declare the edge (→ [`integration/001`](../integration/001_the_crate_thirty_one_of_thirty_three_depend_on.md)); the discriminants-here rule that keeps behaviour out (→ [`pattern/001`](../pattern/001_discriminants_here_handlers_elsewhere.md)).

### Invariant Statement

**`ring_types` declares no dependencies — not a workspace crate, not a
third-party crate, not an error library.**

```text
T1.  ring_types has zero entries under [dependencies]
T2.  therefore ring_types is in no dependency cycle, for any graph containing it
T3.  therefore any crate may add an edge to ring_types without a cycle check
```

**T2 is not a claim about the current graph; it is a claim about every possible
graph.** A vertex with out-degree zero cannot lie on a directed cycle, since a
cycle requires every vertex on it to have an outgoing edge on the cycle. The
family's forest is not audited for cycles — it is shaped so the audit is
unnecessary, and this crate is where the shape is anchored.

**T3 is the property the other 32 crates actually use.** Thirty-one of them
declare `ring_types`, and none of them had to check whether doing so closed a
loop. That is the whole return on T1.

**The rule is stated in the crate's own header and is a rule rather than an
observation:**

```sh
cd "$(git rev-parse --show-toplevel)"
awk '/Every other crate in the family depends on this one/,/construction\.$/' ring_types/src/lib.rs
```

Live output:

```
//! Every other crate in the family depends on this one; this one depends on
//! nothing, including no error crate — that is what lets tier 0 compile in
//! isolation and what makes the family's dependency forest acyclic by
//! construction.
```

### Enforcement Mechanism

**There is no enforcement mechanism. That is the finding.**

| # | Candidate | Enforces T1 | Reality |
|---|-----------|-------------|---------|
| M1 | The empty `[dependencies]` block | — | It is the *state*, not a guard on the state |
| M2 | `cargo` itself | ❌ | Adding a dependency is the ordinary operation `cargo add` performs |
| M3 | Gate G5 (`bench_harness`) | ❌ | Reads `gate/declared/ring/export_surface.txt` — which crates may be *imported by consumers*, not which this crate imports |
| M4 | The workspace `[lints]` | ❌ | Lints see code, not manifests |
| M5 | The test suite | ❌ | A test cannot observe its own crate's manifest |
| M6 | The header comment | ⚠️ | States the rule to a reader. Nothing reads it mechanically |

**The invariant that anchors the family's acyclicity is protected by a comment.**
One `cargo add error_tools` in this directory breaks T1, compiles cleanly,
passes every test, passes clippy, and passes G5.

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/\[dependencies\]/,/^\[lints\]/p' ring_types/Cargo.toml
```

Live output:

```
[dependencies]


[lints]
```

**A gate is writable and does not exist.** The check is three lines — assert
that `ring_types`'s `[dependencies]` section is empty — and it would sit
naturally beside the family's existing export-surface gate, which already parses
manifests. Recorded as a pending question in
[`decisions/`](../decisions/readme.md).

**What T1 costs, paid in one place:**

| Cost | Where | Size |
|------|-------|------|
| Hand-written `Display` for `RingError` | `src/error.rs:162–182` | 20 lines, nine `match` arms |
| No `#[ from ]` conversions | Everywhere | Variants are built by literal syntax at the failure site |
| No `error_tools`, against workspace convention | `Cargo.toml` | A documented, deliberate exception |

**Twenty lines is the whole price**, and `error.rs` states the trade in its own
header (`src/error.rs:9–10`). This is worth recording precisely because the
convention it breaks is a strong one: a reader who knows the workspace uses
`error_tools` everywhere will read its absence here as an omission unless told
otherwise.

**T1 is stronger than "no workspace dependencies".** It also excludes
third-party crates with no workspace edges of their own — `thiserror` could not
create a cycle within the family — because the second half of the rule is
compile-in-isolation: tier 0 must build with nothing else present, so a
consumer, a test harness, or a bisect can compile it alone.

### Violation Consequences

| # | Violation | Immediate effect | How it presents |
|---|-----------|------------------|-----------------|
| V1 | A third-party dependency is added | T2 still holds; T1 and compile-in-isolation are gone | Nothing fails. The property is lost silently and no one learns until it is needed |
| V2 | A workspace `ring_*` dependency is added | T2 is gone — a cycle becomes possible | `cargo` refuses the cycle *if one is actually formed*, with an error naming the crates, not the rule |
| V3 | A dev-dependency is added | T1 as stated survives; compile-in-isolation of the *tests* does not | Invisible — dev-deps do not affect consumers, so nothing downstream notices |
| V4 | `error_tools` replaces the hand-written `Display` | V1, with a locally excellent justification | Reads as a cleanup. Removes 20 lines and the invariant with them |

**V4 is the one that will actually happen.** It is the change a reviewer would
approve: it deletes hand-written boilerplate, adopts the workspace convention,
and touches nothing else. The 20 lines it removes are the *price of the
invariant*, so removing them is not a saving — and the diff gives no sign of
that.

**V2 is the only violation with a mechanical backstop, and it is a weak one.**
`cargo` errors on a cycle when the cycle is closed, which may be several edges
and several weeks after the edge that made it possible. The error names the
participants, not the rule that was supposed to prevent them.

**V1 and V3 have no backstop at all.** Nothing in the workspace observes them.

**The asymmetry with [`invariant/001`](001_every_capacity_has_a_valid_mask.md) is
worth naming.** That invariant is enforced structurally — no program can violate
it, because the field is private. This one is enforced by nobody: it is a
property of a text file that any edit can change, protecting a guarantee 31
crates rely on. **Two invariants in one crate, one unbreakable and one
undefended**, and the undefended one is the one with the wider blast radius.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_vocabulary_surface.md](../api/001_the_vocabulary_surface.md) | The surface T1 keeps free of foreign types |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_error_enum_as_a_closed_copy_set.md](../data_structure/002_the_error_enum_as_a_closed_copy_set.md) | The hand-written `Display` T1 forces, and the two other constraints beside it |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_the_crate_thirty_one_of_thirty_three_depend_on.md](../integration/001_the_crate_thirty_one_of_thirty_three_depend_on.md) | T3's beneficiaries — the 31 crates that added the edge without checking |
| [../integration/002_the_registry_that_declined_the_shared_error.md](../integration/002_the_registry_that_declined_the_shared_error.md) | The one that did not, and what it built instead |

### Invariants

| File | Relationship |
|------|--------------|
| [001_every_capacity_has_a_valid_mask.md](001_every_capacity_has_a_valid_mask.md) | The crate's other invariant — structurally unbreakable, where this one is undefended |

### Items

| File | Relationship |
|------|--------------|
| [../item/implementation/003_impl_display_for_ring_error.md](../item/implementation/003_impl_display_for_ring_error.md) | The twenty lines T1 costs |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_errors_and_positions_do_not_allocate.md](../non_functional_requirement/001_errors_and_positions_do_not_allocate.md) | The other property the hand-written error protects |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_discriminants_here_handlers_elsewhere.md](../pattern/001_discriminants_here_handlers_elsewhere.md) | The companion rule — T1 keeps dependencies out, that one keeps behaviour out |

### Sources

| File | Relationship |
|------|--------------|
| [`Cargo.toml`](../../Cargo.toml) | The empty `[dependencies]` — the invariant's entire state |
| [`src/lib.rs`](../../src/lib.rs) | Lines 5–8, M6 — the rule stated to a reader and to nothing else |
| [`src/error.rs`](../../src/error.rs) | Lines 10–11, the `error_tools` exception stated in place |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/types_test.rs`](../../tests/types_test.rs) | ❌ **Nothing here can assert this invariant.** A test compiles *with* its crate's dependencies already resolved, so it cannot observe that the list was empty. The check belongs to a manifest gate that does not exist — which is what makes M6, a comment, the current enforcement |

### TY36 — Nothing Asserts the Empty Dependency Table

```sh
cd "$(git rev-parse --show-toplevel)"/ring_types
awk '/^\[dependencies\]/{f=1;next} /^\[/{f=0} f' Cargo.toml | grep -c . || true
```

Live output:

```
0
```

Empty today. The gate that would keep it empty is P5 in
[`../decisions/readme.md`](../decisions/readme.md), and it is filed as a pending
question rather than a task because the violation it would catch —
`error_tools` replacing the hand-written `Display` impl — is the workspace's own
house convention applied correctly.

### TY37 — The Crate Uses `core` Throughout and Now Declares `#![ no_std ]`

```sh
cd "$(git rev-parse --show-toplevel)"/ring_types
printf 'no_std attrs: '; grep -rc 'no_std' src/ | grep -v ':0' | wc -l
printf 'std:: paths:  '; grep -r 'std::' src/*.rs | wc -l
printf 'core:: paths: '; grep -r 'core::' src/*.rs | wc -l
```

Live output:

```
no_std attrs: 1
std:: paths:  1
core:: paths: 2
```

`no_std attrs: 1` counts matching *files*, not occurrences — `grep -c` reports
one hit per file, and exactly one file (`src/lib.rs`, line 27) is nonzero. The
one `std:: paths` hit is not an import: it is the load-bearing comment at
`src/lib.rs:33` explaining the attribute itself ("a `use std::` here would be a
`std` dependency for the entire family"), the same line
[`../item/use_declaration/006`](../item/use_declaration/006_use_core_fmt.md)
already accounts for. No code path names `std::` anywhere in this crate.

The requirement in
[`../non_functional_requirement/003`](../non_functional_requirement/003_the_crate_compiles_without_std.md)
is now met in fact and by mechanism both. `#![ no_std ]` converted the
convention into a compile error, and the crate still builds — it imports
nothing from `std`, so nothing broke.
