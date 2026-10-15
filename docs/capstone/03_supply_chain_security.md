# Topic 3: Supply-chain & dependency security

## The gap

No `deny.toml`, no `audit.toml` anywhere in the repo. Nothing currently
checks the workspace's external dependencies (`crossbeam-queue`,
`crossbeam-channel`, `crossbeam-utils`, `criterion`, `loom`, `trybuild`,
etc.) for license compatibility or known security advisories.

## Deliverables

- **`cargo-deny`**, configured for:
  - License policy — everything must stay compatible with the workspace's
    own `license = "MIT"` (`../../Cargo.toml`)
  - Duplicate-version and banned-crate checks
- **`cargo-audit`**, wired into CI ([Topic 1](01_ci_cd.md)) against the
  RUSTSEC advisory database
- A documented policy for what happens when either check goes red — does a
  red `cargo-deny`/`cargo-audit` block a merge outright, or file a tracked
  issue with a grace period? Either is defensible; write down which one
  this project chose and why.

## Why it matters for a capstone

This is the topic that's least glamorous and most directly "production
readiness" in the boring-but-essential sense recruiters and reviewers
actually check for: does the project know what's in its own dependency
tree, and does it have a process for when one of those dependencies turns
out to have a problem.
