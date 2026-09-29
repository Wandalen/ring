# algorithm

Two computations, and this crate wrote neither of them. `available` is a
`map_or` over a fold that lives in `ring_cursor`; `wait_for` is a call into
`ring_wait` followed by a re-read. What is local is the choice of default, the
decision to read twice, and the cost of both.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [The Frontier in Two Delegations](001_the_frontier_in_two_delegations.md) | The five-step chain from `available` to one `saturating_sub`, and the clamp that costs `ring_gating` a sixth |
| 002 | [`wait_for` Asks Twice](002_wait_for_asks_twice.md) | A predicate loop and an independent second read, their three endings, and the one input class where they disagree |

### The Two Chains

| | Steps | Crates | Allocations per call |
|--|------:|-------:|---------------------:|
| `frontier` | 3 | 3 | 1 |
| `available` | 5 | 4 | 1 |
| `admits` | 6 | 4 | 1 |
| `wait_for` (n spins) | — | 5 | up to n + 1 |

```sh
cd "$(git rev-parse --show-toplevel)"
grep -vE "^[[:space:]]*(///|//!)" ring_barrier/src/lib.rs \
  | grep -oE 'ring_(cursor|seq|wait)::[a-z_]+' | sort | uniq -c
# 1 ring_cursor::slowest
# 1 ring_wait::wait_until
```

Two delegated calls in the whole crate — one per method that needs one, and no
`ring_seqno` among them.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BR5 | `ring_barrier` | n/a — unenforced | `wait_for` is the only `ring_wait::wait_until` caller in the family that discards the returned attempt count |
| BR22 | `ring_barrier` | n/a — observation | A successful `wait_for` folds the whole dependency slice once per attempt and once more to build its return value, so a wait succeeding on attempt *n* performs *n + 1* folds — 8,200 atomic loads at the default budget with eight dependencies, documented nowhere |
| BR23 | `ring_barrier` | **misleading doc** | The rustdoc calls the returned sequence "the frontier at the moment the wait succeeded"; it is read four lines after that moment, so the value returned is never the one `admits` accepted and is only guaranteed to be at least as far |
| BR24 | `ring_barrier` | n/a — inconsistency | `wait_for` produces `RingError::Empty` on two lines for conditions sharing no cause — a budget that ran out, which says wait longer, and a barrier over an empty slice, which will never admit anything and makes the retry the variant invites an infinite loop |
