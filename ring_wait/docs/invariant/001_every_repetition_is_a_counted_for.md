# Invariant: Every Repetition Is a Counted `for`

### Scope

- **Purpose**: State the crate's structural invariant — no unbounded loop exists anywhere in it — say what the invariant buys, and be precise about where its guarantee ends.
- **Responsibility**: Give the property, the check that enforces it, the two loops it covers, and the three ways a caller can defeat it from outside.
- **In Scope**: Boundedness of every repetition in `ring_wait`.
- **Out of Scope**: What each iteration costs — see [`non_functional_requirement/002`](../non_functional_requirement/002_what_each_strategy_costs_per_attempt.md).

### The Property

> Every repetition in this crate is a `for` over a counted range. No `loop {`,
> no `while true`, and no `break`-driven exit that could be reached with the
> counter removed.

Two loops satisfy it:

| Loop | Range | Bound |
|------|-------|-------|
| `wait_until` (`:183`) | `0..spins.max( 1 )` | the caller's budget, clamped to at least one |
| `pause`'s `Spin` arm (`:121`) | `0..=( attempt % 8 )` | 8, unconditionally |

The second is bounded by a constant and cannot be given a larger bound by any
caller: `attempt % 8` is at most 7, so the inner loop runs at most 8 times
regardless of what `attempt` is
([`pitfall/002`](../pitfall/002_the_backoff_that_resets_every_eight_attempts.md)).
So the crate's total repetition count is bounded by `spins × 8`, and both
factors are visible in the source.

### The Check

W1 in `tests/manual/readme.md`:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -vE "^[[:space:]]*//" ring_wait/src/lib.rs \
  | grep -E "^[[:space:]]*loop[[:space:]]*(\{|$)|while[[:space:]]+true" \
  || echo '(no unbounded loop form in this crate)'
# control: the identical expression over ring_publish, whose `publish` is one
grep -vE "^[[:space:]]*//" ring_publish/src/lib.rs \
  | grep -E "^[[:space:]]*loop[[:space:]]*(\{|$)|while[[:space:]]+true"
```

Live output:

```
(no unbounded loop form in this crate)
    loop
```

**Expected: no match in this crate.** The control runs the identical expression
over `ring_publish` and returns its `loop` line, so the empty result above is a
measured absence rather than a pattern that never fires — which is exactly what
it used to be, and what WT53 below records.

The manual plan is explicit about why this is a *reading* rather than a test:
a test cannot assert the absence of a `loop {}` — it can only time out. And a
timeout-based test is the worst kind: it passes on a fast machine, fails on a
loaded CI box, and tells you nothing about which of those happened.

Note the filter. `^[[:space:]]*//` drops `///`, `//!` and plain `//` alike,
because this crate's `Park` arm carries a six-line ordinary comment mentioning
`thread::park`, and a filter that only dropped doc comments would read that
prose as code. The same filter appears in all six of this crate's manual checks,
and `tests/manual/readme.md:18-21` states the reason once for all of them.

### Why the Budget Is in the Loop Header

The invariant is stronger than "the loop terminates". `wait_until` could be
written to terminate with the counter in the body:

```rust
// not the shape used
let mut attempt = 0;
loop
{
  if ready() { return Ok( attempt ); }
  attempt += 1;
  if attempt >= spins { break; }
  if !pause( kind, attempt ) { break; }
}
```

Behaviourally identical, and one deleted line from unbounded. The `for` header
form cannot lose its bound by an edit that still compiles — removing
`0..spins.max( 1 )` leaves `for attempt in` with no iterator, which is a syntax
error.

That is what makes W1 a *structural* check rather than a stylistic one, and why
it greps for `loop {` rather than reasoning about termination.

`ring_poll` — which re-implements this crate's shape three times because it may
not depend on it — chose the counter-in-the-body form:

```rust
// ring_poll/src/lib.rs:289-315
let mut attempt = 0;
while attempt < budget.attempts()
{
  …
  attempt += 1;
  …
}
```

That is bounded and correct, and it is exactly the shape W1 exists to keep out
of this crate. Neither is wrong; the difference is that one of them survives an
edit that deletes a line and the other does not. W1's own pattern would not fire
on `while attempt < budget.attempts()` either — it looks for `while true` — so
the invariant is enforced here by writing the loop a particular way, not by a
grep that could catch every alternative.

### What the Invariant Does Not Cover

Three ways to build an unbounded wait out of this crate, none of which W1 sees:

| Escape | Example | Bounded by |
|--------|---------|-----------|
| A predicate that blocks | `wait_until( None, 1, \|\| { sleep( a_minute ); false } )` | nothing |
| A caller's own retry loop | `while wait( kind, p ).is_err() { }` | nothing |
| `usize::MAX` as the budget | `wait_until( Park, usize::MAX, p )` | ~70 million years, at the measured 120 µs/attempt |

The first two are the caller's loops, not this crate's, and the invariant is
scoped to *this crate* by construction. The third is inside the invariant and
useless anyway — a bound nothing will reach is a bound only in the formal sense.

`ring_poll` closes the second escape for the tick path by refusing the whole
dependency ([`integration/001`](../integration/001_two_dependencies_two_dependents_and_a_roster.md)),
and nothing closes the first anywhere.

### Why the Family Cares

The module documentation states the stake at `:41-46`:

> An unbounded wait on a ring whose producer has died is a hung thread with no
> diagnostic, and the `Park` strategy makes it a hung thread that never even
> burns CPU to show it.

That last clause is the sharp one. A spinning hang is visible in `top`; a
sleeping hang is a process at 0% CPU that looks healthy. Returning
`RingError::Empty` hands the caller a decision it can act on, and the whole
point of the budget is that the decision *arrives*.

`ring_publish` takes the opposite position deliberately and documents its
reasoning (`ring_publish/src/lib.rs:42-53`): its `loop {}` waits on a
predecessor that has already committed to finishing, so there is no failure mode
where the wait does not end. The distinction between the two crates is not
discipline, it is what is being waited *for*.


### WT33 — "Panics: Never" Is Stated of a Function Whose Body Is a Caller's Closure

`wait_until` documents a panic-freedom claim and gives its reason.

```sh
cd "$(git rev-parse --show-toplevel)"
grep -A3 '# Panics' ring_wait/src/lib.rs
# what the function actually runs
grep 'ready()\|F : FnMut' ring_wait/src/lib.rs
# and whether anything catches an unwind
grep -c 'catch_unwind\|panic::' ring_wait/src/lib.rs || echo '0 — nothing catches'
```

Live output:

```
/// # Panics
///
/// Never. The budget is a `usize` count and the loop is bounded by it.
pub fn wait_until< F >( kind : WaitKind, spins : usize, mut ready : F ) -> Result< usize, RingError >
  F : FnMut() -> bool,
    if ready()
  F : FnMut() -> bool,
0
0 — nothing catches
```

The text is *"Never. The budget is a `usize` count and the loop is bounded by
it."* The justification is exact and it justifies a narrower claim than the word
"never" makes. What is bounded is the loop; what runs inside it is `ready()`, a
caller-supplied `FnMut() -> bool` that the crate treats as its entire extension
point ([`api/002`](../api/002_the_predicate_is_the_parameter.md)). A predicate
that panics unwinds straight through `wait_until` with nothing to catch it.

By the ordinary rustdoc convention this is defensible: a `# Panics` section
documents panics the function itself raises, and a caller's closure panicking is
the caller's own event. The reason it is worth recording anyway is the shape of
this particular function. Its body is thirteen lines of which the interesting one
is somebody else's code, and it is the one item in the crate that other crates
call — three call sites, each passing a closure that reads atomics through
`ring_cursor` and `ring_barrier`. "Never" is the strongest word available, spent
on the function with the least of its own behaviour to make the promise about.

Nothing tests it either way, in this crate or in the two that call it.


### WT34 — The Budget Sweep Stops One Short of the Period It Would Reveal

The crate has exactly one test that sweeps a range of budgets, and its range ends
where the `Spin` arm's pattern begins to repeat.

```sh
cd "$(git rev-parse --show-toplevel)"
grep -A12 'fn the_budget_is_a_count_and_is_honoured_exactly' ring_wait/tests/wait_test.rs
# the modulus the pause arm folds attempt through
grep 'attempt % ' ring_wait/src/lib.rs
```

Live output:

```
fn the_budget_is_a_count_and_is_honoured_exactly()
{
  for budget in 1..8usize
  {
    let mut looks = 0;
    let _ = wait_until( WaitKind::Spin, budget, ||
    {
      looks += 1;
      false
    } );

    assert_eq!( looks, budget, "budget {budget} produced {looks} looks" );
  }
      for _ in 0..=( attempt % 8 )
```

The sweep is `for budget in 1..8usize` — an exclusive range, so budgets 1 through
7. The `Spin` arm emits `0..=( attempt % 8 )` hints, so attempt 8 is the first
one that folds back to attempt 0's behaviour.

The test is not looking for the fold. It asserts `looks == budget`: that the
budget is honoured exactly as a count, which is true at every budget and needs no
particular range to demonstrate. It is a correct test of its own question.

What it means is that the sawtooth WT5 records — a period-8 pattern called a
backoff by the source comment and by the test file, asserted by neither — is not
merely untested but sits one iteration outside the reach of the only test whose
loop would have run into it. A sweep of `1..=8` would cost nothing and would put
the wrap inside the test's range, though it would still take a different
assertion to notice it: `looks` counts predicate calls, and the fold is in hint
counts, which no test observes at all.

### Invariants

| File | Relationship |
|------|--------------|
| [002_none_looks_exactly_once.md](002_none_looks_exactly_once.md) | The behavioural invariant on the same loop |

### WT53 — The Loop Guard Could Not Match This Codebase's Brace Style

The check above asserts that no unbounded loop form appears in the crate. Until
this was measured, it asserted nothing: its pattern required a brace on the same
line as the keyword, and the family's style puts it on the next one.

```sh
cd "$(git rev-parse --show-toplevel)"
# the old pattern, run family-wide
grep -r --include=*.rs -E "loop[[:space:]]*\{|while[[:space:]]+true" . \
  | grep -v '/docs/' || echo '(the old pattern matches nothing in the family)'
# the corrected one, per crate
for c in ring_*/src/lib.rs; do
  n=$( grep -vE "^[[:space:]]*//" "$c" \
       | grep -cE "^[[:space:]]*loop[[:space:]]*(\{|$)|while[[:space:]]+true" )
  [ "$n" -gt 0 ] && printf '%-16s %s\n' "$( basename "$( dirname "$( dirname "$c" )" )" )" "$n"
  true
done
```

Live output:

```
(the old pattern matches nothing in the family)
ring_bench       5
ring_publish     1
```

The old pattern returns nothing anywhere in 33 crates. Not because the family has
no unbounded loops — the corrected pattern finds six, one in `ring_publish` and
five in `ring_bench` — but because every one of them is written `loop` on its own
line, which is this codebase's brace convention throughout.

Six matches are not six defects. Only one of the six is a wait: `ring_bench`'s
five are drains bounded by a finite ring rather than by a budget, and
`ring_publish`'s one waits on a predecessor already committed to finishing. This
invariant has an opinion about exactly one of the six, which is the reason a
guard that merely counted them would still have been the wrong instrument — what
it has to distinguish is what is being waited *for*, and no grep can see that.

So the guard passed for `ring_wait` for the same reason it would have passed for
`ring_publish`: it cannot fail. And `ring_publish` is not a hypothetical
counter-example — it is the one this instance's own next paragraph names, citing
`publish`'s bare `loop` at `src/lib.rs:200-210` as the family's deliberate
exception. The prose knew about the loop the check could not see.

The reading the check supports is still true. `ring_wait` has one `for` over a
counted range and nothing else, which the corrected pattern now confirms against
a control that fires. What was missing was any capacity to report otherwise, on a
crate whose central claim — every wait is bounded — is exactly what an unbounded
loop would falsify.

This is the third instrument in this crate's corpus found measuring something
narrower than its heading claims, after `PARKING_CRATES`' substring scan (WT10)
and its self-referential third entry (WT11). All three share a shape: a check
whose passing state is indistinguishable from a check that cannot run.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_one_loop_and_the_two_ways_out.md](../algorithm/001_one_loop_and_the_two_ways_out.md) | The loop, its exits, and WT17's twice-written clamp |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_predicate_is_the_parameter.md](../api/002_the_predicate_is_the_parameter.md) | The first escape — a closure this crate cannot inspect |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_budget_and_the_attempt_index.md](../data_structure/002_the_budget_and_the_attempt_index.md) | The budget, and why it counts looks rather than time |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_two_dependencies_two_dependents_and_a_roster.md](../integration/001_two_dependencies_two_dependents_and_a_roster.md) | The crate that closes the second escape by banning the edge |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_what_each_strategy_costs_per_attempt.md](../non_functional_requirement/002_what_each_strategy_costs_per_attempt.md) | What `spins × 8` actually costs, per strategy |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_the_backoff_that_resets_every_eight_attempts.md](../pitfall/002_the_backoff_that_resets_every_eight_attempts.md) | The inner loop's constant bound |

### Sources

| File | Relationship |
|------|--------------|
| `ring_publish/src/lib.rs:42-53,200-210` | The family's one deliberate unbounded *wait*, and its argument |
| `ring_bench/src/lib.rs:1092,1144,1176,1226,1262` | Five more bare `loop`s, all drains bounded by a finite ring rather than a budget |
| `ring_poll/src/lib.rs:289-315` | A bounded retry that may not use this crate |

### Tests

| File | Relationship |
|------|--------------|
| `tests/manual/readme.md` § W1 | The grep, and why it is a reading rather than a test |
| `tests/manual/readme.md:18-21` | The comment filter, stated once for all six checks |
| `tests/wait_test.rs:223-237` | Budgets 1 through 7, each honoured exactly |
| `tests/wait_test.rs:206-221` | A zero budget still looks once |
