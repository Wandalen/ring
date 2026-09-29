# workaround

External constraints `ring_flush` absorbs.

### Scope

- **Purpose**: Record every external constraint this crate compensates for, so each one carries a cost and a deletion condition.
- **Responsibility**: Document this crate's workarounds, or record explicitly that it has none.
- **In Scope**: Constraints originating outside this repository — the language, the toolchain, and Cargo's own compilation model.
- **Out of Scope**: This crate's own design decisions, however unusual they look (→ [`decisions/`](../decisions/readme.md)).

### Overview

**Two, and both are consequences of the same thing: this crate's central
obligations fall on its *caller*, and Rust has no way to state an obligation
that binds a caller.**

The dependency surface is three workspace siblings and no published crate —
confirm before reading further, since a published dependency would be the usual
source of workarounds and there is none:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_flush
cargo tree --depth 1
```

Live output:

```
ring_flush v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_flush)
├── ring_core v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_core)
├── ring_tls v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_tls)
└── ring_types v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_types)
[dev-dependencies]
└── ring_config v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_config)
```

| # | Constraint | Compensated by | Cost | Deleted when |
|---|-----------|----------------|------|--------------|
| W1 | **Rust has no linear types.** A value can always be dropped without any method having been called on it. There is no `where Self: MustCall`, and `#[must_use]` does not help — it fires on a discarded *expression*, not on a bound variable that is later dropped unused | Documentation, a name that reads as an obligation (`drain_final`), and a [`FlushOutcome`](../type/002_flush_outcome.md) that makes a rejection visible enough to act on | **Records staged at drop are lost, silently** (→ [`lifecycle/001`](../lifecycle/001_the_consolidation_cycle.md)'s U5). The one class of data loss this crate cannot detect, report, or prevent | Linear or must-use-on-drop types land in the language. Not on any roadmap this crate can name |
| W2 | **`cfg( test )` does not reach integration tests.** Cargo compiles the crate under test as an ordinary dependency for `tests/*.rs`, with `cfg( test )` off — so a structure gated on it is invisible to exactly the test that needs it | A cargo feature (`flush-log` or similar), enabled by the test profile and off by default (→ [`data_structure/002`](../data_structure/002_the_flush_log.md)) | The acceptance criterion holds only under a non-default feature. A gate that forgets to enable it runs the suite against a build with no log and asserts nothing — **passing** | Never, structurally. It is how Cargo separates unit from integration tests, by design |

**W1 is the crate's sharpest edge and it is genuinely external.** The design
that produces it — no flush on `Drop` — is a decision with reasons
(→ [`pattern/002`](../pattern/002_driven_not_self_firing.md)), and those reasons
would still hold in a language with linear types. What that language would
provide is a compiler error for the consumer who forgets, instead of silent
loss. The workaround is not the design; it is that the design's one hazard has
no mechanical detector.

**W2's cost is worse than it first reads.** A missing feature flag does not
produce a build error or a skipped test — it produces a test that runs, asserts
against an empty log, and passes. That is the failure mode
[`bench_harness`](../../../bench_harness/docs/invariant/001_gate_non_vacuity.md)
exists to catch at gate grain, arriving here at crate grain. Whichever
compilation boundary Pending 2 settles on, the gate invoking this crate's suite
must enable it, and something must assert that the log is capable of recording
before asserting on what it recorded.

#### Two near-misses, recorded so they are not filed here later

**A dev-dependency cycle is not a constraint.** The append-path cost
measurement belongs in `ring_tls`'s suite, which would need a dev-dependency on
this crate — and this crate already depends on `ring_tls`. That reads as a cycle
and Cargo permits it, because dev-dependencies are outside the normal build
graph. Confirm rather than assume:

```sh
cd "$(git rev-parse --show-toplevel)"
# a two-crate scratch workspace: a → b as a dependency, b → a as a dev-dependency
d=$( mktemp -d )
trap 'rm -rf "$d"' EXIT
mkdir -p "$d/a/src" "$d/b/src" "$d/b/tests"
printf '[workspace]\nresolver = "2"\nmembers = ["a", "b"]\n' > "$d/Cargo.toml"
printf '[package]\nname = "a"\nversion = "0.1.0"\nedition = "2024"\n\n[dependencies]\nb = { path = "../b" }\n' > "$d/a/Cargo.toml"
printf 'pub fn f() -> i32 { 42 }\n' > "$d/a/src/lib.rs"
printf '[package]\nname = "b"\nversion = "0.1.0"\nedition = "2024"\n\n[dev-dependencies]\na = { path = "../a" }\n' > "$d/b/Cargo.toml"
printf '// b never uses a; only its dev-dependency does\n' > "$d/b/src/lib.rs"
printf '#[test]\nfn calls_a() { assert_eq!( a::f(), 42 ); }\n' > "$d/b/tests/it.rs"
if ( cd "$d" && cargo check --workspace ) >/dev/null 2>&1
then echo 'cargo check --workspace: succeeds'
else echo 'cargo check --workspace: FAILS'
fi
if ( cd "$d" && cargo test -p b ) >/dev/null 2>&1
then echo 'cargo test -p b: passes'
else echo 'cargo test -p b: FAILS'
fi
```

Live output:

```
cargo check --workspace: succeeds
cargo test -p b: passes
```

The measurement gap in
[`non_functional_requirement/002`](../non_functional_requirement/002_the_decision_costs_nothing_on_the_append_path.md)
is therefore an unwritten test, not a blocked one. **"It would be a cycle" is
the reason such a test does not get written, and it is not true.**

**`OnBarrier`'s blindness is not a workaround either.** The policy cannot
observe a barrier and must be told
(→ [`pitfall/001`](../pitfall/001_on_barrier_cannot_see_the_barrier.md)). That
looks like compensating for a missing capability, and it is not. `ring_barrier`
is not reachable from here — and when it was, four edges away through the
consumer's side of the ring, the policy could not use it either. What is missing
is the barrier *instance* the consumer is gated on and a notification when it
advances; neither is a thing a dependency edge carries, so there is no external
constraint here to absorb. Declining to observe the barrier is a design choice
with a stated cost, which belongs in [`decisions/`](../decisions/readme.md) and
[`pattern/002`](../pattern/002_driven_not_self_firing.md), not here.

### Data Structures

| File | Relationship |
|------|-----------------|
| [`../data_structure/002_the_flush_log.md`](../data_structure/002_the_flush_log.md) | W2's subject; the compilation-boundary table is this constraint worked out |

### Lifecycles

| File | Relationship |
|------|-----------------|
| [`../lifecycle/001_the_consolidation_cycle.md`](../lifecycle/001_the_consolidation_cycle.md) | W1's cost, as U4 and U5 |
| [`../lifecycle/002_from_configuration_to_the_final_drain.md`](../lifecycle/002_from_configuration_to_the_final_drain.md) | W1's compensation — teardown as an explicit phase; its W3 is the obligation that cannot bind |

### Non-Functional Requirements

| File | Relationship |
|------|-----------------|
| [`../non_functional_requirement/002_the_decision_costs_nothing_on_the_append_path.md`](../non_functional_requirement/002_the_decision_costs_nothing_on_the_append_path.md) | The first near-miss — a gap that is unwritten rather than blocked |

### Patterns

| File | Relationship |
|------|-----------------|
| [`../pattern/002_driven_not_self_firing.md`](../pattern/002_driven_not_self_firing.md) | The design W1 compensates for, which stands on its own reasons |

### Sources

| File | Relationship |
|------|-----------------|
| `Cargo.toml` | The dependency surface examined for this finding — three path dependencies, no published crates |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_flush/docs/workaround
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### FL[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| FL[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| FL49 | the `#[must_use]` sites | n/a — unenforced | The outcome type carries the obligation for three driver methods and the builder returns `Self` unguarded, so the W1 failure mode is reachable in one statement |
| FL50 | the drop test | n/a — observation | The loss W1 calls unpreventable is pinned by a passing test, which turns a documented hazard into a contract a well-meaning `Drop` would have to break |
| FL51 | the compensation column | n/a — drift | W2's compensation names a cargo feature the crate never grew; four family manifests carry a `[features]` section and none declares it |
| FL52 | the readme recipe | n/a — coverage | The recipe that settles the dependency count sits one line below the wrong count, fenced so the gate never runs it and never reports it missing |
