# docs

Design documentation for `bench_harness`, as typed doc definitions.

| Directory | Responsibility |
|------|-----------------|
| `acceptance/` | The binary reached-test each graded feature is measured against |
| `definition/` | Module Index — every definition and every instance in this crate, in one place |
| `guide/` | How to reproduce the family's verdicts, what they establish, and what they miss |
| `invariant/` | Measurable constraints this crate's machinery must hold |
| `plan/` | Staged validation plans whose stages this crate's gates grade |
| `workaround/` | External constraints this crate absorbs, with costs and deletion conditions |

Scope of this crate: family-neutral validation machinery — seeded workloads, a
byte-parity oracle, and the stage gates that grade the family's write path.

Implemented. The gates under `gate/` run, and the seeded workload generator,
accumulator semantics, and byte-parity oracle in `src/` were delivered by
task 129 at 100% line coverage across 12 tests.

The remaining doc definitions — `algorithm/`, `data_structure/`, `item/`,
`pattern/`, `pitfall/`, `type/`, and the rest — are not yet written. They are
populated as the machinery's design is worked out; this file is the record of
what exists now, not of what is planned.
