# docs

Repository-level documentation that isn't scoped to a single crate. Each
`ring_*` crate keeps its architectural decisions in its own `docs/decisions/`,
and `bench_harness/docs/` documents `bench_harness` itself.

| Path | Responsibility |
|------|-----------------|
| [`capstone/`](capstone/readme.md) | Rust bootcamp capstone assignment. Production-readiness topics, one file per topic |
| [`crates_overview.md`](crates_overview.md) | Family-level crate guide. What each crate does, with usage and a plain-words explanation |

## Where a crate's design knowledge lives

Write each fact once, on the item that implements it. Every other place links to that item
instead of restating it.

- Behaviour a caller programs against goes in the item's rustdoc, with `# Errors`, `# Panics` and
  `# Safety` where they apply.
- An invariant, an algorithm choice, a lifecycle rule or a trap goes in the same rustdoc, under
  one of four headings kept from the per-crate doc corpus this repository used to carry. The
  heading takes the level the surrounding documentation uses.
  - `# Invariant: <what holds>` states the invariant, then what it deliberately excludes under
    **Excluded.**, then the test or assertion that checks it under **Enforced by.**
  - `# Algorithm: <what it computes>` gives the procedure where the code does not make it
    obvious, and why it is done this way rather than the way a reader would try first.
  - `# Lifecycle: <phase>` gives the phases, the transitions between them, and what cleanup each
    one owes.
  - `# Pitfall: <the trap>` is written as **Trap.**, **Failure.** and **Mitigation.**
- A section that spans several items goes in the module documentation (`//!`) instead.
- A decision with rejected alternatives and a revisit trigger goes in the crate's
  `docs/decisions/` as an ADR.

These sections name items as intra-doc links, such as ``[`Guarded::try_push_batch`]``, so a
rename fails CI's `cargo doc` run, which denies warnings. Tests, and items in crates the crate
does not depend on, are named in backticks, and nothing checks them. The sections never quote line
numbers, counts or command output, because the next refactor makes those wrong without anything
failing.
