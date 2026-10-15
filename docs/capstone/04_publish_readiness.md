# Topic 4: crates.io publish readiness & release process

## The gap

Every one of the 34 crates is already `version = "0.1.0"` with `publish =
true` set (see `../../ring_core/Cargo.toml`) — this family is *meant* to
ship to crates.io — but nothing currently verifies it actually can, and
there's no release process at all.

## The five crates that matter most here

Per the root [`../../readme.md`](../../readme.md), only 5 of the 33
`ring_*` crates are meant to be depended on from outside the family:
`ring_types`, `ring_handle`, `ring_tls`, `ring_flush`, `ring_factory`. These
are where a metadata gap or an accidental breaking change actually costs
something — prioritize them.

Separately, by dependency fan-in, three *internal* crates are worth extra
semver care even though outside consumers never import them directly,
because so much of the rest of the family sits on top of them: `ring_types`
(31 of 32 other crates depend on it directly), `ring_config` (11
dependents), and `ring_cursor` (10 dependents). A breaking change to any of
these three ripples through nearly the whole workspace in one step.

## Deliverables

- `cargo publish --dry-run` passing for all 34 crates, in dependency order
  (the root readme's mermaid graph gives you that order already — start at
  Tier 0, finish at `ring_bench`)
- An audit of per-crate `Cargo.toml` metadata gaps (`keywords`,
  `categories`, `documentation`) for crates.io discoverability
- A documented Minimum Supported Rust Version (MSRV) policy, tested in CI
- `cargo-semver-checks` wired in so a 0.1.x → 0.1.(x+1) bump can't silently
  break a public API — run it at minimum against the 5 external-facing
  crates and the 3 high-fan-in internal ones named above
- A `CHANGELOG` convention (per-crate, or workspace-wide — pick one and
  justify it)
