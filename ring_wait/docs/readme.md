# docs

Design documentation for `ring_wait`, as typed doc definitions.

| Directory | Responsibility |
|------|-----------------|
| `algorithm/` | One loop with two exits, and the two wrappers that fix its predicate |
| `api/` | Seven items, one production caller, and a parameter that is the design |
| `data_structure/` | A crate that declares nothing and reads the scheduler, a clock, and other people's memory |
| `decisions/` | Choices with live alternatives, recorded with their arguments |
| `definition/` | Module Index — every definition and every instance in this crate, in one place |
| `integration/` | Two dependencies, two dependents, and a roster naming this crate as the hazard |
| `invariant/` | Properties that must hold for every input, one of them an absence |
| `item/` | Per-item contracts and coverage, arm by arm |
| `lifecycle/` | A wait from the first look to one of two endings, and a ladder nobody climbs |
| `non_functional_requirement/` | What a look costs in atomic loads, and what an attempt costs in nanoseconds |
| `pattern/` | Two shapes this crate is the reference instance of |
| `pitfall/` | Two things that are not what their names suggest |
| `type/` | Two integer domains that never mix, and the annotation that went to the wrong item |
| `workaround/` | External constraints this crate absorbs, with costs and deletion conditions |

Scope of this crate: how a thread waits for space or data, and how long it is
willing to.

Start at [`definition/readme.md`](definition/readme.md) — it carries the full
instance table and the fifty-three findings this corpus recorded.

### What a Crate With No Types and One Loop Is Documented For

`ring_wait` is 269 lines. It declares no `struct`, no `enum`, no `trait` — one of
three crates in the family that declare nothing, and the only one of the three
that is not pure arithmetic
([`data_structure/001`](data_structure/001_a_crate_with_no_type_of_its_own.md) § WT14).
It performs no atomic operation, names no `Ordering`, contains no `unsafe`, no
cast, and no bare `loop`. Its whole executable content is a thirteen-line counted
`for` at `:183-195` and a four-arm `match` on a one-byte enum.

What is left is what the crate is actually for: **deciding when to stop asking,
and what to do between asks.** Both are policy rather than computation, which is
why a crate this thin carries a corpus this size. The loop's structure is the
subject of [`algorithm/001`](algorithm/001_one_loop_and_the_two_ways_out.md); the
four things it can do between asks are
[`item/001`](item/001_the_four_arms_of_the_pause.md); what each of those four
costs — a measured 1700× span on one enum argument — is
[`non_functional_requirement/002`](non_functional_requirement/002_what_each_strategy_costs_per_attempt.md) § WT6.

### The Three Threads Running Through the Corpus

**A generic loop with almost no callers.** Seven public items, and exactly one is
called from another crate's `src/`
([`api/001`](api/001_seven_items_and_the_one_with_a_caller.md) § WT1). The two
named wrappers — `for_space` and `for_data`, the crate's most specific and most
useful-looking surface — have no production caller at all, and the reason is
structural: fixing the predicate is exactly what makes them unreachable, so
`ring_shutdown` needed one extra exit condition and had to re-type the closure
over `wait_until` rather than wrap `for_space`
([`algorithm/002`](algorithm/002_two_wrappers_over_a_predicate_they_fix.md) § WT3,
[`integration/002`](integration/002_the_wrapper_that_had_to_be_rewritten.md)).
`escalation_hint` is further out still: a complete three-arm ladder over four
variants, carrying the crate's only `#[ must_use ]`, called by nothing anywhere
([`lifecycle/002`](lifecycle/002_the_escalation_ladder_nobody_climbs.md),
[`type/002`](type/002_one_return_type_and_the_one_must_use.md) § WT8). The
generality is real, correct, and waiting for a consumer that has not arrived —
and where a consumer did arrive, it wrote its own copy.

**A hazard everyone routes around.** This crate blocks, so the tick path must not
reach it, so the family needs a ban Cargo cannot express and substitutes a
substring instead
([`workaround/002`](workaround/002_a_crate_name_as_a_load_bearing_string.md)).
The consequences are everywhere. `ring_poll` open-codes the same
predicate-pause-budget loop three times rather than depend on this crate, because
the crate is the only granularity a manifest can enforce
([`pattern/001`](pattern/001_the_predicate_the_pause_and_the_budget.md),
[`integration/001`](integration/001_two_dependencies_two_dependents_and_a_roster.md) § WT12),
and the `spins == 0` clamp is written once here and once in `ring_poll`'s
constructor for the same reason
([`algorithm/001`](algorithm/001_one_loop_and_the_two_ways_out.md) § WT17).
`ring_handle` bans seven parking-shaped names in its own source, three of which
are in this crate's code
([`integration/001`](integration/001_two_dependencies_two_dependents_and_a_roster.md) § WT13).
The ban holds today — verified by dependency closure, not by the substring the
roster actually matches — but the roster's stated meaning is transitive and its
predicate is not
([`workaround/002`](workaround/002_a_crate_name_as_a_load_bearing_string.md) § WT10, § WT11).

**Costs the type system does not carry.** `Ok( n )` counts attempts, never time
([`data_structure/002`](data_structure/002_the_budget_and_the_attempt_index.md)),
and every attempt costs two `Acquire` loads through the predicate — up to 2048 of
them for a default-budget `for_data`
([`non_functional_requirement/001`](non_functional_requirement/001_two_atomic_loads_for_every_look.md) § WT16).
The budget is `usize` and the item count is `u64`, two domains that meet only at
`for_data`'s signature and never in an expression
([`type/001`](type/001_a_usize_budget_and_a_u64_count.md)). `Park` asks for 50 µs
and delivers 112–119, while two crates in two other places budget against the 50
([`workaround/001`](workaround/001_a_sleep_where_a_park_belongs.md) § WT7). And
the loop pauses after its final look, for a look that never happens — 100% of a
budget-1 `Park` wait, guarded in `ring_poll`'s copies by one comparison and tested
nowhere ([`pattern/001`](pattern/001_the_predicate_the_pause_and_the_budget.md) § WT24).
The one thing the type system *does* carry is the one-byte enum that makes the
whole discriminants-here-handlers-there split free
([`decisions/002`](decisions/002_the_discriminants_live_in_ring_types.md) § WT19).
