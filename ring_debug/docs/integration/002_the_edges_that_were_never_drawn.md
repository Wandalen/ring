# Integration: The Edges That Were Never Drawn

### Scope

- **Purpose**: Measure the outbound half of this crate's integration — who calls it — against [`integration/001`](001_reaching_the_cursors_of_a_live_ring.md)'s account of who could.
- **Responsibility**: The consumer side of every seam: which crates take the edge, which section of which manifest it would appear in, and what the absence costs the crate's own documents.
- **In Scope**: The eight crates named as natural callers; `[dev-dependencies]`; `ring_testkit`; the acceptance criterion's evidence.
- **Out of Scope**: Why the strong checks are unreachable from the top (→ [`integration/001`](001_reaching_the_cursors_of_a_live_ring.md)); the door built to route around it (→ [`workaround/001`](../workaround/001_the_door_ring_core_does_not_open.md)).

### System Description

[`integration/001`](001_reaching_the_cursors_of_a_live_ring.md) answers *what can
this crate reach*. This one answers the other question, which turns out to have a
shorter answer: **nothing reaches it.**

That is not a complaint. It is the missing term in several of this crate's other
documents, each of which reasons about a consumer's experience — what a caller
loses, what a caller must not conclude, what a caller would want deferred — and
the population of callers is empty.

### Integration Points

| Direction | Edges | Count |
|---|---|---|
| Inbound — what this crate depends on | `ring_core`, `ring_cursor`, `ring_types`, `ring_atomic` | 4 |
| Outbound — what depends on this crate | — | **0** |

The zero holds across every manifest section, which matters because the natural
one is specific. A diagnostic called from another crate's tests appears in that
crate's `[dev-dependencies]`, not its `[dependencies]` — it is a build-time edge
for the test target only, costing nothing at run time and nothing to a downstream
consumer. That section, family-wide, names this crate zero times.

#### The eight crates the argument names

[`integration/001`](001_reaching_the_cursors_of_a_live_ring.md) closes its case for
the crate's value with a specific list: `ring_spsc`, `ring_mpsc`, `ring_gating`,
`ring_claim`, `ring_publish`, `ring_barrier`, `ring_consume`, `ring_wait` — *"each
of them can call `check` against its own pair from its own tests."*

Every one of the eight takes the `ring_cursor` edge, which is the prerequisite the
argument identifies. None of the eight takes this one. **The argument for the
crate's usefulness is complete, correct, and untaken**, and the gap is not a
missing capability but a missing line in eight manifests.

#### Two crates built for the same reader

The family also has `ring_testkit`, whose whole subject is helping other ring
crates test themselves. It does not name `ring_debug`, and `ring_debug` does not
name it.

They were built for the same consumer — someone writing a test against a ring —
and neither knows the other exists. A testkit is the one place in the family where
a cursor-corruption check is not merely usable but idiomatic: the helper that
builds a ring for a test is exactly the helper that could check it afterwards.

### Compatibility Requirements

| # | Obligation | On whom | Currently |
|---|---|---|---|
| J5 | Take a `[dev-dependencies]` edge on this crate | Any crate that manipulates cursors | Taken by none of the eight |
| J6 | Call `check` against a pair the crate already holds | The same | No call site outside this crate |
| J7 | Keep the acceptance criterion's evidence somewhere a consumer would look | The family | Satisfied inside this crate's own suite |

**J7 is the one with a real consequence.** The acceptance criterion is that
`ring_debug`'s invariant check catches a deliberately corrupted cursor, and it
does — in `a_consumer_ahead_of_its_producer_is_caught`, against a `CursorPair`
this crate constructs for itself. The criterion is met by the crate
demonstrating itself on its own fixture, which is the weakest form of evidence
that still counts as evidence.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '-- inbound edges --'
sed -n '/^\[dependencies\]/,/^\[/p' ring_debug/Cargo.toml | command grep -E '^ring_' | sed 's/^/  /'
echo '-- outbound: manifests outside this crate naming it, in any section --'
# Cargo.toml (the workspace root) is not a consuming crate's manifest —
# excluded so this counts dependents, not the workspace listing this crate
# as one of its own members.
printf '  %s\n' "$( command grep -rl 'ring_debug' --include=Cargo.toml | command grep -v '^Cargo\.toml$' | command grep -cv '^ring_debug/' || true )"
echo '-- the eight crates the argument names: prerequisite taken, this edge taken --'
for c in ring_spsc ring_mpsc ring_gating ring_claim ring_publish ring_barrier ring_consume ring_wait ; do
  printf '  %-13s ring_cursor:%s  ring_debug:%s\n' "$c" \
    "$( command grep -c 'ring_cursor' $c/Cargo.toml || true )" \
    "$( command grep -c 'ring_debug' $c/Cargo.toml || true )"
done
echo '-- dev-dependency lines naming it, family-wide, excluding its own manifest --'
printf '  %s\n' "$( for f in ring_*/Cargo.toml ; do
  case "$f" in ring_debug/*) continue ;; esac
  awk '/^\[dev-dependencies\]/{f=1;next} /^\[/{f=0} f && /ring_debug/' "$f"
done | wc -l )"
echo '-- and the testkit built for the same reader --'
printf '  ring_testkit naming ring_debug: %s\n' "$( command grep -c 'ring_debug' ring_testkit/Cargo.toml || true )"
printf '  ring_debug naming ring_testkit: %s\n' "$( command grep -c 'ring_testkit' ring_debug/Cargo.toml || true )"
```

Live output:

```
-- inbound edges --
  ring_core = { path = "../ring_core" }
  ring_cursor = { path = "../ring_cursor" }
  ring_types = { path = "../ring_types" }
  ring_atomic = { path = "../ring_atomic" }
-- outbound: manifests outside this crate naming it, in any section --
  0
-- the eight crates the argument names: prerequisite taken, this edge taken --
  ring_spsc     ring_cursor:1  ring_debug:0
  ring_mpsc     ring_cursor:1  ring_debug:0
  ring_gating   ring_cursor:1  ring_debug:0
  ring_claim    ring_cursor:1  ring_debug:0
  ring_publish  ring_cursor:1  ring_debug:0
  ring_barrier  ring_cursor:1  ring_debug:0
  ring_consume  ring_cursor:1  ring_debug:0
  ring_wait     ring_cursor:1  ring_debug:0
-- dev-dependency lines naming it, family-wide, excluding its own manifest --
  0
-- and the testkit built for the same reader --
  ring_testkit naming ring_debug: 0
  ring_debug naming ring_testkit: 0
```

### Integrations

| File | Relationship |
|------|--------------|
| [001_reaching_the_cursors_of_a_live_ring.md](001_reaching_the_cursors_of_a_live_ring.md) | The inbound half, and the eight-crate argument this measures |

### Workarounds

| File | Relationship |
|------|--------------|
| [../workaround/001_the_door_ring_core_does_not_open.md](../workaround/001_the_door_ring_core_does_not_open.md) | DB18 — the same emptiness, seen as an unfalsifiable cost |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_a_watch_does_not_latch.md](../decisions/002_a_watch_does_not_latch.md) | DB16 — a deferral waiting on evidence from a consumer that does not exist |

### Sources

| File | Relationship |
|------|--------------|
| [`Cargo.toml`](../../Cargo.toml) | The four inbound edges |

### Tests

| Test | Relationship |
|------|--------------|
| `a_consumer_ahead_of_its_producer_is_caught` | J7 — the acceptance criterion, satisfied on this crate's own fixture |
| `the_two_ends_of_a_live_ring_agree` | The one test that builds a real `ring_core::Ring`, and it builds it here |

### DB31 — the argument for the crate's value names eight callers, all of whom took the prerequisite and none of whom took the edge

`ring_spsc`, `ring_mpsc`, `ring_gating`, `ring_claim`, `ring_publish`,
`ring_barrier`, `ring_consume` and `ring_wait` are named as the crates where this
check is *"both reachable and useful"*, on the correct grounds that they hold
cursors and can corrupt them. All eight depend on `ring_cursor`, so the
reachability half is confirmed. None of the eight depends on `ring_debug`, in any
manifest section.

The distance between the two is one line per manifest, which is what makes this
worth recording rather than shrugging at. **A capability gap needs a design; an
untaken edge needs a line, and eight of them have gone untaken while the document
arguing for them sat one directory away.**

The section is specific and its absence is measurable: a diagnostic invoked from
tests belongs in `[dev-dependencies]`, costing nothing at run time and nothing
downstream, and family-wide that section names this crate zero times. The
prerequisite for the argument is satisfied everywhere and the conclusion nowhere.

### DB32 — the family has a testkit and a checker, and neither names the other

`ring_testkit` exists to help ring crates test themselves. `ring_debug` exists to
check a ring's cursors from a test. Neither appears in the other's manifest.

They are two answers to the same question, built separately, and the seam between
them is the obvious one: a helper that constructs a ring for a test is precisely
the place a cursor check would be idiomatic rather than remembered. As it stands a
test author must know both crates exist and wire the second one themselves, which
is the step the first crate was created to remove.

Recorded as an observation, because which of them should take the edge is a real
question this crate cannot answer alone — `ring_testkit` depending on `ring_debug`
makes the check available to every crate already using the testkit, and the reverse
direction makes no sense at all. **The finding is that the question has not been
asked**, in a family that has otherwise documented every edge it has, including the
four this crate was not supposed to have
([`decisions/001`](../decisions/001_four_edges_not_two.md)).
