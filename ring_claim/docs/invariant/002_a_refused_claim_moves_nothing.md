# Invariant: A Refused Claim Moves Nothing

### Scope

- **Purpose**: State the second invariant — that a claim which returns `Err` leaves the cursor exactly where it found it — and the third, that no grant ever extends past the consumer limit.
- **Responsibility**: Show why the first makes retrying safe, and reconstruct the argument for why the test of the second reads the limit *after* the grant rather than before.
- **In Scope**: Cursor movement on refusal, and the grant-versus-limit relation.
- **Out of Scope**: Exclusivity between producers — see [`invariant/001`](001_no_two_producers_hold_one_sequence.md).

### The Two Invariants

| | Statement | Held by |
|--|-----------|---------|
| **I1** | a call returning `Err` leaves `claimed()` unchanged | the exchange being the only mutation, and only on the `Ok` path |
| **I2** | for every granted `Claim c`, `c.end() <= limit` at the moment of the grant | the gate being the loop condition |

I1 is the one that makes a retry loop safe to write, and `tests/claim_test.rs:208`
states the counterfactual in two lines:

> The property that makes retrying safe. A claim that moved the cursor and then
> reported failure would leak a slot per attempt.

A producer that spins on `Err( Full )` waiting for a consumer is the intended
usage. If each failed attempt cost a sequence, that loop would consume the ring
while waiting for the ring — it would be a livelock with a leak, and the harder
the caller retried the worse it would get.

The test asserts it with a hundred failures against a full ring and one
comparison:

```rust
let _taken = claimer.claim( 4 ).unwrap();
for _ in 0..100 { assert!( claimer.claim( 1 ).is_err() ); }
assert_eq!( claimer.claimed(), Seq( 4 ), "100 failures moved the cursor" );
```

### CL25 — Two Ways to Move Nothing, and One of Them Succeeds

I1 says failure moves nothing. The crate also has a *success* that moves
nothing, and the two are tested next to each other:

| Call | Result | Cursor after | Atomic issued |
|------|--------|--------------|---------------|
| `claim( 1 )` on a full ring | `Err( Full )` | unchanged | **none** — the `while` never admits |
| `claim( 0 )` on a full ring | **`Ok`**, empty | unchanged | **one `compare_exchange`** |

So the two operations that leave the cursor identical differ in every other
respect: one returns an error and issues no atomic instruction at all, the other
returns success and issues the most expensive instruction in the crate
([`algorithm/002`](../algorithm/002_two_loops_that_disagree_at_zero.md)).

That asymmetry is worth stating because a reader would reasonably predict the
opposite — that the cheap path is the successful one. The refusal is cheap
precisely *because* the gate is the loop condition: a claim that cannot be
granted is rejected by a plain load and a comparison, never reaching the
exchange. The rejected `fetch_add` design would have had the same property, and
`GatingSet::check` still does — which is why the refusal path is not where the
correctness argument lives.

`a_failed_claim_advances_nothing` and `claiming_zero_succeeds_and_moves_nothing`
are adjacent in the file, `:208` and `:224`, and between them they pin the
cursor's behaviour on both non-advancing paths.

### CL26 — The Limit Is Read After the Grant, and That Is What Makes the Test Sound

`no_grant_ever_passes_the_limit_under_contention` is the crate's only test of
I2, and its comment is the sharpest piece of test reasoning in the file:

> The bound is read *after* each grant, and that is what makes the assertion
> both sound and able to fail. Sound: the limit only ever rises, since a
> consumer only advances, so a claim granted legitimately against some earlier
> limit is still within any later one — a correct implementation can never trip
> this. Able to fail: a `fetch_add` grant overshoots the limit that held at the
> moment it was made, and unless the consumer happens to sweep past it before
> the read, it overshoots the later one too.
>
> Reading the limit *before* the claim would give the opposite: an assertion
> that fails on correct code whenever the consumer advances in between.

The argument turns entirely on monotonicity. Written out:

| Read order | Correct implementation | Broken implementation |
|------------|------------------------|-----------------------|
| limit **after** grant | `end <= limit_at_grant <= limit_after` — always passes | `end > limit_at_grant`; passes only if the consumer sweeps past in the window |
| limit **before** grant | `limit_before <= limit_at_grant`, so `end` may exceed `limit_before` — **fails on correct code** | fails, but for the wrong reason |

There is no third option that is both sound and sensitive, because the limit at
the exact instant of the grant is not observable from the test thread — it is
inside the same atomic step as the grant itself. Reading after is the tightest
observable bound that a correct implementation cannot violate.

The test's other design decisions follow from the same pressure:

| Choice | Why |
|--------|-----|
| capacity 4 | the gate must bind constantly, or the overshoot has room to hide |
| 3 producers × 3,000 claims | enough attempts that a rare interleaving occurs |
| a consumer releasing 4,000 times | so producers keep finding room and keep racing rather than all parking |
| `yield_now` after each release | *"Without it the consumer finishes in a burst, races far ahead of the producers, and an overrunning grant is absorbed by a limit that has already moved past it"* |
| a `sort_unstable` + `dedup` over every collected range | so the run also asserts exclusivity, under the one configuration where the gate binds |
| `assert!( granted > 0 )` at the end | so a run that granted nothing cannot report success |

The last row is the one most tests omit. A concurrency test whose loop body
never executes passes silently; this one refuses to — and here the risk is real
rather than theoretical, because every claim is wrapped in `if let Ok( … )` and
a run in which the gate refused all 9,000 attempts would otherwise be green.

The row above it is the crate's only overlap between its two invariants. Note
what it does *not* assert: there is no `TOTAL` comparison and no first/last
check, because a run whose claims are refused at an unpredictable rate has no
predicted count to compare against. Duplicate-freedom survives that; contiguity
does not, which is why the exclusivity tests need a gate that never refuses
([`invariant/001`](001_no_two_producers_hold_one_sequence.md)).

### What Neither Invariant Covers

| Not asserted | Why |
|--------------|-----|
| a global order across producers | not promised — `each_producers_own_claims_stay_in_issue_order:442-444` says asserting it "would fail correct implementations" |
| that a granted range is ever *written* | outside this crate — that is the producer's obligation ([`pitfall/001`](../pitfall/001_dropping_a_claim.md)) |
| that a granted range is ever published | likewise — that is `ring_publish`'s obligation |
| I2 under `loom` | this crate has no `loom` model; `ring_publish/tests/handshake_test.rs` has the family's two |

The last is the notable one, and the test file says it in its own header
(`tests/claim_test.rs:28-31`):

> Concurrency tests are probabilistic. Each is sized to fail reliably rather
> than occasionally against the implementation it targets, and **none of them
> proves absence of a race — that is `loom`'s job in
> `ring_publish/tests/handshake_test.rs`.**

The Tier 5 sibling models its ordering under `loom` at capacity 2; this crate
verifies its equivalent property with three threads and a `yield_now`. Both are
reasonable — a `loom` model of a CAS retry loop with an unbounded retry count
does not terminate without an artificial bound — but the asymmetry is real, and
this crate's own tests point at the other crate for the guarantee they cannot
give. `ring_publish`'s *simpler* algorithm got the exhaustive
model; this crate's retry loop got a statistical test whose sensitivity had
to be measured by hand and recorded in a manual plan.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_the_gate_inside_the_retry.md](../algorithm/001_the_gate_inside_the_retry.md) | Why the refusal path issues no atomic |
| [../algorithm/002_two_loops_that_disagree_at_zero.md](../algorithm/002_two_loops_that_disagree_at_zero.md) | The success that moves nothing |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_compare_exchange_rather_than_fetch_add.md](../decisions/001_compare_exchange_rather_than_fetch_add.md) | The design I2 exists to exclude |

### Invariants

| File | Relationship |
|------|--------------|
| [001_no_two_producers_hold_one_sequence.md](001_no_two_producers_hold_one_sequence.md) | The invariant tested under the opposite configuration |

### Non Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_what_contention_costs.md](../non_functional_requirement/002_what_contention_costs.md) | Why a refusal is cheap and a zero-width success is not |

### Sources

| File | Relationship |
|------|--------------|
| `ring_claim/src/lib.rs:436-447` | The single mutation, on the `Ok` path only |
| `ring_gating/src/lib.rs:296-324` | `limit`, the bound I2 is stated against |
| `ring_publish/tests/handshake_test.rs` | The family's two `loom` models, in the other half |

### Tests

| File | Relationship |
|------|--------------|
| `tests/claim_test.rs:208` — `a_failed_claim_advances_nothing` | I1, with 100 refusals |
| `tests/claim_test.rs:224` — `claiming_zero_succeeds_and_moves_nothing` | The non-advancing success |
| `tests/claim_test.rs:154` — `claiming_stops_exactly_at_the_gate` | I2, single-threaded and exact |
| `tests/claim_test.rs:476` — `no_grant_ever_passes_the_limit_under_contention` | I2 under contention, and the read-order argument |
| `tests/manual/readme.md § C6` | The contention constants, checked so they cannot be quietly lowered |
