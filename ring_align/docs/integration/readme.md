# Integration Doc Definition

### Scope

- **Purpose**: Document how this crate joins the 33-crate family — one declared dependency it does not use, one declared consumer, and a name that reached three crates the dependency graph never touched.
- **Responsibility**: Give the measured edges in both directions, the discrepancies between declared and actual, and the reason the constant sits in a crate of its own.
- **In Scope**: `ring_align`'s inbound and outbound edges; the placement argument.
- **Out of Scope**: The family-wide ownership restriction, which is [`invariant/002`](../invariant/002_one_constant_for_the_whole_family.md); the reach of the *name*, which is [`api/001`](../api/001_the_reading_surface.md) § The Name Travelled Further Than the Function.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [One Dependency, One Consumer](001_one_dependency_one_consumer.md) | Both edges measured — and the outbound one is declared but unused, which nothing in the workspace currently checks | 🔄 |
| 002 | [Why the Constant Lives Here](002_why_the_constant_lives_here.md) | The argument for a separate crate when a single consumer could have held the constant itself | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- declared edges, both directions --'
command grep 'ring_align' ring_*/Cargo.toml
echo '  -- the outbound edge, referenced by no .rs file in the crate --'
cat ring_align/src/*.rs ring_align/tests/*.rs | sed 's://.*::' | command grep -c 'ring_types'
echo '  -- control: the same measure in the crate downstream --'
cat ring_cursor/src/*.rs ring_cursor/tests/*.rs | sed 's://.*::' | command grep -c 'ring_types'
```

Live output:

```
  -- declared edges, both directions --
ring_align/Cargo.toml:name = "ring_align"
ring_cursor/Cargo.toml:ring_align = { path = "../ring_align" }
  -- the outbound edge, referenced by no .rs file in the crate --
0
  -- control: the same measure in the crate downstream --
3
```

The `sed` strips comments before counting, which is not cosmetic: the crate's own
module doc contains the sentence "Depends on `ring_types`", so a count taken over
raw text reports 1 and the finding below disappears.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| AL21 | `ring_align → ring_types` | n/a — observation | Declared in `Cargo.toml` and named by no `.rs` file once comments are stripped, against 2 references in `ring_cursor` measured the same way. Two crates in the family carry an unused `ring_types` edge — this one and `ring_registry` — and nothing in the workspace fails on either |
| AL22 | `src/lib.rs:5` | **wrong doc** | The module doc's "Depends on `ring_types`" is the only thing in the crate that mentions the dependency, so the single place the edge is described is also the reason a naive grep concludes it is used |
| AL23 | The inbound edge | n/a — observation | One declared consumer, `ring_cursor` — so a crate justified as shared infrastructure currently serves one caller, and the argument for its existence is about the *shape* of the dependency graph rather than about reuse that has happened |
| AL24 | `on_distinct_lines` | n/a — duplication | Four crates carry the name and neither `ring_mpsc` nor `ring_spsc` declares `ring_align`: each defines its own method of that name, delegating to `CursorPair::on_distinct_lines`, which calls this crate's free function. The capability travels two hops by delegation while the Cargo graph shows one edge — and the shared spelling means a grep for callers over-reports by three |
