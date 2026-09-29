# workaround

External constraints `ring_factory` absorbs.

### Scope

- **Purpose**: Record every external constraint this crate compensates for, so each one carries a cost and a deletion condition.
- **Responsibility**: Document this crate's workarounds, or record explicitly that it has none.
- **In Scope**: Constraints originating outside this repository — the language, the toolchain, and Cargo's own visibility model.
- **Out of Scope**: This crate's own open trade-offs, however awkward (→ [`decisions/`](../decisions/readme.md)).

### Overview

**Two, and both are cases where the thing this crate is *for* has no expression
in the language.** It exists to turn a runtime value into one of two types, and
to be the only door into a family — and Rust can state neither.

The dependency surface is five workspace siblings and no published crate —
confirm before reading further, since a published dependency would be the usual
source of workarounds and there is none:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_factory
cargo tree --depth 1
```

Live output:

```
ring_factory v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_factory)
├── ring_config v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_config)
├── ring_core v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_core)
├── ring_handle v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_handle)
├── ring_registry v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_registry)
└── ring_types v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_types)
```

| # | Constraint | Compensated by | Cost | Deleted when |
|---|-----------|----------------|------|--------------|
| W1 | **A function cannot return one of two unrelated types.** `ring_spsc::Ring< S >` and `ring_mpsc::Ring< S >` share no trait — every `impl` on either is from `core` — and `impl Trait` in return position needs one concrete type, not a runtime choice between two | An enum with two variants, matched per operation (→ [`algorithm/001`](../algorithm/001_selecting_a_backend_from_one_boolean.md)'s A1) | **One branch per ring operation, forever.** It predicts perfectly — the discriminant never changes after construction — but it is in the publish path this family exists to measure, and it is there because of a decision made once at build time | Rust gains anonymous sum types, or the two backends acquire a shared trait. The second is in this repository's power and is a real option; the first is not on any roadmap |
| W2 | **Cargo has no "may only be depended on by" visibility.** `pub( crate )` stops at the crate edge, `#[ doc( hidden ) ]` hides without preventing, and there is no manifest key expressing "internal to this family" | A declared text file plus a shell gate outside the language: `export_surface.txt` and `g5_export_surface.sh` (→ [`integration/002`](../integration/002_the_crate_the_export_surface_routes_through.md)) | **Crate granularity only, and it runs only when invoked.** The gate can forbid depending on `ring_spsc`; it cannot forbid calling `Ring::with_config` once `ring_core` is a legal dependency. Every routing guarantee this crate makes inherits that limit | Cargo gains per-consumer visibility, or the family collapses into one crate with real `pub( crate )` boundaries. The second is a design the family deliberately rejected |

**W1 is the sharper of the two because its cost is on the measured path.**
Everything else in the family works to keep atomics and allocations out of a
publish; W1 puts a predictable branch back in, and it is unavoidable given a
runtime-selected backend. The two escapes both cost more: a trait object
replaces the branch with an indirect call (worse), and making the backend a
compile-time parameter takes the selection away from `RingConfig` and defeats
the crate's purpose (→ [`pattern/001`](../pattern/001_configuration_as_data.md)).

**W1's deletion condition is unusually reachable.** A shared trait over the two
backends is a change this repository can make, and it would replace the enum
with a generic — moving the branch to monomorphisation time. That is worth
noting as a real option rather than a hypothetical, and it is not filed as a
pending decision because nothing currently forces the choice: the enum is
correct, and the trait is an optimisation nobody has measured a need for.

**W2's cost is the one that will be misread.** A passing G5 currently means
nothing at all — no crate outside the family depends on any `ring_*` crate, so
the gate has nothing to check
(→ [`bench_harness`](../../../bench_harness/docs/invariant/001_gate_non_vacuity.md)).
A gate that passes vacuously and a gate that passes meaningfully are
indistinguishable from the exit code, which is the same failure shape
[`ring_flush`](../../../ring_flush/docs/workaround/readme.md) records for its
own feature-gated flush log: a check that runs, asserts against nothing, and
reports success.

#### Two near-misses, recorded so they are not filed here later

**`RingConfig` being unnameable by a compliant consumer is not a workaround.**
`build`'s only argument is a type outside the Contract's five
(→ [`api/001`](../api/001_the_build_surface.md)'s guarantee 3). That reads like
a language or tooling constraint and is not one — nothing prevents adding
`ring_config` to the surface, moving the type into `ring_types`, or re-exporting
it. It is an unmade decision with three available answers, and it belongs in
[`decisions/`](../decisions/readme.md) as Pending 4.

**`ring_wait` being outside this crate's closure is not one either.** The `wait`
field names a strategy this crate cannot construct
(→ [`pitfall/002`](../pitfall/002_a_wait_strategy_it_can_read_and_cannot_honour.md)).
Adding the dependency is one manifest line and is available today — what is
missing is a ruling on whether the factory or the backend resolves the strategy.
An absent edge that anyone may add is a design question, not an external
constraint; recorded as Pending 5.

**Both near-misses share a tell worth carrying to the other crates:** if the
compensation is "add a line to a file in this repository", it is a decision. A
workaround is what remains when no line anyone here can write would fix it.

### Algorithms

| File | Relationship |
|------|-----------------|
| [`../algorithm/001_selecting_a_backend_from_one_boolean.md`](../algorithm/001_selecting_a_backend_from_one_boolean.md) | W1's subject; A1–A4 are the four responses and the reasons three lose |

### Integrations

| File | Relationship |
|------|-----------------|
| [`../integration/002_the_crate_the_export_surface_routes_through.md`](../integration/002_the_crate_the_export_surface_routes_through.md) | W2's mechanism, and X2 — the call-site granularity it cannot reach |

### Invariants

| File | Relationship |
|------|-----------------|
| [`../invariant/002_construction_is_the_only_path.md`](../invariant/002_construction_is_the_only_path.md) | What W2's granularity limit costs — four public constructors the gate cannot see |

### Non-Functional Requirements

| File | Relationship |
|------|-----------------|
| [`../non_functional_requirement/002_construction_cost_is_paid_once.md`](../non_functional_requirement/002_construction_cost_is_paid_once.md) | W1's cost, as P1 and the requirement it strains |

### Workarounds

| File | Relationship |
|------|-----------------|
| [`../../../ring_flush/docs/workaround/readme.md`](../../../ring_flush/docs/workaround/readme.md) | W2's cost shape — a check that passes against nothing — recorded there as W2 of that crate |

### Sources

| File | Relationship |
|------|-----------------|
| `Cargo.toml` | The dependency surface examined for this finding — five path dependencies, no published crates |
| [`bench_harness/gate/g5_export_surface.sh`](../../../bench_harness/gate/g5_export_surface.sh) | W2's compensation |
| [`bench_harness/gate/declared/ring/export_surface.txt`](../../../bench_harness/gate/declared/ring/export_surface.txt) | W2's declaration half |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_factory/docs/workaround
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### FC[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| FC[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| FC49 | `ring_factory` | n/a — observation | `RegistryError::NameTaken` is the family's one tuple-payload error, carrying the refused value back to its caller, and this crate — its only caller — drops the payload with `_refused` |
| FC50 | `ring_factory` | n/a — observation | The name is preserved by the callee and discarded by the door: `ring_registry` returns the rejected `String` and `BuildError::NameTaken` carries nothing, so a caller who wants it back must have kept it |
| FC51 | `ring_factory` | n/a — observation | The invariant is preserved by making it not apply — the crossbeam feature is declared in four manifests, names a dependency in exactly one, and is enabled nowhere in the workspace |
| FC52 | `ring_factory` | n/a — unenforced | The deletion condition is reachable and nothing is watching for it: `build_crossbeam` folds away the moment backend selection becomes a config value, and no pending, task or gate records that |
