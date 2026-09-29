# Integration: One Dependency, One Consumer

### Scope

- **Purpose**: Measure both of this crate's dependency edges against what the crate says about itself, and record that the outbound edge is declared but never used.
- **Responsibility**: Give the edges with the commands that regenerate them, name the discrepancy, and say what would catch it.
- **In Scope**: `ring_align → ring_types`; `ring_cursor → ring_align`; the prose mention in `ring_flush`.
- **Out of Scope**: Why the constant is placed here at all, which is [`integration/002`](002_why_the_constant_lives_here.md); the name's spread beyond the graph, which is [`api/001`](../api/001_the_reading_surface.md).

### The Edges

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/\[dependencies\]/,/^\[/p' ring_align/Cargo.toml   # outbound
# `ring/Cargo.toml` is an untracked, in-progress workspace-restructuring
# manifest that also names `ring_align` as a member path — excluded so this
# stays a dependency census rather than a hit on that unrelated artifact.
grep -rl 'ring_align' --include=Cargo.toml . \
  | command grep -v '^ring/Cargo\.toml$'                       # inbound
```

Live output:

```
[dependencies]

[lints]
ring_align/Cargo.toml
ring_cursor/Cargo.toml
```

| Direction | Edge | Declared | Used in code |
|-----------|------|:--------:|:------------:|
| Outbound | `ring_align` → `ring_types` | **no (removed)** | n/a |
| Inbound | `ring_cursor` → `ring_align` | yes | yes — `use ring_align::{ on_distinct_lines, CacheAligned };` |
| Inbound | every other one of the 32 `ring_*` crates | no | no |

**One inbound edge in a 33-crate family.** That is the whole consumer list, and
it is the fact [`integration/002`](002_why_the_constant_lives_here.md) has to
justify: a crate whose entire audience is one other crate.

### The Outbound Edge Is Not Used

**Correction (2026-09-28):** the Outbound row above used to read `yes` /
`**no**` — declared but unused — which this whole section argues out below.
Commit `ce60ae6e8` ("Remove unused dependencies from Cargo.toml files")
deleted the `ring_types = { path = "../ring_types" }` line from
[`Cargo.toml`](../../Cargo.toml) entirely, so there is no longer an edge to
be unused. What follows is kept as the historical record of the finding that
removal resolved — every present-tense "is declared" below describes the
manifest as it stood when this section was written, not as it stands now.

`ring_types` was declared and never referenced:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- .rs mentions with rustdoc comments stripped --'
# one line survives, and it is a plain `//` implementation comment naming a
# `ring_types` path it does not call. `//` is not `///`, so the filter below
# does not reach it and this is not the empty result an earlier caption here
# promised — see the paragraph under this block.
command grep -r 'ring_types' --include=*.rs ring_align/ \
  | command grep -vE ': *(///|//!)' | LC_ALL=C sort
echo '  -- the control: every non-docs mention in the crate --'
# `command grep` bypasses the shell's ugrep shim, whose hit order varies run
# to run; `sort` pins the rest. Line numbers are deliberately not printed —
# they drift on every edit to the files being cited.
command grep -r 'ring_types' ring_align/ --exclude-dir=docs | LC_ALL=C sort
```

Live output:

```
  -- .rs mentions with rustdoc comments stripped --
ring_align/src/lib.rs:// AL1). `ring_types::Capacity::new` asserts the analogous precondition for
  -- the control: every non-docs mention in the crate --
ring_align/readme.md:Depends on [`ring_types`](../ring_types/readme.md).
ring_align/readme.md:write-path — a 33-crate dependency forest rooted at `ring_types`, acyclic
ring_align/src/lib.rs:// AL1). `ring_types::Capacity::new` asserts the analogous precondition for
ring_align/src/lib.rs://! Depends on `ring_types`.
ring_align/task/unverified/101_implement_ring_align.md:**Depends on:** `ring_types`
ring_align/task/unverified/101_implement_ring_align.md:matching the module doc's claim. The declared `ring_types` dependency is
```

**The filters are load-bearing, and both captions above them were wrong.** The
`.rs` search was captioned `# no output` and has never produced none. An earlier
revision ran it unfiltered and returned the `//!` module-doc line; the rustdoc
filter was added and the caption kept — and one line still stands through the
filter, `src/lib.rs`'s plain `//` comment naming `ring_types::Capacity::new`,
because `//` is not `///`. The control was captioned `# 4 hits, all prose` and
ran over the whole crate, returning 39 until `--exclude-dir=docs` was added,
then 5, then 6 once the task file gained an implementation record. Neither
caption was ever wrong about the *fact* — no code uses `ring_types` — and both
were wrong as published output, which is the failure this pair exists to
prevent. Both now describe what their command does rather than what its result
was expected to be.

**Correction (2026-09-28):** this paragraph and the table below previously
counted six hits across four files, one of them `Cargo.toml`'s own
`ring_types = { path = "../ring_types" }` line. That line is gone (see the
Correction above), and `readme.md` has since grown a second `ring_types`
mention of its own — "a 33-crate dependency forest rooted at `ring_types`,
acyclic by construction" — a claim about the *family's* shape rather than
this crate's own manifest. The count is unchanged at six; the file it
dropped from and the file it landed an extra hit in are the same file, so
three files carry all six now, not four.

Six hits across three files. Three are *direct claims that the dependency
exists*, one more implies it without naming this crate specifically, and the
other two are not claims at all:

| Location | Kind | Text |
|----------|------|------|
| `src/lib.rs` | module doc comment | `//! Depends on ring_types.` |
| `readme.md` | prose assertion | "Depends on `ring_types`." — as a markdown link to the sibling crate |
| `readme.md` | architecture prose | "a 33-crate dependency forest rooted at `ring_types`" — about the family's shape, not this crate's manifest |
| `task/unverified/101_…` | task specification | `**Depends on:** ring_types` |
| `src/lib.rs` | implementation comment | `// AL1). ring_types::Capacity::new asserts the analogous precondition…` — a cross-reference to a sibling's behaviour, not a call |
| `task/unverified/101_…` | implementation record | "The declared `ring_types` dependency is unused by this crate's own code" — written afterwards, citing this document |

The last row is the finding's own paper trail landing inside the sweep that
produced the finding: recording it here added a line to the crate that the
recipe measuring it counts. Four to six, with no new claim made.

A Rust crate cannot use a dependency without naming it in source — either
`use ring_types::…` or a `ring_types::` path. Zero occurrences *outside
comments* across every `.rs` file in the crate, `src/` and `tests/` alike,
settles it. Neither comment hit is an exception. `//!` and `//` text alike is
stripped before the compiler ever resolves a path, so a `ring_types::Capacity`
written inside one can no more constitute a use than the `readme.md` line can —
which is exactly why the `.rs` search above is not the empty result its old
caption promised, and is still not evidence of a use.

**This is a real finding and a small one.** The cost is a build-order edge that
does not need to exist and a documentation claim that is false in three places.
It is not a correctness problem: nothing behaves differently.

**What is interesting is that four artifacts agree with each other and all four
are wrong.** The task file said the crate would depend on `ring_types`, the
`Cargo.toml` was written to match the task file, and both doc comments were
written to match the `Cargo.toml`. The implementation then did not need it. No
step in that chain re-checked the step before it against the code, and the
consistency of the four is what makes the error invisible to a reader.

**Recorded, not fixed.** Removing the line is a change to `Cargo.toml` plus
three prose corrections, which belongs to this crate's own change with its own
verification run rather than to a documentation pass.

### What Would Catch It

| # | Check | Status |
|---|-------|--------|
| H1 | `cargo +nightly udeps --all-targets --all-features` | Exists as **Level 4** of the workspace verification ladder — it is not run at Level 3, which is what ordinary work uses |
| H2 | The compiler | Never. An unused dependency is not a warning; `unused_crate_dependencies` is an allow-by-default lint the workspace does not enable |
| H3 | A test | Cannot. There is no observable behaviour to assert on |
| H4 | Reading the readme | Actively counterproductive — the readme confirms the wrong answer |

**H1 is the only one, and it sits one rung above where verification normally
stops.** That is the general shape of this finding rather than a fact about
this crate: a defect whose sole detector is a tool nobody runs during ordinary
work is a defect that accumulates silently across a 33-crate family.

### The Fifth Mention

A workspace-wide grep also finds `ring_align` in `ring_flush`:

```
ring_flush/tests/append_cost_test.rs:10://! `ring_slot`, `ring_align` — each justifying it in its own
```

That is prose in a doc comment naming sibling crates, not a dependency —
`ring_flush`'s `Cargo.toml` declares `ring_tls`, `ring_core`, `ring_types`, and
dev-dependency `ring_config`, and none of them is this crate.

**Worth recording because it is the shape of a false positive.** A grep for a
crate name across a workspace returns dependency edges, doc-comment mentions,
and readme links indistinguishably; only the `Cargo.toml` manifest answers the
dependency question. Any future gate written against H1's shape needs to read
manifests, not source text.

### AL21 — The Declared Dependency Is Named by No Source File

```
  -- the outbound edge, referenced by no .rs file in the crate --
0
  -- control: the same measure in the crate downstream --
2
```

`ring_align → ring_types` was declared in `Cargo.toml` and, once comments
were stripped, appeared in no `.rs` file in the crate. `ring_cursor`,
measured identically, names it twice.

**Correction (2026-09-28):** commit `ce60ae6e8` removed the declaration
itself (see the Correction under § The Edges, above), so this finding now
describes a resolved, historical state rather than the current manifest —
there is no longer an outbound edge for the count above to be counting, and
the "two crates" in the Finding below is now one: `ring_registry` alone
still carries an unused `ring_types` edge.

**Finding.** Two crates in the family carry an unused `ring_types` edge — this
one and `ring_registry` — and nothing in the workspace fails on either.
`cargo +nightly udeps` runs only at verification level 4, so the family's
routine gates never see it.

---

### AL22 — The Only Place the Edge Is Described Is What Hides It

The crate's module doc at `src/lib.rs:5` says "Depends on `ring_types`." That
sentence is the crate's single mention of the dependency, and it is a comment.

**Finding.** A count taken over raw text reports 1 and AL21 disappears; the
`sed` that strips comments is what makes the measurement true. So the one
statement describing the edge is simultaneously the reason a naive check
concludes the edge is used — a doc that is both the description and the
camouflage.

**Disposition:** declined — removing the unused `ring_types` edge means
editing `Cargo.toml` plus three prose sites (`src/lib.rs:5`, `readme.md:5`,
`task/unverified/101_implement_ring_align.md:19`); this instance's own §
"The Outbound Edge Is Not Used" already rules that out as this crate's own
change with its own verification run, not a corpus disposition pass.

**Correction (2026-09-28):** the `Cargo.toml` half of that has since
happened — commit `ce60ae6e8` dropped the dependency line, unprompted by any
disposition here, as part of a repository-wide "remove unused dependencies"
sweep. The three prose sites named above have not: `src/lib.rs:5`,
`readme.md:5`, and the task file still say "Depends on `ring_types`" as of
this writing — this AL22 finding itself is one of the three, still standing
exactly as described above. The declined disposition holds for what remains.

---

### AL24 — The Capability Travels Two Hops While the Cargo Graph Shows One

Four crates carry the name `on_distinct_lines` and only one of them declares
`ring_align`. `ring_mpsc/src/lib.rs:860` and `ring_spsc/src/lib.rs:362` each
define their *own* method of that name; the `ring_spsc` one calls
`self.cursors.on_distinct_lines()`, which is `CursorPair`'s, which calls this
crate's free function.

**Finding.** The capability reaches two more crates by delegation while the
dependency graph shows a single edge — a healthy shape, since it means the two
top-tier crates depend on the abstraction rather than on the constant. The cost
is that the shared spelling makes a grep for "callers of `on_distinct_lines`"
over-report by three, which is exactly the measurement anybody assessing this
crate's reach would run first.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_reading_surface.md](../api/001_the_reading_surface.md) | Four crates carry the predicate's name against this one inbound edge — the reach that is not a dependency |

### Integrations

| File | Relationship |
|------|--------------|
| [002_why_the_constant_lives_here.md](002_why_the_constant_lives_here.md) | Why one consumer is enough to justify a crate |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_one_constant_for_the_whole_family.md](../invariant/002_one_constant_for_the_whole_family.md) | Q2 — "only one crate needs the number" is this edge count, and it is why the duplication count is one |

### Non Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_the_crate_costs_nothing_at_runtime.md](../non_functional_requirement/002_the_crate_costs_nothing_at_runtime.md) | The unused edge costs build time and nothing else, which is why it is a finding rather than a defect |

### Sources

| File | Relationship |
|------|--------------|
| `ring_align/Cargo.toml` | The declaration measured above |
| `ring_cursor/Cargo.toml` | The one inbound edge |
| `ring_align/task/unverified/101_implement_ring_align.md` | Where the `ring_types` claim originates |
| [`../../../readme.md`](../../../readme.md) | The 33-crate family the edge count is measured against |

### Tests

| File | Relationship |
|------|--------------|
| `tests/align_test.rs` | Imports only `ring_align` itself — the test file is part of the evidence that `ring_types` is unused |
| `tests/manual/readme.md` | No manual check covers dependency hygiene; H1 is the mechanism and it is not wired to anything |
