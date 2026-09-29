# Non-Functional Requirement: "Zero When Not" Is a Count, Not a Cost

### Scope

**Purpose:** Separate the requirement the acceptance criterion imposes from the
one the crate reads into it, record that the crate held itself to the harder
standard, and establish that nothing in the workspace measures whether it met it.

**Responsibility:** The criterion's wording, the module doc's reading of it, what
the three disabled tests assert, the absence of anything time-bearing in the
crate, and whether the family's measurement crate knows this one exists.

**In Scope:**
`bench_harness/docs/acceptance/001_feature_reached_tests.md:53`;
`ring_trace/src/lib.rs:8-12`; the three disabled tests in
`ring_trace/tests/trace_test.rs`; `ring_bench/`.

**Out of Scope:** The measured number is
[`algorithm/001`](../algorithm/001_the_early_return_that_is_the_whole_feature.md).
Where the criterion actually lives is
[`integration/002`](../integration/002_the_criterion_lives_one_document_away.md).
The `std` requirement is
[`non_functional_requirement/002`](002_the_one_crate_that_genuinely_needs_std.md).

---

## A Counting Requirement, Read as a Timing One

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- what the criterion asks for --'
command grep -o 'ring_trace` records one entry per sequence operation when enabled and zero when not' bench_harness/docs/acceptance/001_feature_reached_tests.md | sed 's/^/    /'
echo '  -- and what the crate reads it as --'
command grep -m1 -A4 -F '//! pair of numbers: `ring_trace` "records one entry per sequence operation' ring_trace/src/lib.rs
echo '  -- what each disabled test asserts, body only --'
for t in a_disabled_trace_records_zero a_disabled_trace_records_zero_of_every_operation_kind a_disabled_trace_stays_empty_under_contention; do
  printf '    %s\n' "$t"
  awk -v f="fn $t()" 'index( $0, f ) { g = 1 } g && /^}/ { g = 0 } g' ring_trace/tests/trace_test.rs \
    | command grep -o 'assert[a-z_]*!( [^;]*' | sed 's/^/      /'
done
printf '    time-bearing items anywhere in the crate: %s\n' \
  "$( command grep -rl 'Instant\|Duration\|nanos' --include=*.rs ring_trace/ 2>/dev/null | wc -l )"
echo '  -- and whether the family measurement crate knows this one exists --'
printf '    ring_bench code and manifest mentioning ring_trace: %s\n' \
  "$( command grep -rl 'ring_trace' ring_bench/src ring_bench/tests ring_bench/examples ring_bench/Cargo.toml 2>/dev/null | wc -l )"
printf '    ring_bench documents mentioning ring_trace:        %s\n' \
  "$( command grep -rl 'ring_trace' ring_bench/docs 2>/dev/null | sed 's|ring_bench/||' | tr '\n' ' ' )"
```

Live output:

```
  -- what the criterion asks for --
    ring_trace` records one entry per sequence operation when enabled and zero when not
  -- and what the crate reads it as --
//! pair of numbers: `ring_trace` "records one entry per sequence operation
//! when enabled and **zero when not**." Both halves are assertions, and the
//! second is the harder one — a trace that costs something when disabled is a
//! trace nobody leaves compiled in, and the family's whole output is a measured
//! comparison that an always-on trace would distort.
  -- what each disabled test asserts, body only --
    a_disabled_trace_records_zero
      assert_eq!( trace.len(), 0, "disabled means zero, not few" )
      assert!( trace.is_empty() )
      assert!( trace.entries().is_empty() )
    a_disabled_trace_records_zero_of_every_operation_kind
      assert_eq!( trace.count_of( op ), 0, "{op} was recorded despite the trace being off" )
      assert_eq!( trace.len(), 0 )
    a_disabled_trace_stays_empty_under_contention
      assert_eq!( trace.len(), 0, "8000 calls, zero entries" )
    time-bearing items anywhere in the crate: 1
  -- and whether the family measurement crate knows this one exists --
    ring_bench code and manifest mentioning ring_trace: 2
    ring_bench documents mentioning ring_trace:        docs/integration/002_the_only_consumer_of_two_contract_names.md 
```

---

### TR33 — The Crate Volunteered a Requirement Its Criterion Does Not Impose

The criterion is a counting statement: `ring_trace` "records one entry per
sequence operation when enabled and zero when not". Both halves are about the
number of entries, and both are met — the three disabled tests assert `len() ==
0`, `is_empty()`, `entries().is_empty()` and `count_of( op ) == 0` for all five
kinds, including under four-way contention with 8,000 calls.

The module doc reads the second half differently. It calls it "the harder one"
and explains why in cost terms: "a trace that costs something when disabled is a
trace nobody leaves compiled in, and the family's whole output is a measured
comparison that an always-on trace would distort". That reading is a performance
requirement, and it is a good one — genuinely harder than counting, and exactly
the right concern for a diagnostic in a benchmark family.

It is also entirely self-imposed. The criterion says nothing about time, and the
crate is graded on entries. Having set itself the higher bar, the crate then
measures nothing against it: there is no `Instant`, no `Duration`, no `nanos`
anywhere in the crate — zero files carry any of them, across source, tests and
manual plan alike.

**Finding.** Recorded as a self-imposed non-functional requirement with full
functional coverage and no measurement at all. The functional side is not the
gap; three tests cover it thoroughly. What is missing is that the crate stated a
timing claim in the same paragraph as the counting one, in language that reads as
though both were imposed, and only one of the two is checked by anything. The
number exists —
[`algorithm/001`](../algorithm/001_the_early_return_that_is_the_whole_feature.md)
measures the disabled call at 2.75 ns against a 0.68 ns floor, all of it
un-inlinable cross-crate call — and it is not what the paragraph implies.

---

### TR34 — The Family's Measurement Crate Does Not Know This Crate Exists

`ring_bench` is the family's harness: it is the crate that owns `Instant`,
`VecDeque`, `Mutex` and the scoped threads that drive the comparisons, and the
"measured comparison" the module doc worries about distorting is its output. No
executable file under `ring_bench/` mentions `ring_trace` — not the
manifest, not the source, not the tests. One *document* does: its
`integration/002`, which counts the consumers of each Contract name and notes in
passing that this crate has none. A citation in a corpus is not an edge in a
build.

So the crate that argued its disabled path must be free, because a benchmark
would otherwise be distorted, is invisible to the benchmark. The distortion it
guards against cannot occur, because there is no configuration in which the
harness has a trace to switch on; and the freedom it claims cannot be confirmed,
because the one crate equipped to time anything has no edge to it.

**Finding.** The pair completes TR33: the requirement is self-imposed, unmeasured
by this crate, and unmeasurable by the only crate that could. That is not a defect
in either — `ring_bench` has no reason to depend on a crate nobody calls, and this
crate has no reason to import a harness. It is a coverage hole with a specific
shape, and the specific remedy is one bench case in `ring_bench` driving a ring
with the trace disabled against the same ring with no trace at all, which would
turn the crate's central argument from a claim into a number and simultaneously
give `ring_trace` its first caller.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`algorithm/001`](../algorithm/001_the_early_return_that_is_the_whole_feature.md) | The number the requirement lacks |
| [`integration/002`](../integration/002_the_criterion_lives_one_document_away.md) | Where the criterion is actually filed |
| [`integration/001`](../integration/001_a_vocabulary_for_crates_that_never_call_it.md) | The absent caller a bench case would supply |
| [`non_functional_requirement/002`](002_the_one_crate_that_genuinely_needs_std.md) | The other requirement nothing states |

### Sources

| Fact | Where |
|------|-------|
| The criterion's counting wording | `bench_harness/docs/acceptance/001_feature_reached_tests.md:53` |
| The crate's cost reading of it | `ring_trace/src/lib.rs:8-12` |
| The three disabled tests' assertions | Census above |
| Nothing time-bearing in the crate | Census above |
| `ring_bench` never naming the crate | Census above |

### Tests

| Test | Covers |
|------|--------|
| `a_disabled_trace_records_zero` | The counting requirement, 50 calls |
| `a_disabled_trace_records_zero_of_every_operation_kind` | The same across all five kinds |
| `a_disabled_trace_stays_empty_under_contention` | The same at 8,000 calls on four threads |
