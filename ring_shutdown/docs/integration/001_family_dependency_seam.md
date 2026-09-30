# Integration: The Family Dependency Seam

### Scope

- **Purpose**: Record the four edges out of this crate, the named item each one exists for, and the crates deliberately absent from the list.
- **Responsibility**: Per-edge justification with a use site, the measured closure, and the argument for each absence.
- **In Scope**: `ring_core`, `ring_cursor`, `ring_wait`, `ring_types`, and `ring_config` as a dev-dependency.
- **Out of Scope**: The family DAG as a whole, beyond this crate's own four edges.

### System Description

`ring_shutdown` sits above `ring_core` and below the export Contract. It owns
one `AtomicBool` and operates on ends somebody else built — it never constructs
a ring, which is why its only config edge is a dev-dependency.

Four runtime edges, all in-house: no edge declared here admits an external
crate. That is a claim about this crate's own manifest, not about the linked
binary — the workspace-wide `--all-features` build every change is gated on
pulls in two external crates one edge down (→ SD18) — and it is what makes the
flag in `Shutdown` the only atomic *this crate's own edges* are answerable for.

### Integration Points

Each edge is justified by a named item, not by a topic — an edge whose
justification is "this crate is about rings too" is an edge that stays after
its reason leaves.

| Edge | Item used | Where |
|---|---|---|
| `ring_core` | `Consumer< '_, T >`, `Producer< '_, T >` | `Stopped::drain_all`, `Guarded` |
| `ring_cursor` | `CursorPair` | `for_space_or_close`'s `may_claim()` check |
| `ring_wait` | `wait_until` | `wait_for_close`, `for_space_or_close` |
| `ring_types` | `RingError`, `WaitKind` | `admit`, `Refusal::reason`, both waits' signatures |

Verify each is a real use rather than a stale line. Comment and doc lines are
filtered out, because a crate named in prose is exactly the stale line this
check exists to catch, and the census cannot count it as evidence against
itself:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -h '^use ring_\|ring_wait::' ring_shutdown/src/lib.rs | command grep -v '^ *//' | sed 's/^ *//' | sort -u
```

Live output:

```
let outcome = ring_wait::wait_until( kind, spins, ||
ring_wait::wait_until( kind, spins, || shutdown.is_closed() )
use ring_core::{ Consumer, Producer };
use ring_cursor::CursorPair;
use ring_types::{ RingError, WaitKind };
```

**`ring_config` is a dev-dependency only.** It appears in every doc test and in
the test suite, because building a ring needs a config — but nothing in `src/`
constructs a ring. This crate operates on ends that already exist.

#### No edge declared here admits an external crate

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'non-family crates, default:    %s\n' "$( cargo tree -p ring_shutdown -e normal 2>/dev/null | command grep -cv 'ring_' )"
printf 'family crates, default:        %s\n' "$( cargo tree -p ring_shutdown -e normal 2>/dev/null | command grep -oE 'ring_[a-z_]+ v' | sort -u | wc -l )"
printf 'features this crate declares:  %s\n' "$( awk '/^\[features\]/{f=1;next} /^\[/{f=0} f&&NF&&!/^#/{n++} END{ print n+0 }' ring_shutdown/Cargo.toml )"
printf 'non-family, --all-features:    %s\n' "$( cargo tree -p ring_shutdown -e normal --all-features 2>/dev/null | command grep -cv 'ring_' )"
printf 'where the externals do enter:  %s\n' "$( cargo tree -p ring_core -e normal --all-features 2>/dev/null | command grep -v 'ring_' | command grep -oE '[a-z-]+ v[0-9.]+' | sort -u | tr '\n' ' ' )"
printf 'the feature that admits them:  %s\n' "$( command grep -o 'crossbeam = .*' ring_core/Cargo.toml )"
printf 'what the crate is tested with: %s\n' "$( command grep -o 'cargo nextest run --all-features' verb/test | head -1 )"
```

Live output:

```
non-family crates, default:    0
family crates, default:        18
features this crate declares:  0
non-family, --all-features:    0
where the externals do enter:  crossbeam-queue v0.3.14 crossbeam-utils v0.8.23 
the feature that admits them:  crossbeam = ["dep:crossbeam-queue"]
what the crate is tested with: cargo nextest run --all-features
```

**0 non-family crates, 18 family crates** in the default build. No edge
declared in this crate's own manifest admits an external crate — which is
narrower than "the closure is in-house," since the closure actually exercised
by `verb/test` is one edge down, per SD18 below.

Note the asymmetry with `ring_core`, whose closure *can* contain
`crossbeam-queue` and `crossbeam-utils` under a cargo feature (→
[`ring_core/docs/integration/001`](../../../ring_core/docs/integration/001_family_dependency_seam.md)).
Adding `--all-features` to the command above changes nothing, because this crate
declares no features of its own; the two external crates arrive only when
`ring_core`'s `crossbeam` feature is enabled, which a workspace-wide
`--all-features` build does — and that is what `verb/test` runs (→ SD18).

#### Pointedly absent

| Crate | Why it is not a dependency |
|---|---|
| `ring_stats` | Counting drops at teardown is exactly the kind of instrumentation `ring_stats` exists for, and reaching for it here would put a counter on a teardown path that `ring_core` guarantees adds no atomic of its own. `reset` returns its discard count instead, so a caller who wants the number gets it without the edge |
| `ring_handle` | The dependency runs the other way. `ring_handle` is an export-Contract crate above this one; an edge from here would invert the DAG and make the export surface unbuildable |
| `ring_poll` | Nothing reachable from a handle may park, but that constraint is discharged by *not* parking, which needs no code from anywhere. `wait_for_close` uses `ring_wait`'s bounded spin, which returns rather than blocking |
| `ring_barrier` | A barrier-aware close would need a barrier instance and a notification when it advances, which is the same trap `ring_flush`'s `OnBarrier` documents. No arrangement of `[dependencies]` supplies either |

### Error Handling

One error type crosses the seam, in one direction, and it belongs to neither
side of any single edge:

| Edge | What crosses | Handled how |
|---|---|---|
| `ring_types` | `RingError` | Returned verbatim from `admit` and from both waits — this crate re-raises the family vocabulary rather than defining a second one |
| `ring_wait` | Exhaustion of a bounded spin | Arrives as `RingError` already; passed through untouched, since "the spin gave up" is the same event to a caller here as it is there |
| `ring_core` | A full-ring refusal | Never an error — `Guarded::try_push` converts it into the `Full` arm of its own [`Refusal< T >`](../type/002_refusal_carries_the_record.md), which still carries the record |
| `ring_cursor` | `may_claim()`'s answer | A `bool`; no failure shape at all |

**Nothing is wrapped, prefixed, or re-rendered.** A teardown layer that added
its own error enum would give the family a second name for conditions
`ring_types` already names, which is the thing depending on `ring_types` is for.

### Compatibility Requirements

This crate sits above `ring_core` and below the export Contract. It depends on
the composition point and on two of its inputs. One crate now depends on it:
`ring_testkit`, whose `use ring_shutdown::{ Refusal, Shutdown };` is the first
inbound edge — it arrived earlier than this section originally
anticipated. `ring_factory` remains the expected second, since building a ring
and building its shutdown are the same decision, and five further crates already
name this one in prose without an edge (→ [`002`](002_what_the_family_says_about_this_crate.md)).

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_shutdown
printf 'runtime deps in the manifest:  %s\n' "$( awk '/^\[dependencies\]/{f=1;next} /^\[/{f=0} f&&NF{ sub( / .*/, "" ); printf "%s ", $0 }' Cargo.toml )"
printf 'dev deps in the manifest:      %s\n' "$( awk '/^\[dev-dependencies\]/{f=1;next} /^\[/{f=0} f&&NF{ sub( / .*/, "" ); printf "%s ", $0 }' Cargo.toml )"
printf 'what the crate doc says:       %s\n' "$( command grep -o 'Depends on .*' src/lib.rs )"
printf 'crates it names there:         %s\n' "$( command grep -o 'Depends on .*' src/lib.rs | command grep -oE 'ring_[a-z_]+' | sort -u | wc -l )"
printf 'named in the manifest but not: %s\n' "$( comm -13 <( command grep -o 'Depends on .*' src/lib.rs | command grep -oE 'ring_[a-z_]+' | sort -u ) <( awk '/^\[dependencies\]/{f=1;next} /^\[/{f=0} f&&NF{ sub( / .*/, "" ); print }' Cargo.toml | sort -u ) | tr '\n' ' ' )"
printf 'edges the table above lists:   %s\n' "$( awk '/^\| Edge \|/{f=1;next} f&&/^\|---/{next} f&&/^\| /{n++} f&&!/^\| /{exit} END{ print n+0 }' docs/integration/001_family_dependency_seam.md )"
printf 'what the omitted one supplies: %s\n' "$( command grep -v '^ *///' src/lib.rs | command grep -ohE 'use ring_types::\{[^}]*\}' | tr '\n' ' ' )"
printf 'manifests depending on this:   %s\n' "$( cd ..; command grep -rl 'ring_shutdown' ring_*/Cargo.toml | cut -d/ -f1 | command grep -v ring_shutdown | tr '\n' ' ' )"
printf 'what compatibility says today: %s\n' "$( command grep -o 'nothing in the family depends on it yet' docs/integration/001_family_dependency_seam.md | head -1 )"
```

Live output:

```
runtime deps in the manifest:  ring_cursor ring_wait ring_core ring_types 
dev deps in the manifest:      ring_config 
what the crate doc says:       Depends on `ring_cursor`, `ring_wait`, `ring_core`, `ring_types`.
crates it names there:         4
named in the manifest but not: 
edges the table above lists:   4
what the omitted one supplies: use ring_types::{ RingError, WaitKind } 
manifests depending on this:   ring_testkit 
what compatibility says today: nothing in the family depends on it yet
```

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_exactly_one_liveness_flag.md](../invariant/001_exactly_one_liveness_flag.md) | Why `ring_core` has no flag for this crate to depend on, and what the closure measurement supports |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_core/docs/integration/001`](../../../ring_core/docs/integration/001_family_dependency_seam.md) | The same document one layer down, including the external-crate measurement this one contrasts with |

### Tests

| File | Relationship |
|------|--------------|
| `tests/manual/readme.md` | D4 — the two `cargo tree` measurements above, which no unit test can make |


### SD17 — The Crate's Front Page Names Three Dependencies and the Manifest Has Four

`src/lib.rs`'s module doc, five lines in, tells a reader what this crate sits
on: *"Depends on `ring_cursor`, `ring_wait`, `ring_core`."*

`Cargo.toml` lists four: `ring_cursor`, `ring_wait`, `ring_core`, `ring_types`.

The omitted one is not incidental. `ring_types` supplies `RingError` and
`WaitKind` — the vocabulary in which `admit` fails, `Refusal::reason` answers,
and both free waiters declare their signatures. This document's own edge table
lists it as one of four edges and this document's own Error Handling section
builds its central argument on it: *"this crate re-raises the family vocabulary
rather than defining a second one … which is the thing depending on
`ring_types` is for."*

So the crate's most-read sentence about its own position omits the dependency
its integration doc calls load-bearing, four sections apart in the same
repository. A reader who trusts the module doc concludes this crate defines its
own error shapes; the first `use` statement in the file says otherwise.

The mechanical form of the defect is worth naming because it is the cheapest
class to catch and nothing catches it: a hand-maintained prose list beside a
machine-readable manifest, with no check comparing the two. The corpus has four
checkers and none of them reads `Cargo.toml`.

**Disposition:** applied — `src/lib.rs`'s module doc now names all four
manifest dependencies, including `ring_types`. Now prints: `ring_types`

```sh
cd "$(git rev-parse --show-toplevel)"/ring_shutdown
command grep -o 'Depends on .*' src/lib.rs
```

Live output:

```
Depends on `ring_cursor`, `ring_wait`, `ring_core`, `ring_types`.
```

### SD18 — The Closure Is Measured in a Configuration the Crate Is Never Tested In

The headline property of this seam is that no external crate appears in the
closure — *"what makes the flag in `Shutdown` the only atomic this layer is
answerable for."* The measurement backing it runs `cargo tree -p ring_shutdown
-e normal` in the default feature set, and returns 0.

That measurement cannot fail, for a reason the document does not give: this
crate declares no `[features]` at all, so its default and `--all-features`
trees are identical. Adding `--all-features` here changes nothing — the
contrast the document reaches for is real but arrives elsewhere.

`ring_core` declares `crossbeam = [ "dep:crossbeam-queue" ]`, and enabling it
pulls in `crossbeam-queue` and `crossbeam-utils`. Per-package that requires
`cargo tree -p ring_core --all-features`; workspace-wide it happens by feature
unification whenever any member enables it — which is what
`RUSTFLAGS="-D warnings" cargo nextest run --all-features` does, and that is the
command `verb/test` runs.

So the property is true in the build nobody runs and false in the build that
gates every change to this crate. Not wrong, and not harmless either: the
sentence a reader takes away is *"this layer has no external atomics"*, and
during verification it does, one edge down. The honest statement is narrower and
still worth making — **no edge declared here admits an external crate** — which
is a claim about this crate's manifest rather than about the linked binary, and
which the same command already proves.

**Disposition:** applied — the System Description's headline sentence, the
subsection heading, and the closure-measurement summary now all state the
narrower, accurate claim (each ends "...admits an external crate", qualified
by "here" or "in this crate's own manifest") instead of the closure-wide claim
the tested build contradicts. The check below flattens line-wraps before
matching, since two of the three instances wrap across lines in the source.
Now prints: `cargo nextest run --all-features`

```sh
cd "$(git rev-parse --show-toplevel)"/ring_shutdown/docs/integration
command sed -n '1,180p' 001_family_dependency_seam.md | tr '\n' ' ' | command grep -oi 'admits an external crate' | wc -l
command sed -n '1,180p' 001_family_dependency_seam.md | tr '\n' ' ' | command grep -ci 'no external crate in it at all\|closure has no external crate in it\|closure is in-house, which is what makes'
# The command above can legitimately exit nonzero -- a count of zero, a diff
# that differs -- and the sweep that keeps this output current skips any block
# that exits nonzero, so without this the block freezes instead of refreshing.
:
```

Live output:

```
3
0
```
