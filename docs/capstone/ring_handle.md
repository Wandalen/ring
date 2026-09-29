# `ring_handle` — capstone notes

**Role:** Shareable producer and consumer ends
**Tier:** 7/10
**Depends on:** `ring_core`, `ring_config`, `ring_types`
**Depended on by:** `ring_factory`, `ring_registry`
**Unsafe:** forbidden (default) — not on the allowlist, despite wrapping the allowlisted `ring_core`
**no_std:** not yet audited
**loom:** not covered
**External API surface:** yes — one of the 5 crates meant for outside consumption

**This is where the one currently-known failing test lives:**
`tests/ui/producer_shared_across_threads.rs` —
`ring_handle::ui_test::the_forbidden_programs_are_rejected` — a `trybuild`
compile-fail test whose recorded expected-stderr likely embeds a
workspace-relative path that shifted when the crate moved out of the
monorepo. Whoever picks this crate up (or Topic 8) should start here.

## Most relevant topics

- **Topic 8 (extraction debt):** the trybuild fix belongs to this crate
  specifically — see above. Don't just re-bless the snapshot; read what the
  test is actually proving (that `Producer`/`Consumer` correctly reject
  being shared across threads in ways that would break the single-producer
  or single-consumer invariant) and confirm the *new* diagnostic still
  proves the same thing before overwriting it.
- **Topic 5 (rustdoc):** external-facing, and the most likely crate an
  outside consumer actually imports first — prioritize its public docs.
- **Topic 4 (semver):** external-facing — include in the
  `cargo-semver-checks` rollout; the `ui_test` above is itself a form of
  semver contract (what *shouldn't* compile) worth keeping in sync with any
  API change.
