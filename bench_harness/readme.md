# bench_harness

Family-neutral validation machinery: seeded workloads, a byte-parity oracle,
and stage gates that grade a declared crate family from one runner
(`gate/run_all.sh --family <name>`). Families are declared under
`gate/declared/`, and `gate/declared/family.txt` names the one a bare run
grades.

This crate is a leaf. It depends on nothing else in this workspace, and
specifically on no `ring_*` crate.

That independence is the reason the crate exists. `ring_bench` imports the
crates it measures, so anything hosted there can run only once the family
already builds. Stage gates have to run *before* the work they grade, against
empty skeletons, so they cannot live downstream of it. `bench_harness` is a
grader that predates what it grades, and `ring_bench` grades write-path
candidates. The two are different jobs, and the dependency direction keeps
them apart.

It sits inside this workspace because the `ring_*` family was its first
grading target and is the family declared today. Its grading role is not
limited to that family. Another family is a new directory of declarations
under `gate/declared/`.

`gate/run_all.sh --family <name>` prints the current reading for a family.

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build. See the workspace [verb/readme.md](../verb/readme.md) |
| `docs/decisions/` | Architecture decision records: [001](docs/decisions/001_a_gate_counts_only_after_failing_for_its_own_reason.md) when a gate's REACHED counts as evidence, [002](docs/decisions/002_defects_are_graded_by_mutation_not_coverage.md) what coverage and the other gates do not prove |
| `gate/` | The stage gates, their per-family declarations, and the runner. See [gate/readme.md](gate/readme.md) |
| `src/lib.rs` | Crate root. Re-exports the workload generator, accumulator semantics, and byte-parity oracle |
| `tests/` | The workload and oracle suites |
