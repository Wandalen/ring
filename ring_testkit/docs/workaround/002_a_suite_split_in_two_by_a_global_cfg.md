# Workaround: A Suite Split In Two By A Global Cfg

### Scope

- **Purpose**: Record why this crate's tests live in two files that can never be compiled together, what the split buys, and what it costs to have half a suite behind a flag no automated command sets.
- **Responsibility**: The constraint, the two file-level attributes it forces, the placement decision recorded in the manifest, and how each half actually gets run.
- **In Scope**: `tests/testkit_test.rs`'s `#![ cfg( not( loom ) ) ]`, `tests/exhaustive_test.rs`'s `#![ cfg( loom ) ]`, and the `[target.'cfg(loom)'.dev-dependencies]` section.
- **Out of Scope**: The allocations the same constraint forces (→ [`001`](001_two_allocations_loom_cannot_avoid.md)); what the models assert once they run (→ [`../integration/002`](../integration/002_the_edge_that_only_exists_under_a_cfg.md)).

### The constraint

`--cfg loom` is a compiler flag, so it applies to a whole build. Under it,
`ring_atomic` swaps `core::sync::atomic` for `loom::sync::atomic`, whose atomics
panic when touched outside a `loom::model` closure. A test that constructs a ring
and drives it directly is therefore not merely uninteresting under the cfg — it
aborts.

Five `ring_*` crates declare a `cfg(loom)` target section, and `ring_atomic` is
the odd one: its section is `.dependencies`, not `.dev-dependencies`. The other
four — `ring_mpsc`, `ring_publish`, `ring_spsc` and this crate — pull loom in for
their own tests. `ring_atomic` changes what the *library* is built against, which
is why the flag reaches every crate above it whether or not that crate mentions
loom at all.

There is no per-test escape. The only granularity the language offers is a `cfg`
attribute, so the suite is partitioned at the file level:

| File | Opening attribute | Compiled when |
|---|---|---|
| `tests/testkit_test.rs` | `#![ cfg( not( loom ) ) ]` | the cfg is **absent** |
| `tests/exhaustive_test.rs` | `#![ cfg( loom ) ]` | the cfg is **present** |

Exactly one of the two exists in any given build. There is no configuration in
which both are compiled, so there is no single command that runs the whole suite.

### The placement decision, and how it is read

The manifest records the reasoning on the `loom` dev-dependency:

> The model lives in `tests/`, not `src/`, so the scripted fixture stays
> measurable by a coverage run that does not set the cfg.

That is correct and it is worth having: models inside `src/` would put
loom-shaped code in the crate's own coverage denominator, and the fixture's
measured coverage would drop for a reason unrelated to how well it is tested.

The same sentence is also the concession. A coverage run that does not set the
cfg does not measure the loom half *at all* — it does not compile it → TK54.

### How each half actually runs

| Half | Command | Who issues it |
|---|---|---|
| Scripted | `cargo nextest run --all-features`, via `verb/test` | every automated verification |
| Exhaustive | `RUSTFLAGS="--cfg loom" cargo test -p ring_testkit --test exhaustive_test` | a human, following a written note |

`verb/test` sets `RUSTFLAGS="-D warnings"` and `RUSTDOCFLAGS="-D warnings"`, and
sets the loom cfg nowhere. The command in the second row is written down six
times across four files and issued by no script → TK53.

### Deletion condition

| Condition | Whose change | What it removes |
|---|---|---|
| loom stops substituting at the crate level | loom's | the split itself — one file, one build |
| a `verb/` target runs the suite under the cfg | this repository's | not the split, only TK53 |

The two are unequal and the second is the reachable one. Nothing about the file
split is wrong — it is the only shape the language offers for a whole-build
substitution, and it would survive a scoped-thread API in loom untouched
(unlike [`001`](001_two_allocations_loom_cannot_avoid.md), which that change
deletes outright). What is missing is not a better split but a second command,
and adding one costs a script line rather than a design.

### Evidence

| # | Claim | Test |
|---|---|---|
| X1 | The scripted half is excluded under the cfg | `tests/testkit_test.rs` line 1 |
| X2 | The exhaustive half is excluded without it | `tests/exhaustive_test.rs` line 30 |
| X3 | The loom dependency is declared only for that target | `Cargo.toml`, `[target.'cfg(loom)'.dev-dependencies]` |
| X4 | Both halves compile clean under their own cfg | `tests/manual/readme.md` M6 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit
printf 'the scripted half opens with:  %s\n' "$( command grep -m1 -oE '#!\[ cfg\( not\( loom \) \) \]' tests/testkit_test.rs )"
printf 'the exhaustive half opens with: %s\n' "$( command grep -m1 -oE '#!\[ cfg\( loom \) \]' tests/exhaustive_test.rs )"
printf 'tests in the scripted half:    %s\n' "$( command grep -c '^#\[ test \]' tests/testkit_test.rs || true )"
printf 'tests in the exhaustive half:  %s\n' "$( command grep -c '^#\[ test \]' tests/exhaustive_test.rs || true )"
printf 'the manifest target section:   %s\n' "$( command grep -m1 "target.'cfg(loom)'" Cargo.toml )"
printf 'crates with a dev-dep section: %s\n' "$( cd .. && command grep -rlE 'cfg\(loom\).\.dev-dependencies' ring_*/Cargo.toml | sed 's|/Cargo.toml||' | tr '\n' ' ' )"
printf 'crates with a real dep section: %s\n' "$( cd .. && command grep -rlE 'cfg\(loom\).\.dependencies' ring_*/Cargo.toml | sed 's|/Cargo.toml||' | tr '\n' ' ' )"
printf 'RUSTFLAGS verb/test sets:      %s\n' "$( command grep -ohE 'RUSTFLAGS="[^"]*"' verb/test | sort -u | tr '\n' ' ' )"
printf 'verb/test lines setting loom:  %s\n' "$( command grep -c 'cfg loom' verb/test || true )"
printf 'repo scripts setting the cfg:  %s\n' "$( cd ../.. && command grep -rl 'cfg loom' verb/ 2>/dev/null | wc -l )"
printf 'the command, written down at:  %s sites\n' "$( command grep -rc 'RUSTFLAGS="--cfg loom"' src tests Cargo.toml 2>/dev/null | awk -F: '{ s += $2 } END{ print s+0 }' )"
printf 'across these files:            %s\n' "$( command grep -rl 'RUSTFLAGS="--cfg loom"' src tests Cargo.toml 2>/dev/null | tr '\n' ' ' )"
```

Live output:

```
the scripted half opens with:  #![ cfg( not( loom ) ) ]
the exhaustive half opens with: #![ cfg( loom ) ]
tests in the scripted half:    33
tests in the exhaustive half:  3
the manifest target section:   [target.'cfg(loom)'.dev-dependencies]
crates with a dev-dep section: ring_mpsc ring_publish ring_spsc ring_testkit 
crates with a real dep section: ring_atomic 
RUSTFLAGS verb/test sets:      RUSTFLAGS="-D warnings" 
verb/test lines setting loom:  0
repo scripts setting the cfg:  0
the command, written down at:  6 sites
across these files:            tests/exhaustive_test.rs tests/testkit_test.rs tests/manual/readme.md Cargo.toml 
```

### Workarounds

| File | Relationship |
|------|--------------|
| [001_two_allocations_loom_cannot_avoid.md](001_two_allocations_loom_cannot_avoid.md) | The allocations the same constraint forces |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_the_edge_that_only_exists_under_a_cfg.md](../integration/002_the_edge_that_only_exists_under_a_cfg.md) | The `ring_atomic` seam the cfg reaches, and who else declares it |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_what_a_fixture_owes_its_consumers.md](../non_functional_requirement/002_what_a_fixture_owes_its_consumers.md) | Cfg neutrality as an obligation, and TK36's version of the same seam |

### Sources

| File | Relationship |
|------|--------------|
| [`Cargo.toml`](../../Cargo.toml) | The target section and its comment |
| [`ring_atomic/src/lib.rs`](../../../ring_atomic/src/lib.rs) | The switch the cfg actually throws |

### Tests

| File | Relationship |
|------|--------------|
| `tests/testkit_test.rs` | X1 |
| `tests/exhaustive_test.rs` | X2 |

### TK53 — no automated command in the repository sets the cfg the exhaustive half needs

`verb/test` is the crate family's full verification command. It sets
`RUSTFLAGS="-D warnings"` and `RUSTDOCFLAGS="-D warnings"`, runs nextest, the
doctests and `cargo doc`, and sets `--cfg loom` in none of them. No file under
`verb/` sets it either.

The crate is not quiet about it. `RUSTFLAGS="--cfg loom"` appears at six sites
across four files: the manifest comment, the exhaustive file's own module doc, the
scripted file's module doc twice — it names the sibling it excludes — and
`tests/manual/readme.md` twice, once for the models and once for clippy. Six
written records, zero executable ones.

That makes the exhaustive half a human procedure with a Run Record, exactly like
[`../non_functional_requirement/002`](../non_functional_requirement/002_what_a_fixture_owes_its_consumers.md)'s
E4 and for the same structural reason. Prose is not a gate; whichever of the six
sites a maintainer happens to read, nothing checks that they then ran it.

Three loom models spawning six threads — the crate's entire answer to *does this
hold under interleaving* — run when someone remembers to run them.

### TK54 — the placement rationale describes the benefit and not the matching cost

The manifest justifies keeping the models in `tests/` rather than `src/` on the
grounds that *"the scripted fixture stays measurable by a coverage run that does
not set the cfg"*. True, and a good reason.

What the same arrangement means is that a coverage run without the cfg does not
under-measure the exhaustive half — it does not see it. `#![ cfg( loom ) ]` is a
file-level attribute, so without the flag the file is not merely uncovered, it is
not compiled. A change that breaks it — a renamed function, a changed signature,
a removed helper — produces a completely green `verb/test`.

The two halves are also unequal in what they would lose. The scripted file's
`#![ cfg( not( loom ) ) ]` protects it from a build where it would panic, so
excluding it under the cfg is a correctness requirement. The exhaustive file's
exclusion in the ordinary build is a consequence, not a requirement: nothing about
it is unsound without loom, it simply cannot compile without the dependency the
cfg brings in.

The manifest comment reads as though the split were free. It costs one direction
of compile checking, permanently, and the sentence a maintainer reads while
deciding where to put a model is the one that does not say so.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit
command grep -F 'does not compile it — a break there still leaves' Cargo.toml
```

Live output:

```
# the exhaustive file, it does not compile it — a break there still leaves
```

**Disposition:** applied — the manifest comment now states the cost directly:
a cfg-less run does not merely under-cover the exhaustive file, it does not
compile it, so a break there — a renamed function, a changed signature, a
removed helper — still leaves `verb/test` fully green, with a pointer to this
finding for the full argument.
Now prints: `does not compile it — a break there still leaves`
