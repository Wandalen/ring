# docs

Design documentation for `ring_barrier`, as typed doc definitions.

| Directory | Responsibility |
|------|-----------------|
| `algorithm/` | The two computations, and the two endings one of them has |
| `api/` | The nine methods, and what one borrowed slice commits them to |
| `data_structure/` | Sixteen bytes pointing at other people's cursors |
| `decisions/` | Choices with live alternatives, recorded with their arguments |
| `definition/` | Module Index — every definition and every instance in this crate, in one place |
| `integration/` | Three dependencies, one dependent, and the edge that is dev-only |
| `invariant/` | Properties that must hold for every input, including two absences |
| `item/` | Per-method contracts and coverage |
| `lifecycle/` | A value with one state, and the drain loop that gives it a purpose |
| `non_functional_requirement/` | What a frontier read costs, and what a non-blocking wait must not do |
| `pattern/` | Shapes this crate participates in rather than invents |
| `pitfall/` | Wrong uses that compile and look right |
| `type/` | The width the crate never converts, and the traits it could not have |
| `workaround/` | External constraints this crate absorbs, with costs and deletion conditions |

Scope of this crate: consumer gating so it never reads past the slowest thing it
depends on.

Start at [`definition/readme.md`](definition/readme.md) — it carries the full
instance table and the twenty-one findings this corpus recorded.

### What a Crate That Computes One Subtraction Is Documented For

`ring_barrier` is 288 lines with one type, nine methods, and 57 lines of code.
It performs no atomic load, no fold, no loop, no `unsafe`, no `impl Drop`, names
no `Ordering`, and takes no `&mut self` — every one of those a verified absence
([`lifecycle/readme.md`](lifecycle/readme.md) carries the counting commands). Its
entire arithmetic is one `saturating_sub` inside `Seq::distance_to` and one
`<=`.

What is left is the crate's actual content: **what to ask of the cursors, and
what to hand back.** `frontier` delegates twice and reaches a `Vec` allocation it
does not want ([`algorithm/001`](algorithm/001_the_frontier_in_two_delegations.md));
`wait_for` asks the same question twice and returns what it found rather than
what was requested, which is a deliberate choice with a measurable price
([`algorithm/002`](algorithm/002_wait_for_asks_twice.md),
[`pitfall/002`](pitfall/002_returning_the_request_instead_of_the_frontier.md)).
Both are decisions rather than computations, which is why a crate this thin
carries a corpus this size.

### The Two Threads Running Through the Corpus

**A borrow that makes the shape and stops making guarantees.** The whole type is
`&'a [ PaddedCursor ]` — 16 bytes, `Copy`, `Send`, `Sync`, no destructor,
zero-cost construction ([`data_structure/001`](data_structure/001_one_field_and_a_sixteen_byte_view.md),
[`type/002`](type/002_the_traits_derived_and_the_traits_absent.md)). That shape
was not chosen for elegance: the aggregate signature made
`ring_publish/tests/handshake_test.rs` unwriteable, and the ownership rule was
recovered from that failure ([`pattern/001`](pattern/001_the_borrowed_view_and_the_owned_set.md)).
What the borrow gives up is everything the compiler could have said about
*which* cursors — a barrier over a slice nobody advances waits forever and no
signature can tell ([`decisions/002`](decisions/002_a_slice_rather_than_an_aggregate.md)) —
and it also silently weakened the check guarding the crate's central claim
([`workaround/001`](workaround/001_the_check_that_capacity_stays_out.md) § BR10).

**A ladder whose top rung nothing climbs.** The family's three gating types each
grew a third rung past *quantity* and *predicate*; four such rungs shipped, with
three placements and three return types, and **none has a caller in any `src/`**
([`pattern/002`](pattern/002_the_quantity_the_predicate_and_the_wait.md) § BR20).
This crate's is `wait_for`, whose six callers are all its own tests
([`item/002`](item/002_the_five_accessors_and_the_wait.md) § BR4), which is what
makes `admits` a one-caller method ([`item/001`](item/001_the_three_barrier_readings.md) § BR3)
and `available` a one-library-caller method under it. Meanwhile the one method
production actually leans on is re-derived term-for-term by its only consumer
([`integration/001`](integration/001_three_dependencies_and_one_dependent.md) § BR2),
and its per-read cost — 8 bytes per dependency, multiplied by the wait budget —
is measured by nothing in the family's benchmark crate
([`non_functional_requirement/001`](non_functional_requirement/001_every_frontier_read_allocates_nothing.md) § BR13).
The generality is real, the bill is real, and the caller that would settle either
does not exist yet.
