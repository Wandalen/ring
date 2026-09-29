# docs

Design documentation for `ring_registry`, as typed doc definitions. Thirteen
definitions, twenty-six instances, fifty-two findings; the counts are
regenerable from [`definition/readme.md`](definition/readme.md).

| Directory | Responsibility |
|------|-----------------|
| `algorithm/` | The two branches of the write path, and the four reads |
| `api/` | The eight operations, and the receiver split that shapes them |
| `data_structure/` | Measured widths, and the only `HashMap` in the family |
| `decisions/` | Closed and open trade-offs, each with what settled or would settle it |
| `definition/` | Module Index — every definition and every instance in this crate, in one place |
| `integration/` | Why one dependency edge of the three assigned is enough |
| `invariant/` | One name, one ring; one ring, one owner |
| `item/` | Declaration-level detail — attributes, lints, and what they reach |
| `lifecycle/` | Where a ring's ownership sits, from registration to drop |
| `non_functional_requirement/` | The setup-time premise, and what the reads cost |
| `pattern/` | The refusal contract, and how the family spells it three ways |
| `pitfall/` | The one-line mistake that passes every obvious test and destroys data |
| `type/` | `RegistryError` and `Registry` — what each declares and what it does not |
| `workaround/` | External constraints this crate absorbs, with costs and deletion conditions |

Scope of this crate: a named registry that owns the rings it holds.

## What the corpus found

`ring_registry` is the family's outlier by construction — the only one of
thirty-three crates holding a `HashMap`, one of two writing a reasoned `allow`,
the only one returning an error with the caller's payload beside it — and it
argues each unusual choice at unusual length, from the source, in prose nothing
measures. Measured, the arguments come apart in three directions: some are
contradicted outright, some are right about the wrong quantity, and the one
construct on the hot path that costs measurable time is unremarked.

Twenty-six of the fifty-two findings are reachable and twenty-six are records.
None is a failing test. The full breakdown, and the eight shapes the gaps fall
into, are in [`definition/readme.md`](definition/readme.md).

## Where to start

**[`pitfall/001`](pitfall/001_insert_would_have_replaced_silently.md)** — it is
the reason the crate is shaped the way it is, and the reason the acceptance
criterion asks for a drop counter rather than a count.

Then [`invariant/001`](invariant/001_one_name_one_ring.md) for the two properties
that hold at every moment, [`lifecycle/001`](lifecycle/001_a_ring_from_registration_to_drop.md)
for where ownership sits at each of them, and
[`api/001`](api/001_the_registry_surface.md) for the surface.

For the cost argument specifically, read
[`workaround/001`](workaround/001_a_key_bought_back_from_the_map.md) against
[`algorithm/001`](algorithm/001_two_branches_and_what_the_refusal_costs.md) — the
two together are what turned twenty lines of reasoning about a `Result`'s width
into a measurement of the one line beside it.
