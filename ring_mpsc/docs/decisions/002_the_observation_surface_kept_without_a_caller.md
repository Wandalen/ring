# Decision: The Observation Surface Kept Without a Caller

**Status:** open. The cost of keeping them is a public surface twice the size
of what is used, in a crate whose one consumer is `ring_core`. The cost of
removing them is three tests that can no longer be written where the family
requires tests to live.

### Scope

- **Purpose**: Record that seven public methods reporting cursor and stamp state have no caller in either dependent crate, and state what keeping them costs.
- **Responsibility**: The measurement, what the seven are for, and the three dispositions.
- **In Scope**: `committed`, `published_through`, `stamps`, `claimed`, `on_distinct_lines`, `sequences`, `position`.
- **Out of Scope**: The reach measurement's method (→ [`../item/001`](../item/001_eighty_items_and_the_four_names_that_leave_the_crate.md)); the five constants, which are also unreached but for a different reason (→ [`../item/002`](../item/002_five_public_ordering_constants.md)).

### The Seven

Every one reports where the ring is rather than moving it:

| Method | Reports |
|--------|---------|
| `committed` | The consumer cursor |
| `published_through` | The highest contiguously published sequence |
| `stamps` | The whole stamp array, borrowed |
| `claimed` | The claim cursor |
| `position` | The consumer's read position |
| `sequences` | The sequence range a batch covers |
| `on_distinct_lines` | Whether the two cursors are on separate cache lines |

**Zero production callers in `ring_core` or `ring_bench`.** What does use them is
this crate's own test suite — `the_claim_advances_before_the_publish_and_the_two_are_separate_cursors`,
`the_claim_cursor_and_the_consumer_cursor_are_on_distinct_cache_lines`,
`stamps_start_unstamped_and_there_is_exactly_one_per_slot` — each of which needs
exactly one of these to assert what it asserts.

### Three Readings

| Reading | Consequence |
|---------|-------------|
| They are test affordances that leaked into the API | Correct diagnosis, awkward fix: a `#[ cfg( test ) ]` accessor cannot be reached from `tests/`, so making them private means moving the tests into `src/`, which the family's test-placement rule forbids |
| They are API for a consumer that does not exist yet | `integration/002` names prospective consumers. A monitoring or backpressure layer would want all seven. None is built |
| They are correct as public API | A ring that cannot be observed cannot be instrumented, and instrumentation is not an optional feature of a queue meant to carry a frame budget |

**The first two are both true**, which is what makes this a decision rather than
a cleanup. The methods exist because tests needed them; they are plausible API
independently; and nothing distinguishes the two cases from outside.

### MP16 — The Seven Unreached Methods Are Exactly the Observation Surface

That partition is what makes this a decision rather than seven separate
observations. The split is clean: `claim`, `push`, `drain`, `drain_up_to`,
`available` all carry traffic; `committed`, `published_through`, `stamps`,
`claimed`, `position`, `sequences`, `on_distinct_lines` carry none.

A queue whose mutators are used and whose accessors are not is a queue nobody is
instrumenting. Whether that is a gap in the consumers or a surplus here is the
open question.

### MP17 — Three Tests Depend on Methods Nothing Else Uses

`the_claim_advances_before_the_publish_and_the_two_are_separate_cursors` needs
`claimed` and `committed`; `the_claim_cursor_and_the_consumer_cursor_are_on_distinct_cache_lines`
needs `on_distinct_lines`; `stamps_start_unstamped_and_there_is_exactly_one_per_slot`
needs `stamps`.

All three live in `tests/`, which is where the family requires tests to live, and
`tests/` sees only the public API. **So the surface is public because the tests
are external**, and that is a real constraint rather than an excuse — the
alternative is moving the tests into `src/`, which the family's own rule
forbids.
