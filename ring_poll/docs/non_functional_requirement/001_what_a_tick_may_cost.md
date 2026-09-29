# Non-Functional Requirement: What A Tick May Cost

### Scope

- **Purpose**: State the cost a caller is buying when they pass a `Budget`, and record where the stated unit and the actual work diverge.
- **Responsibility**: The per-attempt cost of each of the four operations, and the one place the crate can allocate.
- **In Scope**: What one attempt costs per helper, the batch inner loop, and `drain_up_to`'s writes into a caller-owned `Vec`.
- **Out of Scope**: Whether the cost has ever been measured (→ [`002`](002_the_cost_nobody_has_measured.md)); the latency the budget does not bound (→ [`../pitfall/001`](../pitfall/001_non_parking_is_not_bounded_latency.md)).

### The requirement

A tick-path helper must return in bounded work, without parking, so a system's
turn on the ring cannot overrun the frame that scheduled it. The crate meets the
parking half structurally — the module doc is explicit that between attempts the
helpers emit *"a CPU pause hint and nothing else — no yield, no sleep, no park"*
— and it meets the bounded half through `Budget`.

It is also honest about the limit, in the module doc's own words: *"Non-parking
is necessary and not sufficient. [`Budget::new`] lets a caller ask for a million
attempts; that never deadlocks and will still blow a frame budget."*

What none of that settles is the size of the unit being counted.

### What one attempt actually costs

| Helper | Loop bound | One attempt does | Cost of one attempt |
|---|---|---|---|
| `push_within` | `while attempt < budget.attempts()` | one `try_push` | 1 ring operation |
| `recv_within` | `while attempt < budget.attempts()` | one `try_recv` | 1 ring operation |
| `push_batch_within` | `while attempt < budget.attempts()` | one `try_push_batch` | **up to the ring's free space** |
| `drain_up_to` | `while taken < max` | one `try_recv` | 1 ring operation |

`ring_core::try_push_batch` is a loop of its own:

```rust
for record in records.by_ref()
{
  if self.try_push( record ).is_err() { break; }
  accepted += 1;
}
```

Nothing the caller passes to `push_batch_within` bounds it. It stops when the
ring refuses or the iterator runs dry, whichever comes first → PL33.

### Where the crate can allocate

The crate allocates nothing itself — no `Box`, no `String`, no internal buffer.
`Vec` appears three times: in `drain_up_to`'s signature, in `Tick::drain`'s, and
in the doctest that shows how to call it —

```rust
let mut out = Vec::new();     // in drain_up_to's doctest, the published example
…
out.push( record );           // in drain_up_to's body, the only write
```

`out` is the caller's. `Vec::push` reallocates when it is full, which copies
every element already in it. That is the one operation in the crate whose cost
is neither a ring operation nor bounded by anything in the signature, and the
example a caller copies starts it at capacity zero → PL34.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'budget-bounded loops:       %s\n' "$( command grep -c 'while attempt < budget.attempts()' ring_poll/src/lib.rs || true )"
printf 'the other loop bound:       %s\n' "$( command grep -oE 'while taken < max' ring_poll/src/lib.rs )"
printf 'pause hint per loop:        %s\n' "$( command grep -c 'spin_loop' ring_poll/src/lib.rs || true )"
printf 'sleep/yield/park calls:     %s\n' "$( command grep -cE 'thread::sleep|yield_now\(|::park\(' ring_poll/src/lib.rs || true )"
printf 'batch delegates to:         %s\n' "$( command grep -oE 'producer\.try_push_batch\( records \)' ring_poll/src/lib.rs )"
printf 'and that fn loops on:       %s\n' "$( command grep -oE 'for record in records.by_ref\(\)' ring_core/src/lib.rs )"
printf 'what stops it:              %s\n' "$( awk '/pub fn try_push_batch/{f=1} f&&/accepted$/{exit} f' ring_core/src/lib.rs | command grep -oE 'if self.try_push\( record \).is_err\(\)' )"
printf 'callers bounding it:        %s\n' "$( command grep -cE 'try_push_batch\( records, [a-z]' ring_poll/src/lib.rs || true )"
printf 'allocating types here:      %s\n' "$( command grep -oE 'Box|String|VecDeque|HashMap' ring_poll/src/lib.rs | sort -u | tr '\n' ' ' )"
printf 'Vec mentions:               %s\n' "$( command grep -c 'Vec' ring_poll/src/lib.rs || true )"
printf 'and the write into it:      %s\n' "$( command grep -oE 'out\.push\( record \)' ring_poll/src/lib.rs )"
printf 'with_capacity in the crate: %s\n' "$( command grep -rc 'with_capacity' ring_poll/src ring_poll/tests 2>/dev/null | command grep -v ':0$' | wc -l )"
printf 'docs mentioning presizing:  %s\n' "$( command grep -rciE 'with_capacity|presiz|reserve' ring_poll/src/lib.rs || true )"
printf 'doc sections on that unit:  %s\n' "$( command grep -cE '^/// # (An Attempt Is Not The Same Size Everywhere|The Budget Bounds Rounds, Not Records)' ring_poll/src/lib.rs || true )"
printf 'what the batch row says:    %s\n' "$( command grep -m1 -oE 'one whole .try_push_batch. — up to the ring.s free space' ring_poll/src/lib.rs | tr -d '\140' )"
```

Live output:

```
budget-bounded loops:       3
the other loop bound:       while taken < max
pause hint per loop:        3
sleep/yield/park calls:     0
batch delegates to:         producer.try_push_batch( records )
and that fn loops on:       for record in records.by_ref()
what stops it:              if self.try_push( record ).is_err()
callers bounding it:        0
allocating types here:      
Vec mentions:               3
and the write into it:      out.push( record )
with_capacity in the crate: 0
docs mentioning presizing:  0
doc sections on that unit:  2
what the batch row says:    one whole try_push_batch — up to the ring's free space
```

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [002_the_cost_nobody_has_measured.md](002_the_cost_nobody_has_measured.md) | Whether any of the costs above has ever been put on a clock |

### Invariants

| File | Relationship |
|------|--------------|
| [`../invariant/002_a_budget_bounds_attempts_not_time.md`](../invariant/002_a_budget_bounds_attempts_not_time.md) | PL23 — the same budget, read as a correctness boundary rather than a cost one |

### Pitfalls

| File | Relationship |
|------|--------------|
| [`../pitfall/001_non_parking_is_not_bounded_latency.md`](../pitfall/001_non_parking_is_not_bounded_latency.md) | The caller who raises the budget and pays in wall-clock |
| [`../pitfall/002_a_bigger_budget_is_not_a_bigger_batch.md`](../pitfall/002_a_bigger_budget_is_not_a_bigger_batch.md) | The count side of PL33's cost side |

### Lifecycles

| File | Relationship |
|------|--------------|
| [`../lifecycle/002_where_a_record_can_end_up.md`](../lifecycle/002_where_a_record_can_end_up.md) | PL31 — what the same batch inner loop destroys while it runs |

### Items

| File | Relationship |
|------|--------------|
| [`../item/002_what_the_crate_does_not_declare.md`](../item/002_what_the_crate_does_not_declare.md) | PL28 — the missing `#[ inline ]` on the accessors this path calls |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | The four loops, the pause hint, and the one `out.push` |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/poll_test.rs`](../../tests/poll_test.rs) | `a_large_budget_spins_rather_than_sleeping` — the only test that bounds cost at all |

### PL33 — the budget counts attempts, and an attempt is not one size

`Budget` is documented as bounding attempts, and the loop in every helper that
takes one is `while attempt < budget.attempts()`. For `push_within` and
`recv_within` that is a genuine cost bound: one attempt is one `try_push` or one
`try_recv`, so `Budget::new( n )` buys at most `n` ring operations.

`push_batch_within` runs the same loop and one of its attempts is
`producer.try_push_batch( records )`, which is itself
`for record in records.by_ref() { if try_push fails break; }`. That inner loop
takes no limit from `push_batch_within` and none from `Budget` — it ends when the
ring refuses or the iterator is exhausted. Its length is bounded by the ring's
free space, which is a runtime property of a structure shared with other threads.

So `Budget::once()` — the default, and the value the module documentation
recommends precisely because it is the conservative one — buys one ring operation
from `push_within` and up to a whole ring's worth from `push_batch_within`. The
two calls look identical at the call site and differ in cost by the ring's
capacity.

This is the right design. Batching exists to amortise, and a batch that stopped
at one record per attempt would be `push_within` with extra steps; `Tick::drain`'s
own doc comment already makes exactly this argument for keeping a drain limit
separate from a budget. The finding is that the argument was made for `drain` and
not for `push_batch`, so the crate has a considered exemption in one place and an
unstated one in the other.

What a caller needed and did not have was a sentence saying the budget bounds
*rounds*, not *work*, and that for the batch helper the work per round is the
ring's free space. `docs/pitfall/002` covers the adjacent misreading — that a
bigger budget moves more records — but from the count side; nothing stated the
cost side. Presizing the ring is what actually bounds it, and no document
connected `RingConfig`'s capacity to the tick-path cost it silently determines.

**Disposition:** applied — two doc sections were added to `src/lib.rs`. `Budget`
carries `# An Attempt Is Not The Same Size Everywhere`, a table giving what one
attempt costs per helper, whose `push_batch_within` row states the cost as one
whole `try_push_batch` bounded by the ring's free space rather than by anything
the caller passed. `push_batch_within` itself carries
`# The Budget Bounds Rounds, Not Records`, which says the same thing at the call
site a reader is actually looking at, and points at `RingConfig`'s capacity as
the value that determines the per-round cost. Both are prose rather than a
signature change: bounding the inner loop would turn batching into `push_within`
with extra steps, which is the design the finding explicitly endorses. The row a
caller now reads is: Now prints: `one whole try_push_batch — up to the ring's free space`

### PL34 — the one unbounded allocation sits on the path that exists to bound cost

The crate allocates nothing. No `Box`, no `String`, no internal buffer, and its
only mention of `Vec` is `drain_up_to`'s `out : &mut Vec< T >` parameter — which
it writes to, once, with `out.push( record )`.

`Vec::push` is amortised O(1) and worst-case O(n): when the vector is at
capacity, it allocates a larger buffer and moves every element already stored.
For a drain of `max` records into a vector that starts empty, that is a sequence
of reallocations at the growth points, each one an allocator call and a copy —
on a code path whose stated reason for existing is that a system's turn on the
ring must not overrun its frame.

The cost is the caller's to control and trivially controlled: `Vec::with_capacity( max )`
before the loop makes every `push` a store. That string appears nowhere in the
crate — not in `src`, not in `tests`, not in a doc comment — while
`drain_up_to`'s own doctest opens with `let mut out = Vec::new();`, so the one
worked example the crate publishes is the one that starts at capacity zero and
reallocates its way up. A caller copying the example inherits the growth
sequence; nothing they read suggests the destination's shape is theirs to decide.

It is a small thing next to a `Park`, which is what the crate is actually
defending against, and that is the reason to write it down rather than the reason
not to: the crate's whole argument is that a bounded, non-parking path is worth
building deliberately, and this is the one call in it whose cost is neither
bounded nor deterministic. A caller who has taken the argument seriously enough
to use `Budget::once()` has earned the sentence about presizing `out`.
