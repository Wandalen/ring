# Invariant: A Budget Bounds Attempts, Not Time

### Scope

- **Purpose**: State that every helper in this crate returns after at most `budget.attempts()` ring operations, and that nothing here bounds how long that takes.
- **Responsibility**: Keep the termination claim and the latency claim apart, and record why the weaker one is still worth stating.
- **In Scope**: The bound the loop structure establishes, the measurement that distinguishes spinning from sleeping, and what the distinction leaves to the caller.
- **Out of Scope**: Why no operation parks in the first place (→ [`001_no_parking_operation_on_the_tick_path.md`](001_no_parking_operation_on_the_tick_path.md)); what a caller should do about a large budget (→ [`../pitfall/002`](../pitfall/002_a_bigger_budget_is_not_a_bigger_batch.md)).

### Invariant Statement

Every helper in this crate that takes a `Budget` returns after at most
`budget.attempts()` ring operations. Nothing here bounds how long that takes.

The qualifier is load-bearing: three of the four free functions take a budget and
`drain_up_to` does not, because it bounds successes rather than retries. The
statement was written without it → PL23.

#### The two statements, kept apart

| Statement | Holds? | Established by |
|---|---|---|
| Every call returns | **Yes**, unconditionally | The loop is `while attempt < budget.attempts()`, and `attempts()` is a finite `usize` fixed before the loop starts |
| Every call returns *soon* | **No** | `attempts()` is whatever the caller passed |

Collapsing these is the mistake this crate's non-parking rule makes easy to make, because
"non-blocking" is usually said as though it meant "fast". It means the thread is
never descheduled — a different and weaker property.

### Enforcement Mechanism

**The upper statement is structural.** Every loop in the crate is bounded by a
`usize` read before it starts, and `Budget`'s constructor guarantees that
`usize` is at least one
(→ [`../type/001`](../type/001_budget_clamps_to_one.md)), so there is no
zero-iteration case and no unbounded one.

**The lower statement is measured, not enforced**, and the measurement is what
separates a spinning implementation from a sleeping one. A spin costs
nanoseconds per attempt; `ring_wait`'s `Park` sleeps 50µs per attempt. Over
20 000 attempts that is the difference between single-digit milliseconds and
about one second — three orders of magnitude, which is enough that a wall-clock
assertion can tell them apart without being flaky:

The test bounds 20 000 attempts at 500 ms: 2× under what a sleeping
implementation would cost, and roughly 100× over what the spinning one does. The
upper margin is written into the failure message; the lower one is not, and the
difference matters more than it looks → PL24.

### Violation Consequences

**The failure this converts, and the failure it leaves behind, are different in
kind.** The failure mode `invariant/001`'s non-parking rule exists to prevent is a deadlock: a tick
that parks on a full ring and never wakes. That failure is *unrecoverable* — no
later tick runs. A tick that spins 1 000 000 times is a dropped frame: bad,
visible, recoverable, and gone by the next tick.

So the invariant is worth having even though it is weaker than it sounds. It
converts an unbounded failure into a bounded one. What it does not do is make
the bounded one acceptable, which is
[`../pitfall/001`](../pitfall/001_non_parking_is_not_bounded_latency.md)'s subject.

#### What follows for callers

`Budget::once()` is the default, and it is the only budget whose cost is a
property of this crate rather than of the caller. Anything above one is a
latency decision the caller is making, and
[`../api/001`](../api/001_tick_path_surface.md) marks it as such in the surface
table rather than hiding it behind a plausible-looking parameter.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_poll
printf 'free functions:            %s\n' "$( command grep -oE '^pub fn [a-z_]+' src/lib.rs | sed 's/^pub fn //' | tr '\n' ' ' )"
printf 'of those, taking a Budget: %s\n' "$( command grep -oE '^pub fn [a-z_]+' src/lib.rs | sed 's/^pub fn //' | while read -r n; do awk -v n="$n" '$0 ~ "^pub fn "n"$" || $0 ~ "^pub fn "n"<" {f=1} f{print} f&&/^\{$/{exit}' src/lib.rs | command grep -q 'Budget' && printf '%s ' "$n"; done )"
printf 'loop bounds in the file:   %s\n' "$( command grep -oE 'while [a-z]+ < [a-z_.()]+' src/lib.rs | sort | uniq -c | tr '\n' ' ' | tr -s ' ' )"
printf 'the one not on a budget:   %s\n' "$( command grep -oE 'while [a-z]+ < [a-z_.()]+' src/lib.rs | command grep -v 'budget' | sort -u )"
printf 'and what bounds it:        %s\n' "$( awk '/^pub fn drain_up_to/{f=1} f&&/^\{/{exit} f' src/lib.rs | command grep -oE '[a-z_]+ : usize' )"
printf 'timing test asserts:       %s\n' "$( awk '/fn a_large_budget_spins/{f=1} f&&/^\}$/{exit} f' tests/poll_test.rs | command grep -c 'assert' || true )"
printf 'its wall-clock ceiling:    %s\n' "$( awk '/fn a_large_budget_spins/{f=1} f&&/^\}$/{exit} f' tests/poll_test.rs | command grep -oE 'Duration::from_millis\( [0-9]+ \)' )"
printf 'attempts it spends:        %s\n' "$( awk '/fn a_large_budget_spins/{f=1} f&&/^\}$/{exit} f' tests/poll_test.rs | command grep -oE 'Budget::new\( [0-9_]+ \)' )"
printf 'margins in its messages:   %s\n' "$( awk '/fn a_large_budget_spins/{f=1} f&&/^\}$/{exit} f' tests/poll_test.rs | command grep -oE 'would cost about [0-9.]+ s|spinning[^"]*' | tr '\n' ' ' )"
printf 'sleep length in ring_wait: %s\n' "$( cd .. && command grep -oE 'from_micros\( [0-9]+ \)' ring_wait/src/lib.rs )"
```

Live output:

```
free functions:            push_within push_batch_within recv_within drain_up_to 
of those, taking a Budget: push_within push_batch_within recv_within 
loop bounds in the file:    3 while attempt < budget.attempts() 1 while taken < max 
the one not on a budget:   while taken < max
and what bounds it:        max : usize
timing test asserts:       3
its wall-clock ceiling:    Duration::from_millis( 500 )
attempts it spends:        Budget::new( 20_000 )
margins in its messages:   would cost about 1.0 s 
sleep length in ring_wait: from_micros( 50 )
```

### APIs

| File | Relationship |
|------|--------------|
| [`../api/001_tick_path_surface.md`](../api/001_tick_path_surface.md) | The "who chose it" column exists because of this invariant; guarantee 2 is its structural half |

### Algorithms

| File | Relationship |
|------|--------------|
| [`../algorithm/001_bounded_retry.md`](../algorithm/001_bounded_retry.md) | The loop whose bound this invariant states |

### Invariants

| File | Relationship |
|------|--------------|
| [`001_no_parking_operation_on_the_tick_path.md`](001_no_parking_operation_on_the_tick_path.md) | The stronger sibling. This one is what remains true once that one is granted |

### Pitfalls

| File | Relationship |
|------|--------------|
| [`../pitfall/001_non_parking_is_not_bounded_latency.md`](../pitfall/001_non_parking_is_not_bounded_latency.md) | The consequence of collapsing the two statements above |
| [`../pitfall/002_a_bigger_budget_is_not_a_bigger_batch.md`](../pitfall/002_a_bigger_budget_is_not_a_bigger_batch.md) | The second misreading a budget parameter invites |

### Types

| File | Relationship |
|------|--------------|
| [`../type/001_budget_clamps_to_one.md`](../type/001_budget_clamps_to_one.md) | Bounds the budget below at one; explicitly does not bound it above, which is why this invariant is needed |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | Every `while attempt < budget.attempts()` loop — the bound is read once, before the loop |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/poll_test.rs`](../../tests/poll_test.rs) | `a_large_budget_spins_rather_than_sleeping` is the wall-clock separator with both margins written down; `a_multi_attempt_budget_spends_every_attempt_before_giving_up` covers the attempt count itself |


### PL23 — "every helper" covers a helper that has no budget

The invariant was stated over every helper in the crate. One of the four free
functions does not take a `Budget` at all: `drain_up_to( consumer, out, max )` is
bounded by `while taken < max`, and `max` is a caller-supplied `usize` with none
of `Budget`'s clamping.

The exemption is deliberate and the source says so plainly. `Tick::drain`'s doc
comment reads: *"The ceiling is `max` rather than the budget: a budget bounds
retries of a failed operation, a drain limit bounds successes. Collapsing the two
would make `Budget::once()` mean 'take at most one record per tick', which is not
what a single attempt means anywhere else here."* That is the right call, well
argued, and written down.

It is written down in `src/`, and the invariant that generalises over the same
four functions was written without it. Nothing checks the two against each other,
so the file that states the crate's termination property claims a bound for a
function that does not have one, while the function's own doc explains why it
must not.

The correction is a qualifier, applied above. What the finding is really about is
that a whole-crate invariant and a per-function exemption are the two documents
most likely to contradict each other and the two least likely to be read
together — the exemption lives where the code is, the generalisation lives where
the claims are, and the drift is invisible from either end.

`drain_up_to` does still terminate, for a different reason: it breaks on the
first `None`, so it is bounded by ring contents as well as by `max`. That is a
stronger guarantee than the budget helpers have, which is the other half of why
nobody noticed — the exempt function is the safe one.

### PL24 — the margin that would catch a regression is the one not written down

The timing test asserts `elapsed < 500 ms` for 20 000 attempts, with the message
*"a sleeping pause would cost about 1.0 s"*. That documents the upper margin:
the ceiling is 2× below what a `Park`-based implementation would take, so the
test fails if the crate starts sleeping.

The lower margin is not in the test. A spin of 20 000 attempts costs single-digit
milliseconds, which is roughly 100× under the ceiling, and nothing records that.
This file did claim *"both margins are stated in the test"*, which was not true of
the test as written.

The asymmetry is the point. The written margin catches the failure the test was
built for — parking sneaking onto the tick path. The unwritten one is what would
tell a future reader whether a run at 400 ms is fine or a two-orders-of-magnitude
regression that happens to still pass. A pure-spin loop drifting from 5 ms to
400 ms is a real defect, and this assertion is green for all of it.

That gap also swallows build-profile differences. The 100× headroom is a debug
build's headroom or a release build's, and the test does not say which it was
measured on; a wall-clock assertion whose only stated margin is the far one gives
a slow machine 80× of slack before anyone learns anything from a failure.

Recording the lower bound in the message — or asserting it — would cost one line.
The claim that it was already there was the more expensive error, because it made
the gap unlookable-for.
