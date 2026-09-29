# Integration: A Vocabulary for Crates That Never Call It

### Scope

**Purpose:** Record that no manifest declares this crate and no source imports
it, establish that its five operation names describe work owned by crates that
have no edge to it, and locate the three places in the workspace's source where
the crate's name appears at all — every one of them a comment.

**Responsibility:** The declaration census across every manifest in the
workspace, the import census across every `.rs` file, the mapping from
`TraceOp`'s discriminants to the crates that perform those operations, and the
three prose mentions.

**In Scope:** `ring_trace/src/lib.rs:59-82`; every `Cargo.toml` and
`.rs` file under `any crate root`, `ring/`, `the spike root` and `the substrate root`;
`ring_bench/tests/bench_test.rs:1441`, `ring_bench/src/lib.rs:1007`
and `ring_debug/tests/debug_test.rs:6`.

**Out of Scope:** The document that assigns this crate its criterion is
[`integration/002`](002_the_criterion_lives_one_document_away.md). The
undeclared dependency the crate's own readme claims is
[`workaround/002`](../workaround/002_a_third_dependency_two_documents_still_name.md).

---

## Nothing In, Nothing Out

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the five operations the trace names --'
command grep 'pub const ALL' -A 8 ring_trace/src/lib.rs | command grep 'Self::'
echo '  -- the crate that owns each of them, and whether it declares ring_trace --'
for op in claim publish consume commit drop; do
  owner=$( ls -d ring_$op 2>/dev/null | head -1 )
  if [ -z "$owner" ]; then owner="(no ring_$op crate)"; d="-"; else
    d=$( command grep -c 'ring_trace' "$owner"/Cargo.toml 2>/dev/null || true ); fi
  printf '    %-9s %-22s declares ring_trace: %s\n' "$op" "$owner" "$d"
done
echo '  -- who declares this crate at all --'
n=0
# every crate is a top-level directory here, so a bare `*/` glob already reaches
# all of them in a single pass — nothing is missed by depth
for c in */ ; do
  k=$( command grep -c '^ring_trace *=' "$c"Cargo.toml 2>/dev/null || true )
  n=$(( n + k ))
done
printf '    manifests declaring ring_trace, every root: %s\n' "$n"
echo '  -- and every mention of it in the workspace source, outside the crate --'
# `grep -r` with no path argument already recurses from the repository root, so
# one search covers every crate in a single pass — no per-root list is needed
command grep -r 'ring_trace' --include=*.rs . | command grep -v '^ring_trace/'
command grep -r 'use ring_trace\|ring_trace::' --include=*.rs . | command grep -v '^ring_trace/' || echo '    no import anywhere'
```

Live output:

```
  -- the five operations the trace names --
    Self::Claim,
    Self::Publish,
    Self::Consume,
    Self::Commit,
    Self::Drop,
  -- the crate that owns each of them, and whether it declares ring_trace --
    claim     ring_claim        declares ring_trace: 0
    publish   ring_publish      declares ring_trace: 0
    consume   ring_consume      declares ring_trace: 0
    commit    (no ring_commit crate) declares ring_trace: -
    drop      (no ring_drop crate)   declares ring_trace: -
  -- who declares this crate at all --
    manifests declaring ring_trace, every root: 0
  -- and every mention of it in the workspace source, outside the crate --
ring_bench/tests/bench_test.rs:/// `ring_trace::Trace::entries_guard` already uses for its own shared log.
ring_bench/src/lib.rs:  // poisoning to protect, the same reasoning `ring_trace::Trace::entries_guard`
ring_debug/tests/debug_test.rs://! `ring_trace`. Every corruption here is performed through
ring_bench/tests/bench_test.rs:/// `ring_trace::Trace::entries_guard` already uses for its own shared log.
ring_bench/src/lib.rs:  // poisoning to protect, the same reasoning `ring_trace::Trace::entries_guard`
```

---

### TR17 — Five Names for Five Operations, Owned by Crates With No Edge Here

`TraceOp` declares `Claim`, `Publish`, `Consume`, `Commit` and `Drop`, and each
carries a doc line describing work that happens somewhere specific: a producer
taking ownership of sequences, a producer making them visible, a consumer
reading, a consumer's cursor advancing, a publish refused to the overflow policy.
Three of those names are crate names — `ring_claim`, `ring_publish`,
`ring_consume` — and none of the three declares this crate. The other two,
`Commit` and `Drop`, have no eponymous crate; the operations belong to
`ring_cursor` and `ring_overflow`, which likewise do not.

Zero manifests anywhere in the workspace declare `ring_trace`, as a dependency
or a dev-dependency — the census walks all four crate roots, `any crate root` (with
`any crate root` under it), `ring/`, `the spike root` and `the substrate root`, not the
`any crate root`-and-`ring/` pair the family occupied when this was first measured.
That is a stricter isolation than the corpus has found before:
`ring_event`, the other crate in this family with no consumers, is at least
declared once under `[dev-dependencies]`. This crate is declared nowhere at all,
so nothing in the workspace would fail to build if the directory were deleted.

**Finding.** The vocabulary is not wrong — it is a good and complete description
of a ring's sequence operations, which is exactly what a trace of a ring should
name. What is unrecorded is that it was designed against crates that have no
route to it, so every discriminant is a claim about work this crate cannot
observe. The remedy is not code: it is one sentence in the module documentation
saying that the five operations name the surface the trace is *intended* to be
called from, that the calls do not yet exist, and which crates would make them —
which turns a set of names that look like an integration into a stated design
target.

---

### TR18 — The Crate Appears in the Workspace's Source Only Inside Other Crates' Comments

The import census is empty in the sense that matters: no `use ring_trace`, and no
`ring_trace::` path in executable position, anywhere in the workspace outside the
crate itself. Three `.rs` files elsewhere name it at all, and all three name it in
a comment:

| Where | What it says |
|-------|--------------|
| `ring_bench/tests/bench_test.rs:1441` | A doc comment citing `ring_trace::Trace::entries_guard` as the precedent for a shared log |
| `ring_bench/src/lib.rs:1007` | A line comment citing the same `entries_guard` reasoning about poisoning |
| `ring_debug/tests/debug_test.rs:6` | A module header explaining that an external feature carries three acceptance criteria and that "the other two belong to `ring_stats` and `ring_trace`" |

Two of the three do carry a `ring_trace::` path — but inside a comment, where it
names a precedent rather than calls one. The third names no path at all: it is a
sibling's test header describing a document rather than an interaction, and that
sibling is the crate assigned to the same feature by the same ruling, naming this
one only to say which parts of a shared criterion are not its own.

**Finding.** Worth recording because it is the exact measure of the crate's
integration: not low, but zero. Every trace of it in the corpus is prose — two
citations of one method as a design precedent, and one cross-reference between two
documents — and prose does not link. It also means the family has no compile-time
signal that would notice if this crate broke — no downstream build depends on it,
so the twenty tests in `tests/trace_test.rs` are the entire safety net, and a
change to the `TraceOp` vocabulary that contradicted `ring_overflow`'s policy
names or `ring_cursor`'s commit semantics would compile and pass everywhere. That
is the concrete cost of the isolation, and nothing states it.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`integration/002`](002_the_criterion_lives_one_document_away.md) | The document that put the crate in the family at all |
| [`lifecycle/002`](../lifecycle/002_a_finished_crate_in_the_unverified_stage.md) | The task file that still calls the work unstarted |
| [`workaround/002`](../workaround/002_a_third_dependency_two_documents_still_name.md) | The edge the crate's own documents claim to have |
| [`type/001`](../type/001_five_discriminants_and_the_array_beside_them.md) | The vocabulary itself, as a type |

### Sources

| Fact | Where |
|------|-------|
| The five discriminants | `ring_trace/src/lib.rs:72-81` |
| Zero declaring manifests | Census above |
| Zero imports anywhere, in executable position | Census above |
| The three comment mentions | `ring_bench/tests/bench_test.rs:1441`, `ring_bench/src/lib.rs:1007`, `ring_debug/tests/debug_test.rs:6` |
| `ring_claim`, `ring_publish`, `ring_consume` without the edge | Census above |

### Tests

| Test | Covers |
|------|--------|
| `the_operation_kinds_are_exactly_the_five_declared` | The vocabulary, asserted against itself |
| `every_operation_kind_has_its_own_name` | That the five names are distinct |
| `an_enabled_trace_records_one_of_every_operation_kind` | All five exercised, by the crate's own suite only |
