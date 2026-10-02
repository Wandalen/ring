# docs

Repository-level documentation that isn't scoped to a single crate. Each
`ring_*` crate's own `docs/` (`docs/api/`, `docs/algorithm/`, etc.) documents
that crate's own public contract and internals, and `bench_harness/docs/`
documents `bench_harness` itself.

| Path | Responsibility |
|------|-----------------|
| [`capstone/`](capstone/readme.md) | Rust bootcamp capstone assignment. Production-readiness topics, one file per crate |
| [`crates_overview.md`](crates_overview.md) | Family-level crate guide. What each of the 33 `ring_*` crates + `bench_harness` does, with usage + plain-words explanation |
