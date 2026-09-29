# Lifecycle: The Escalation Ladder Nobody Climbs

### Scope

- **Purpose**: Read `escalation_hint` as the crate's second lifecycle — a strategy's progression from cheap-latency to cheap-CPU — and record that it has no caller anywhere.
- **Responsibility**: Give the ladder, the three tests that pin it, the two different meanings of its `None`, and the missing input that keeps it unused.
- **In Scope**: `escalation_hint` at `:65-91`.
- **Out of Scope**: The lifecycle of one wait — see [`lifecycle/001`](001_a_wait_from_the_first_look_to_one_of_two_endings.md).

### The Ladder

```rust
// ring_wait/src/lib.rs:83-91
pub const fn escalation_hint( kind : WaitKind ) -> Option< WaitKind >
{
  match kind
  {
    WaitKind::Spin => Some( WaitKind::Yield ),
    WaitKind::Yield => Some( WaitKind::Park ),
    WaitKind::Park | WaitKind::None => None,
  }
}
```

| From | To | Depth to the top |
|------|-----|-----------------:|
| `Spin` | `Yield` | 2 |
| `Yield` | `Park` | 1 |
| `Park` | — | 0 |
| `None` | — | 0 |

Three arms for four variants — `Park` and `None` share one, which is the first
sign that the two `None` returns are not the same fact
([`item/001`](../item/001_the_four_arms_of_the_pause.md) covers the four-arm
`pause` that this one is usually mistaken for).

The ordering is by what the wait is spending: `Spin` spends a core to buy
nanoseconds, `Yield` spends a scheduling slot, `Park` spends latency to buy the
core back. The doc comment says which direction is which (`:68-71`):

> [`WaitKind::Spin`] burns a core, which is right for a wait measured in
> nanoseconds and wrong for one measured in milliseconds. A caller that knows
> its own latency budget picks the strategy; this is the hint for one that does
> not.

### Two `None`s, Two Meanings

The function's own doctest is where the distinction is written down, in the
assertion messages rather than in prose (`:79-80`):

```rust
assert_eq!( escalation_hint( WaitKind::Park ), None, "nothing cheaper to escalate to" );
assert_eq!( escalation_hint( WaitKind::None ), None, "None never waits, so never escalates" );
```

| Input | `None` means |
|-------|--------------|
| `Park` | the ladder is finished — you are already at the cheapest-CPU rung |
| `None` | there is no ladder — this strategy never waited in the first place |

`tests/wait_test.rs:336-342` isolates the second case into its own test, and its
comment gives the stake rather than the behaviour:

> Escalating out of `None` would put a blocking strategy on the tick path, which
> is the one thing feature 173 names `None` to prevent.

So the shared arm is carrying a safety property, not a coincidence of both
returning `None`. A future edit that split the arm and gave `Park` something to
escalate to would have to leave `None` alone, and the test — not the arm — is
what says so.

### The Ladder Terminates, and That Is Asserted

```rust
// ring_wait/tests/wait_test.rs:344-359
for start in WaitKind::ALL
{
  let mut kind = start;
  let mut steps = 0;

  while let Some( next ) = escalation_hint( kind )
  {
    kind = next;
    steps += 1;
    assert!( steps <= WaitKind::ALL.len(), "escalation from {start:?} cycles" );
  }
}
```

This is the only place in the crate where a `while` loop exists at all, and it is
in a test rather than in `src/` — W1's grep covers only `src/lib.rs`
([`invariant/001`](../invariant/001_every_repetition_is_a_counted_for.md)).

It asserts a graph property: the transition relation has no cycle reachable from
any of the four starting points. A hint function that returned `Spin => Yield`
and `Yield => Spin` would be a perfectly plausible edit and would hang any caller
that trusted it; the test bounds the walk by `WaitKind::ALL.len()` and fails
rather than looping.

The other two escalation tests split the happy path from the safety case:

| Test | Lines | Asserts |
|------|-------|---------|
| `escalation_walks_from_cheapest_latency_to_cheapest_cpu` | `:328-334` | the three ordinary transitions |
| `none_never_escalates` | `:336-342` | `None`'s dead end, with the reason |
| `escalation_terminates_from_every_starting_point` | `:344-359` | no cycle from any start |

Three tests and four doctest assertions for a nine-line function with no caller.

### It Has No Caller

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell's ugrep shim, which searches in parallel and
# emits hits in completion order; `sort` pins the control's four lines, which
# are the only part of this pair with more than one line to order
command grep -r --include=*.rs 'escalation_hint' . | command grep -v '^ring_wait/' | LC_ALL=C sort \
  | command grep . || echo '(no reference outside ring_wait)'
# control: the identical expression for the item that is reached from outside
command grep -r --include=*.rs 'wait_until' . | command grep -v '^ring_wait/' | LC_ALL=C sort
```

Live output:

```
ring_types/src/policy.rs:  /// ones are `ring_wait`'s `escalation_hint` and `pause` in production source
ring_barrier/src/lib.rs:    ring_wait::wait_until( kind, spins, || self.admits( from, count ) )?;
ring_shutdown/src/lib.rs:    // `budget.max( 1 )` matches `ring_wait::wait_until`'s reading of its own
ring_shutdown/src/lib.rs:  let outcome = ring_wait::wait_until( kind, spins, ||
ring_shutdown/src/lib.rs:  ring_wait::wait_until( kind, spins, || shutdown.is_closed() )
```

**One match, and it is a doc comment.** The control below it — the identical
expression for `wait_until` — returns the three external call sites, which is
what makes this a measured absence rather than a failed grep. The single hit
outside this crate is `ring_types`' own prose, naming `escalation_hint` as an
example of a wildcard-free `match`; nothing *calls* it. Every other mention
is inside this crate — six in `src/lib.rs` (five of them doc comment and
doctest, one the declaration), six in its test file, two in the manual plan.
Nothing in `ring_barrier`, `ring_shutdown`, `ring_core`, or `ring_config`
names it at all.

That is not surprising, and the reason is structural rather than an oversight.

### The Missing Input

Escalation is the natural response to *"the budget ran out"*, and the budget
running out is exactly the case in which this crate returns least:

| The caller has | After `Ok( n )` | After `Err( Empty )` |
|----------------|-----------------|----------------------|
| an attempt count | yes — `n` | **no** |
| a reason | ready | budget exhausted *or* `None` refused to loop |
| the strategy it used | yes — it passed it | yes |

A caller wanting to escalate needs to know it exhausted the budget rather than
being told not to wait, and `Err( Empty )` does not distinguish them
([`algorithm/001`](../algorithm/001_one_loop_and_the_two_ways_out.md) — exits 2
and 3 return the same value from the same line). It also gets no attempt count
on the error path, so it cannot tell a near-miss from an immediate refusal.

`escalation_hint` therefore has the right shape for a caller the crate never
gives enough information to be
([`data_structure/002`](../data_structure/002_the_budget_and_the_attempt_index.md)).
It is a `const fn` over a one-byte enum with three tests, and it is complete,
correct, and unreachable in practice.

### Whether That Is a Problem

Two readings, and the honest answer is that it depends on something not yet
decided:

| Reading | Argument |
|---------|----------|
| It is dead weight | Nine lines and three tests for a function with no caller — YAGNI says delete it and let the first real escalating consumer bring it back |
| It is the cheap half of a real feature | The expensive half is a richer error, and having the ladder already written and tested means the expensive half arrives alone |

The second is the stronger argument only while an escalating consumer is
actually expected, and nothing points to one:

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell's ugrep shim, whose hit order varies run to
# run; `sort` pins the rest. The first search prints its hits rather than
# asserting emptiness — see the paragraph below the block for why it changed.
echo '  -- every ring task file naming escalation, and what it says there --'
command grep -r 'escalat' ring_*/task/ 2>/dev/null | LC_ALL=C sort
echo '  -- the family stored WaitKind, and what writes it --'
command grep 'wait : WaitKind' ring_config/src/lib.rs | LC_ALL=C sort
```

Live output:

```
  -- every ring task file naming escalation, and what it says there --
ring_poll/task/unverified/122_implement_ring_poll.md:(flat here, escalating in `ring_wait`), a difference the family's
ring_wait/task/unverified/107_implement_ring_wait.md:the same number of times on every machine); `escalation_hint` (the cost ladder
  -- the family stored WaitKind, and what writes it --
        wait : WaitKind::default(),
  pub const fn with_wait( mut self, wait : WaitKind ) -> Self
  wait : WaitKind,
```

**The first search used to be captioned "no ring crate's task set mentions
escalation" and to print nothing, and both halves of that have expired.** Two
task files now name escalation, and neither is what the caption was guarding
against. `ring_poll`'s describes its own backoff as "flat here, escalating in
`ring_wait`" — a sibling naming *this crate's* property in passing. This
crate's own task 107 names `escalation_hint` directly, in an implementation
record listing what was built. Both are descriptions of the ladder; neither is
a consumer proposing to climb it. The argument the search supports is
untouched, and the search's own result is no longer empty.

The distinction matters more than the correction does. A substring search for
`escalat` stands in for the concept "a consumer that intends to climb the
ladder", and the substring is the wider of the two: it also matches anyone
merely *describing* the ladder. The clearest demonstration of the gap is the
second hit, because it is this crate's own task file — the document furthest
from being an outside consumer is now indistinguishable, to this search, from
one. An emptiness assertion built on a search wider than its own claim survives
only until the first person writes the word for an unrelated reason, and here
the first person was the crate itself. Printing the hits and classifying them
is the form that does not expire, and it is why the recipe above shows what it
found instead of asserting there was nothing.

`RingConfig` stores exactly one `WaitKind`, at `ring_config/src/lib.rs:45`,
set by `with_wait` (`:89`) and read by `wait` (`:167`), with no field for a
second to escalate to and no method that changes it in place.

The finding to carry is the fact, not the verdict: **the ladder is fully
specified, fully tested, and has never been climbed.**


### WT41 — The Ladder's Top Rung Is the Arm That Costs the Most

`escalation_hint` walks `Spin → Yield → Park → None`. The test naming the
ordering calls it a walk from cheapest latency to cheapest CPU. The terminal rung
is the one WT7 measures as the most expensive per attempt.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -A6 'pub const fn escalation_hint' ring_wait/src/lib.rs
command grep 'fn escalation_walks_from_cheapest_latency_to_cheapest_cpu' -A6 \
  ring_wait/tests/wait_test.rs
```

Live output:

```
pub const fn escalation_hint( kind : WaitKind ) -> Option< WaitKind >
{
  match kind
  {
    WaitKind::Spin => Some( WaitKind::Yield ),
    WaitKind::Yield => Some( WaitKind::Park ),
    WaitKind::Park | WaitKind::None => None,
fn escalation_walks_from_cheapest_latency_to_cheapest_cpu()
{
  assert_eq!( escalation_hint( WaitKind::Spin ), Some( WaitKind::Yield ) );
  assert_eq!( escalation_hint( WaitKind::Yield ), Some( WaitKind::Park ) );
  assert_eq!( escalation_hint( WaitKind::Park ), None );
}

```

The ordering is right on its own terms. `Spin` holds a core and answers as fast
as the cursor changes; `Park` gives the core up entirely. Escalating means
trading latency for CPU, and the ladder is that trade in three steps.

The rung it ends on is a 50 µs `sleep`, which WT6 places at the far end of a
1700× cost span and WT7 measures at 112–119 µs actually delivered. So "cheapest
CPU" and "most expensive attempt" are the same arm, and both descriptions are
accurate — the first is about the core the waiter is not holding, the second
about the wall clock the caller is paying.

The ladder also terminates at `Park` rather than at `None`: `escalation_hint`
folds `Park` and `None` into one arm returning `None`, so a wait that escalates
all the way never arrives at the non-blocking variant. That is correct — a caller
who escalated because it was willing to wait longer should not be handed the
strategy that refuses to wait — and it means the four-variant enum is a
three-rung ladder with the fourth variant reachable only by choosing it outright.


### WT42 — The Most Heavily Tested Item Is the One With No Callers

Three of the file's twenty-four tests exist for `escalation_hint`. Nothing in
the family calls it.

```sh
cd "$(git rev-parse --show-toplevel)"
# tests naming the function, and total tests
grep '^fn .*escalation\|escalation_hint' ring_wait/tests/wait_test.rs
grep -c '#\[ test \]' ring_wait/tests/wait_test.rs
# every occurrence outside this crate's own test file
grep -r 'escalation_hint' --include=*.rs . --exclude-dir=docs \
  | grep -v 'ring_wait/tests/' | grep -v 'ring_wait/src/'
# The command above can legitimately exit nonzero -- a count of zero, a diff
# that differs -- and the sweep that keeps this output current skips any block
# that exits nonzero, so without this the block freezes instead of refreshing.
:
```

Live output:

```
use ring_wait::{ escalation_hint, for_data, for_space, pause, wait, wait_until, DEFAULT_SPINS };
fn escalation_walks_from_cheapest_latency_to_cheapest_cpu()
  assert_eq!( escalation_hint( WaitKind::Spin ), Some( WaitKind::Yield ) );
  assert_eq!( escalation_hint( WaitKind::Yield ), Some( WaitKind::Park ) );
  assert_eq!( escalation_hint( WaitKind::Park ), None );
  assert_eq!( escalation_hint( WaitKind::None ), None );
fn escalation_terminates_from_every_starting_point()
    while let Some( next ) = escalation_hint( kind )
24
ring_types/src/policy.rs:  /// ones are `ring_wait`'s `escalation_hint` and `pause` in production source
```

`escalation_walks_from_cheapest_latency_to_cheapest_cpu`, `none_never_escalates`,
and `escalation_terminates_from_every_starting_point` — three dedicated tests,
one of which walks the ladder to termination from all four starting points. The
last command returns a single line, and it is prose: outside
`tests/wait_test.rs` and its own definition, the only mention in the family is
a `ring_types` doc comment citing it as an example. Nothing calls it.

Three tests for a seven-line `const fn` is not over-testing. Termination from
every start is exactly the property a hint-returning ladder needs, and it is
cheap to check. The point is the distribution: the item with the most tests per
line is the item with the fewest callers, and `wait_until` — the one function
another crate's `src/` calls — has no test dedicated to it by name.

This matches what WT26 finds in the doctests from the other direction. Both
measurements land on the same shape: this crate's verification effort tracks how
easy an item is to test, not how much of the family depends on it.

### Lifecycles

| File | Relationship |
|------|--------------|
| [001_a_wait_from_the_first_look_to_one_of_two_endings.md](001_a_wait_from_the_first_look_to_one_of_two_endings.md) | The wait that would call this, and does not |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_one_loop_and_the_two_ways_out.md](../algorithm/001_one_loop_and_the_two_ways_out.md) | Why `Err` cannot tell an escalating caller what it needs |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_seven_items_and_the_one_with_a_caller.md](../api/001_seven_items_and_the_one_with_a_caller.md) | WT1 — this is one of the six with no external caller |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_budget_and_the_attempt_index.md](../data_structure/002_the_budget_and_the_attempt_index.md) | The attempt count that does not survive the error path |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_the_discriminants_live_in_ring_types.md](../decisions/002_the_discriminants_live_in_ring_types.md) | Why the enum is elsewhere and the ordering is here |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_every_repetition_is_a_counted_for.md](../invariant/001_every_repetition_is_a_counted_for.md) | The crate's only `while` loop, and why it does not violate W1 |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_four_arms_of_the_pause.md](../item/001_the_four_arms_of_the_pause.md) | The four-arm match this three-arm one is often confused with |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_one_return_type_and_the_one_must_use.md](../type/002_one_return_type_and_the_one_must_use.md) | WT8 — the crate's only `#[ must_use ]`, on this function |

### Sources

| File | Relationship |
|------|--------------|
| `ring_types/src/policy.rs:21-34,58` | The four discriminants and `ALL`, which the termination test iterates |
| `ring_config/src/lib.rs:45,89,167` | The one stored `WaitKind` in the family, with no second field |
| `ring_config/src/lib.rs:238-241` | `is_tick_safe` — the per-variant judgement a config makes and this crate does not |

### Tests

| File | Relationship |
|------|--------------|
| `tests/wait_test.rs:328-334` | The three ordinary transitions |
| `tests/wait_test.rs:336-342` | `None`'s dead end, and the tick-path reason |
| `tests/wait_test.rs:344-359` | No cycle from any starting point |
| `tests/manual/readme.md` § W2 | The two `RingError::` mentions this function is not one of |
