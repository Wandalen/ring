# Lifecycle: The Factory Outlives Nothing

### Scope

- **Purpose**: Establish that a `Factory` owns nothing, retains nothing, and can be dropped at any point without consequence — and name the two things that would each make that false.
- **Responsibility**: State the phases, transitions, dependencies, and cleanup requirements.
- **In Scope**: The factory's own life; what it does and does not hold across a build; the naming path's exception.
- **Out of Scope**: The build arc itself (→ [`lifecycle/001`](001_from_a_record_to_a_handle_pair.md)); the ring's operating life.

### Lifecycle Phases

**Three phases, and two of them are empty.**

| # | Phase | What happens | Duration |
|---|-------|-------------|----------|
| F1 | Creation | `Factory` comes into existence | Free. No inputs, no allocation, cannot fail (→ [`type/001`](../type/001_factory.md)) |
| F2 | Service | Zero or more builds | Arbitrary. **The factory is unchanged by each one** |
| F3 | Drop | The value goes away | Free. Nothing to release |

**F2 being non-mutating is the whole claim.** A factory that has built a
thousand rings is indistinguishable from one that has built none — same size,
same state, same behaviour on the next call. That is
[`invariant/001`](../invariant/001_configuration_fully_determines_the_ring.md)'s
V2 stated as a lifecycle property: no counter, no cache, no issued-handle list.

**A factory may be created per call and thrown away.** `Factory.build( cfg )`
as a temporary is as correct as a factory stored for the program's life, and
nothing observable distinguishes the two. That is a useful property to state
because it tells a consumer they need make no decision about where to keep one.

### Phase Transitions

| # | Transition | Trigger | Note |
|---|-----------|---------|------|
| G1 | ∅ → F1 | A `Factory` value is written | Total |
| G2 | F1 → F2 | First build | No initialisation happens on the way |
| G3 | F2 → F2 | Each subsequent build | **Self-loop with no state change.** This is the transition that must stay empty |
| G4 | F1 → F3, F2 → F3 | The value is dropped | Identical from either state |

**G3 is the transition to watch, because every plausible feature request adds
something to it.** A build counter for diagnostics; a cache of constructed
backends; a weak list of issued handles so the factory can report what it made.
Each is individually reasonable and each makes G3 mutating, which breaks
`invariant/001` — the ring built by the tenth call would differ from the first,
or could.

**There is no `Drop` impl and there must not be one.** A `Drop` on `Factory`
would be a claim that dropping it matters, and the only thing it could
plausibly do — unregister the rings it created — is exactly the behaviour
[`api/002`](../api/002_the_named_build_surface.md)'s registration exists to
avoid: a named ring that vanishes because a temporary went out of scope.

### Dependencies

**The factory holds no dependency alive.** Its five declared dependencies are
compile-time; at runtime it references nothing from any of them.

| # | Crate | Held across F2 | Note |
|---|-------|---------------|------|
| K1 | `ring_config` | No | Configs arrive per call, by value |
| K2 | `ring_core` | No | Backends are constructed and handed away |
| K3 | `ring_handle` | No | Both halves leave with the caller |
| K4 | `ring_stats` | No | Unused in the build arc entirely |
| K5 | `ring_tls` | No | Same |
| K6 | `ring_registry` | **This is the exception.** See below | Under the registry-holding variant, yes |

**K6 is the whole of the uncertainty.** Two shapes are live, from
[`type/001`](../type/001_factory.md)'s N2:

| Variant | F2's self-loop | This instance's title |
|---------|---------------|-----------------------|
| Registry passed per call | Still empty. The factory holds nothing | **True** |
| Registry held as a field | **Mutating** — the field's contents change with each named build | **False.** The factory outlives every ring registered through it |

**Under the registry-holding variant this instance is simply wrong**, and that
is the honest statement rather than a hedge. The factory would own a map, that
map would outlive individual handles, dropping the factory would drop the
registry, and every named ring would become unreachable at once. None of that is
catastrophic — it is a coherent design — but it is a different design, and it
needs its own lifecycle instance rather than a caveat on this one.

**The variant is not chosen.** Recorded in [`decisions/`](../decisions/readme.md),
alongside the D1/D2/D3 backend-shape question, which pushes in the same
direction: D2 would *also* force the factory to hold something
(→ [`data_structure/002`](../data_structure/002_the_handle_pair_as_output.md)).
**Two independent open questions both resolve to "does the factory have a
field," and neither document currently notices the other.**

### Cleanup Requirements

| # | Requirement | Status |
|---|------------|--------|
| Q1 | Dropping a factory destroys no ring | Holds under the stateless variant; **fails under registry-holding**, where the registry goes with it |
| Q2 | Dropping a factory invalidates no handle | Holds under both. Handles never reference the factory |
| Q3 | Dropping a factory leaves no allocation behind | Holds under the stateless variant trivially |
| Q4 | A ring outlives the factory that built it | Holds under the stateless variant; under registry-holding, **a named ring does not** |
| Q5 | Dropping a factory mid-build is impossible | Holds under both — `build` takes `&self`, so the borrow prevents it |

**Q2 holding under both variants is what makes the uncertainty survivable.** A
caller who holds a `Producer` and a `Consumer` is unaffected by the factory's
fate either way; only *lookup by name* is at risk. So the blast radius of
getting K6 wrong is confined to the named-build path, and every unnamed build is
safe under either answer.

**Q5 is worth stating because it is free and it is the failure a stateful
factory would otherwise need to guard.** `&self` means a live borrow for the
duration of the call, so no drop can race a build. This is the borrow checker
doing the work that a hand-written factory in another language would do with a
lock.

**Q4 is where a consumer will be surprised.** "The thing that made this is
gone, so the thing is gone" is a reasonable intuition and is false under the
stateless variant and true under the other. Whichever is chosen, it should be
in the crate's own doc comment rather than only here.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_build_surface.md](../api/001_the_build_surface.md) | B3 — `&self`, which gives Q5 for free |
| [../api/002_the_named_build_surface.md](../api/002_the_named_build_surface.md) | K6's operation, and Q1/Q4's exception |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_handle_pair_as_output.md](../data_structure/002_the_handle_pair_as_output.md) | The other open question that resolves to "does the factory have a field" |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_configuration_fully_determines_the_ring.md](../invariant/001_configuration_fully_determines_the_ring.md) | V2 and V3 — what G3 must not acquire |

### Lifecycles

| File | Relationship |
|------|--------------|
| [001_from_a_record_to_a_handle_pair.md](001_from_a_record_to_a_handle_pair.md) | The arc F2 services; its C2 and C3 are Q1–Q3 |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_factory.md](../type/001_factory.md) | N2, stated as a type question; this instance is the same question as a lifetime question |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/factory_test.rs`](../../tests/factory_test.rs) | ✅ `a_ring_outlives_the_factory_that_built_it` — the factory is created inside a scope block, the `Split` outlives it, and the publish/drain happens after the drop. Written as a scope block rather than as prose so the compiler checks the ordering. It was the one test that would have distinguished the two variants at runtime; with the registry-holding variant now ruled out (→ [`decisions/001`](../decisions/001_the_owner_is_the_return_value.md)) it is a guard against reintroducing it |

### FC30 — Nothing Can Be Retained, Because There Is No Retaining Machinery in the Crate

"Owns nothing, retains nothing, droppable at any point" is asserted by a test
that outlives one factory. It is guaranteed by there being nothing to outlive:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- retention machinery, doc comments excluded --'
for k in 'impl Drop' 'impl Default' 'fn new' 'static ' 'Arc<' 'Rc<' 'Box<' 'Vec<'; do
  printf '  %-13s %s\n' "$k" \
    "$( command grep -vE '^\s*(///|//!)' ring_factory/src/lib.rs | command grep -c "$k" )"
done
echo '  -- and the test that outlives one --'
command grep 'fn a_ring_outlives_the_factory_that_built_it' ring_factory/tests/factory_test.rs
```

Live output:

```
  -- retention machinery, doc comments excluded --
  impl Drop     0
  impl Default  0
  fn new        0
  static        0
  Arc<          0
  Rc<           0
  Box<          0
  Vec<          0
  -- and the test that outlives one --
fn a_ring_outlives_the_factory_that_built_it()
```

Zero of each. No `Drop` impl, so dropping a `Factory` runs nothing; no owning
container anywhere in the crate, so there is nothing a factory could be holding;
no `new`, so the only way to obtain one is the unit literal.

The two things named as making the claim false are a field on `Factory` and a
retained registry. Both are additions, and both would show up in this block as a
non-zero. So the lifecycle documented here is not merely simple — it is the
lifecycle of a value the compiler treats as free, and the test asserting it is a
regression guard against a future edit rather than evidence about the current
one.

Worth stating because the same test would pass on a `Factory` that held an
`Arc< Registry >`: outliving one factory proves the *ring* is independent, not
that the factory was empty. The emptiness is proved by the zeros, and only by
them.
