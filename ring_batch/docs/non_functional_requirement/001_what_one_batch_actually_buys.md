# Non-Functional Requirement: What One Batch Actually Buys

### Scope

**Purpose:** Measure the claim this crate's own contract states and this crate's module comment
quotes — that a batch of sixty-four costs roughly what a single item costs — and
record where on the size curve it stops being worth anything.

**Responsibility:** The cost of `claim( cursor, n, order )` against `n` calls of
`claim( cursor, 1, order )`, at one thread and at eight.

**In Scope:** `ring_batch/src/lib.rs:7-10`, `:221-224`;
this crate's own stated claim.

**Out of Scope:** That no benchmark in the repository measures this is
[`integration/002`](../integration/002_the_feature_is_planned_its_problems_are_addressed.md)
BA21. Allocation is
[`non_functional_requirement/002`](002_sixteen_bytes_and_no_allocation.md).

---

## The Claim, in the Crate's Own Words

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the number the crate has to make true --'
command grep -m1 -A3 -F '//! `docs/feature/177_batch_claim_and_batch_drain.md` states the claim this crate' ring_batch/src/lib.rs
echo '  -- and the whole of what it does to make it true --'
command grep -m1 -A3 -F 'pub fn claim< C : SeqCell >( cursor : &C, count : usize, order : Ordering ) -> BatchClaim' ring_batch/src/lib.rs
```

Live output:

```
  -- the number the crate has to make true --
//! `docs/feature/177_batch_claim_and_batch_drain.md` states the claim this crate
//! has to make true: "the memory fences that make the handshake correct are paid
//! per operation, not per item, so a batch of sixty-four costs roughly what a
//! single item costs."
  -- and the whole of what it does to make it true --
pub fn claim< C : SeqCell >( cursor : &C, count : usize, order : Ordering ) -> BatchClaim
{
  BatchClaim::new( cursor.fetch_add( count as u64, order ), count )
}
```

---

### BA34 — The Claim Holds, and Holds Better the More Producers There Are

Measured: claiming `n` sequences in one operation against claiming them one at a
time, same total sequences either way.

```
--- claiming N sequences, one operation against N ---
  -- 1 thread(s) --
    items        one claim   that many claims      ratio
        1          5.93 ns            6.12 ns       1.0x
        8          7.51 ns           49.44 ns       6.6x
       64         11.49 ns          382.10 ns      33.2x
     1024          7.56 ns         6151.59 ns     813.4x
  -- 8 thread(s) --
    items        one claim   that many claims      ratio
        1         80.92 ns           82.00 ns       1.0x
        8         93.28 ns          642.86 ns       6.9x
       64         63.59 ns         5239.01 ns      82.4x
     1024        212.05 ns        86322.36 ns     407.1x
```

**Finding.** Take the feature's sentence literally. A batch of sixty-four costs
11.49 ns on one thread; a single item costs 5.93 ns. Within a factor of two —
"roughly what a single item costs" is accurate.

Under contention it is better than accurate. At eight threads a batch of
sixty-four costs 63.59 ns and a *single-item* claim costs 80.92 ns, so the batch
of sixty-four is cheaper than the claim of one. The reason is the same
instruction in both cases: `fetch_add` takes the cursor's cache line exclusively
once, and how many sequences that grants makes no difference to the transfer. A
batching producer therefore holds the line one sixty-fourth as often, which
leaves it available and cuts everybody's cost including its own.

That is the amortisation the family is built around, and the sixty-four figure
turns out to be conservative — at 1024 items the ratio is 813× on one thread and
407× on eight.

---

### BA35 — And Below Eight Items It Buys Nothing

The same table read from the top: at one item the batched and unbatched costs are
indistinguishable — 5.93 against 6.12 on one thread, 80.92 against 82.00 on
eight. Both ratios are 1.0×.

**Finding.** There is nothing to amortise across a single item, so a batch of one
is a claim of one with an extra `usize` in a struct. The crossover is somewhere
between one and eight; by eight the ratio is 6.6×.

That matters because the family's only caller does not look. `TlsBuffer::flush_into`
claims whatever is staged, and the crate's own doc records the same asymmetry at
the other end — flushing an empty buffer costs a full atomic, which
[`lifecycle/002`](../lifecycle/002_the_empty_claim_as_a_first_class_state.md)
BA33 measures at 2× a working producer's per-claim cost.

So the two degenerate batch sizes are exactly the two where the crate's reason
for existing does not apply: zero, where the operation is pure overhead borne by
other threads, and one, where it is precisely break-even. A caller that flushes
per push gets the full machinery and none of the benefit, and neither crate says
where the curve turns.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'nothing to amortise' ring_batch/src/lib.rs
```

Live output:

```
//! roughly eight items there is nothing to amortise: a batch of one costs the
```

**Disposition:** applied — `ring_batch`'s own module comment now states the
crossover directly, right after the claim it qualifies: below roughly eight
items there is nothing to amortise, and a batch of zero costs a full atomic
paid by other threads. The `ring_tls` end of the same asymmetry stays
recorded, not fixed, per BA33's own disposition immediately above in this
crate's own docs.
Now prints: `nothing to amortise`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`non_functional_requirement/002`](002_sixteen_bytes_and_no_allocation.md) | The other cost dimension, which the crate wins outright |
| [`integration/002`](../integration/002_the_feature_is_planned_its_problems_are_addressed.md) | Why these numbers came from a throwaway probe |
| [`lifecycle/002`](../lifecycle/002_the_empty_claim_as_a_first_class_state.md) | The zero-item end of the same curve |
| [`algorithm/001`](../algorithm/001_one_fetch_add_whatever_the_count.md) | The single instruction the whole curve rests on |

### Sources

| Fact | Where |
|------|-------|
| The claim | `ring_batch/src/lib.rs:7-10` |
| The implementation | `ring_batch/src/lib.rs:221-224` |
| The curve at one and eight threads | Release probe, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `a_claim_of_sixty_four_issues_one_operation_not_sixty_four` | The operation count, which is the claim's mechanism |
| `the_cost_of_a_claim_does_not_depend_on_its_size` | That the operation count is flat in `count` |
| *(to create)* | The times themselves, and the crossover between one and eight items |
