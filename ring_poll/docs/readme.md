# docs

Design documentation for `ring_poll`, as typed doc definitions.

| Directory | Responsibility |
|------|-----------------|
| `algorithm/` | The bounded retry everything is built on, and what one attempt costs |
| `api/` | Eight declarations, each cost attributed to whoever chose it, and the roster |
| `data_structure/` | Three values in three shapes, and a `Copy` accumulator that counts |
| `decisions/` | Two open questions, both about the hand-written roster |
| `definition/` | Module Index — every definition, instance and finding in one place |
| `integration/` | The family seam this crate refuses to cross, and what actually crosses it |
| `invariant/` | What cannot be reached from the tick path, and what a budget does not bound |
| `item/` | Four operations behind eight entry points, and what the crate declines to declare |
| `lifecycle/` | The states a tick moves through, and every place a record can end up |
| `non_functional_requirement/` | What a tick may cost, and the cost nobody has measured |
| `pattern/` | Enforcing a rule the language cannot express; accounting over free functions |
| `pitfall/` | Two ways one `usize` parameter gets read more generously than it means |
| `type/` | A budget that clamps to one, and a progress value with no zero-made |
| `workaround/` | Constraints absorbed into the shape of the code, with costs and expiry |

Scope of this crate: try-only operations on the tick path.

## How to Read These in Order

| Read | For |
|---|---|
| [`invariant/001`](invariant/001_no_parking_operation_on_the_tick_path.md) | The claim the crate exists to hold, and how a graph property is asserted at all |
| [`invariant/002`](invariant/002_a_budget_bounds_attempts_not_time.md) | The weaker property that is actually true, kept apart from the one it gets mistaken for |
| [`api/001`](api/001_tick_path_surface.md) | The surface, and the "who chose it" column that the two invariants explain |
| [`pitfall/001`](pitfall/001_non_parking_is_not_bounded_latency.md) | The trap, which is the underside of invariant 002 |
| [`pattern/001`](pattern/001_enforcement_by_dependency_graph.md) | The technique extracted, if you want it elsewhere |
| [`definition/readme.md`](definition/readme.md) | Everything else — the Module Index, with all 26 instances and all 52 findings in one table |

## What Makes This Crate Different From Its Siblings

**It is the only crate whose central claim is about a crate it does not depend
on.** This crate's rule says the parking operations must not be *reachable*; a claim
of that shape cannot be tested by calling something, because there is nothing to
call. So the assertion runs against the dependency graph, and the roster of
crates permitted to park is part of the public surface rather than a constant in
a test file.

**It enforces a constraint on a crate that would not notice breaking it.**
The same rule also binds `ring_handle` — nothing reachable from a handle may park
— and `ring_handle`'s own suite stays green whether or not its manifest grows a
`ring_wait` dependency. The check has to live somewhere that fails when
*another* crate changes, which is here.

**Its helpers exist so the rule stays cheap to obey.** `ring_core::Producer` was
already try-only; nothing needed hiding. What needed doing was making the
non-parking spelling the short one, on the theory that a restriction survives
exactly as long as following it is easier than working around it.

## Verification

Every figure below is derived rather than typed — the counts that used to sit in
trailing comments here went stale three times before this block replaced them.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_poll
printf 'doc definitions:             %s\n' "$( ls -d docs/*/ | command grep -cv '/definition/$' )"
printf 'instances:                   %s\n' "$( ls docs/*/[0-9][0-9][0-9]_*.md | wc -l )"
printf 'findings:                    %s\n' "$( command grep -rhoE '^### PL[0-9]+ — ' docs/*/[0-9][0-9][0-9]_*.md | wc -l )"
printf 'tests in the suite:          %s\n' "$( command grep -c '#\[ test \]' tests/poll_test.rs || true )"
printf 'doctests in src:             %s\n' "$( command grep -c '^/// ```$' src/lib.rs | awk '{ print $1 / 2 }' )"
printf 'manual probes:               %s\n' "$( command grep -coE '^## P[0-9]+ — ' tests/manual/readme.md || true )"
printf 'ring_wait in the closure:    %s\n' "$( cd .. && cargo tree -p ring_poll 2>/dev/null | command grep -c 'ring_wait' || true )"
printf 'the point of the crate:      %s\n' "$( awk '/^\[dependencies\]/{f=1;next} /^\[/{f=0} f' Cargo.toml | command grep -c 'ring_wait' || true )"
```

Live output:

```
doc definitions:             13
instances:                   26
findings:                    52
tests in the suite:          32
doctests in src:             8
manual probes:               4
ring_wait in the closure:    0
the point of the crate:      0
```

## Related Crates

| Crate | Relationship |
|---|---|
| [`ring_core`](../../ring_core/readme.md) | The producer and consumer every helper here operates on; the only runtime dependency |
| [`ring_wait`](../../ring_wait/readme.md) | The parking primitive this crate exists to keep out of reach — named everywhere, depended on nowhere |
| [`ring_handle`](../../ring_handle/readme.md) | Constrained by this crate's non-parking rule without claiming it; the constraint is asserted here |
| [`ring_shutdown`](../../ring_shutdown/readme.md) | A permitted parker, and the crate where the drop-newest trap this suite re-asserts was first found |
