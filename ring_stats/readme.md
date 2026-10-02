# ring_stats

Ring counters.

Depends on [`ring_types`](../ring_types/readme.md).

One of the 33 `ring_*` crates that make up this family's concurrency
write-path. The 33 crates form a dependency forest rooted at `ring_types`,
acyclic by construction. Build order follows [`../Cargo.toml`](../Cargo.toml)'s
member list. [`../readme.md`](../readme.md) describes the family as a whole.

Implemented. It delivers the counters of features 184 and 185.
[`tests/stats_test.rs`](tests/stats_test.rs) asserts the behaviour, which is also
read by hand against [`tests/manual/readme.md`](tests/manual/readme.md). Every
line is covered.

The design documentation is a full typed corpus: 13 doc definitions, 26
instances and 52 verified findings. Each finding is backed by a command whose
output is quoted where it is used. All 52 are about this crate, which no other
corpus in the family has produced. `ring_stats` declares one dependency and two
crates declare it, and neither of those two reaches a counter from a production
path, so there is nothing above or below to find fault in. Twenty-one findings
are reachable, and they concentrate on the same three of the fourteen methods:
`in_flight`, `dropped_total` and `reset`, the three that touch more than one
counter. Start at [`docs/readme.md`](docs/readme.md) § Where to Start.
[`docs/definition/readme.md`](docs/definition/readme.md) indexes every finding and
ranks it by severity.

| File | Responsibility |
|------|-----------------|
| `verb/` | Crate-scoped test/lint/build. See [verb/readme.md](verb/readme.md) |
| `docs/` | The design corpus: 13 definitions, 26 instances, 52 findings. See [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | Crate root, holding the crate's whole public API |
| `tests/` | `stats_test.rs` and the manual plan under `manual/` |
