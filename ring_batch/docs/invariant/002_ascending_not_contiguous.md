# Invariant: Ascending, Not Contiguous

### Scope

**Purpose:** Record the per-thread ordering invariant this crate's own contract
asks for, why the test asserts `<=` rather than `==`, and the third invariant — the
capacity bound — that no test asserts at all.

**Responsibility:** `a_threads_own_batches_stay_in_its_issue_order`, the gap
distribution it tolerates, and the bound `claim_gated` exists to hold.

**In Scope:** `ring_batch/tests/batch_test.rs:369-404`;
`ring_batch/src/lib.rs:306-329`.

**Out of Scope:** Disjointness is
[`invariant/001`](001_disjointness_is_free.md). The measured consequences of the
missing bound are
[`pitfall/001`](../pitfall/001_the_window_between_the_gate_and_the_advance.md).

---

## What a Thread Is Promised About Its Own Claims

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the requirement, and the assertion --'
command grep -m1 -A1 -F '  // Hard problem 118'"'"'s requirement: whatever the interleaving between threads,' ring_batch/tests/batch_test.rs
command grep -m1 -A10 -F '  for batches in &per_thread' ring_batch/tests/batch_test.rs
echo '  -- the hard problem it cites --'
command grep -m1 -F 'Parallel systems each write commands into their own thread-local space, and the tick has to see those writes as one sequence with a defined apply order. The merge is where the parallelism has to collapse back into a single ordering, and it is the only point in the write path where "which system wrote first" becomes a question with an answer.' docs/hard_problem/118_merging_thread_local_writes_into_one_stream.md
```

Live output:

```
  -- the requirement, and the assertion --
  // Hard problem 118's requirement: whatever the interleaving between threads,
  // one thread's own claims come back ascending.
  for batches in &per_thread
  {
    for pair in batches.windows( 2 )
    {
      assert!
      (
        pair[ 0 ].end().0 <= pair[ 1 ].start().0,
        "a thread's later claim started before its earlier one ended"
      );
    }
  }
  -- the hard problem it cites --
Parallel systems each write commands into their own thread-local space, and the tick has to see those writes as one sequence with a defined apply order. The merge is where the parallelism has to collapse back into a single ordering, and it is the only point in the write path where "which system wrote first" becomes a question with an answer.
```

---

### BA24 — The `<=` Is Load-Bearing: Three Quarters of a Thread's Adjacent Claims Have a Gap

Written as `==`, the assertion would say a thread's claims are contiguous. They
are not, and cannot be — every other thread's `fetch_add` lands between them.
Measured on the same shape, twenty runs:

```
  of 7984 adjacent pairs per run, summed over 20 runs:
  pairs that were contiguous                39469          39129
  widest gap between own claims               680           1816
```

**Finding.** Just under a quarter of adjacent pairs came back contiguous —
39,469 of 159,680 for `claim`, 39,129 for `claim_gated` — so the other three
quarters had another thread's claim wedged between them. The widest single gap
was 680 sequences ungated and 1816 gated: eighty-five and two hundred and
twenty-seven foreign batches, respectively, between one thread's consecutive
claims.

That is exactly what the per-thread ordering requirement asks for and no more. It wants a staged
buffer to land as one contiguous claim so a system's own writes keep their
relative order through the merge — a statement about each claim's interior and
about the order of claims, never about adjacency between them. The test asserts
the requirement and nothing beyond it, which is why it passes under any
interleaving rather than flaking under some.

The one thing to notice is that the comparison runs through `end()`, the crate's
only unchecked addition — see
[`pitfall/002`](../pitfall/002_the_addition_with_no_panics_section.md). At four
thousand sequences the assertion is nowhere near it, but the assertion that
protects the ordering invariant is itself built on the arithmetic that can panic.

---

### BA25 — The Third Invariant Has No Test

Two invariants are asserted: sequences are disjoint, and a thread's own claims
ascend. There is a third, and it is the only one that can actually be violated —
the number of sequences outstanding must never exceed the ring's free space.
That is what `claim_gated` exists to enforce:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the bound, and the whole of its enforcement --'
command grep -m1 -A5 -F '  let at = producer.load( Ordering::Acquire );' ring_batch/src/lib.rs
echo '  -- every test that spawns a thread --'
command grep 'fn concurrent_batch_claims_never_overlap\|fn a_threads_own_batches_stay_in_its_issue_order' ring_batch/tests/batch_test.rs
echo '  -- and what they call --'
command grep -c 'claim( cursor' ring_batch/tests/batch_test.rs
echo "  -- gated calls under a thread : $( command grep -n 'claim_gated' ring_batch/tests/batch_test.rs | awk -F: '$1 > 316' | wc -l ) --"
```

Live output:

```
  -- the bound, and the whole of its enforcement --
  let at = producer.load( Ordering::Acquire );
  let behind = consumer.load( Ordering::Acquire );
  if ( free_slots( at, behind, capacity ) as usize ) < count
  {
    return Err( RingError::Full );
  }
  -- every test that spawns a thread --
fn concurrent_batch_claims_never_overlap()
fn a_threads_own_batches_stay_in_its_issue_order()
  -- and what they call --
2
  -- gated calls under a thread : 0 --
```

**Finding.** Two loads and a comparison, with no atomicity joining them to the
advance that follows. Every test of that code path is single-threaded; both
tests that spawn threads call the ungated `claim`. So the invariant that holds
unconditionally is asserted twice, the invariant that describes each claim's own
interior is asserted twice more, and the invariant that the crate's second entry
point exists to maintain is asserted nowhere.

The gap is not that the tests are weak — they are careful, and
`an_oversized_request_is_refused_before_the_ring_is_even_consulted` even pins the
step order by operation count. It is that a bound between two threads cannot be
observed from one, and the suite never runs the gate from two.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`invariant/001`](001_disjointness_is_free.md) | The invariant that cannot be violated, and its two witnesses |
| [`pitfall/001`](../pitfall/001_the_window_between_the_gate_and_the_advance.md) | What the missing test would have found |
| [`pitfall/002`](../pitfall/002_the_addition_with_no_panics_section.md) | The `end()` the ordering assertion is built on |
| [`algorithm/002`](../algorithm/002_check_then_advance.md) | The three steps, and the two that are not atomic together |

### Sources

| Fact | Where |
|------|-------|
| The requirement and the assertion | `ring_batch/tests/batch_test.rs:372-373`, `:393-403` |
| The gate's whole enforcement | `ring_batch/src/lib.rs:321-326` |
| Gap distribution and widest gap | Release probe, quoted above |
| No gated call under a thread | Census above |

### Tests

| Test | Covers |
|------|--------|
| `a_threads_own_batches_stay_in_its_issue_order` | Per-thread ascending order, adjacent pairs, `<=` |
| `a_gated_claim_succeeds_while_the_ring_has_room` | The gate, on one thread |
| `a_consumer_advancing_reopens_the_gate` | The gate reopening, on one thread |
| *(to create)* | The capacity bound under contention — the one invariant with no test and the only one that fails |
