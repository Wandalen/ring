# docs

Design documentation for `ring_shutdown`, as typed doc definitions.

| Directory | Responsibility |
|------|-----------------|
| `algorithm/` | The one procedure this crate owns — emptying a closed ring |
| `api/` | The whole surface, with the guarantee-by-construction column |
| `data_structure/` | One atomic field and its orderings; five types and their derive sets |
| `decisions/` | The one question this crate cannot settle on its own evidence |
| `definition/` | Module Index — every definition and every instance in this crate, in one place |
| `integration/` | Four edges out, each justified by a named item, and four pointed absences |
| `invariant/` | One liveness flag in the family, and a drain that terminates |
| `item/` | Twenty-two declarations and their attributes; four of them read closely |
| `lifecycle/` | The teardown and reuse cycle, and the two states with three edges that it traverses |
| `non_functional_requirement/` | What a guarded push costs, and the drain that takes the slow path |
| `pattern/` | The proof-token technique, generalized, with its failure mode |
| `pitfall/` | A close a raw producer ignores, and an `Ok` that kept nothing |
| `type/` | A proof token and a two-armed refusal |
| `workaround/` | External constraints this crate absorbs, with costs and deletion conditions |

Scope of this crate: publisher stop, drain, and waiter join.

## How to Read These in Order

| Read | For |
|---|---|
| [`api/001`](api/001_shutdown_surface.md) | What the crate offers, and the guarantee column that everything else explains |
| [`type/001`](type/001_stopped_proof_token.md) | The mechanism behind three of the six **construction** rows |
| [`invariant/002`](invariant/002_drain_terminates_because_close_preceded_it.md) | Why that mechanism is load-bearing rather than decorative |
| [`pitfall/001`](pitfall/001_close_is_advisory_to_an_unguarded_producer.md) | Where the guarantee stops, which is the thing to know before using the crate |
| [`pitfall/002`](pitfall/002_ok_does_not_mean_kept_under_drop_newest.md) | The second trap, which is about configuration rather than about this crate |
| [`pattern/001`](pattern/001_proof_token_orders_two_operations.md) | The technique extracted, if you want it elsewhere |
| [`pattern/002`](pattern/002_a_proof_token_must_be_scarce.md) | The rule the extracted technique is missing, which this crate breaks |
| [`definition/readme.md`](definition/readme.md) | The Module Index — every definition, instance, decision and finding in one table |

## What Makes This Crate Different From Its Siblings

**It owns the family's only liveness flag.** Every other crate that could
plausibly carry an `is_closed` deliberately does not, and the absence is
recorded on both sides — most explicitly in `ring_core`, whose handle-surface
divergence table lists `is_closed` as ***settled** — see below* and then argues
the settlement. Measured:

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'files in the family holding one: %s\n' "$( command grep -rln 'AtomicBool' ring_*/src | tr '\n' ' ' )"
printf 'crates among them:               %s\n' "$( command grep -rln 'AtomicBool' ring_*/src | cut -d/ -f2 | sort -u | wc -l )"
printf 'what ring_core records instead:  %s\n' "$( command grep -oh '^| .is_closed. |.*settled.*' ring_core/docs/integration/002_*.md | head -1 )"
```

Live output:

```
files in the family holding one: ring_shutdown/src/lib.rs 
crates among them:               1
what ring_core records instead:  | `is_closed` | present | absent | **settled** — see below |
```

**It guarantees an ordering, not a state.** Three of its four guarantees hold
because the wrong call does not compile. The fourth — that publication actually
stops — holds only for callers who opted in, and
[`pitfall/001`](pitfall/001_close_is_advisory_to_an_unguarded_producer.md)
says so rather than implying otherwise. That gap is the price of the flag being
here instead of in `ring_core`, and the price is worth naming.

## Verification

Two commands verify this crate — `cargo nextest run -p ring_shutdown` and
`cargo test --doc -p ring_shutdown`. The inventory each one covers is derived
rather than written down, because every hardcoded figure in this section was
stale the last time it was read:

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'doc definitions:            %s\n' "$( ls -d ring_shutdown/docs/*/ | command grep -cv '/definition/$' )"
printf 'doc instances under them:   %s\n' "$( ls ring_shutdown/docs/*/[0-9][0-9][0-9]_*.md | wc -l )"
printf 'findings across those:      %s\n' "$( command grep -rhoE '^### SD[0-9]+ — ' ring_shutdown/docs/*/[0-9][0-9][0-9]_*.md | wc -l )"
printf 'tests nextest runs:         %s\n' "$( command grep -c '^#\[ test \]' ring_shutdown/tests/shutdown_test.rs )"
printf 'doc tests rustdoc runs:     %s\n' "$(( $( command grep -c '/// ```' ring_shutdown/src/lib.rs ) / 2 ))"
printf 'of those, compile-fail:     %s\n' "$( command grep -c '/// ```compile_fail' ring_shutdown/src/lib.rs || true )"
printf 'probes run by hand instead: %s\n' "$( command grep -c '^## D[0-9]' ring_shutdown/tests/manual/readme.md )"
```

Live output:

```
doc definitions:            13
doc instances under them:   26
findings across those:      52
tests nextest runs:         20
doc tests rustdoc runs:     9
of those, compile-fail:     0
probes run by hand instead: 4
```

The last two lines are the gap [`workaround/002`](workaround/002_a_compile_failure_probe_with_no_harness.md)
is about: four properties are established by a human running a command and
reading the result, in a workspace where three sibling crates already have those
same properties checked on every run.

## Related Crates

| Crate | Relationship |
|---|---|
| [`ring_core`](../../ring_core/readme.md) | The producer and consumer this crate wraps and drains, and the crate whose missing flag is this one's reason to exist |
| [`ring_wait`](../../ring_wait/readme.md) | The bounded spin both close-aware waits are built on |
| [`ring_cursor`](../../ring_cursor/readme.md) | `CursorPair`, whose `may_claim()` is the room half of `for_space_or_close` |
| [`ring_handle`](../../ring_handle/readme.md) | The export-Contract crate above this one; the shutdown a handle consults will be threaded there |
