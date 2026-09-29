# Algorithm Doc Definition

### Scope

- **Purpose**: Procedures this crate owns, stated as steps with the decisions inside them named.
- **Responsibility**: One shape shared by three helpers, and the one place the batch version departs from it.
- **In Scope**: The bounded-retry loop; the batch early exit; why `drain_up_to` is a different shape.
- **Out of Scope**: What `try_push` itself does (→ [`ring_core`](../../../ring_core/docs/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Bounded Retry](001_bounded_retry.md) | The retry shape, the three decisions in it, and the batch helper's extra exit | 🔄 |
| 002 | [What An Attempt Costs](002_what_an_attempt_costs.md) | What one attempt buys, and what a refused batch attempt destroys on the way | 🔄 |

**The shape, then the price.** `001` is the procedure — one loop, three decisions
inside it, and the fourth helper that does not use it. `002` is what running that
procedure costs, which is a separate question because the loop is correct and the
cost is still unstated.

They go stale on different edits: `001` on a change to a loop or a guard, `002`
on a change to `ring_core::try_push_batch` or to a doc comment. Neither would
touch the other.

The four findings run from the least to the most consequential. Three decisions
presented alike have unequal evidence (PL1); the one helper outside the loop drops
retrying entirely and the type never says so (PL2); the wrapper multiplies a
documented data loss and redocuments none of it (PL3); and the accounting layer
counts what arrived and never what it cost to get there (PL4).

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_poll/docs/algorithm
printf 'instances:                  %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:    %s\n' "$( command grep -hoE '^### PL[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:    %s\n' "$( command grep -coE '^\| PL[0-9]+ ' readme.md )"
printf 'each instance has a recipe: %s\n' "$( command grep -lc '^### Regenerate' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'helpers with a retry loop:  %s\n' "$( command grep -c 'while attempt < budget.attempts()' ../../src/lib.rs || true )"
printf 'pause hint call sites:      %s\n' "$( command grep -c 'spin_loop' ../../src/lib.rs || true )"
printf 'the helper outside it:      %s\n' "$( command grep -oE 'while taken < max' ../../src/lib.rs )"
printf 'spin_loop named in tests:   %s\n' "$( command grep -c 'spin_loop' ../../tests/poll_test.rs || true )"
printf 'lines reading self.budget:  %s\n' "$( command grep -c 'self.budget' ../../src/lib.rs || true )"
printf 'fields on Tick:             %s\n' "$( awk '/^pub struct Tick$/{f=1} f&&/^\}$/{exit} f' ../../src/lib.rs | command grep -cE '^  [a-z_]+ :' || true )"
printf 'batch loss named in src:    %s\n' "$( command grep -ciE 'eaten|eats|destroy' ../../src/lib.rs || true )"
printf 'batch loss named in tests:  %s\n' "$( command grep -ciE 'eaten|eats|destroy' ../../tests/poll_test.rs || true )"
```

Live output:

```
instances:                  2
finding headings inside:    4
rows in the table below:    4
each instance has a recipe: 2
helpers with a retry loop:  3
pause hint call sites:      3
the helper outside it:      while taken < max
spin_loop named in tests:   0
lines reading self.budget:  4
fields on Tick:             3
batch loss named in src:    4
batch loss named in tests:  13
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| PL1 | three decisions presented alike, with unequal evidence behind them | n/a — coverage | The Algorithm section names the pause's placement inside the guard, `spin_loop` over `yield_now`, and `while` over `loop` as deliberate decisions in one list with one justification style, but only the second is observable from the suite and only indirectly — `a_large_budget_spins_rather_than_sleeping` separates parking from not-parking, a `yield_now` would likely pass it too, the identifier `spin_loop` appears nowhere in the test file, and the `while`/`loop` choice is a coverage-instrument property no test could see. |
| PL2 | the one helper outside the retry loop drops retrying, and the type never says so | n/a — doc gap | The closing section explains why `drain_up_to` keeps its own `max` rather than reusing the budget — a budget bounds retries, a drain limit bounds successes — but not that the drain then has no retry behaviour at all: it breaks at the first `None` while the other three spin, so `Tick::new( Budget::new( 100 ) )` retries a hundred times on `push` and `recv` and zero times on `drain`, and `Tick::budget` is a public accessor for a value that one of the type's four operations silently ignores. |
| PL3 | the wrapper multiplies a documented data loss and redocuments none of it | **misleading doc** | `ring_core::try_push_batch` destroys the record it could not place, says so, and pins it with a doctest; `push_batch_within` calls it in a budget-driven loop and its own doc offers only *"Returns how many records were published"* under a doctest where five records fit an eight-slot ring and nothing is refused — so the crate that turns one refusal into N is the one that does not mention the cost, and the paragraph that states it exactly sits in `push_batch_within_eats_one_record_per_attempt`, which `cargo doc` does not publish. |
| PL4 | the accounting layer counts what arrived and never what it cost | **latent hazard** | `Tick` holds `budget` and `moved` and nothing else, so a tick that spent three attempts against a saturated ring, published four records and destroyed three reports `Progress::Made( 4 )` — the two numbers a scheduler wants collapsed into the first — and the collapse binds hardest exactly where the type claims to add value, since its doc says each method *"delegates to the free function of the same shape and adds only the accounting"* while a free-function caller still holds the iterator the loss is measurable in and a `Tick` caller has handed it away. |
