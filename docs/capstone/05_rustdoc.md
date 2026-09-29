# Topic 5: Public API docs — docs.rs-grade rustdoc

## The gap

This project has an unusually rich *internal* doc corpus — every crate
carries its own `docs/api/`, `docs/algorithm/`, `docs/decisions/`,
`docs/invariant/`, `docs/pitfall/`, and more. But that's design-rationale
documentation for contributors, not the `///` rustdoc that actually renders
on docs.rs for a crates.io consumer. `cargo doc --no-deps` currently builds
clean across all 34 crates, which is a good floor, not a ceiling — a clean
build says nothing about whether the public items are actually documented.

## Deliverables

- `#![warn(missing_docs)]` (or `#![deny(missing_docs)]`, if the team's
  ambitious) added across all 34 crates
- Every public item gets a doc comment; the ones that benefit from it (the
  5 external-facing crates especially — `ring_types`, `ring_handle`,
  `ring_tls`, `ring_flush`, `ring_factory`) get a runnable example
- A polished top-level `lib.rs` doc comment per crate that *links back* to
  the richer internal `docs/` corpus instead of duplicating it — the two
  are complementary, not competing: rustdoc for "how do I call this,"
  `docs/decisions/` for "why does it work this way"

## Suggested priority order

1. `ring_factory` — the highest-level assembly point; most consumers call
   it first, so its docs should show "how do I actually build a ring"
   end to end
2. The other 4 external-facing crates
3. The 3 crates with the widest internal fan-in (`ring_types`,
   `ring_config`, `ring_cursor`) — not externally consumed, but everything
   downstream inherits whatever's unclear here
4. Everything else, tier by tier
