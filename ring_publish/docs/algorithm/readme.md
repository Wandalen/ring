# algorithm

Two operations, thirteen lines of body between them, and one atomic instruction
underneath both. `try_publish` is a `compare_exchange` with the failure value
mapped; `publish` is that call in a bare `loop`. There is no third algorithm in
the crate.

What is worth documenting is not the code — it fits on one screen — but the two
things about it that look wrong on a first reading and are not: an exchange that
*refuses* rather than accommodating a caller who arrived early, and a retry loop
with no budget, no strategy, and no way to fail, in a family where every other
wait is counted.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [The Compare-Exchange That Refuses](001_the_compare_exchange_that_refuses.md) | PB7, PB8 — the three positions a `start` can be in, the family's three exchange sites and what each retries against, the asymmetric orderings, and the accepted zero-length case |
| 002 | [A Loop With No Budget](002_a_loop_with_no_budget.md) | PB9 — the family's only unbounded spin, its four-step termination argument, the one caller input that hangs it, and why `spin_loop` rather than `yield_now` |

### One Operation, Two Contracts

| | `try_publish` (001) | `publish` (002) |
|--|---------------------|-----------------|
| Body | 2 statements | 1 `loop`, 1 branch |
| Returns | `Result< Seq, Seq >` | `Seq` |
| On a start past the frontier | `Err( published )`, immediately | spins |
| On a start behind the frontier | `Err( published )`, immediately | **spins forever** |
| Can fail | yes, and the failure is expected | no — the signature has no failure |
| Can hang | no | yes, on one caller bug it cannot detect |
| Termination depends on | nothing | a precondition held by the caller |

The second column's last two rows are the whole reason these are two documents.
`try_publish` is total: every input returns. `publish` is total only under a
precondition — *publish the range you claimed* — that lives outside this crate
and is checked by nothing ([`pitfall/001`](../pitfall/001_publishing_a_range_you_never_claimed.md)).

### The Three Positions

`compare_exchange( start, end, … )` succeeds exactly when the cursor reads
`start`, which splits every possible input into three cases and gives two of them
the same code path:

| `start` vs frontier | Meaning | `try_publish` | `publish` |
|---------------------|---------|---------------|-----------|
| `>` | a predecessor has not published yet | `Err( published )` | spins, then succeeds |
| `==` | this producer's turn | `Ok( start + len )` | returns `start + len` |
| `<` | already published, or never claimed | `Err( published )` | **never returns** |

The two `Err` rows are one branch and two entirely different situations —
ordinary contention, and a caller bug. Refusing the third is what stops the
frontier from ever moving backwards and un-publishing slots a consumer is
already reading; `tests/publish_test.rs:87-99` is the test for that direction,
and the doc example at `src/lib.rs:148-160` covers only the first.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# every genuine compare-exchange call site in the family, forwarding impls excluded
grep -rn 'compare_exchange' */src/*.rs \
  | grep -vE '^\S+:[0-9]+:\s*///' | grep -vE 'fn compare_exchange|compare_exchanges'

# every bare unbounded loop
for f in ring_*/src/*.rs; do
  n=$( grep -vE '^[[:space:]]*//' "$f" \
       | grep -cE '^[[:space:]]*loop[[:space:]]*\{?[[:space:]]*$' )
  # the trailing `true` keeps a final non-matching file from making the loop —
  # and therefore this block — exit non-zero
  [ "$n" -gt 0 ] && printf "%-40s %s\n" "$f" "$n"
  true
done

# the failure value: asserted where, consumed where
grep -rn 'Err( Seq' ring_publish/src/lib.rs ring_publish/tests/*.rs

# neither rejected alternative is present — comment lines dropped on purpose,
# and the empty result is the answer, so `grep`'s exit 1 is not a failure here
grep -vE "^[[:space:]]*//" ring_publish/src/lib.rs \
  | grep -niE "bitmap|contiguous|while|for |max\(|highest" || true
```

| | Value |
|--|------:|
| State-changing operations in the crate | 2 |
| Atomic read-modify-writes | **1** |
| Branches in `try_publish` | 0 |
| Loops in `try_publish` | 0 |
| Genuine compare-exchange call sites, family-wide | 3, in 2 crates |
| …that retry against a *moved* target | 2 — both `ring_claim` |
| …that retry against a *fixed* target | 1 — this crate |
| …that discard the failure value | **1** — this crate |
| `ring_*` crates containing a bare `loop` | 2 |
| …bare loops in `ring_bench` | 5, each bounded by an item count |
| …bare loops in `ring_publish` | **1**, bounded by nothing |
| `Err( Seq` occurrences in source and tests | 5 |
| …that bind the value and use it | **0** |
| Orderings named inline | 0 — `PUBLISH` at `:67`, `GATING` imported at `:57` |
| Output of the rejected-alternatives grep | empty |

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| PB7 | family | n/a — observation | Three genuine compare-exchange sites exist in the family; the two in `ring_claim` retry against a moved target and feed the failure value back as the next input, while this one retries against a fixed target and discards it — the difference between a contest and a turn-gate on the same primitive |
| PB8 | `ring_publish` | n/a — drift | The failure value is asserted against a literal five times and bound zero times; two documents describe it as *"what to try against next"*, borrowing `compare_exchange`'s idiom, but `start` is the caller's own claim and cannot be substituted |
| PB9 | family | n/a — observation | `publish` is the only bare unbounded loop in any `ring_*` library; it is correct because of *what* it waits on — a predecessor committed to a slot write it cannot abandon — not because of how long the wait is |
| PB47 | `ring_publish` | n/a — observation | The `advanced_by` call precedes the exchange with no branch between them, so a refused publication computes `start + len` and discards it — putting `advanced_by`'s undocumented overflow behaviour on the path that was going to return `Err` |
