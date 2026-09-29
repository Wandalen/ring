# integration

This crate has no integration. Zero manifests in `/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/` declare it, as a
dependency or a dev-dependency; no source file anywhere imports it; nothing in
the workspace would fail to build if the directory were deleted. That is a
stricter isolation than the corpus has recorded before — `ring_event`, the other
crate here with no consumers, is at least named once under `[dev-dependencies]`.

So what this definition documents is not a set of edges but the shape of their
absence: a five-name vocabulary describing work owned by five crates that have no
route here, one prose mention in a sibling's test header, and a criterion the
crate quotes exactly and attributes to the wrong document. The crate is
well-made, correct, and connected to the family only by paper.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_a_vocabulary_for_crates_that_never_call_it.md) | A Vocabulary for Crates That Never Call It | The declaration and import censuses, and the one place the name appears |
| [002](002_the_criterion_lives_one_document_away.md) | The Criterion Lives One Document Away | Where the acceptance clause actually is, and how three crates cite it |

## Five Names, Five Crates, No Edges

`TraceOp` names `Claim`, `Publish`, `Consume`, `Commit` and `Drop`. Three of
those are crate names in this family — `ring_claim`, `ring_publish`,
`ring_consume` — and the other two are operations owned by `ring_cursor` and
`ring_overflow`. None of the five declares this crate. The vocabulary is a
correct and complete description of a ring's sequence operations, which is
exactly what a trace should name; what nothing says is that it was designed
against a surface that has no call into it yet.

The practical cost is that no compile-time signal watches this crate. Nothing
downstream builds against it, so its own nineteen tests are the entire safety
net, and a change to `TraceOp` that contradicted `ring_overflow`'s policy names
or `ring_cursor`'s commit semantics would compile and pass everywhere.

## A Sentence Quoted Exactly From the Wrong Place

Both the module doc and the test header attribute the crate's acceptance
criterion to an external feature document. The sentence they quote is
verbatim real — and it is at
`bench_harness/docs/acceptance/001_feature_reached_tests.md:53`. The
feature document names `ring_trace` zero times, has no acceptance-criterion or
reached-test section, and is about counters.

Three crates share that criterion. `ring_stats`, which owns the feature, cites
the acceptance file by full path. `ring_debug` and `ring_trace`, ruled into the
feature by a separate orphan-crate pass, both cite the feature document
instead. The chain runs plan → ruling → acceptance file, and the two ruled-in
crates cite the middle step's target rather than the last step's text.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- who declares this crate --'
n=0
for c in */ ; do
  k=$( command grep -c '^ring_trace *=' "$c"Cargo.toml 2>/dev/null || true )
  n=$(( n + k ))
done
printf '    manifests in the crate tree declaring ring_trace: %s\n' "$n"
echo '  -- who imports it --'
command grep -rn 'use ring_trace\|ring_trace::' --include=*.rs */ \
  | command grep -v '^ring_trace/' || echo '    no import anywhere'
echo '  -- the criterion the crate quotes, and the document it names --'
printf '    occurrences of ring_trace in docs/feature/185_ring_stats.md: '
command grep -c 'ring_trace' docs/feature/185_ring_stats.md || true
command grep -n 'records one entry per sequence operation' \
  bench_harness/docs/acceptance/001_feature_reached_tests.md \
  | command grep -o '^[0-9]*' | sed 's/^/    the sentence is at line /'
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TR17 | `ring_trace` | n/a — doc gap | `TraceOp`'s five discriminants each carry a doc line describing work owned somewhere specific — a producer claiming sequences, a producer publishing them, a consumer reading, a cursor committing, a publish refused to the overflow policy — and three of the five names are crate names in this family while the other two belong to `ring_cursor` and `ring_overflow`, yet none of those five crates declares this one and zero manifests in `/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/` declare it at all, as dependency or dev-dependency, which is stricter isolation than `ring_event`'s single dev-dependency edge and means nothing in the workspace would fail to build if the directory were deleted; the vocabulary is not wrong, it is a good and complete description of a ring's sequence operations, but every discriminant is a claim about work this crate cannot observe, and one sentence saying the five names are the surface the trace is *intended* to be called from, that the calls do not yet exist, and which crates would make them, would turn a set of names that read like an integration into a stated design target |
| TR18 | `ring_trace` | n/a — doc gap | The import census is empty — no `use ring_trace`, no `ring_trace::` path anywhere in the workspace outside the crate — and exactly one `.rs` file elsewhere contains the string at all, `ring_debug/tests/debug_test.rs:6`, in a doc comment saying an external feature carries three acceptance criteria of which "the other two belong to `ring_stats` and `ring_trace`", so the single trace of this crate in the family's source is a sibling describing a document rather than an interaction, and that sibling is the crate assigned to the same feature by the same ruling naming this one only to say which parts of a shared criterion are not its own; the concrete cost is that no downstream build watches this crate, its own nineteen tests are the entire safety net, and a change to the `TraceOp` vocabulary contradicting `ring_overflow`'s policy names or `ring_cursor`'s commit semantics would compile and pass everywhere, which nothing states |
| TR19 | `ring_trace` | **wrong doc** | Both `src/lib.rs:7` and `tests/trace_test.rs:3` attribute the crate's acceptance criterion to an external feature document, one saying that document "gives this crate one clause" and the other that the crate claims its "`ring_trace` clause" whose "reached-test reads in part" — and the quoted sentence is verbatim exact but lives at `bench_harness/docs/acceptance/001_feature_reached_tests.md:53`, while the named feature document contains the string `ring_trace` zero times, has no acceptance-criterion or reached-test section (its headings are Definition, If Missing, Hard Problems, Workstreams, Sources) and is entirely about counters; this is a citation defect rather than a fabrication, the clause being real, authoritative and met, but a reader following the citation to check the crate against its criterion arrives at a page about counters with the crate's name nowhere on it, and of the three crates sharing this criterion the one that owns the feature cites the acceptance file by full path while the two ruled into it separately both cite the feature document |
| TR20 | `ring_trace` | n/a — inconsistency | The criterion row covers three crates and carries three clauses, one each, but its Test column holds a single path — `ring_stats/tests/stats_test.rs` — so the row authorising this crate does not name this crate's tests, and `trace_test.rs`, which asserts the clause nineteen ways, is not named by the document it claims; the connection is itself a ruling rather than a derivation, a separate ruling assigning the crate to the feature on the basis that its own one-line description, "Optional sequence-operation trace log", "is the trace half of the same", against a plan that states four crates including this one map to no feature at all — so the crate was placed under that feature by judgement, correctly recorded as judgement, and then cites the feature document as though the mapping had come from it |
