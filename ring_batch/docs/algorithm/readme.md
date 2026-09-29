# algorithm

Two functions claim sequences, and between them they perform exactly three
atomic operations in the whole crate: one `fetch_add` and two `load`s. There is
no compare-and-swap, no retry loop, and no spin anywhere in `ring_batch` — the
ungated claim is a single read-modify-write, and the gated one is the same
instruction with two reads in front of it.

That shape is what makes both instances short and what makes the second one
interesting. A single atomic operation is unconditionally correct; a check
followed by a separate operation is not, and the whole difference between the
two functions is those two extra `load`s.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_one_fetch_add_whatever_the_count.md) | One `fetch_add`, Whatever the Count | `claim`'s single instruction, and the measured cost curve across four batch sizes |
| [002](002_check_then_advance.md) | Check, Then Advance | `claim_gated`'s three steps, the test that pins their order, and a cast the lint gate cannot see |

## Why the Return Value Is the Claim

`fetch_add` returns the value the cell held before the addition. That return
value is the caller's first sequence, and the addition that produced it is the
same instruction that moved the cursor past the range. So there is no interval
during which the claim exists and the cursor has not advanced — the two facts
are one memory operation, and no other producer can be handed a sequence inside
the range even in principle.

This is why `BatchClaim` carries no token, no lock, and no back-reference to the
cursor it came from. It is sixteen bytes of `Copy` data describing a range that
was already made private before the struct was built.

## The Two Extra Loads

`claim_gated` reads the producer cursor, reads the consumer cursor, computes
headroom, and only then calls `claim`. Every one of those steps is correct in
isolation. Together they are a check whose answer can be stale by the time step
three acts on it, and the crate's documentation does not say so.

The instance records the shape and the measurement lives in
[`pitfall/001`](../pitfall/001_the_window_between_the_gate_and_the_advance.md),
because the failure is not visible from the algorithm — reading these fifteen
lines does not tell you how often the window is actually hit, and the answer
turns out to be often enough to matter.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the two claim functions, whole --'
awk '/^pub fn claim< C : SeqCell >\( cursor : &C, count : usize, order : Ordering \) -> BatchClaim$/{ n1 = NR } n1 && NR >= n1 && NR <= n1 + 3 { print } /^-> Result< BatchClaim, RingError >$/{ n2 = NR } n2 && NR >= n2 + 1 && NR <= n2 + 15 { print }' ring_batch/src/lib.rs
echo '  -- every atomic operation either one performs --'
command grep -nE 'fetch_add|\.load\(|compare_exchange|\.store\(' ring_batch/src/lib.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BA1 | `ring_batch` | n/a — observation | The amortisation is `fetch_add` returning the old value: publishing the claim and advancing the cursor are one instruction, which is why nothing can be unclaimed |
| BA2 | `docs/feature` | n/a — doc gap | A claim costs 6.4 ns at every size from 1 to 1024; this crate's own contract states "roughly what a single item costs" and omits the 115× and 1841× ratios a batch-size/tail-latency tradeoff needs |
| BA3 | `ring_batch` | n/a — observation | The size check precedes the cursor reads, and the test pins that by asserting both cells were never touched — the crate's only body-ordering decision with a test rather than a comment behind it |
| BA4 | `ring_batch` | n/a — observation | `free_slots( .. ) as usize` casts `usize` to `usize`; `clippy::unnecessary_cast` warns on all three same-crate forms of it and is silent across a crate boundary, so `-D warnings` cannot see it |
