# Decision: A Plain Spin Rather Than a `WaitKind`

### Scope

- **Purpose**: Record why `publish` loops on a bare `spin_loop` hint with no strategy and no budget, in a family where every other wait takes both.
- **Responsibility**: State the argument, place it against the four crates that do take a `WaitKind` — including a Tier 5 sibling in the same feature — show what a budget's exhaustion would have to mean, and name what would reverse it.
- **In Scope**: The absence of a `WaitKind` parameter and of a spin budget on `publish`.
- **Out of Scope**: The loop's mechanics — see [`algorithm/002`](../algorithm/002_a_loop_with_no_budget.md).

### The Ruling

`src/lib.rs:42-53`:

> [`Publisher::publish`] loops on a `spin_loop` hint with no wait strategy and no
> budget, which everywhere else in this family would be a bug. Here it is the
> correct shape, and the difference is what is being waited *for*.
>
> Waiting for space is unbounded: it depends on a consumer that may be slow,
> stalled, or gone, so it needs a strategy and a give-up. Waiting for your
> predecessor to publish is bounded by that producer finishing a slot write it
> has already started and cannot abandon — it is not blocked on anything itself.
> A `WaitKind` here would offer a `Park` that can only ever hurt, and a budget
> whose exhaustion has no correct handling.

The argument is not "spinning is fast enough". It is that the two other design
elements — a strategy and a budget — have no meaning for this wait:

- **A strategy chooses how to yield the core.** The three non-trivial `WaitKind`
  arms are `Spin`, `Yield` and `Park`. `Yield` puts a scheduler round-trip
  between the predecessor's publication and this thread noticing it, on a wait
  whose expected duration is one slot write. `Park` — which in `ring_wait` is a
  50 µs sleep, not a real park
  (`ring_wait/src/lib.rs:134-141`) — puts a 50 µs floor on a wait that is
  typically nanoseconds. Both are strictly worse than `Spin`, so the choice is
  not a choice.
- **A budget needs an answer for exhaustion.** `wait_until` returns
  `Err( RingError::Empty )` when its `spins` run out, and the caller decides. Here
  the caller cannot: the range is claimed, the slots are written, and there is no
  way to un-publish, un-claim, or defer. The only correct action after a budget
  expires is to keep waiting — which is what having no budget does, without the
  ceremony.

### PB18 — Three Crates Wait, and Two of Them Take a Strategy

```sh
cd "$(git rev-parse --show-toplevel)"
for f in ring_*/src/*.rs; do
  h=$( grep -vE '^[[:space:]]*//' "$f" \
       | grep -cE 'spin_loop|yield_now|thread::sleep|::park' )
  [ "$h" -gt 0 ] && printf "%-38s %s\n" "$f" "$h"
done
grep -rE '^\s*pub (const )?fn .*WaitKind' ring_*/src/*.rs
```

Live output:

```
ring_poll/src/lib.rs              3
ring_publish/src/lib.rs           1
ring_wait/src/lib.rs              3
ring_barrier/src/lib.rs:  pub fn wait_for( &self, from : Seq, count : u64, kind : WaitKind, spins : usize )
ring_config/src/lib.rs:  pub const fn with_wait( mut self, wait : WaitKind ) -> Self
ring_config/src/lib.rs:  pub const fn wait( &self ) -> WaitKind
ring_shutdown/src/lib.rs:pub fn wait_for_close( shutdown : &Shutdown, kind : WaitKind, spins : usize )
ring_wait/src/lib.rs:pub const fn escalation_hint( kind : WaitKind ) -> Option< WaitKind >
ring_wait/src/lib.rs:pub fn pause( kind : WaitKind, attempt : usize ) -> bool
ring_wait/src/lib.rs:pub fn wait_until< F >( kind : WaitKind, spins : usize, mut ready : F ) -> Result< usize, RingError >
ring_wait/src/lib.rs:pub fn wait< F >( kind : WaitKind, ready : F ) -> Result< usize, RingError >
ring_wait/src/lib.rs:pub fn for_space( pair : &CursorPair, kind : WaitKind, spins : usize ) -> Result< usize, RingError >
ring_wait/src/lib.rs:pub fn for_data( pair : &CursorPair, count : u64, kind : WaitKind, spins : usize )
```

Of 33 crates, exactly three contain a waiting primitive at all:

| Crate | Hits | Takes a `WaitKind` | Has a budget | Returns |
|-------|-----:|:------------------:|:------------:|---------|
| `ring_wait` | 3 | ✔ (5 of its 6 fns) | ✔ `spins` | `Result< usize, RingError >` |
| `ring_poll` | 3 | — (open-coded, `Budget`) | ✔ `Budget` | its own progress types |
| **`ring_publish`** | **1** | **—** | **—** | **`Seq`** |

And ten public functions across four crates take a `WaitKind`: `ring_wait` (5),
`ring_barrier` (1), `ring_shutdown` (1), `ring_config` (2, as a config field).
This crate is not among them.

`ring_poll` is the interesting middle case: it also declines `ring_wait`, but for
the opposite reason — it must *never* block, so it open-codes a budgeted loop
three times rather than take a dependency on a crate that can. This crate
declines it because it must *always* eventually succeed. Same rejection, opposite
grounds, and between them they bracket what `ring_wait` is actually for: waits
that may legitimately give up.

### The Sibling That Waits and Does Take a Strategy

The sharpest comparison is inside the same protocol. `ring_barrier::wait_for`
(`ring_barrier/src/lib.rs:282-287`) is the handshake's *consumer-side* wait:

```rust
pub fn wait_for( &self, from : Seq, count : u64, kind : WaitKind, spins : usize )
-> Result< Seq, RingError >
{
  ring_wait::wait_until( kind, spins, || self.admits( from, count ) )?;
  self.frontier().ok_or( RingError::Empty )
}
```

Two Tier 5 crates, one feature, two waits, opposite shapes:

| | `ring_barrier::wait_for` | `ring_publish::publish` |
|--|--------------------------|-------------------------|
| Waits for | a **producer** to publish | a **peer producer** to publish |
| That party is | possibly slow, stalled, or gone | mid-write, committed, unblocked |
| Strategy parameter | `kind : WaitKind` | none |
| Budget parameter | `spins : usize` | none |
| On exhaustion | `Err( RingError::Empty )` — "no data yet", a real answer | would have no correct handling |
| Delegates to | `ring_wait::wait_until` | nothing |

They wait on the *same event* — a publication — and differ entirely on who is
waiting. A consumer with no data has somewhere else to be; a producer holding a
written, claimed, unpublished range does not. `RingError::Empty` is a meaningful
answer for the first and has no meaning for the second.

That symmetry is why the decision is defensible from inside the family rather
than being a local exception: the family's rule is not *"waits take a
`WaitKind`"* but *"waits that can correctly give up take a `WaitKind`"*, and this
one cannot.

### What a Budget Would Actually Have To Do

Suppose `publish( start, len, spins )` returning `Result< Seq, RingError >`. On
exhaustion the caller holds a claim whose slots are written and unpublished, and
has exactly these options:

| Option | Outcome |
|--------|---------|
| Retry | identical to no budget, plus a caller-side loop |
| Give up and return | the range is permanently unpublished; the frontier can never pass it; **the ring is dead** |
| Un-claim | impossible — `ring_claim` has no retraction, by design (its cursor advances by compare-exchange and never retreats) |
| Publish out of order anyway | that is the rejected design of [`decisions/001`](001_refused_rather_than_reordered.md) |

Only the first is correct, and it is what the current signature does. The budget
would be a parameter whose every non-trivial value is wrong — which is worse than
absent, because it looks like a knob.

`try_publish` is the escape hatch for the genuinely different caller: it returns
immediately, so a caller who wants a deadline, a diagnostic, or a give-up writes
its own loop and owns the exhaustion decision. `src/lib.rs:186-189` names it as
exactly that.

### Reversal Conditions

| Would reverse this if | Because | Currently |
|-----------------------|---------|-----------|
| A producer could abandon a claim | exhaustion would gain a correct handling | `ring_claim` has no retraction and is not planned to |
| Producer payloads grew large enough that spinning wasted real cycles | `Yield` would stop being strictly worse | unmeasured; no production caller ([`non_functional_requirement/002`](../non_functional_requirement/002_what_the_spin_costs.md)) |
| The crate acquired a tick-path caller | a tick has a deadline, and any wait misses it — `ring_poll`'s whole reason for existing | no callers at all ([`api/001`](../api/001_six_methods_and_no_caller.md) § PB10) |

The second is the live one. The termination argument holds for any payload size,
but "spinning is the cheapest option" is an argument about *duration*, and the
only measurement in the repository uses a one-word payload
(`tests/handshake_test.rs:268-271`). At a large enough record size the balance
between spinning and yielding must flip; nothing has established where.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_a_loop_with_no_budget.md](../algorithm/002_a_loop_with_no_budget.md) | The loop this decision leaves bare, and its termination argument |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_six_methods_and_no_caller.md](../api/001_six_methods_and_no_caller.md) | The parameter this decision keeps off the signature |
| [../api/002_a_result_whose_error_is_not_an_error.md](../api/002_a_result_whose_error_is_not_an_error.md) | Why `publish` returns `Seq` and not `Result< Seq, RingError >` |

### Decisions

| File | Relationship |
|------|--------------|
| [001_refused_rather_than_reordered.md](001_refused_rather_than_reordered.md) | The refusal that creates the wait in the first place |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_the_two_crates_that_declined.md](../integration/002_the_two_crates_that_declined.md) | `ring_mpsc` declining precisely this wait |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_what_the_spin_costs.md](../non_functional_requirement/002_what_the_spin_costs.md) | What an iteration costs, and the payload size nothing has measured |

### Sources

| File | Relationship |
|------|--------------|
| `ring_publish/src/lib.rs:42-53,167-210` | The argument, and the loop it justifies |
| `ring_barrier/src/lib.rs:282-287` | The same feature's consumer-side wait, with both parameters |
| `ring_wait/src/lib.rs:112-146,179-195` | The four arms, the `Park` that sleeps, and the budgeted loop |
| `ring_poll/src/lib.rs` | The other crate that declines `ring_wait`, on opposite grounds |
| `ring_config/src/lib.rs:89,167` | `WaitKind` as a config field, which this crate's callers cannot pass in |

### Tests

| File | Relationship |
|------|--------------|
| `tests/publish_test.rs:126-155` | The wait, isolated: B blocks until A publishes and nothing else releases it |
| `tests/publish_test.rs:157-181` | Four out-of-order producers, all spinning, ending contiguous |
| `tests/handshake_test.rs:497-561` | Three producers × 3 000 items — the only measurement of the spin at scale |
