# ring_stats

Ring counters.

Depends on [`ring_types`](../ring_types/readme.md).

One of the 33 `ring_*` crates that make up this family's concurrency
write-path — a 33-crate dependency forest rooted at `ring_types`, acyclic
by construction. Build order follows [`../Cargo.toml`](../Cargo.toml)'s
member list; the family as a whole is described in
[`../readme.md`](../readme.md).

Implemented, delivering features 184 and 185's counters. Behaviour is asserted by
[`tests/stats_test.rs`](tests/stats_test.rs) and read by hand against
[`tests/manual/readme.md`](tests/manual/readme.md); every line is covered.

Design documentation is a full typed corpus — 13 doc definitions, 26 instances,
52 verified findings, each backed by a command whose output is quoted where it is
used. All 52 are about this crate, which no other corpus in the family has
produced: `ring_stats` declares one dependency and two crates declare it, and
neither of those two reaches a counter from a production path, so there is
nothing above or below to find fault in. Twenty-one are reachable, and they
concentrate on the same three of the fourteen methods — `in_flight`,
`dropped_total` and `reset`, the three that touch more than one counter. Start at
[`docs/readme.md`](docs/readme.md) § Where to Start; every finding is indexed and
ranked by severity in [`docs/definition/readme.md`](docs/definition/readme.md).

| File | Responsibility |
|------|-----------------|
| `docs/` | The design corpus — 13 definitions, 26 instances, 52 findings; see [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | Crate root — the crate's whole public surface |
| `tests/` | `stats_test.rs` and the manual plan under `manual/` |
