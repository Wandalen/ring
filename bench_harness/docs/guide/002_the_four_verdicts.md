# Guide: the four verdicts

### Scope

- **Purpose**: State what this crate's comparison established, with the evidence behind each finding and an honest reading of how strongly each is supported.
- **Responsibility**: Separate the results that reproduce exactly from the one that does not, so neither is quoted as if it were the other.
- **In Scope**: The four findings; their evidence; their confidence; the two documents each overturned.
- **Out of Scope**: How to reproduce them (→ [`001`](001_running_the_verdicts_yourself.md)); the limits of the machinery that graded them (→ [`003`](003_what_the_gates_do_not_prove.md)).

**Three of the four are deterministic.** They are record accounting, which
reproduces exactly on any machine. **One is a timing result**, and it is stated coarsely on
purpose. The distinction matters more than any individual number below.

| # | Verdict | Kind | Reproduces |
|---|---|---|---|
| 1 | `Ok` is not evidence a record was kept | Accounting | Exactly |
| 2 | The Contract door caps what the structures do not | Structural | Exactly |
| 3 | Staging converts a shortfall into a stall | Accounting | Exactly |
| 4 | Lock-free beats the mutex baseline by an order of magnitude | Timing | Coarsely, and only coarsely |

---

### Verdict 1 — `Ok` is not evidence a record was kept

**Established:** under `OverflowPolicy::default()`, which is `DropNewest`, a
`try_push` into a full ring **returns `Ok` for a record it discarded**. Two of
six candidates report 256 successes on a workload where 16 records survive.

```
candidate          offered  reported  received  dropped   silent
contract_ring          256       256        16      240      240
off_the_shelf          256       256        16      240      240
mutex_queue            256        16        16      240        0
```

**Why it is a verdict and not a curiosity:** the benchmark's output is a
*recommendation about which write path to adopt*. Counting the API's own
successes made `contract_ring` read as **lossless**, and as **fast**. The speed
reading was correct, because discarding is the cheapest thing a queue can do. The
first working version of this benchmark ranked the candidate that threw away 94%
of the workload as the winner, with evidence.

**The failure is not a wrong number in a column.** Every count above is a
faithful measurement of something real. The failure is which number the verdict
was computed from.

**What changed:** `Outcome::received`, drained from the consumer after the
clock stopped, is the only count any judgement uses. `reported` is kept and the
gap is published as `silently_discarded`, because that gap is the only thing
distinguishing a path that applies back-pressure from one that absorbs.

**Confidence: total.** Deterministic counts, asserted by
`a_dropnewest_ring_reports_successes_it_did_not_keep` with
`a_failing_policy_closes_the_gap_for_every_candidate` as its control, which uses
the same capacity and record count with one field changed. Either test alone would
be consistent with the benchmark having no notion of the distinction.

**What it overturned:** `ring_types` had already written the predicate
`drops_silently()` before this crate existed. It answered exactly the question
that decides whether an `Ok` is evidence, and nothing consulted it.
*A predicate that answers a question nobody thinks to ask does not prevent the
error it was written for.*

→ [`ring_bench/docs/pitfall/003`](../../../ring_bench/docs/pitfall/003_ok_is_not_kept_and_the_verdict_inverts.md)

---

### Verdict 2 — the Contract door caps what the structures do not

**Established:** at four producers, **four of six candidates refuse to run at
all**, and only one of those four refusals belongs to the data structure.

| Candidate | Ceiling | Whose limit |
|---|---|---|
| `mutex_queue` | none | n/a |
| `direct_mpsc` | none | n/a |
| `contract_ring` | 1 | **the door**, since `ring_mpsc`'s ring is multi-producer |
| `off_the_shelf` | 1 | **the door**, since crossbeam's `ArrayQueue` is multi-producer |
| `tls_over_ring` | 1 | `ring_flush`'s staging |
| `direct_spsc` | 1 | genuine, because it is a single-producer ring |

`ring_factory::build` returns a `ring_handle::Split`, whose `Ends::split` yields
exactly one producer and offers no way to ask for a second.
`ring_core::Producer::try_clone` is the operation that would, and returns `Some`
for both Mpsc and Crossbeam. But `ring_handle::Producer` deliberately does not
re-expose it. It exposes exactly four methods (`try_push`, `try_push_batch`,
`free_capacity`, `is_full`).

**Why it is a verdict:** feature 186 requires the candidates be compared "under
the same producer counts". Through the export Contract, they cannot be. That is
a finding about the Contract's shape, produced by trying to satisfy the feature
and failing.

**Confidence: total, and structural rather than measured.** The ceilings are
declared constants; the refusals are computed from them. `direct_mpsc` exists
solely so the gap is *measurable* rather than merely asserted. It runs the same
structure through a different door and completes 1024 records where
`contract_ring` refuses.

**What it overturned:** the assumption, implicit across the family's docs, that
the five-name export Contract is a complete public API. It is complete for one
producer.

→ [`ring_bench/docs/pitfall/001`](../../../ring_bench/docs/pitfall/001_the_door_caps_what_the_structure_does_not.md)

---

### Verdict 3. Staging converts a shortfall into a stall

**Established:** on the cramped workload every candidate loses records.
`tls_over_ring` loses **all of them**.

```
tls_over_ring          256         0         0      256        0         9640
```

Zero reported, zero received, and **the fastest write time on the page**.

**The mechanism:** thread-local staging accumulates records and flushes in
batches. The full ring rejects the first flush. The rejected batch stays
staged, the stage stays full, and every subsequent append fails against it. The
benchmark does not retry, so the failure is absorbing rather than transient.

**Why it is a distinct verdict rather than a worse version of verdict 1:** the
other five candidates degrade *proportionally*. They keep what fits. Staging
degrades *categorically*. A capacity shortfall of 240 records becomes a total
loss of 256. The two failure modes need different mitigations and the comparison
would be misleading if it reported only a `dropped` column.

**Confidence: total for the counts; the mechanism is inferred.** That 0/0/256 is
what the run produces is deterministic. That the cause is a retained rejected
batch is read from `ring_flush`'s implementation, not measured by an assertion
in this crate.

→ [`ring_bench/docs/data_structure/002`](../../../ring_bench/docs/data_structure/002_three_counts_that_are_not_interchangeable.md)

---

### Verdict 4. Lock-free beats the mutex baseline by an order of magnitude

**Established, and stated exactly this coarsely:** with room for the whole
workload, the lock-free paths complete in roughly **one fifteenth** of the mutex
baseline's time, with no overlap between the two groups across any run observed.

```
mutex_queue    499485 ns
direct_spsc     32280 ns      ← 15.5x
contract_ring   36481 ns
tls_over_ring   45520 ns
off_the_shelf   46241 ns
direct_mpsc    299283 ns
```

**And nothing finer than that is supportable.** Ten runs of one identical
workload:

| Reading | Run 1 | Run 2 |
|---|---|---|
| Distinct winners in 10 runs | 3 | 2 |
| `contract_ring`'s own spread | 33760 – 188602 ns | 34321 – 72881 ns |
| Ratio | 5.6x | 2.1x |

**One candidate's run-to-run spread exceeds the gap between the leading
candidates in any single run.** The two runs above disagree about how unstable
the measurement is. The instability is itself unstable.

**Confidence: the group separation is solid; every individual ordering is
noise.** Which specific candidate is fastest changed between consecutive
executions of the same command on the same machine, minutes apart.

**What follows from it:** **no test in this family asserts a timing ordering.**
Not one. A flaky assertion inside a benchmark discredits precisely the
measurement the benchmark exists to produce. The suite asserts record accounting
instead. It is deterministic, and it is what makes a timing number mean anything
in the first place. `Comparison::fastest` filters on losslessness *before* comparing
times, and returns `None` when nothing was lossless, because the quickest way to
finish a write phase is to refuse every record.

→ [`ring_bench/docs/decisions/002`](../../../ring_bench/docs/decisions/002_no_test_asserts_an_ordering.md)

---

### What none of the four says

- **Which write path to adopt.** The comparison supplies inputs to that choice;
  it does not make it. Verdicts 2 and 3 constrain it more than verdict 4 does.
- **Anything about read paths.** Every measurement here is a write phase with a
  drain afterwards to establish ground truth.
- **Anything about a real workload.** Four synthetic fixtures, chosen to
  separate candidates, not to resemble a game loop.

### Guides

| File | Relationship |
|------|--------------|
| [001_running_the_verdicts_yourself.md](001_running_the_verdicts_yourself.md) | Every command that reproduces the evidence above |
| [003_what_the_gates_do_not_prove.md](003_what_the_gates_do_not_prove.md) | Why verdict 1 survived undefended at 100% coverage |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_bench/src/lib.rs`](../../../ring_bench/src/lib.rs) | `Candidate`'s declared ceilings, behind verdict 2's table |
| [`ring_types/src/policy.rs`](../../../ring_types/src/policy.rs) | `#[ default ]` on `DropNewest`, the fact verdict 1 rests on |
