# Lifecycle: The Producer Walking a Lap Against a Stall

### Scope

- **Purpose**: Walk the one sequence this crate exists to produce — a producer advancing until a stalled consumer stops it — and show that the crate contributes no loop, no wait and no timeout to it.
- **Responsibility**: Give the sequence step by step, name where each part of it lives, and follow the retry it prescribes to the one place in the family that actually retries.
- **In Scope**: What happens over time when a gate refuses.
- **Out of Scope**: The set's own life — see [`001`](001_a_set_from_construction_to_drop.md).

### The Sequence

Capacity 8, two consumers, one of which never moves. This is the sequence the
crate is built to gate correctly against.

| Step | Producer | Slowest | `headroom` | `admits( p, 1 )` |
|-----:|---------:|--------:|-----------:|:----------------:|
| 0 | 0 | 0 | 8 | ✅ |
| 1 | 1 | 0 | 7 | ✅ |
| … | … | 0 | … | ✅ |
| 7 | 7 | 0 | 1 | ✅ |
| **8** | **8** | **0** | **0** | ❌ |
| 8′ | 8 | **1** | 1 | ✅ |
| 9 | 9 | 1 | 0 | ❌ |

Eight admissions, then refusal — *exactly* one lap, which is the clause's word.
The fast consumer in the test races to `Seq( 1_000 )` and buys the producer
nothing, because the bound is the minimum
([`invariant/001`](../invariant/001_the_bound_is_the_minimum_and_only_the_minimum.md)).

Row `8′` is the release, and it is exact: one slot returned admits one claim and
not two. `tests/gating_test.rs:88-98` asserts both halves of that.

### What the Crate Contributes

| Part of the sequence | Lives in |
|----------------------|----------|
| The bound at each step | `ring_cursor::slowest` → `ring_seqno::free_slots` |
| The refusal | **this crate** — `admits`, `check` |
| The loop that keeps asking | The caller |
| The pause between asks | `ring_wait`, or nothing |
| Giving up | The caller |

This crate's entire contribution is the middle row. It is worth making that
concrete:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -vE "^[[:space:]]*(///|//!)" ring_gating/src/lib.rs \
  | grep -cE '^\s*(while|loop)\b|^\s*for |spin_loop|yield_now|park|sleep|Condvar|thread::' || true
# 0
```

Live output:

```
0
```

**Zero.** No loop, no spin hint, no yield, no sleep, no condvar — in a crate
whose documentation says *"a retry loop should keep going"* eight lines from the
top of `check`. The temporal behaviour is entirely the caller's; this crate
answers a question and returns.

That is the correct division and it is worth naming as one: a gate that spun
would make the spin policy un-substitutable, and a tick-path caller must never
be able to reach a parking operation at all.

### Following the Retry to Where It Actually Happens

Four crates document a retry loop. Their production sources contain none:

| Crate | Says | Has a retry loop |
|-------|------|:----------------:|
| `ring_gating:253-255` | "a retry loop should keep going" | ❌ |
| `ring_claim:398-399` | "a retry loop should keep going" | ❌ |
| `ring_mpsc:750` | "back-pressure, so a retry loop…" | ❌ |
| `ring_spsc:625` | "the consumer has not committed far enough…" | ❌ |

`ring_claim::claim` has a loop, but it retries a *compare-exchange*, not a
refusal — it exits the moment headroom is insufficient and returns `Full`
([`decisions/002`](../decisions/002_a_result_rather_than_a_bool.md) § G8). The
family's one bounded retry over a refusal is elsewhere:

```rust
// ring_poll/src/lib.rs — push_within, stripped of docs
while attempt < budget.attempts()
{
  match producer.try_push( held )
  {
    Ok( () ) => return Ok( () ),
    Err( returned ) =>
    {
      held = returned;
      attempt += 1;
      if attempt < budget.attempts()
      { core::hint::spin_loop(); }
    }
  }
}
Err( held )
```

### G18 — The Retry Loop Does Not Speak the Error Language

`push_within` is the retry loop the whole `Full`/`is_transient` taxonomy was
written for, and it never touches it. `Producer::try_push` returns
`Result< (), T >` — handing the *record* back on refusal, not a `RingError` — so
there is no error to classify:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -c 'is_transient\|RingError' ring_poll/src/lib.rs || true
# 0
```

Live output:

```
0
```

This resolves the open half of
[`decisions/002`](../decisions/002_a_result_rather_than_a_bool.md) § G7 —
`is_transient` and `is_configuration` have 27 test callers and zero production
callers. **Not because nobody retries, but because the one thing that retries
was given a signature that cannot carry the distinction.** The record-returning
shape is right for `try_push` on its own terms (it avoids dropping the payload),
and the consequence is that a predicate designed to steer a retry loop steers
nothing.

| Reading | Consequence |
|---------|-------------|
| The taxonomy is unused | It is documentation, verified by tests, driving no code |
| The retry is untyped | `push_within` retries every refusal identically — correct here, because `try_push`'s only refusal *is* `Full` |
| The two never meet | A future retry over `check` would have to re-derive the classification `is_transient` already encodes |

The third row is the risk. `check`'s two refusals differ in exactly the way a
retry loop must respect ([`pitfall/002`](../pitfall/002_reversing_the_two_refusals.md)),
and no existing loop consults them — so the first caller that loops over `check`
writes that logic from scratch, against a predicate it has no precedent for
calling.

### Termination

Nothing in the sequence guarantees it ends. A consumer that never stores again
leaves `headroom` at zero permanently, and `RingError::Full.is_transient()`
stays `true` forever. `is_transient`'s own doc is careful about this in its
second clause and loose in its first:

> Whether retrying the same operation later could succeed without anything else
> changing — true for the two conditions **a peer's progress clears**.
>
> — `ring_types/src/error.rs:127-128`

*A peer's progress clears it* is the accurate half. *Without anything else
changing* is not — nothing about waiting clears a `Full`; only the consumer
moving does. A caller that reads the first clause and loops unbounded live-locks
against a stalled consumer, which is the same failure
[`pitfall/001`](../pitfall/001_unwrapping_the_empty_set_to_zero.md) reaches by a
different route.

The family's answer is a budget rather than a bound on the ring: `ring_poll`'s
`Budget` and `ring_wait::wait_until`'s `spins` both give up. Neither is reachable
from this crate, and neither should be — but the sequence is only finite because
a caller chose to make it so.

### GT34 — The Retry Loop Is Prescribed, Not Provided

```
loop / spin_loop / yield_now / sleep in src/lib.rs : 3
37://! moves, and a retry loop that could not tell them apart would spin forever
253:  /// loop must stop rather than spin. [`RingError::Full`] when the claim
255:  /// loop should keep going.
```

All three matches are in doc comments. The crate tells a caller how to loop and
contains no loop of its own — correctly, since a gate that blocked would be a
different component.

**Finding.** The crate contains zero loops, spin hints, yields and sleeps, eight lines below documentation prescribing a retry loop. All three matches for those words are in doc comments

---

### GT35 — The Family's One Retry Consumes None of the Taxonomy

```
ring_poll::push_within   -> the family's only bounded retry
try_push                 -> Result< (), T >   -- no RingError to classify
is_transient callers     -> 27 tests, 0 production
```

The error taxonomy exists so a retry loop can tell "try again" from "stop". The
one retry loop the family ships never receives an error from that taxonomy.

**Finding.** The family's one bounded retry over a refusal, and it consumes no part of the error taxonomy: `try_push` returns `Result< (), T >`, so there is no `RingError` to classify — which is *why* `is_transient` has 27 test callers and no production caller

---

### GT36 — `is_transient`'s Two Halves Disagree

```
/// ...without anything else changing        <- false
/// ...a peer's progress clears              <- true
```

`Full` clears when a consumer advances. It does not clear on its own. A caller
that reads the opening clause and retries without waiting for a peer spins
against a stalled consumer forever — which is the exact scenario this instance
traces.

**Finding.** It opens with "without anything else changing" and closes with "a peer's progress clears" — only the second is true, and a caller trusting the first live-locks against a stalled consumer

**Disposition:** declined — the misleading two-clause doc is
`ring_types::RingError::is_transient`'s own comment at
`ring_types/src/error.rs:127-128`; this instance's own text traces
the wording to that crate, not this one's source to correct in a corpus
disposition pass.

---

### GT37 — One Test Walks the Lap

```
a_producer_never_passes_the_limit_over_a_full_lap_with_batches
      ...1 of 23 tests advances the producer repeatedly
      the other 22 assert a single position
```

The cyclic behaviour this instance describes — a producer walking a whole lap
against a fixed consumer — is exercised by one test.

**Finding.** `a_producer_never_passes_the_limit_over_a_full_lap_with_batches` is the only one of twenty-three that walks a lap, so the cyclic behaviour this instance traces rests on a single test; the other twenty-two assert points rather than motion

---


### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_headroom_in_two_delegations.md](../algorithm/001_headroom_in_two_delegations.md) | How each row's `headroom` is computed |
| [../algorithm/002_check_orders_its_two_refusals.md](../algorithm/002_check_orders_its_two_refusals.md) | Which refusal the sequence produces, and why the order matters |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_a_result_rather_than_a_bool.md](../decisions/002_a_result_rather_than_a_bool.md) | G7 and G8 — the taxonomy this document follows to its non-caller |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_the_bound_is_the_minimum_and_only_the_minimum.md](../invariant/001_the_bound_is_the_minimum_and_only_the_minimum.md) | Why the fast consumer buys nothing |

### Lifecycles

| File | Relationship |
|------|--------------|
| [001_a_set_from_construction_to_drop.md](001_a_set_from_construction_to_drop.md) | The set the sequence runs against |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_unwrapping_the_empty_set_to_zero.md](../pitfall/001_unwrapping_the_empty_set_to_zero.md) | The other route to a live-lock |
| [../pitfall/002_reversing_the_two_refusals.md](../pitfall/002_reversing_the_two_refusals.md) | Why a retry loop must read which refusal it got |

### Sources

| File | Relationship |
|------|--------------|
| `ring_gating/src/lib.rs:242-294` | `admits` and `check` — the crate's whole contribution |
| `ring_poll/src/lib.rs` § `push_within` | The family's one bounded retry over a refusal |
| `ring_types/src/error.rs:145-159` | `is_transient`, and its two clauses |
| `ring_wait/src/lib.rs:179-183` | The other giving-up mechanism |

### Tests

| File | Relationship |
|------|--------------|
| `tests/gating_test.rs:62-85` | The sequence, driven one slot at a time to exactly one lap |
| `tests/gating_test.rs:88-98` | Row `8′` — one slot released admits one claim |
| `tests/gating_test.rs:101-109` | The boundary at both ends |
| `tests/gating_test.rs:112-131` | The same sequence in batches of five |
| `tests/gating_test.rs:150-157` | One stalled consumer stopping the producer for everyone |
