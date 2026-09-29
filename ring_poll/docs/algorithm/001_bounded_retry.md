# Algorithm: Bounded Retry

### Scope

- **Purpose**: Record the loop shape shared by `push_within`, `push_batch_within` and `recv_within`, and the one place the three deliberately differ.
- **Responsibility**: The shape, the three decisions inside it that are decisions rather than defaults, the batch helper's extra exit, and why `drain_up_to` is a different shape entirely.
- **In Scope**: Retry structure and its termination; the reasoning behind the early exit.
- **Out of Scope**: What a caller should pass as a budget (→ [`../invariant/002`](../invariant/002_a_budget_bounds_attempts_not_time.md)); the misreadings the early exit invites (→ [`../pitfall/002`](../pitfall/002_a_bigger_budget_is_not_a_bigger_batch.md)).

### Abstract

Three of the crate's four helpers run one loop: try, and on failure spin briefly
and try again, up to a budget fixed before the loop starts. The fourth,
`drain_up_to`, looks similar and is not — its ceiling counts successes rather
than attempts, and conflating the two would change what `Budget::once()` means.

### Algorithm

```
attempt ← 0
while attempt < budget.attempts()
  try the operation
  if it succeeded, return the success
  attempt ← attempt + 1
  if attempt < budget.attempts()
    spin_loop()
return the failure
```

Three things about it are decisions rather than defaults:

**The pause is inside the guard.** A `spin_loop()` after the *last* failed
attempt burns time on the way out for nothing — the function has already decided
to give up. Putting the hint under `if attempt < attempts` costs one comparison
and removes that tail.

**The pause is `spin_loop` and not `yield_now`.** Yielding would be measurably
better under oversubscription and is the operation this crate exists to keep off
the tick path ([`../invariant/001`](../invariant/001_no_parking_operation_on_the_tick_path.md)).
A caller who wants it is outside the tick and should be using `ring_wait`.

**The loop is `while` and not `loop`.** `llvm-cov` opens a coverage region on a
bare `loop` keyword and never attributes a hit to it, so the line reads as
uncovered however hard the tests exercise it — measured in `ring_shutdown` at
72/73 with `loop` against 73/73 with `while`, for an identical suite
([`ring_shutdown/docs/algorithm/001`](../../../ring_shutdown/docs/algorithm/001_drain_to_empty.md)).
This crate was written with that already known, so there is no before-and-after
measurement here; it inherits the finding rather than re-deriving it.

#### Where the batch version differs, and why

`push_batch_within` adds one rule the other two do not have: **an attempt that
moves zero records ends the loop, whatever the budget says.**

The reasoning is about what a retry is *for*. A retry does not make this thread
better at pushing; the ring's state does not depend on how many times it is
asked. What a retry buys is elapsed time in which *another* thread might drain a
slot. So:

| Situation | Second attempt is worth | Because |
|---|---|---|
| First attempt moved some records, then stopped | Something | The ring is full *now*, but a consumer is plainly active — it just took some |
| First attempt moved nothing | Nothing measurable | Nothing about this thread's next attempt differs, and there is no evidence a consumer is running at all |

The second row is where the early exit comes from. It is a heuristic, not a
proof — a consumer could be about to drain — and it is stated as one here rather
than presented as an optimisation with a guarantee behind it.

The consequence for a caller is worth knowing: `push_batch_within` with a budget
of 9 against an empty-of-space ring costs *one* ring operation, not nine. A
caller relying on "budget 9 means it tried nine times" would be wrong, and
[`../pitfall/002`](../pitfall/002_a_bigger_budget_is_not_a_bigger_batch.md) says
so.

#### `drain_up_to` is not this shape

It is a bounded *drain*, not a bounded *retry* — its ceiling counts successes,
and its exit condition is an empty read rather than an exhausted budget:

```
taken ← 0
while taken < max
  record ← try_recv()
  if none, break
  push record to out; taken ← taken + 1
return taken
```

Keeping the two shapes distinct is why `Tick::drain` takes its own `max` rather
than reusing the tick's budget. Collapsing them would make `Budget::once()` mean
"take at most one record per tick", which is not what one attempt means anywhere
else on the surface.

### Algorithms

| File | Relationship |
|------|--------------|
| [`ring_shutdown/docs/algorithm/001`](../../../ring_shutdown/docs/algorithm/001_drain_to_empty.md) | Where the `while`-over-`loop` coverage finding was measured; this crate inherits it |

### APIs

| File | Relationship |
|------|--------------|
| [`../api/001_tick_path_surface.md`](../api/001_tick_path_surface.md) | The four helpers this shape covers, and the one it does not |

### Invariants

| File | Relationship |
|------|--------------|
| [`../invariant/001_no_parking_operation_on_the_tick_path.md`](../invariant/001_no_parking_operation_on_the_tick_path.md) | Why the pause is `spin_loop` rather than `yield_now` |
| [`../invariant/002_a_budget_bounds_attempts_not_time.md`](../invariant/002_a_budget_bounds_attempts_not_time.md) | The termination guarantee this loop structure establishes |

### Pitfalls

| File | Relationship |
|------|--------------|
| [`../pitfall/002_a_bigger_budget_is_not_a_bigger_batch.md`](../pitfall/002_a_bigger_budget_is_not_a_bigger_batch.md) | The caller-facing consequence of the early exit |

### Types

| File | Relationship |
|------|--------------|
| [`../type/001_budget_clamps_to_one.md`](../type/001_budget_clamps_to_one.md) | Guarantees the loop bound is at least one, so there is no zero-iteration case |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | `push_within`, `push_batch_within`, `recv_within` share the shape; `drain_up_to` does not |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/poll_test.rs`](../../tests/poll_test.rs) | `push_batch_within*` cover the early exit on both sides; `drain_up_to*` cover the distinct shape; `a_multi_attempt_budget_spends_every_attempt_before_giving_up` covers the base loop |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_poll
printf 'spin_loop call sites:        %s\n' "$( command grep -c 'spin_loop()' src/lib.rs || true )"
printf 'bare loop keywords:          %s\n' "$( command grep -cE '^ *loop$' src/lib.rs || true )"
printf 'while loops:                 %s\n' "$( command grep -cE '^ *while ' src/lib.rs || true )"
printf 'free helpers in total:       %s\n' "$( command grep -cE '^pub fn [a-z_]+<' src/lib.rs || true )"
printf 'of those taking a budget:    %s\n' "$( command grep -oE '^pub fn [a-z_]+' src/lib.rs | while read -r _ _ n; do awk -v n="$n" '$0 ~ "^pub fn "n"<"{f=1} f && /^\)$/{exit} f' src/lib.rs | command grep -q 'budget : Budget' && printf '%s ' "$n"; done )"
printf 'and the one that does not:   %s\n' "$( command grep -oE '^pub fn [a-z_]+' src/lib.rs | while read -r _ _ n; do awk -v n="$n" '$0 ~ "^pub fn "n"<"{f=1} f && /^\)$/{exit} f' src/lib.rs | command grep -q 'budget : Budget' || printf '%s ' "$n"; done )"
printf 'Tick ops passing self.budget: %s\n' "$( command grep -cE ', self\.budget \)' src/lib.rs || true )"
printf 'Tick ops that move records:  %s\n' "$( awk '/^impl Tick/{f=1} f && /^}$/{exit} f' src/lib.rs | command grep -cE '^  pub fn [a-z_]+<' || true )"
printf 'the tick op with no budget:  %s\n' "$( awk '/^  pub fn drain</{f=1} f && /^  }$/{exit} f' src/lib.rs | command grep -oE 'drain_up_to\( [^)]*\)' )"
printf 'spin_loop in the test file:  %s\n' "$( command grep -c 'spin_loop' tests/poll_test.rs || true )"
printf 'what the tests do say:       %s\n' "$( command grep -ohE 'spin-time|spin hint|spinning|sleeping pause' tests/poll_test.rs | sort -u | tr '\n' ' ' )"
```

Live output:

```
spin_loop call sites:        3
bare loop keywords:          0
while loops:                 4
free helpers in total:       4
of those taking a budget:    push_within push_batch_within recv_within 
and the one that does not:   drain_up_to 
Tick ops passing self.budget: 3
Tick ops that move records:  4
the tick op with no budget:  drain_up_to( consumer, out, max )
spin_loop in the test file:  0
what the tests do say:       sleeping pause spin hint spinning spin-time 
```

### PL1 — the loop's three stated decisions have one test between them

The Algorithm section names three choices as decisions rather than defaults: the
pause sits inside the `attempt < attempts` guard, the pause is `spin_loop` rather
than `yield_now`, and the loop is `while` rather than `loop`. All three are
correct and all three are reasoned.

Only the second is checkable from the suite, and only indirectly:
`a_large_budget_spins_rather_than_sleeping` bounds 20 000 attempts under 500 ms,
which a `yield_now` would likely also pass and a `sleep` would not — so it
discriminates parking from not-parking, not spinning from yielding.

The other two have nothing. No test observes that the last attempt skips the
pause, and the identifier `spin_loop` does not appear in the test file at all —
what appears is the prose of that one timing test, which reasons about *spin-time*
and *sleeping pause* without naming the call. Nothing could observe the
`while`-versus-`loop` choice either, since it is a coverage-instrument property
rather than a behaviour — the document says as much and points at `ring_shutdown`
for the 72/73-versus-73/73 measurement.

This is not an argument for writing those tests. The pause placement costs one
comparison and buys a tail that no caller can time, and a test for it would
assert an implementation detail. It is worth recording because three claims
presented in one list with one shared justification style have unequal evidence
behind them, and the document does not distinguish them.

### PL2 — the fourth helper drops the retry rule, not just the counting rule

The closing section explains why `drain_up_to` keeps its own `max` instead of
reusing the budget: a budget bounds retries of a failed operation, a drain limit
bounds successes, and collapsing them would make `Budget::once()` mean "one
record per tick". That reasoning is right.

What it does not say is that `drain_up_to` then has **no retry behaviour at all**.
The other three helpers spin and try again on an empty or full ring; this one
breaks at the first `None`. `spin_loop` appears at three call sites in `src/`,
and the drain is the one that has none.

`Tick` inherits the asymmetry. Three of its four operations pass `self.budget`
down; `drain` passes `max` and nothing else. So a `Tick::new( Budget::new( 100 ) )`
retries a hundred times on `push` and `recv`, and zero times on `drain` — against
a ring a producer is actively filling, `recv` can succeed on attempt 40 where
`drain` returns 0 and reports no progress.

That is a defensible design: a drain is a bulk operation and an empty ring is a
real answer. But the tick's budget reads as a property of the tick, `Tick::budget`
is a public accessor returning it, and the one operation it does not govern is
not named anywhere on the type.
