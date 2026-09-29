# Integration: Two Declared, One Used

### Scope

**Purpose:** Record what `ring_atomic` depends on, what its prose documents say
it depends on, and a mismatch that existed for a time between them — a declared
`ring_seqno` edge the code never used, later closed by a manifest edit this
document's own "declined" disposition did not anticipate. See AT17's Update.

**Responsibility:** The two incoming dependency edges, the `Depends on` sentence in
the module comment, the readme's dependency line, and the task file that specified
the crate.

**In Scope:** `ring_atomic/Cargo.toml`; `ring_atomic/src/lib.rs:5`,
`:65-70`; `ring_atomic/readme.md:5`;
`ring_atomic/task/unverified/102_implement_ring_atomic.md:19`.

**Out of Scope:** The outgoing edges are
[`integration/002`](002_five_crates_downstream.md). The `loom` dependency, which is
target-conditional and works differently, is
[`workaround/001`](../workaround/001_the_loom_seam_and_the_manifest_above_it.md).

---

## One Declared, One Imported, One Historical Mention Left

*(This section originally censused a live mismatch: two dependencies declared,
one imported, and four prose lines across three files naming the unused one.
The manifest has since been edited independently of this document — see
AT17's Update — and the recipe below, re-run live, now reads the resolved
state: one declared, one imported, agreeing. What follows is that live
recipe, not a reconstruction of the original census; the original counts are
preserved in the prose above and in AT17 below.)*

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the manifest now declares --'
command grep -E '^ring_' ring_atomic/Cargo.toml
echo '  -- the crate imports --'
command grep -E '^use ' ring_atomic/src/lib.rs
echo '  -- and one historical task-file spec still names the removed edge --'
command grep -r 'ring_seqno' ring_atomic/ 2>/dev/null | command grep -v '/docs/' | command grep -v 'Cargo.toml' | sed 's|ring_atomic/|    |'
```

Live output:

```
  -- the manifest now declares --
ring_types = { path = "../ring_types" }
  -- the crate imports --
use loom::sync::atomic::{ AtomicU64, AtomicUsize };
use core::sync::atomic::{ AtomicU64, AtomicUsize };
use core::sync::atomic::Ordering;
use ring_types::Seq;
  -- and one historical task-file spec still names the removed edge --
    task/unverified/102_implement_ring_atomic.md:**Depends on:** `ring_types`, `ring_seqno`
    task/unverified/102_implement_ring_atomic.md:(used, for `Seq`) and `ring_seqno` (declared, unused in this crate's own code) —
```

One document still names `ring_seqno`: the task file that specified the crate
before it was written. It is a pre-implementation spec, left as-is per this
corpus's historical-record convention rather than corrected to match current
reality — see [`lifecycle/002`](../lifecycle/002_from_task_file_to_crate.md).
`src/lib.rs` and `readme.md`, the two prose sites this document is actually
scoped to (see Scope above), have both been corrected to match the manifest.

---

### AT17 — `ring_seqno` Is Declared, Documented Three Times, and Used Nowhere

The `ring_seqno` string appears four times in the crate: once in the manifest and
once each in the module comment, the readme, and the task file that specified the
crate. It appears zero times in any `use`, any path, any doctest, and any test.
Confirmed against every target:

```
unused dependencies:
`ring_atomic v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_atomic)`
└─── dependencies
     └─── "ring_seqno"
Note: They might be false-positive.
      For example, `cargo-udeps` cannot detect usage of crates that are only used in doc-tests.
```

The tool's own caveat is the doc-test case, and it does not apply: the string is
absent from the source entirely except in one `//!` comment, so no doctest can be
reaching it either.

**Finding.** This is the mirror image of the family's usual documentation failure.
Elsewhere — eleven crates of thirty-three — a `Depends on` sentence understates a
correct manifest, because a dependency was added and the prose was not revisited
([`ring_batch` § BA17](../../../ring_batch/docs/integration/001_four_edges_in_two_written_down.md)).
Here all three prose sites are faithful, in agreement with each other, and wrong,
because the manifest they faithfully describe is the thing that is stale.

The origin is visible in the fourth site. The task file's `**Depends on:**
ring_types, ring_seqno` predates the implementation — the edge was specified before
the code existed, declared on that basis, carried into both prose documents, and
then never needed. Nothing in the pipeline from specification to implementation
checks back.

Family-wide the problem is rare, which is what makes it interesting rather than
routine. Sweeping all 33 crates finds exactly two unused dependencies:

```
  ring_align      └─── "ring_types"
  ring_atomic      └─── "ring_seqno"
```

Two of thirty-three, both Tier 0/1 primitives, both specified up front. The
manifests are otherwise clean — this is not a workspace with drifting dependencies,
it is a workspace with two crates whose specifications were slightly larger than
their implementations turned out to be.

**Disposition:** declined — removing the unused `ring_seqno` edge means editing
`Cargo.toml` plus three prose sites (`src/lib.rs:5`, `readme.md:5`,
`task/unverified/102_implement_ring_atomic.md:19`), the same shape of change
`ring_align`'s own AL22 already declined as this crate's own change with its
own verification run, not a corpus disposition pass.

**Update:** the edge was removed anyway. `Cargo.toml` no longer declares
`ring_seqno` — confirmed by direct inspection and by `git log` against the line,
which lands in an unrelated bulk commit ("feat: expand test coverage and
documentation infrastructure") that touched this manifest along with many
others and left no note about why. This crate's own decline never happened as
a deliberate edit; the edge was dropped by work that was not about this
crate's dependencies at all, and the two prose sites this decision was about
(`src/lib.rs:5`, `readme.md:5`) have now been corrected to match, closing this
document's own G15 staleness. The third prose site,
`task/unverified/102_implement_ring_atomic.md:19`, is left as it was: a
pre-implementation task spec, not a live claim about the crate's current
dependencies — see [`lifecycle/002`](../lifecycle/002_from_task_file_to_crate.md).

---

### AT18 — The One Real Edge Is a Single Newtype, Which Is Why It Is Easy to Miss

`ring_types` is the only dependency the crate uses, and it uses exactly one item
from it: `Seq`, a `u64` newtype. Every occurrence of it in the crate is a wrapper
around a value that came out of an `AtomicU64` or is about to go into one.

**Finding.** The dependency is load-bearing and invisible. Delete it and the crate
becomes `AtomicU64` with a different name; keep it and every value crossing the
crate boundary carries the family's sequence type rather than a bare integer, which
is what lets `ring_batch::claim` return a `BatchClaim` of `Seq` without a cast and
what makes `ring_cursor`'s `PaddedCursor` a drop-in.

The contrast with AT17 is the useful part: the crate declares two dependencies, one
of which does the entire job of making the crate part of a family rather than a
standalone wrapper, and one of which does nothing at all — and the two are
indistinguishable from the manifest, from the readme, from the module comment, and
from the task file. Only the compiler knows, and only `cargo udeps` asks it.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`integration/002`](002_five_crates_downstream.md) | The five edges out, and what each takes |
| [`workaround/001`](../workaround/001_the_loom_seam_and_the_manifest_above_it.md) | The third dependency, which is target-conditional and has its own problems |
| [`type/001`](../type/001_what_the_trait_promises.md) | `Seq` as the type the whole surface is phrased in |
| [`lifecycle/002`](../lifecycle/002_from_task_file_to_crate.md) | The task file whose `Depends on` line is where the extra edge came from |

### Sources

| Fact | Where |
|------|-------|
| The one declared dependency, since the `ring_seqno` edge's removal | `ring_atomic/Cargo.toml` |
| The single import | `ring_atomic/src/lib.rs:70` |
| The historical prose sites (two now corrected, one a pre-implementation spec) | Census above |
| `ring_seqno` unused across all targets, at the time this finding was recorded | `cargo +nightly udeps -p ring_atomic --all-targets`, quoted above |
| Two unused dependencies in 33 crates, at the time this finding was recorded | Same, swept per-crate, quoted above |

### Tests

| Test | Covers |
|------|--------|
| *(to create)* | Nothing checks that a declared dependency is used — one `udeps` invocation, run nowhere in the gate |
| *(to create)* | Nothing checks that the `Depends on` sentence matches the manifest, in either direction |
