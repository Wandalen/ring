# Pattern: One Way In

### Scope

- **Purpose**: State the practice that a single construction path is what makes the set of legal rings enumerable, and record honestly that this crate does not currently implement it — there are five ways in, and four of them predate the fifth.
- **Responsibility**: State the problem, solution, applicability, and consequences.
- **In Scope**: Why one entry point; what enforces it; what it costs; its current status.
- **Out of Scope**: The record that entry point takes (→ [`pattern/001`](001_configuration_as_data.md)); the export gate's mechanics (→ [`integration/002`](../integration/002_the_crate_the_export_surface_routes_through.md)).

### Problem

**[`pattern/001`](001_configuration_as_data.md) makes the shape of a ring
enumerable. It does nothing to make the *set of rings* enumerable.** Those are
different claims, and only the second is what a sweep needs.

A configuration record says what shapes are describable. It says nothing about
whether every ring in the process was described by one. If four other functions
can also produce a ring, then the population of rings under measurement is the
union of five different notions of "legal", and the sweep enumerates one fifth
of it while appearing to enumerate all.

**The concrete failure is a benchmark that measures rings it did not
configure.** A harness sweeps `RingConfig` values, builds one ring per row, and
reports a table. Elsewhere in the same binary, a helper called `Ring::new(
capacity )` for a fixture. The fixture ring has default wait, default overflow,
and a batch of one, and appears in no row. If it contends for the same cache
lines or the same consumer thread, it is in the measurement and not in the
table.

### Solution

**One public function constructs a ring; every other route is either absent or
unreachable from outside the family.**

Two mechanisms are named for this, and they are different in kind:

| # | Mechanism | Scope | Kind |
|---|-----------|-------|------|
| E1 | The export surface — only five `ring_*` crates may be named by an outside consumer | Outside the family | **Mechanical**, once gate G5 is non-vacuous |
| E2 | Backend crates not exposing config-taking constructors | Inside the family | **Convention.** Nothing checks it |

**E1 is the strong one and it does not stop family-internal construction**,
which is correct: `ring_core` must be able to call `ring_spsc::Ring::new`, or
nothing could be built at all. The gate skips family manifests by design:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -F 'grep -qx -- "$owner" && continue' bench_harness/gate/g5_export_surface.sh
```

Live output:

```
  printf '%s\n' "${members[@]}" | grep -qx -- "$owner" && continue
```

**So E2 is what governs the inside, and E2 is currently false.** Four public
constructors exist across the two backends, two of which take a `RingConfig`
and read one field of it. The full accounting is in
[`invariant/002`](../invariant/002_construction_is_the_only_path.md); the
summary is that this pattern is documented aspiration, not current behaviour.

**The honest statement of the solution, therefore, is a target with a gap:**

| Route | Should be | Is |
|-------|-----------|-----|
| `Factory::build( cfg )` | The one way | ✅ **Exists.** Plus `build_crossbeam` — a second door, ruled rather than accidental (→ [`decisions/002`](../decisions/002_two_doors_not_one_that_routes.md)) |
| `Ring::new( capacity )` ×2 | Family-internal, called only by `ring_core` | Public, zero external callers today |
| `Ring::with_config( &cfg )` ×2 | **Should not exist.** It occupies this pattern's name and does a fifth of its job | Public. **`ring_core` now calls it on both branches** |
| `ring_core::Ring::new( &cfg )` | Family-internal, called only by this crate | Public, and the *only* route that currently produces a working configured ring |

**Row 3 changed and row 4 is new, both from the same event.** `ring_core` was
implemented and reached for `with_config` (→ [`invariant/002`](../invariant/002_construction_is_the_only_path.md)'s
L2), so the route this pattern says should not exist now has an in-family
caller on the sanctioned path:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -r 'with_config' --include=*.rs | command grep -v '/ring_spsc/\|/ring_mpsc/'
```

Live output:

```
ring_bench/tests/bench_test.rs://! | `with_config` ignores the policy the factory refuses | `the_direct_doors_ignore_the_policy_the_contract_door_refuses` |
ring_bench/tests/bench_test.rs:/// `ring_spsc::Ring::with_config` and `ring_mpsc::Ring::with_config` read one
ring_bench/src/lib.rs:    ring_spsc::Ring::with_config( &workload.config() );
ring_bench/src/lib.rs:    ring_mpsc::Ring::with_config( &workload.config() );
ring_core/src/lib.rs:      true => Storage::Mpsc( ring_mpsc::Ring::with_config( config ) ),
ring_core/src/lib.rs:      false => Storage::Spsc( ring_spsc::Ring::with_config( config ) ),
```

**The pattern is further from holding than when it was written, and the reason
is not carelessness.** With `Factory::build` unwritten, `ring_core` needed *some*
way to construct a backend from a config, and `with_config` is the method whose
name says exactly that. The pattern's target — one way in — cannot be met by a
crate that does not exist yet, so every crate needing a ring in the meantime
establishes a route that will have to be un-established later.

**That is the specific cost of documenting a chokepoint before building it**, and
it is worth naming because the remedy is ordering rather than discipline: the
sooner `build` exists, the fewer alternative routes accumulate behind it.

### Applicability

**Use this when the population of a constructed thing is itself the
measurement.** That is a narrow condition and it is exactly met here.

| Condition | Present here |
|-----------|--------------|
| Something enumerates the constructed population | **Yes — the sweep.** The decisive one |
| Construction is rare relative to use | Yes. One build, then millions of publishes |
| A second constructor would be silently wrong rather than loudly wrong | Yes — an unconfigured ring works fine and appears nowhere |
| There is a boundary that can be mechanically enforced | Yes, at the crate edge. **Not inside it** |

**The third row is what raises the stakes.** A second construction path that
crashed would be self-correcting. A second path that produces a working,
correct, unmeasured ring is not, and nothing in a test suite naturally notices.

**The pattern does not apply within the family**, and pretending otherwise is
how `with_config` came to exist. `ring_core` legitimately constructs backends;
`ring_testkit` will legitimately construct fixtures. The pattern's boundary is
the export surface, not the crate.

### Consequences

**What it buys:**

1. **The set of rings equals the set of swept configs.** The measurement's
   population is known.
2. **One place to add a check.** Tick-safety, name validation, a construction
   log — each is one edit rather than five (→ [`type/002`](../type/002_build_error.md)).
3. **One place to change the backend selection rule.** Today `producers > 1`;
   any future refinement lands in one function.
4. **A consumer has one thing to learn.** The Contract's `ring_factory` entry
   is a single verb.

**What it costs:**

1. **A bottleneck by construction.** Every new capability must pass through
   `build`'s signature, and `build` takes exactly one argument. A capability
   that does not fit in `RingConfig` has nowhere to go — which is why naming
   became a second function rather than a field (→ [`api/002`](../api/002_the_named_build_surface.md)).
2. **Family-internal construction stays unconstrained**, so the pattern's
   guarantee is only as good as the export boundary — which is vacuous today.
3. **`with_config`'s existence is not obviously wrong**, only misnamed. Deleting
   it is a judgement about API surface that this crate does not own, and leaving
   it is a standing invitation to bypass `build` while appearing not to.

**Cost 1 is the interesting one because it has already bitten.** The named registry's
naming requirement did not fit in the record, so it became `build_named`. `ring_stats` and
`ring_tls` are declared dependencies that the build arc never touches
(→ [`lifecycle/001`](../lifecycle/001_from_a_record_to_a_handle_pair.md)'s D5
and D6) — plausibly because whatever they were meant to contribute also did not
fit. A bottleneck pattern makes each such case visible, which is a benefit
disguised as a cost, provided somebody looks.

**The pattern's status should be re-checked rather than assumed.** It is
currently unmet, and the check is two greps:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -cE '^\s*(pub )?(fn|struct|enum|trait|type|const) ' ring_factory/src/lib.rs
command grep -c 'pub fn new\|pub fn with_config' ring_spsc/src/lib.rs ring_mpsc/src/lib.rs
```

Live output:

```
6
ring_spsc/src/lib.rs:2
ring_mpsc/src/lib.rs:2
```

The first is no longer 0 — re-run it rather than trusting a number recorded
here. The second is still 2 per backend, and the pattern is met only when that
number reflects a deliberate decision about each constructor, not when it
happens to have shrunk. **It has not shrunk, and one of the four gained a caller
while this crate was being written** (`ring_core` now calls `with_config` on both
branches), so the gap between the pattern and the code widened during the very
work that was supposed to close it. Pending 8 in [`decisions/`](../decisions/readme.md).

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_build_surface.md](../api/001_the_build_surface.md) | The one way in, and the four constructors P1–P4 it displaces |
| [../api/002_the_named_build_surface.md](../api/002_the_named_build_surface.md) | Cost 1's first casualty — a capability that did not fit the record |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_the_crate_the_export_surface_routes_through.md](../integration/002_the_crate_the_export_surface_routes_through.md) | E1's mechanics, and what this family's own Contract ruling obliges |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_construction_is_the_only_path.md](../invariant/002_construction_is_the_only_path.md) | This pattern as a formal invariant, with the four leaks enumerated |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_from_a_record_to_a_handle_pair.md](../lifecycle/001_from_a_record_to_a_handle_pair.md) | D5 and D6 — two declared dependencies cost 1 may explain |

### Patterns

| File | Relationship |
|------|--------------|
| [001_configuration_as_data.md](001_configuration_as_data.md) | The companion; this pattern is what makes that one's enumerability reach the rings themselves |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_factory.md](../type/001_factory.md) | N1 — the nominal type benefit 4 depends on |
| [../type/002_build_error.md](../type/002_build_error.md) | Benefit 2 — the one place a refusal could live |

### Sources

| File | Relationship |
|------|--------------|
| [`bench_harness/gate/declared/ring/export_surface.txt`](../../../bench_harness/gate/declared/ring/export_surface.txt) | E1's five names |
| [`bench_harness/gate/g5_export_surface.sh`](../../../bench_harness/gate/g5_export_surface.sh) | Line 22 — why E1 does not reach inside the family |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/factory_test.rs`](../../tests/factory_test.rs) | **Still cannot test this pattern**, and that is unchanged by the crate being implemented. A test cannot prove no other constructor exists; the check is the grep above and belongs to a gate. What the suite *can* show is that the sanctioned door is sufficient — `the_contract_surface_is_reachable_without_naming_a_non_contract_crate` builds, registers and retrieves without naming a non-Contract crate, which is evidence the one way in is a usable way in, not that it is the only one |

### FC39 — The Count Was Five and Is Now Nine

This instance's own headline number is stale, and the direction it moved in is
the one that matters:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rE '^\s*pub (const )?fn (new|with_config|new_crossbeam|build|build_named|build_crossbeam)\b' \
  --include=*.rs ring_spsc/src/ ring_mpsc/src/ ring_core/src/ ring_factory/src/ \
  | sed 's|ring/||;s/(.*//'
```

Live output:

```
ring_spsc/src/lib.rs:    pub fn new
ring_spsc/src/lib.rs:    pub fn with_config
ring_mpsc/src/lib.rs:    pub fn new
ring_mpsc/src/lib.rs:    pub fn with_config
ring_core/src/lib.rs:    pub fn new
ring_core/src/lib.rs:    pub fn new_crossbeam
ring_factory/src/lib.rs:    pub fn build<S: Send>
ring_factory/src/lib.rs:    pub fn build_named<S: Send>
ring_factory/src/lib.rs:    pub fn build_crossbeam<S: Send>
```

Nine, and four of them arrived *after* the five were counted: `ring_core`'s two
constructors and this crate's own `build_named` and `build_crossbeam`. Two of the
four are this crate's, which means the crate that exists to be the one way in
has added two more ways in since the pattern was written down.

That is defensible per door — `build_named` is a different operation, and
`build_crossbeam` is ruled in [`decisions/002`](../decisions/002_two_doors_not_one_that_routes.md)
— and it is the exact shape of how a one-way-in pattern erodes: never by a bad
decision, always by a sequence of individually justified ones. The number is the
only thing that notices, and it lives here, in prose, with no gate reading it.

Regenerating it is one command and belongs in this definition's
[`### Regenerate`](readme.md) block, which is where it now is.

### FC40 — Seven Crates Import `ring_core` Directly and One Imports This Crate

The pattern's status has been "not currently implemented" on the strength of
alternative *constructors* existing. The stronger measurement is who uses them —
counting `use` lines rather than any appearance of the name, so a crate that
merely discusses a dependency in a comment is not scored as having one:

```sh
cd "$(git rev-parse --show-toplevel)"
for dep in ring_core ring_factory; do
  imports=''; talks=''
  for c in ring_*/; do
    n="$( basename "$c" )"
    [ "$n" = "$dep" ] && continue
    if command grep -rqE "^ *(pub )?use $dep::" --include=*.rs "$c/src/"; then
      imports="$imports $n"
    elif command grep -rqE "^ *(//|///|//!).*$dep" --include=*.rs "$c/src/"; then
      talks="$talks $n"
    fi
  done
  printf '  %-13s imports it:      %s\n' "$dep" "$imports"
  printf '  %-13s only mentions:   %s\n' "$dep" "$talks"
done
```

Live output:

```
  ring_core     imports it:       ring_debug ring_factory ring_flush ring_handle ring_poll ring_shutdown ring_testkit
  ring_core     only mentions:    ring_bench ring_event ring_mpsc ring_overflow ring_registry ring_slot ring_spsc ring_stats
  ring_factory  imports it:       ring_bench
  ring_factory  only mentions:    ring_config ring_flush ring_registry ring_types
```

Seven to one. Building through `ring_core` is what the family actually does;
building through the factory is what `ring_bench` does.

That is not a defect to fix by adding dependencies on this crate — a handle
library, a debug facility and a shutdown path all legitimately want a ring
without wanting a configuration record. It relocates the pattern's real scope:
"one way in" is a promise to consumers *outside* the family, enforced by the
export surface excluding `ring_core`
(→ [`invariant/002`](../invariant/002_construction_is_the_only_path.md) FC24),
and inside the family it is not a pattern at all — it is one crate's convention
that seven others do not follow.

Written as "five ways in, four of them predate the fifth", the status reads as a
migration that has not finished. Written as the four lists above, it reads as two
audiences with different needs, only one of which the pattern was ever for. The
second reading is the true one and it makes the pattern's status *reached* for
its actual scope, which no version of the first reading can say.

**The second line of each pair is the one that would have gone unnoticed.** Three
crates name `ring_factory` in comments and none of them depends on it: `ring_config`
below it, `ring_registry` beside it, `ring_flush` on the same export surface. The
same shape appears above — `ring_bench`, `ring_registry` and `ring_spsc` discuss
`ring_core` without importing it. Prose coupling runs in directions the dependency
graph forbids, which is *correct* (a comment may reference anything) and is exactly
why a name-appearance count answers the wrong question: measured that way, the two
lists read ten and four instead of seven and one.
