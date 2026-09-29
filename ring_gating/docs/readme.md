# docs

Design documentation for `ring_gating`, as typed doc definitions.

| Directory | Responsibility |
|------|-----------------|
| `algorithm/` | The two computations, and how far each delegates |
| `api/` | The eleven methods and what their shape commits to |
| `data_structure/` | The one type, and what owning its cursors costs |
| `decisions/` | Choices with live alternatives, recorded with their arguments |
| `definition/` | Module Index — every definition and every instance in this crate, in one place |
| `integration/` | Three dependencies, two dependents, and the sibling half in `ring_barrier` |
| `invariant/` | Properties that must hold for every input, including two absences |
| `item/` | Per-method contracts and coverage |
| `lifecycle/` | The set's degenerate life, and the producer's real one |
| `non_functional_requirement/` | Cost and correctness-under-concurrency requirements |
| `pattern/` | Shapes this crate participates in rather than invents |
| `pitfall/` | Wrong uses that compile and look right |
| `type/` | The widths the readings are measured in, and the traits nobody wrote |
| `workaround/` | External constraints this crate absorbs, with costs and deletion conditions |

Scope of this crate: producer gating so it never laps the slowest consumer.

Start at [`definition/readme.md`](definition/readme.md) — it carries the full
instance table and the sixty-three findings this corpus recorded.

### What a Crate That Computes Nothing Is Documented For

`ring_gating` is 325 lines with one type and eleven methods, and it performs no
atomic load, no fold, no arithmetic on a sequence, no loop, no `unsafe` and no
destructor. Every one of those is a verified absence with a manual check
standing behind it, because none of them is expressible to the compiler
([`invariant/002`](invariant/002_this_crate_names_no_ordering.md),
[`workaround/001`](workaround/001_the_manual_check_that_names_a_foreign_method.md)).

What is left is the crate's actual content: **which question to ask, and which
refusal to return.** `headroom` descends five steps through four crates to reach
one subtraction ([`algorithm/001`](algorithm/001_headroom_in_two_delegations.md));
`check` orders two refusals so that a retry loop stops on the one no consumer's
progress can clear ([`algorithm/002`](algorithm/002_check_orders_its_two_refusals.md)).
Both are decisions rather than computations, which is why a crate this thin
carries a corpus this size.

### The Two Threads Running Through the Corpus

**Generality that nothing exercises.** The crate implements a *set* because
that's the specified shape, and exactly one production site constructs a
`GatingSet` — with a literal `1`
([`data_structure/001`](data_structure/001_the_set_that_cannot_grow.md) § G2).
The third rung of its API, `check`, has no caller outside its own tests, and the
error predicates that rung exists to feed have 27 test callers and zero
production callers
([`decisions/002`](decisions/002_a_result_rather_than_a_bool.md) § G7, § G8).
The family's other ring gates the same single consumer through a `CursorPair`
and allocates nothing to do it
([`workaround/002`](workaround/002_the_multi_consumer_path_no_ring_uses.md) § G19)
— which since `b7e075ca` is true of the read here as well, leaving the owned
`Vec` at construction and the delegation depth as what actually still differs.
None of that is wrong; all of it is a bill the general shape sends to the one
caller that does not need it.

**Guarantees checked by the wrong instrument.** The crate's safety requirement is
that the gate never over-reports room, and the concurrency test guarding it
asserts `headroom <= CAPACITY` — true by construction for every possible input,
so the test cannot fail
([`non_functional_requirement/002`](non_functional_requirement/002_the_gate_must_never_over_report.md) § G12).
The properties that *are* load-bearing — the delegated fold, the absent ordering,
the order of the two refusals — are checked by hand-run greps whose expected
output is nothing at all, and one of those checks justified a gap in its own
pattern by naming a method from another crate
([`workaround/001`](workaround/001_the_manual_check_that_names_a_foreign_method.md)).
The pairing is the point: where a test exists it is weaker than its name, and
where the property is strongest there is no test to be had.
