# Integration: One Dependency, Five Declared Dependents

### Scope

- **Purpose**: Census the crate's manifest neighbourhood in both directions, and check the declarations against the code.
- **Responsibility**: Give the inbound dependency and its use, enumerate the crates declaring this one, and identify the declaration that no code backs.
- **In Scope**: `ring_seqno`'s `Cargo.toml` neighbourhood.
- **Out of Scope**: How individual functions travel beyond their direct callers — see [`002`](002_how_the_fold_crossed_four_tiers.md).

### Inbound: One Dependency, Fully Used

```sh
cd "$(git rev-parse --show-toplevel)"/ring_seqno
sed -n '/\[dependencies\]/,/^\[/p' Cargo.toml
```

Live output:

```
[dependencies]
ring_types = { path = "../ring_types" }

[lints]
```

| Dependency | Items used | Where |
|------------|-----------|-------|
| `ring_types` | `Seq`, `Capacity` | `src/lib.rs:27` — one `use`, both named in every signature |

That is the entire inbound edge. `ring_seqno` is a tier-1 crate: it sits directly
on the type vocabulary and on nothing else.

**Notably absent: `ring_index`.** The two are siblings, not a stack — `ring_index`
also depends only on `ring_types`, and neither names the other:

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'ring_seqno\|ring_index' ring_index/Cargo.toml ring_seqno/Cargo.toml
# only the two `name =` lines — no cross-reference in either direction
```

Live output:

```
ring_index/Cargo.toml:name = "ring_index"
ring_seqno/Cargo.toml:name = "ring_seqno"
```

That absence is the crate's reason to exist, and it is the one clause of the
never-fold invariant with real enforcement behind it. See
[`invariant/001`](../invariant/001_the_sequence_is_never_folded_here.md).

### Outbound: Four Declare It, Four Use It

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell's ugrep shim, whose hit order varies run
# to run — the bare form printed this same list in at least three different
# orders across five successive runs. `sort` pins the rest.
command grep -l '^ring_seqno' */Cargo.toml | sed 's|ring/||;s|/Cargo.toml||' | LC_ALL=C sort
```

Live output:

```
ring_batch
ring_consume
ring_cursor
ring_gating
```

| Crate | Declares | Uses | What it calls |
|-------|:--------:|:----:|---------------|
| `ring_cursor` | ✅ | ✅ | `free_slots`, `pending`, `may_claim`, `slowest` — four of five |
| `ring_gating` | ✅ | ✅ | `free_slots` (`:221`) |
| `ring_batch` | ✅ | ✅ | `free_slots` (`:32` import, `:275` call) |
| `ring_consume` | ✅ | ✅ | `pending` (`:342`) |

**Correction (2026-09-28):** this table carried a fifth row, `ring_atomic | ✅
| ❌ | nothing`, and the recipe above used to return five names. Commit
`ce60ae6e8` ("Remove unused dependencies from Cargo.toml files") deleted
`ring_atomic`'s `ring_seqno = { path = "../ring_seqno" }` line, so the census now
returns four crates and all four use what they declare. See § `ring_atomic`
Declares a Dependency It Does Not Use, below, for the finding that removal
resolved.

`ring_cursor` is the principal consumer and the only crate touching more than one
function. Everything else takes exactly one.

### `ring_atomic` Declares a Dependency It Does Not Use

**Finding SQ17.**

**Correction (2026-09-28):** `ring_atomic`'s `Cargo.toml` no longer declares
`ring_seqno` at all. Commit `ce60ae6e8` ("Remove unused dependencies from
Cargo.toml files") deleted the manifest line, and a later documentation pass
also dropped the claim from `readme.md` and `src/lib.rs` — neither mentions
`ring_seqno` any more. Only the task file's implementation record still names
the edge, and it already does so in the past tense. What follows is kept as
the historical record of the finding that removal resolved — every
present-tense "declares"/"asserts" below describes the manifest and prose as
they stood when this section was written, not as they stand now.

```sh
cd "$(git rev-parse --show-toplevel)"
# code and metadata only — ring_atomic's own docs corpus names this crate
# dozens of times and would bury the declarations that matter.
# `command grep` bypasses the shell's ugrep shim, which returns hits in
# completion order: the bare form printed these same lines in two different
# orders across two runs. `sort` pins the rest.
command grep -r 'ring_seqno' ring_atomic/ | command grep -v '/docs/' | LC_ALL=C sort
```

Live output:

```
ring_atomic/task/unverified/102_implement_ring_atomic.md:(used, for `Seq`) and `ring_seqno` (declared, unused in this crate's own code) —
ring_atomic/task/unverified/102_implement_ring_atomic.md:**Depends on:** `ring_types`, `ring_seqno`
```

| File | Kind | Content |
|------|------|---------|
| `ring_atomic/Cargo.toml` | manifest edge | `ring_seqno = { path = "../ring_seqno" }` |
| `ring_atomic/readme.md` | prose assertion | "Depends on [`ring_types`], [`ring_seqno`]." |
| `ring_atomic/src/lib.rs` | module doc comment | `//! Depends on `ring_types` and `ring_seqno`.` |
| `ring_atomic/task/unverified/102_…` | task specification | `**Depends on:** `ring_types`, `ring_seqno`` |
| `ring_atomic/task/unverified/102_…` | implementation record | "…`ring_seqno` (declared, unused in this crate's own code)" |

Four assertions of the dependency. **Zero uses** — no `ring_seqno::` path, no
`use ring_seqno…`, nothing in `src/` or `tests/`. The line in `src/lib.rs` is a
doc comment, not code.

The fifth line is not a fifth assertion. It is the task file's implementation
record, written after the fact, saying that the declared edge is unused — and
citing `ring_atomic/docs/integration/001` for the measurement. So this recipe
now counts its own consequence: recording the finding added a line to the
corpus that the recipe measuring the finding sweeps. The count moved from four
to five without a single new assertion being made, which is worth separating
out because a reader watching only the number would read the crate as having
gotten worse.

Nothing catches it:

```sh
cd "$(git rev-parse --show-toplevel)"
# not one manifest enables the lint that would have caught this edge
grep -r 'unused_crate_dependencies' Cargo.toml */Cargo.toml \
  || echo '(no matches — the lint is enabled nowhere)'
# control — the identical expression over a manifest that does enable it, so
# the empty result above reads as an absence and not as a mistyped pattern
printf '[workspace.lints.rust]\nunused_crate_dependencies = "warn"\n' > /tmp/-ucd_control.toml
grep -r 'unused_crate_dependencies' /tmp/-ucd_control.toml
rm -f /tmp/-ucd_control.toml
```

Live output:

```
(no matches — the lint is enabled nowhere)
unused_crate_dependencies = "warn"
```

`unused_crate_dependencies` is not enabled in the workspace lint table or in any
crate's own. The workspace's `[workspace.lints.rust]` carries five lints and this
is not among them.

**The cost is small and the shape is not.** A path dependency that is never used
adds a build-graph edge — `ring_seqno` must be compiled before `ring_atomic` even
though nothing in it is referenced — and it makes the family's dependency forest
wrong in a document that reads as authoritative. Anyone deriving the tier
structure from the manifests places `ring_atomic` above `ring_seqno`; it is beside
it.

The likely origin is visible in the fourth row: the task file specified the
dependency before the crate was written, the manifest and both doc comments
followed the task, and the implementation turned out not to need it. Nothing in
the process re-checked the assumption afterwards, because nothing in the process
can — no lint is enabled that would.

Repair is one line removed from `Cargo.toml` and three sentences corrected. It
is not applied here: it touches another crate, and the accurate fix is arguably
the *lint* rather than the manifest, since the same drift may exist elsewhere in
a 33-crate family. Both belong to a change with its own verification run.

**Correction (2026-09-28):** this has since happened in full, unprompted by
any disposition here. Commit `ce60ae6e8` ("Remove unused dependencies from
Cargo.toml files") dropped the `Cargo.toml` line, and a later documentation
pass corrected `ring_atomic/readme.md` and `ring_atomic/src/lib.rs`, which no
longer assert the dependency either — three of the four sentences named above
are gone along with the manifest edge. Only the task file's own
implementation record still names it, in the past tense, as the record of
this finding. No lint was added — the repair was manual, so the same drift
this recipe warns about is still uncaught wherever else it recurs in the
family.

**The same check, family-wide, separates two different defects** — and a scan of
`src/` alone conflates them, because a dependency used only by the test suite
looks identical to one used by nothing:

```sh
cd "$(git rev-parse --show-toplevel)"
for c in ring_*/; do
  n=$( basename "$c" )
  for d in $( grep -oE '^ring_[a-z_]+' "$c/Cargo.toml" ); do
    [ "$d" = "$n" ] && continue
    s=$( grep -rhE "(use +${d}|${d}::)" "$c/src"   2>/dev/null | grep -vE '^\s*(///|//!|//)' | wc -l )
    t=$( grep -rhE "(use +${d}|${d}::)" "$c/tests" 2>/dev/null | grep -vE '^\s*(///|//!|//)' | wc -l )
    [ "$s" -eq 0 ] && [ "$t" -eq 0 ] && echo "UNUSED    $n -> $d"
    [ "$s" -eq 0 ] && [ "$t" -gt 0 ] && echo "TEST-ONLY $n -> $d"
  done
done
true   # the loop's own status is that of its last test, which is false whenever
       # the final pair is in use — the listing above is the actual result
```

Live output:

```
TEST-ONLY ring_barrier -> ring_gating
TEST-ONLY ring_debug -> ring_config
TEST-ONLY ring_event -> ring_store
TEST-ONLY ring_flush -> ring_config
TEST-ONLY ring_handle -> ring_config
TEST-ONLY ring_handle -> ring_types
TEST-ONLY ring_poll -> ring_config
TEST-ONLY ring_poll -> ring_types
TEST-ONLY ring_publish -> ring_claim
TEST-ONLY ring_publish -> ring_consume
TEST-ONLY ring_publish -> ring_barrier
TEST-ONLY ring_publish -> ring_gating
TEST-ONLY ring_registry -> ring_config
TEST-ONLY ring_registry -> ring_core
TEST-ONLY ring_shutdown -> ring_config
TEST-ONLY ring_testkit -> ring_config
TEST-ONLY ring_testkit -> ring_types
TEST-ONLY ring_tls -> ring_store
TEST-ONLY ring_tls -> ring_event
TEST-ONLY ring_tls -> ring_slot
```

**Correction (2026-09-28):** this block used to open with two `UNUSED` lines,
`ring_align -> ring_types` and `ring_atomic -> ring_seqno`. Commit `ce60ae6e8`
("Remove unused dependencies from Cargo.toml files") deleted both
declarations, so the loop above no longer has either pair to report.

Across all 33 crates, as of this writing:

| Category | Count | Meaning |
|----------|------:|---------|
| **Unused** — absent from `src/` and `tests/` alike | **0** | Both prior instances (`ring_align → ring_types`, `ring_atomic → ring_seqno`) were removed by commit `ce60ae6e8` |
| **Test-only** — in `[dependencies]`, used only under `tests/` | 20 | Belongs in `[dev-dependencies]`; a different defect, and a milder one |

`ring_atomic`'s declaration was one of exactly two dependencies in the family
that nothing referenced at all; both are now gone from the manifests, so the
census's only remaining drift is the twenty test-only entries — a separate and
much more common pattern, real usage in the wrong table — which this recipe
still exists to keep visible now that the unused category is empty.

The comment filter matters: without it the scan counts `//! Depends on ring_seqno.`
as a use and reports `ring_atomic` clean.

### The Reverse Dependency Is Shallower Than It Looks

Four real consumers is a modest number for a tier-1 crate, and three of them take
exactly one function. The crate's actual reach is much wider, but it runs
*through* `ring_cursor` rather than around it:

| Reached via | Crates |
|-------------|--------|
| `CursorPair::may_claim` → `ring_seqno::may_claim` | `ring_wait:241`, `ring_shutdown:620` |
| `CursorPair::pending` → `ring_seqno::pending` | `ring_wait:268` |
| `CursorPair::free_slots` → `ring_seqno::free_slots` | `ring_spsc` |
| `ring_cursor::slowest` → `ring_seqno::slowest` | `ring_barrier:193`, `ring_gating:194` |

So the arithmetic reaches at least nine crates while the manifest shows four
(see the Correction under § Outbound, above — it showed five when this count
was written).
That indirection is the healthy case — `ring_cursor` adds the atomic loads, and
its callers get the arithmetic without needing to know it exists.

### SQ17 — A Dependency Declared Four Times and Used Never

The edge exists in the manifest and in prose, and nowhere in the code:

```
ring_atomic/Cargo.toml:10   ring_seqno = { path = "../ring_seqno" }
ring_atomic/src/lib.rs        //! Depends on `ring_types` and `ring_seqno`.
                              (the only mention in the source, and it is a comment)
```

**Finding.** `ring_atomic` declares `ring_seqno` in `Cargo.toml` and asserts the edge in three more documents; nothing in the crate uses it, and `unused_crate_dependencies` is not enabled anywhere in the workspace.

**Correction (2026-09-28):** resolved — see the Correction under § `ring_atomic`
Declares a Dependency It Does Not Use, above. Commit `ce60ae6e8` removed the
`Cargo.toml` line and a later documentation pass corrected the other two
prose sites; this finding is kept as the historical record.

---

### SQ18 — Two Crates Skip the Wrapper, One of Them Pays for It

The direct callers are two, and the cast lives in one of them:

```
ring_gating/src/lib.rs   ring_seqno::free_slots( producer, slowest, self.capacity )
ring_batch/src/lib.rs    use ring_seqno::free_slots;
ring_batch/src/lib.rs:323  if ( free_slots( at, behind, capacity ) as usize ) < count
                                                     ^^^^^^^^ already usize
```

**Finding.** Only `ring_gating` and `ring_batch` call these functions directly; every other consumer reaches them through `ring_cursor`'s wrappers, and `ring_batch` — the one that bypasses the wrapper layer — is also the one that wrote a redundant cast around the result.

---

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_the_slowest_fold.md](../algorithm/002_the_slowest_fold.md) | The chain in the last table, in detail |

### Integrations

| File | Relationship |
|------|--------------|
| [002_how_the_fold_crossed_four_tiers.md](002_how_the_fold_crossed_four_tiers.md) | Why the indirection worked rather than forking |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_the_sequence_is_never_folded_here.md](../invariant/001_the_sequence_is_never_folded_here.md) | Why `ring_index` is absent from the inbound edge |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_three_capacity_readings.md](../item/001_the_three_capacity_readings.md) | Per-function caller counts |

### Sources

| File | Relationship |
|------|--------------|
| `ring_seqno/Cargo.toml` | The one inbound edge |
| `ring_atomic/Cargo.toml:10` | The declaration nothing backs |
| `ring_atomic/src/lib.rs:5` | And its doc comment |
| `ring_index/Cargo.toml` | The sibling that is not a dependency |
| `Cargo.toml` § `[workspace.lints.rust]` | Five lints; `unused_crate_dependencies` not among them |

### Tests

| File | Relationship |
|------|--------------|
| `tests/seq_test.rs:15-16` | Both `use` lines — the crate's whole import surface |
| — | No test asserts the dependency set. The census above is the only check |
