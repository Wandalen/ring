# docs

Repository-level documentation that isn't scoped to a single crate — as
opposed to each `ring_*` crate's own `docs/` (`docs/api/`, `docs/algorithm/`,
etc.), which documents that crate's own public contract and internals, or
`bench_harness/docs/`, which documents the gate harness itself.

| Path | Responsibility |
|------|-----------------|
| [`capstone/`](capstone/readme.md) | Rust bootcamp capstone assignment — production-readiness topics, one file per crate |
| [`crates_overview.md`](crates_overview.md) | Family-level crate guide — what each of the 33 `ring_*` crates + `bench_harness` does, with usage + plain-words explanation |
