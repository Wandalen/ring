# Integration: The Family Dependency Seam

### Scope

- **Purpose**: Record what `ring_poll` depends on, what it deliberately does not, and the measurement for each.
- **Responsibility**: The three edges out, the absence that is the crate's reason to exist, the four plausible edges that were declined, and how a violation of the seam is detected.
- **In Scope**: Runtime and dev edges, transitive closure, and the strictness of the check that guards them.
- **Out of Scope**: The reachability claim itself (→ [`../invariant/001`](../invariant/001_no_parking_operation_on_the_tick_path.md)); the general shape the check instantiates (→ [`../pattern/001`](../pattern/001_enforcement_by_dependency_graph.md)).

### System Description

`ring_poll` sits at the seam between the tick path and the rest of the family. It
has **one runtime dependency**, and that is the whole point of its shape: the
fewer edges, the fewer ways a parking operation arrives by accident.

The closure it inherits is entirely family:

| Measure | Value |
|---|---|
| External (non-family) crates, at any depth | **0** |
| Family crates in the closure, including itself | **17** |
| Of those, reachable parking crates | **0** |

The 17 are inherited almost entirely through `ring_core`, whose own closure is
most of tier 1 and tier 2. This crate adds none of its own.

### Integration Points

| Edge | Kind | Use site | Why it is needed |
|---|---|---|---|
| `ring_core` | dependency | `Producer`, `Consumer` in every helper signature | The ends this crate operates on. It is the *only* runtime dependency |
| `ring_config` | dev-dependency | `RingConfig::new` in the suite and every doc test | Building a ring to test against. Not reachable from the surface |
| `ring_types` | dev-dependency | `OverflowPolicy::Fail` in `refusing_ring` | The default policy discards rather than refusing, so budget-exhaustion tests need a refusing ring |

#### The absence that is the feature

**`ring_wait` is not here, at any depth.** It is not a dependency, not a
dev-dependency, and not reachable transitively through `ring_core`. The
converse — crates that reach `ring_wait` *without* naming it — is measured in
[`002`](002_what_actually_reaches_ring_wait.md), and there are two of them.

#### Pointed absences

| Not depended on | Why not, given it would be plausible |
|---|---|
| `ring_wait` | The crate's reason to exist. Two of its four `WaitKind` variants are tick-safe, and taking the dependency to use them would put the other two in scope ([`../invariant/001`](../invariant/001_no_parking_operation_on_the_tick_path.md)) |
| `ring_shutdown` | A tick that stops on a close is a real requirement, and it is `ring_handle`'s to satisfy — the handle knows the shutdown, and this crate knows only the ends. Wiring it here would give every tick-path helper a `&Shutdown` parameter it usually ignores |
| `ring_stats` | Counting refusals is exactly what a saturated tick wants to report, and it is the natural next edge. It was declined because nothing consumed the numbers yet — a premise three crates have since falsified → PL18 |
| `ring_cursor` | `CursorPair::may_claim` would let `push_within` check for room before attempting rather than after failing. Measurably worthwhile or not is a question for future `ring_bench` measurement; today `try_push` already answers it in one operation |

#### Consumers

None. Exactly one manifest in the family names `ring_poll`, and it is this
crate's own. `ring_handle` is the intended consumer — this crate's non-parking
rule constrains it, and this crate carries the assertion of that constraint — but the edge does not
exist in either direction, while `ring_handle` refers to `ring_poll` by name four
times in prose and built a second guard of its own because it cannot import this
one → PL17.

### Error Handling

A violated seam surfaces as a failing test in *this* crate rather than in the
crate that changed — that asymmetry is the point, since a crate that newly
depends on a parking operation does not call it yet and so breaks nothing of its
own.

**The manifest scan does not distinguish the three rows above, and that is
deliberate.** `PARKING_CRATES` is compared against the whole `Cargo.toml`, so a
`ring_wait` *dev*-dependency would fail the suite even though nothing on the tick
path could reach it. That is stricter than
[`../invariant/001`](../invariant/001_no_parking_operation_on_the_tick_path.md)
strictly requires, and it is the conservative direction: a parking call in a test
is a parking call someone will move into `src/` on a Tuesday. Measured in
`tests/manual/readme.md` P1, where a probe importing both a missing runtime crate
and a present dev one produced one error rather than two.

**The scan is also a proxy, and it under-detects in one direction.** Reading a
manifest for a substring tests a *name*; the claim is about *reachability*. A
direct-manifest scan would pass even if `ring_core` grew a `ring_wait` dependency
and handed parking down the chain. The `cargo tree` form above is the transitive
check that actually matches the claim — the manifest scan in `tests/poll_test.rs`
is the fast one, and
[`../pattern/001`](../pattern/001_enforcement_by_dependency_graph.md) names this
gap as the pattern's second failure mode.

### Compatibility Requirements

| Requirement | Holds for |
|---|---|
| A new runtime edge must not name, or transitively reach, any crate in `PARKING_CRATES` | Any future dependency added to this crate |
| Changing the roster is an API change, not a test edit | `PARKING_CRATES` is `pub` for exactly this reason |
| A consumer must not hand parking down through an edge this crate cannot see | `ring_handle`, when the intended edge is finally built |
| The transitive check is the authoritative one; the manifest scan is its fast approximation | Both, though only the scan is what the suite runs |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'runtime dependencies:        %s\n' "$( awk '/^\[dependencies\]/{f=1;next} /^\[/{f=0} f' ring_poll/Cargo.toml | command grep -oE '^ring_[a-z_]+' | tr '\n' ' ' )"
printf 'dev-dependencies:            %s\n' "$( awk '/^\[dev-dependencies\]/{f=1;next} /^\[/{f=0} f' ring_poll/Cargo.toml | command grep -oE '^ring_[a-z_]+' | tr '\n' ' ' )"
printf 'external, non-family:        %s\n' "$( cargo tree --manifest-path ring/Cargo.toml -p ring_poll -e normal 2>/dev/null | command grep -v '/ring_' | command grep -c . )"
printf 'family crates in closure:    %s\n' "$( cargo tree --manifest-path ring/Cargo.toml -p ring_poll -e normal 2>/dev/null | command grep -oE 'ring_[a-z_]+ v' | sort -u | wc -l )"
printf 'ring_wait, at any depth:     %s\n' "$( cargo tree --manifest-path ring/Cargo.toml -p ring_poll 2>/dev/null | command grep -c ring_wait || true )"
printf 'manifests naming ring_poll:  %s\n' "$( command grep -l 'ring_poll' ring_*/Cargo.toml 2>/dev/null | tr '\n' ' ' )"
printf 'ring_handle depends on it:   %s\n' "$( command grep -c 'ring_poll' ring_handle/Cargo.toml 2>/dev/null || true )"
printf 'ring_handle names it, prose: %s\n' "$( command grep -rc 'ring_poll' ring_handle/src/lib.rs ring_handle/tests/handle_test.rs 2>/dev/null | paste -sd+ | sed 's|[a-z_/.]*:||g' | bc )"
printf 'ring_stats consumers now:    %s\n' "$( for c in ring_*/Cargo.toml; do n=$( basename $( dirname "$c" ) ); [ "$n" = ring_stats ] && continue; command grep -q 'ring_stats' "$c" && printf '%s ' "$n"; done )"
printf 'ring_cursor consumers now:   %s\n' "$( for c in ring_*/Cargo.toml; do n=$( basename $( dirname "$c" ) ); [ "$n" = ring_cursor ] && continue; command grep -q 'ring_cursor' "$c" && echo x; done | wc -l )"
printf 'declined edges, as listed:   %s\n' "$( awk '/^#### Pointed absences/{f=1} f && /^#### Consumers/{exit} f' ring_poll/docs/integration/001_family_dependency_seam.md | command grep -oE '^\| .ring_[a-z_]+.' | command grep -oE 'ring_[a-z_]+' | tr '\n' ' ' )"
```

Live output:

```
runtime dependencies:        ring_core 
dev-dependencies:            ring_config ring_types 
external, non-family:        0
family crates in closure:    17
ring_wait, at any depth:     0
manifests naming ring_poll:  ring_poll/Cargo.toml 
ring_handle depends on it:   0
ring_handle names it, prose: 4
ring_stats consumers now:    ring_bench ring_factory ring_overflow 
ring_cursor consumers now:   10
declined edges, as listed:   ring_wait ring_shutdown ring_stats ring_cursor 
```

### Invariants

| File | Relationship |
|------|--------------|
| [`../invariant/001_no_parking_operation_on_the_tick_path.md`](../invariant/001_no_parking_operation_on_the_tick_path.md) | The claim this seam exists to keep true |

### Integrations

| File | Relationship |
|------|--------------|
| [002_what_actually_reaches_ring_wait.md](002_what_actually_reaches_ring_wait.md) | The two crates that reach `ring_wait` without naming it, and why the scan misses them |

### Patterns

| File | Relationship |
|------|--------------|
| [`../pattern/001_enforcement_by_dependency_graph.md`](../pattern/001_enforcement_by_dependency_graph.md) | The general shape; names the direct-vs-transitive gap as its second failure mode |

### Sources

| File | Relationship |
|------|--------------|
| [`Cargo.toml`](../../Cargo.toml) | The three edges out — one dependency, two dev-dependencies |
| [`src/lib.rs`](../../src/lib.rs) | `PARKING_CRATES` — the roster the scan compares against |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/poll_test.rs`](../../tests/poll_test.rs) | `the_tick_path_cannot_reach_a_parking_operation` — the fast manifest scan |
| [`tests/manual/readme.md`](../../tests/manual/readme.md) | P1 measured the dev-vs-runtime strictness; P2 runs the transitive form |

### PL17 — the constraint is enforced here for a consumer that has no edge to here

`ring_poll` exists to keep a parking operation off the tick path, and the crate
that walks the tick path is `ring_handle`. Exactly one manifest in the family
names `ring_poll`, and it is `ring_poll`'s own — the consumer does not depend on
the crate that carries its constraint, in either direction.

The coordination is real anyway, and it is conducted entirely in prose.
`ring_handle` names `ring_poll` four times across its source and its suite,
including a doc comment that explains what `ring_poll::PARKING_CRATES` covers and
concludes *"any such edge fails `ring_poll`'s suite"*. It then implements a
**second** guard — a substring scan over its own source text — described as
closing *"the cheap half of that gap"*, because a `std::thread::sleep` written
inline adds no manifest entry for the first guard to see.

Two crates, two guards, one shared claim, and no code path between them. Neither
guard can reference the other's roster, neither fails when the other is deleted,
and the sentence that ties them together is a comment. The `ring_handle` guard
also inherits a premise the other file measures as untrue: a direct `ring_wait`
edge does fail `ring_poll`'s suite, but an edge to `ring_consume` — which reaches
`ring_wait` in a normal build — does not
([`002`](002_what_actually_reaches_ring_wait.md) PL19).

The absent edge is the reason this is worth recording rather than fixing. A
`ring_handle` that depended on `ring_poll` could assert against the same public
`PARKING_CRATES` array, and the two guards would share one definition of the set
they defend. Today they share a paragraph.

### PL18 — a declined edge whose stated reason three crates have since falsified

The Pointed Absences table declines `ring_stats` on a specific and checkable
ground: *"It is not here because nothing consumes the numbers yet; adding a
counter with no reader is the YAGNI shape."*

Three crates now take `ring_stats` as a runtime dependency — `ring_bench`,
`ring_factory` and `ring_overflow`. The readers exist. Whether `ring_poll`
should have the edge is still an open question, and possibly still answered no,
but it can no longer be answered by the reason on record, because that reason is
a claim about the family that stopped being true.

This is the failure mode a table of deliberate absences has that a table of
dependencies does not. A dependency that goes stale breaks a build. A
justification that goes stale keeps reading like a decision, and the document
gets more persuasive as it gets less accurate — nothing rechecks the premise
because nothing links the premise to a measurement.

The row now points at this finding, and the block above measures the consumer
count on every regeneration, so the next reader gets the number rather than the
sentence. The general shape — a corpus recipe that scans a whole tree is only as
fresh as its last run — is the reason every claim in this file that names a
count is now generated rather than typed.
