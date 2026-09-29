# Integration: What Actually Reaches `ring_wait`

### Scope

- **Purpose**: Record the two crates that reach a parking operation without naming one, the dev-only third, and why the guard in this crate's suite cannot see any of them.
- **Responsibility**: The edges themselves — which crate reaches `ring_wait` through which intermediate, at what kind of edge, and what a manifest substring scan sees instead.
- **In Scope**: The normal-edge closure, the dev-edge delta, and the structure of the check that guards the roster.
- **Out of Scope**: What to do about the mismatch (→ [`../decisions/002`](../decisions/002_reach_or_declaration.md)); the roster's status as public API (→ [`../api/002`](../api/002_the_roster_as_a_public_constant.md)); the seam this crate keeps (→ [`001`](001_family_dependency_seam.md)).

### Abstract

`PARKING_CRATES` holds three names. The set its doc comment describes holds
five. The two extra crates are not exotic and not new — they are ordinary
consumers one hop away from a crate that does declare `ring_wait`, and the
reason they are missing is that the check reads manifests rather than graphs.

### The Closure, Edge By Edge

| Crate | How it reaches `ring_wait` | Edge kind | Named in its own manifest? |
|---|---|---|---|
| `ring_wait` | it *is* the parking crate | — | yes |
| `ring_barrier` | direct dependency | normal | yes |
| `ring_shutdown` | direct dependency | normal | yes |
| `ring_consume` | → `ring_barrier` → `ring_wait` | normal | **no** |
| `ring_testkit` | → `ring_shutdown` → `ring_wait` | normal | **no** |
| `ring_publish` | → `ring_barrier` → `ring_wait` | **dev** | no |

Three of the six are invisible to a manifest scan. Two of those three are
reachable in a normal build, which is the closure a consumer links against.

### Why The Scan Cannot See Them

The guard in `tests/poll_test.rs` walks `/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/`, reads each `Cargo.toml`, and
keeps the crates whose file text `contains( "ring_wait" )`. That is a test on
the *string* `ring_wait` appearing in one file.

`ring_consume`'s manifest does not contain that string. It contains
`ring_barrier`, and `ring_barrier`'s manifest is a file the scan reads
separately, for a different row of the answer. Nothing joins the two. The scan
has every fact it would need and no step that composes them, which is the
difference between reading a set of manifests and resolving a graph.

This is the direct-vs-transitive gap
[`../pattern/001`](../pattern/001_enforcement_by_dependency_graph.md) names as
the pattern's second failure mode, with the crates that occupy it now measured
rather than hypothetical → PL19.

### The Second Half Of The Guard

After comparing the scan's result against `PARKING_CRATES`, the test loops over
three hard-coded names and asserts none of them is in the measured set:

```
for tick_path in [ "ring_poll", "ring_handle", "ring_core" ]
```

Read in order, that loop runs only when the `assert_eq!` above it has already
passed — and if it passed, the measured set *is* `PARKING_CRATES`, a compile-time
array of three literals that contains none of those three names. The loop
therefore cannot fail on any state of the family's manifests → PL20.

It is not dead, though. It fails on exactly one thing: somebody adding a
tick-path crate to `PARKING_CRATES` to silence the assertion above. That is a
narrow and real guard — against a bad fix rather than against drift — and it is
worth knowing that is the whole of what it covers.

### What Holds Today Anyway

The invariant the check exists for is currently true, and true more strongly
than the check can establish. None of the three tick-path crates reaches
`ring_wait` at any depth, on normal edges or with dev edges included:

| Crate | Normal closure | With dev edges |
|---|---|---|
| `ring_poll` | 0 | 0 |
| `ring_handle` | 0 | 0 |
| `ring_core` | 0 | 0 |

So this is not a live breach. It is a guard measuring one thing and credited
with another, on a family where the two answers happen to agree.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'crates in the family:           %s\n' "$( ls -d ring_*/ | wc -l )"
printf 'manifest names ring_wait:       %s\n' "$( cd ring && for c in ring_*/Cargo.toml; do command grep -q 'ring_wait' "$c" && echo "${c%/Cargo.toml}"; done | tr '\n' ' ' )"
printf 'reach it, normal edges:         %s\n' "$( for c in ring_*/; do n=$( basename "$c" ); cargo tree --manifest-path ring/Cargo.toml -e normal -p "$n" 2>/dev/null | command grep -q 'ring_wait' && printf '%s ' "$n"; done )"
printf 'reach it without naming it:     %s\n' "$( for c in ring_*/; do n=$( basename "$c" ); cargo tree --manifest-path ring/Cargo.toml -e normal -p "$n" 2>/dev/null | command grep -q 'ring_wait' && ! command grep -q 'ring_wait' "$c/Cargo.toml" && printf '%s ' "$n"; done )"
for n in ring_consume ring_testkit; do
  for d in $( cargo tree --manifest-path ring/Cargo.toml -e normal -p "$n" --depth 1 2>/dev/null | command grep -oE 'ring_[a-z_]+ v' | sed 's/ v//' | tail -n +2 ); do
    cargo tree --manifest-path ring/Cargo.toml -e normal -p "$d" 2>/dev/null | command grep -q ring_wait && printf '  %-14s reaches it via: %s\n' "$n" "$d"
  done
done
printf 'reach it, dev edges included:   %s\n' "$( for c in ring_*/; do n=$( basename "$c" ); cargo tree --manifest-path ring/Cargo.toml -p "$n" 2>/dev/null | command grep -q 'ring_wait' && echo x; done | wc -l )"
printf 'dev-edge-only reacher:          %s\n' "$( for c in ring_*/; do n=$( basename "$c" ); a=$( cargo tree --manifest-path ring/Cargo.toml -e normal -p "$n" 2>/dev/null | command grep -c ring_wait ); b=$( cargo tree --manifest-path ring/Cargo.toml -p "$n" 2>/dev/null | command grep -c ring_wait ); [ "$a" = 0 ] && [ "$b" != 0 ] && printf '%s ' "$n"; done )"
printf '  and its use sites:            %s\n' "$( command grep -rn 'use ring_barrier' ring_publish/tests/*.rs | sed 's|ring_publish/||; s/:  use.*//' | tr '\n' ' ' )"
printf 'tick-path crates, any depth:    %s\n' "$( for n in ring_poll ring_handle ring_core; do printf '%s=%s ' "$n" "$( cargo tree --manifest-path ring/Cargo.toml -p "$n" 2>/dev/null | command grep -c ring_wait )"; done )"
printf 'the guard the suite runs:       %s\n' "$( command grep -oE 'contains\( "ring_wait" \)' ring_poll/tests/poll_test.rs )"
printf 'the tick-path list, as written: %s\n' "$( command grep -oE 'for tick_path in \[.*\]' ring_poll/tests/poll_test.rs | sed 's/for tick_path in //' )"
printf 'its length vs the family:       %s of %s\n' "$( command grep -oE 'for tick_path in \[.*\]' ring_poll/tests/poll_test.rs | command grep -o '"ring_' | wc -l )" "$( ls -d ring_*/ | wc -l )"
```

Live output:

```
crates in the family:           33
manifest names ring_wait:       ring_barrier ring_shutdown ring_wait 
reach it, normal edges:         ring_barrier ring_consume ring_shutdown ring_testkit ring_wait 
reach it without naming it:     ring_consume ring_testkit 
  ring_consume   reaches it via: ring_barrier
  ring_testkit   reaches it via: ring_shutdown
reach it, dev edges included:   6
dev-edge-only reacher:          ring_publish 
  and its use sites:            tests/handshake_test.rs:65 tests/handshake_test.rs:241 
tick-path crates, any depth:    ring_poll=0 ring_handle=0 ring_core=0 
the guard the suite runs:       contains( "ring_wait" )
the tick-path list, as written: [ "ring_poll", "ring_handle", "ring_core" ]
[ "ring_poll", "ring_handle", "ring_core" ]
its length vs the family:       6 of 33
```

### Integrations

| File | Relationship |
|------|--------------|
| [001_family_dependency_seam.md](001_family_dependency_seam.md) | This crate's own edges, and the absence this file measures from the other side |

### APIs

| File | Relationship |
|------|--------------|
| [`../api/002_the_roster_as_a_public_constant.md`](../api/002_the_roster_as_a_public_constant.md) | PL7 and PL8 — the same gap read as an API question |

### Decisions

| File | Relationship |
|------|--------------|
| [`../decisions/002_reach_or_declaration.md`](../decisions/002_reach_or_declaration.md) | The choice this measurement forces, and the dev-edge ambiguity it inherits |

### Invariants

| File | Relationship |
|------|--------------|
| [`../invariant/001_no_parking_operation_on_the_tick_path.md`](../invariant/001_no_parking_operation_on_the_tick_path.md) | The claim, which holds; this file is about what checks it |

### Patterns

| File | Relationship |
|------|--------------|
| [`../pattern/001_enforcement_by_dependency_graph.md`](../pattern/001_enforcement_by_dependency_graph.md) | The general shape, whose second failure mode this file populates |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | `PARKING_CRATES` and the sentence describing it |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/poll_test.rs`](../../tests/poll_test.rs) | `the_tick_path_cannot_reach_a_parking_operation` — both halves |

### PL19 — two crates reach a parking operation and the guard reads clean

`ring_consume` depends on `ring_barrier`. `ring_barrier` depends on `ring_wait`.
`ring_testkit` depends on `ring_shutdown`, and `ring_shutdown` depends on
`ring_wait`. Both edges are normal dependencies, present in a release build, one
hop away from a crate already on the roster.

Neither crate is in `PARKING_CRATES`, and the guard that keeps the roster honest
reports no drift — because it reads each manifest for the substring `ring_wait`
and neither manifest contains it. The scan holds every fact needed to find these
two and has no step that joins them; composing manifests into a graph is exactly
what it does not do.

The consequence is scoped by who reads the constant. It is `pub`, its doc
comment says it names *"the family crates from which a parking operation is
reachable"*, and a scheduler author checking whether a crate is safe to call from
a tick gets `false` for `ring_consume` — which can sleep. That is the shape of
error the roster was promoted to public API to prevent.

What keeps this from being urgent is that the tick-path crates themselves are
clean at any depth, dev edges included, so the invariant the guard defends is
intact. The guard is measuring a proxy that agrees with the target by accident
of the current graph, which is the state that stops being true without anything
failing.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -F "every crate a parking operation is transitively reachable from" ring_poll/src/lib.rs
```

Live output:

```
/// every crate a parking operation is transitively reachable from; see
```

**Disposition:** applied — `PARKING_CRATES`'s doc comment in `src/lib.rs` (the
same fix recorded under [`../api/002`](../api/002_the_roster_as_a_public_constant.md)
PL7) no longer claims the roster names every crate a parking operation is
transitively reachable from; it now points here for the two edges — through
`ring_barrier` and `ring_shutdown` — that this file measures. Now prints: `every crate a parking operation is transitively reachable from`

### PL20 — the tick-path assertion cannot fail while the assertion above it passes

The test does two things in order. It compares the scanned set against
`PARKING_CRATES`, then loops over `[ "ring_poll", "ring_handle", "ring_core" ]`
asserting none of them is in the scanned set.

The second step runs only if the first passed, and if the first passed the
scanned set is equal to `PARKING_CRATES` — three string literals, fixed at
compile time, none of them a tick-path crate. So the loop is asserting a
property of a constant that the constant's own text already shows. No change to
any manifest in the family can make it fire.

There is one input that does reach it: an edit to `PARKING_CRATES` itself. If a
tick-path crate acquired a `ring_wait` dependency and somebody made the resulting
failure go away by adding that crate to the roster, the loop catches it. That is
a genuine guard against a specific bad fix, and it is worth having.

It is not, however, what the loop reads as. Written as a scan over measured data
it looks like a check on the family; it is a check on three literals in the file
it is written in. The distinction matters because the more valuable version —
does any crate the tick path reaches end up at `ring_wait` — is one `cargo tree`
away and is not what is running.

The list is also hand-written and three long, against a family of thirty-three.
Nothing derives it, nothing checks it stays current, and a fourth crate joining
the tick path is covered by the same amount of test as it was before it existed.
