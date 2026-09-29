# Integration: The Criterion Lives One Document Away

### Scope

**Purpose:** Record that the acceptance criterion this crate quotes is verbatim
accurate and is not in the document both of the crate's files attribute it to,
locate where it actually lives, and set the crate's citation against the two
others that share the same criterion.

**Responsibility:** The two citations in this crate, the contents of the feature
document they name, the acceptance file that holds the sentence, the ruling that
connected the crate to the feature at all, and how `ring_stats` and `ring_debug`
cite the same thing.

**In Scope:** `ring_trace/src/lib.rs:7-12`;
`ring_trace/tests/trace_test.rs:3-5`;
`bench_harness/docs/acceptance/001_feature_reached_tests.md:53`.

**Out of Scope:** The crate's absence of callers is
[`integration/001`](001_a_vocabulary_for_crates_that_never_call_it.md). Whether
the criterion is met is
[`invariant/001`](../invariant/001_disabled_means_zero_forever.md).

---

## Three Citations of One Sentence

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- what the crate says its criterion is, and where it says it lives --'
command grep -m1 -A2 -F '//! Claims `docs/feature/185_ring_stats.md`. Its' ring_trace/src/lib.rs
command grep -m1 -A2 -F '//! Claims the `ring_trace` clause of `docs/feature/185_ring_stats.md`. Its' ring_trace/tests/trace_test.rs
echo '  -- what that document actually contains --'
printf '    occurrences of ring_trace in docs/feature/185_ring_stats.md: '
command grep -c 'ring_trace' docs/feature/185_ring_stats.md || true
printf '    acceptance-criterion or reached-test sections in it: '
command grep -c 'acceptance criterion\|reached-test' docs/feature/185_ring_stats.md || true
printf '    its section headings: '
command grep -o '^#\+ .*' docs/feature/185_ring_stats.md | tr '\n' ' '
echo
echo '  -- where the sentence actually lives --'
command grep -n 'records one entry per sequence operation' bench_harness/docs/acceptance/001_feature_reached_tests.md | command grep -o '^[0-9]*' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
echo '  -- and how the three crates sharing that criterion cite it --'
command grep -n 'feature/185' ring_stats/tests/stats_test.rs ring_debug/tests/debug_test.rs ring_trace/tests/trace_test.rs ring_trace/src/lib.rs | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
echo '  -- the ruling that put this crate under that feature --'
command grep -n '`ring_trace` | \[185\]' docs/decision/121_workstream_008_contract_gaps_ruled.md | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
```

Live output:

```
  -- what the crate says its criterion is, and where it says it lives --
//! Claims `docs/feature/185_ring_stats.md`. Its acceptance criterion, filed at
//! `bench_harness/docs/acceptance/001_feature_reached_tests.md`, is a
//! pair of numbers: `ring_trace` "records one entry per sequence operation
//! Claims the `ring_trace` clause of `docs/feature/185_ring_stats.md`. Its
//! acceptance criterion, filed at
//! `bench_harness/docs/acceptance/001_feature_reached_tests.md`, reads
  -- what that document actually contains --
    occurrences of ring_trace in docs/feature/185_ring_stats.md: 0
    acceptance-criterion or reached-test sections in it: 0
    its section headings: # Feature: Ring Stats ## Definition ## If Missing ### Hard Problems ### Workstreams ### Sources 
  -- where the sentence actually lives --
273
  -- and how the three crates sharing that criterion cite it --
ring_stats/tests/stats_test.rs://! Claims `docs/feature/185_ring_stats.md`. Its acceptance criterion, filed at
ring_debug/tests/debug_test.rs://! `docs/feature/185_ring_stats.md`'s acceptance criterion is "`ring_debug`'s
ring_trace/tests/trace_test.rs://! Claims the `ring_trace` clause of `docs/feature/185_ring_stats.md`. Its
ring_trace/src/lib.rs://! Claims `docs/feature/185_ring_stats.md`. Its acceptance criterion, filed at
  -- the ruling that put this crate under that feature --
| `ring_trace` | [185](../feature/185_ring_stats.md) | "Optional sequence-operation trace log" is the trace half of the same |
```

---

### TR19 — The Quotation Is Exact and the Address Is Wrong

Both of this crate's files attribute its acceptance criterion to an external
feature document. `src/lib.rs:7` says that document "gives this
crate one clause"; `trace_test.rs:3` says the crate "claims the `ring_trace`
clause" of it, "whose reached-test reads in part" — and both then quote the same
sentence.

The sentence is exact. It reads, verbatim, "`ring_trace` records one entry per
sequence operation when enabled and zero when not", and it is at
`bench_harness/docs/acceptance/001_feature_reached_tests.md:53`. It is not
in the feature document. That file contains the string `ring_trace` zero times,
has no acceptance-criterion or reached-test section — its headings are
Definition, If Missing, Hard Problems, Workstreams, Sources — and its Definition
is entirely about counters: items claimed, published, dropped and nanoseconds
spent waiting.

**Finding.** This is a citation defect and not a fabrication, which is the useful
distinction: the clause exists, is authoritative, and is met. But two files claim
a named document contains a clause and a reached-test that it does not contain,
and a reader who follows the citation to check the crate against its criterion
finds a page about counters with the crate's name nowhere on it. The fix is the
path, in both files: the criterion is filed at
`bench_harness/docs/acceptance/001_feature_reached_tests.md:53`, which is
the address `ring_stats` already uses — "Its acceptance criterion, filed at
`bench_harness/docs/acceptance/001_feature_reached_tests.md`". Of the
three crates sharing this criterion, the one that owns the feature cites the
criterion's real home and the two ruled into it separately cite the feature
document instead.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A2 -F '//! Claims `docs/feature/185_ring_stats.md`' ring_trace/src/lib.rs
```

Live output:

```
//! Claims `docs/feature/185_ring_stats.md`. Its acceptance criterion, filed at
//! `bench_harness/docs/acceptance/001_feature_reached_tests.md`, is a
//! pair of numbers: `ring_trace` "records one entry per sequence operation
```

**Disposition:** applied — both citations now name the criterion's real address
instead of the feature document. `src/lib.rs`'s module doc and
`trace_test.rs`'s header both now read "Claims `docs/feature/185_ring_stats.md`.
Its acceptance criterion, filed at
`bench_harness/docs/acceptance/001_feature_reached_tests.md`, ...",
matching the citation form `ring_stats` already uses. `cargo test --release -p
ring_trace --doc` confirms 9/9 doctests still pass. Now prints: `Its acceptance criterion, filed at`

---

### TR20 — The Criterion Names One Test File for Three Crates

The criterion row covers three crates — `ring_stats`, `ring_debug`, `ring_trace` —
and carries three clauses, one per crate. Its final column, which names the test
that claims it, holds a single path: `ring_stats/tests/stats_test.rs`. So the
row that authorises this crate's existence does not point at this crate's tests,
and `trace_test.rs`, which asserts the clause nineteen ways, is not named by the
document it claims.

The connection between crate and feature is itself a ruling rather than a
derivation. A separate ruling assigns `ring_trace` to that feature with the
basis "'Optional sequence-operation trace log' is the trace half of the same" —
that is, the crate's own one-line description, matched against the feature by
subject. An earlier plan states the premise plainly: four crates,
this one among them, map to no feature at all. So the crate was placed under
that feature by judgement, correctly recorded as a judgement, and the crate
then cites the feature document as though the mapping had come from it.

**Finding.** The citation defect in TR19 has a cause worth writing down beside
it: the crate is reading its authorisation from the wrong end of a three-step
chain — plan says unmapped, a ruling assigns it to the feature, acceptance file
states the clause — and cites the middle step's target instead of the last
step's text. Two repairs, both one line. The criterion row's test column should
name all three test files rather than one, so a reader arriving from the
criterion reaches the tests that claim it. And the crate's two citations should
point at the acceptance file and, ideally, at the ruling that assigned it here
for why a trace crate answers to a feature about counters at all.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`integration/001`](001_a_vocabulary_for_crates_that_never_call_it.md) | The other half of the crate's place in the family |
| [`invariant/001`](../invariant/001_disabled_means_zero_forever.md) | The criterion itself, met |
| [`lifecycle/002`](../lifecycle/002_a_finished_crate_in_the_unverified_stage.md) | The other document about this crate that is out of date |
| [`workaround/002`](../workaround/002_a_third_dependency_two_documents_still_name.md) | The third documentary claim that the code does not support |

### Sources

| Fact | Where |
|------|-------|
| The module doc's attribution | `ring_trace/src/lib.rs:7-12` |
| The test header's attribution | `ring_trace/tests/trace_test.rs:3-5` |
| Zero mentions and no criterion section in the feature | Census above |
| The criterion's real address | `bench_harness/docs/acceptance/001_feature_reached_tests.md:53` |
| `ring_stats` citing that address by full path | `ring_stats/tests/stats_test.rs:3` |

### Tests

| Test | Covers |
|------|--------|
| `an_enabled_trace_records_exactly_one_entry_per_operation` | The criterion's first half |
| `a_disabled_trace_records_zero` | Its second half |
| `a_disabled_trace_records_zero_of_every_operation_kind` | The second half, exhaustively |
