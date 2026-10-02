# bench_harness

Family-neutral validation machinery: seeded workloads, a byte-parity oracle,
and stage gates that grade several crate families from one runner
(`gate/run_all.sh --family <name>`; declared families live under
`gate/declared/`).

This crate is a leaf. It depends on nothing else in this workspace, and
specifically on no `ring_*` crate.

That independence is the crate's whole point. `ring_bench` sits at tier 12 of
the ring family's dependency forest and imports six of the crates it measures,
so anything hosted there can only run once the family already builds. Stage
gates have to run *before* the work they grade, against empty skeletons, on
day one. So they cannot live downstream of it. `bench_harness` is therefore a
grader that predates what it grades, and `ring_bench` is the grader of
write-path candidates specifically. The two are different jobs, and the
dependency direction keeps them apart.

It sits inside this workspace because the `ring_*` family was its first
grading target and remains one of its declared families. Its grading role is
not limited to this family. See `gate/declared/` for the rest.

Implemented. The baseline the gates were built against, taken on 2026-08-28
when the family was still 33 skeletons, read 0/6. Both family and gate count
have grown since. The ring family now declares 14 gates
(`gate/declared/ring/gates.txt`) and orbital 11
(`gate/declared/orbital/gates.txt`). For both, `run_all.sh --family <name>`
gives the current, authoritative reached-count. Re-run it rather than trusting
a number restated here. This paragraph has already gone stale twice: a
"7/7"-for-ring figure predating gates g12–g21, and a "12 tests" count
predating T13/T14. The seeded workload
generator and byte-parity oracle in `src/` are covered by 14 tests (6 in
`tests/workload_test.rs`, 8 in `tests/oracle_test.rs`, `cargo nextest run`
confirms).

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build. See the workspace [verb/readme.md](../verb/readme.md) |
| `docs/` | Scope, invariants, and open trade-offs. See [docs/readme.md](docs/readme.md) |
| `gate/` | The stage gates, their per-family declarations, and the runner. See [gate/readme.md](gate/readme.md) |
| `src/lib.rs` | Crate root. Re-exports the workload generator, accumulator semantics, and byte-parity oracle |
