# workaround

External constraints `ring_handle` absorbs.

### Scope

- **Purpose**: Record every external constraint this crate compensates for, so each one carries a cost and a deletion condition.
- **Responsibility**: Document this crate's workarounds, or record explicitly that it has none.
- **In Scope**: Constraints originating outside this repository — for a crate whose only dependency is a workspace sibling, that means the language, the toolchain, and the targets.
- **Out of Scope**: This crate's own design decisions, which are not workarounds however unusual they look; constraints compensated in shared tooling, which record their own workarounds in their own crate rather than here.

### Overview

**One, and it is a toolchain constraint rather than a dependency one.**

`ring_handle`'s **runtime** surface depends on a single workspace sibling —
`ring_core` — and on no published crate; W1's own compensation adds one
published `dev-dependency`, `trybuild = "1.0"` (→ HD45). Verify the surface
with:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_handle
cargo tree --depth 1
```

Live output:

```
ring_handle v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_handle)
└── ring_core v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_core)
[dev-dependencies]
├── ring_config v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_config)
├── ring_types v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_types)
└── trybuild v1.0.121
```

| # | Constraint | Compensated by | Cost | Deleted when |
|---|-----------|----------------|------|--------------|
| W1 | **Rust cannot express "this method must not exist" as a bound.** There is no negative trait bound, no `where Self: !Drain`, and no way to assert an absence in the type system itself | A `trybuild` compile-fail suite — a second compilation, driven from a test, comparing against pinned stderr | A test dependency; brittle expected-output files that break on toolchain upgrades; a regeneration command (`TRYBUILD=overwrite`) that is also how a genuine regression gets accepted | Negative bounds or a stable "assert this does not compile" mechanism lands in the language. Not on any roadmap this crate can name |

**W1 is a real workaround and not a design decision, which is the distinction
this directory exists to draw.** The *design* is to enforce by withholding
(→ [`pattern/001`](../pattern/001_enforce_by_withholding.md)); that stands on
its own merits. The *workaround* is that the design's central guarantee has no
first-class way to be asserted, so a whole second compilation pipeline exists
to check something a bound would express in six characters.

**Its cost is not theoretical.** The pinned-stderr brittleness is the mechanism
by which this crate's only real detector decays: a compiler upgrade changes the
message, the expected file is regenerated, and a genuine regression regenerated
at the same moment is indistinguishable
(→ [`non_functional_requirement/001`](../non_functional_requirement/001_proven_by_code_that_must_not_compile.md)'s
measurement 4).

#### One near-miss, recorded so it is not filed here later

The hand-written forwarding methods
(→ [`algorithm/002`](../algorithm/002_delegating_to_the_backend.md)) look like a
workaround for the absence of a "delegate these methods" facility, and they are
not. `Deref` and delegation macros both exist; they are refused because they
would forward *everything*, which is the thing this crate must not do. That is a
design choice with a deliberate cost, and it has no deletion condition — a
better delegation facility would not change it.

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [`../non_functional_requirement/001_proven_by_code_that_must_not_compile.md`](../non_functional_requirement/001_proven_by_code_that_must_not_compile.md) | W1's compensation, stated as this crate's binding acceptance criterion |

### Patterns

| File | Relationship |
|------|--------------|
| [`../pattern/001_enforce_by_withholding.md`](../pattern/001_enforce_by_withholding.md) | The design W1 compensates for the language's inability to assert |

### Sources

| File | Relationship |
|------|--------------|
| `Cargo.toml` | The dependency surface examined for this finding — one runtime path dependency, no published runtime crates, and one published `dev-dependency` (`trybuild`) |
| [`bench_harness/docs/acceptance/001_feature_reached_tests.md`](../../../bench_harness/docs/acceptance/001_feature_reached_tests.md) | This crate's row, which mandates the `trybuild` mechanism W1 describes |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_handle/docs/workaround
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### HD[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| HD[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| HD45 | the dependency claim | **wrong doc** | The workaround readme says this crate depends on no published crate and the manifest declares `trybuild = "1.0"` — the workaround itself is what introduced the dependency the claim denies |
| HD46 | `TRYBUILD=overwrite` | **latent hazard** | The command that regenerates a stale expected-output file and the command that silently accepts a real regression are the same command, and nothing distinguishes the two uses after the fact |
| HD47 | the expected-output files | n/a — observation | One pinned `.stderr` names two crates this one has no dependency edge to, so a rename in either produces a failure in a crate that never mentions them |
| HD48 | `ring_core::ProducerInner` | **latent hazard** | A private `ring_core` type is pinned verbatim in this crate's expected compiler output, making a name no consumer can reach load-bearing in another crate's fixtures |
