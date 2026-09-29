# item

Two things are visible only at the declaration level, and both are about guards:
what the crate's lint reaches and what it cannot, and which of two claimed
compile-time tripwires is real. The crate is unusually deliberate about both — it
sets `deny( missing_docs )` rather than inheriting `warn`, and it writes an
exhaustive `match` in place of a derive with an explicit sentence saying why.

The deliberateness is why the gaps are worth recording. A crate that never
thought about compile-time enforcement would not have a comment describing a
guard that does not exist; a crate careless about documentation would not have
exactly one undocumented thing, and it would not be the output format.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_six_impl_blocks_and_the_three_a_lint_cannot_reach.md) | Six Impl Blocks and the Three a Lint Cannot Reach | `deny( missing_docs )`, trait impls, and the format nothing states |
| [002](002_traceop_all_and_the_tripwire_that_is_not_one.md) | `TraceOp::ALL` and the Tripwire That Is Not One | Two claimed compile-time guards, one of them demonstrated absent |

## A Lint Set Thirty-Three Times

The workspace lints table already carries `missing_docs = "warn"` and
`ring_trace`'s manifest opts in with `[lints] workspace = true`. The crate then
escalates to `deny` with an inner attribute — and so does every other `ring_*`
crate, all 33 of them, identically. A rule adopted unanimously is a workspace
default that has not been promoted; one word in the table deletes 33 lines.

What the lint buys is complete for what it can see: every public item in the
three inherent impl blocks is documented. What it cannot see is items inside
trait impls, and one of those is `TraceEntry`'s `Display` — the crate's only
rendering of a log entry, `"{} {}..{}"`, written down nowhere.

## One Real Tripwire and One That Now Says So

`name()` says it is an exhaustive `match` rather than a derive "so that adding a
discriminant fails to compile here", and that is exactly true. The suite once
claimed the same of its own match — that it made "adding a discriminant without
adding it to ALL fail to compile" — and that was false: adding a variant,
satisfying both matches because the compiler insists, and leaving `ALL` at five
compiles and passes both of the test's assertions, neither of which can fail in
any case, one being `5 == 5` and the other calling a function whose single arm
returns `true`.

TR27 applied the correction. The comment now names the real guard chain —
`name()`'s match forces the edit one compile error earlier, and keeping `ALL` in
step is a manual step this test's exhaustiveness only redundantly copies — so
the recipe below reads the current wording, not the claim it replaced.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the lint, set locally and in the workspace --'
command grep -n 'deny( missing_docs )' ring_trace/src/lib.rs | sed 's/^/    /'
printf '    workspace table: '
command grep 'missing_docs' Cargo.toml | sed 's/^ *//'
h=0
for c in ring_*/; do
  if command grep -q 'deny( missing_docs )' "$c"src/lib.rs 2>/dev/null; then h=$(( h + 1 )); fi
done
printf '    ring_* crates escalating it in source: %s of 33\n' "$h"
echo '  -- impl blocks, and the format inside one of them --'
t=$( command grep -c '^impl .* for ' ring_trace/src/lib.rs || true )
printf '    trait impls: %s   inherent impls: %s\n' "$t" \
  "$(( $( command grep -c '^impl' ring_trace/src/lib.rs || true ) - t ))"
command grep -m1 -F '    write!( f, "{} {}..{}", self.op, self.seq.0, self.end().0 )' ring_trace/src/lib.rs
echo '  -- the two tripwire claims --'
command grep -m1 -A1 -F '  /// Written as an exhaustive `match` rather than a derive so that adding a' ring_trace/src/lib.rs
command grep -m1 -A1 -F '  // ALL must stay in step with the enum, but nothing here forces that: the' ring_trace/tests/trace_test.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TR25 | `ring_trace` | n/a — doc gap | `#![ deny( missing_docs ) ]` at line 53 covers every public item in the crate's three inherent impl blocks with no exception, but `missing_docs` does not apply to items inside a trait impl, and three of the six impl blocks are trait impls — of which one documents its method anyway, `Default::default` carrying "Disabled — the state a ring that was never asked to trace must be in.", while both `Display` impls carry nothing; that blind spot is standard and correct in general and specific here, because it contains `TraceEntry`'s `write!( f, "{} {}..{}", self.op, self.seq.0, self.end().0 )` — the crate's only rendering of a log entry, the format anything printing a trace produces, the shape a human parsing a dump must know — and it is written down neither at the impl, nor at `TraceEntry`, nor in the module doc, so a reader learns it from the `write!`; two lines at the impl giving the format string and noting that `end` is exclusive so a zero-`count` entry prints `claim 7..7` cost nothing and are unreachable by any lint the crate can set |
| TR26 | `ring_trace` | n/a — duplication | The workspace lints table sets `missing_docs = "warn"` and this crate's manifest opts in with `[lints] workspace = true`, after which the crate escalates the same lint to `deny` with an inner attribute that overrides the level the manifest passed — and all 33 `ring_*` crates do exactly this, not most but every one, with the identical line in the identical position, so the family is unanimous that `deny` is the wanted level and expresses that unanimity 33 times in source rather than once in the table already carrying the lint; the current arrangement works and has the minor merit that the level is visible in the file it governs, but it costs a line per crate to say what the table exists to say once and leaves a 34th crate inheriting `warn` unless someone remembers the ritual, where `missing_docs = "deny"` in `[workspace.lints.rust]` would make all 33 attributes deletable |
| TR27 | `ring_trace` | **misleading doc** | `name()`'s doc claims that writing an exhaustive `match` rather than a derive means "adding a discriminant fails to compile here, where a human then has to say what the new operation is called", which is exactly true — but `trace_test.rs`'s own comment claimed its exhaustive match was written "so adding a discriminant without adding it to ALL fails to compile here" — text this finding's fix has since replaced, at `:190-195` — and that was false for the case it named: the match forces an edit when a discriminant is added, which `name()` already forced one compile error earlier, and has nothing to say about `ALL`; a probe replicating `TraceOp`'s exact shape with a sixth discriminant added to the enum and to both matches — everywhere the compiler demanded — but not to `ALL` compiled and passed both of the suite's assertions, with the enum declaring six and `ALL` listing five, and the consequence is not academic since `ALL` is what `an_enabled_trace_records_one_of_every_operation_kind` and `a_disabled_trace_records_zero_of_every_operation_kind` iterate, so a discriminant missing from it silently stops being covered and its `count_of` is never exercised again; no stable check closes this — no `variant_count` outside nightly, no derive in the dependency set — so naming the manual step is the whole available remedy and is worth more than a comment claiming it is automatic |
| TR28 | `ring_trace` | n/a — coverage | `covered` has a single match arm returning `true` and no arm returning `false`, so `assert!( covered( op ) )` cannot fail for any value that reaches it, and `assert_eq!( TraceOp::ALL.len(), 5 )` compares the length of a `[ Self; 5 ]` against the literal `5`, which the type already fixed — both assertions in `the_operation_kinds_are_exactly_the_five_declared` are tautologies, leaving the test's entire content as `covered`'s compile-time exhaustiveness, which duplicates `name()`'s and therefore fires second, since a new discriminant breaks `src/lib.rs` before the test is ever built; the test contributes nothing the source does not already have at either compile time or run time, while its name promises exactly the property TR27 shows nothing checks, and if the manual `ALL` step is to stay manual this test is the natural place to say so, being where a maintainer looking for that guarantee arrives |
