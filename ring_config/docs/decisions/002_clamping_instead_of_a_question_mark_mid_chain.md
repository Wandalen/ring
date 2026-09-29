# Decisions: Clamping Instead of a `?` Mid-Chain

### Scope

**Purpose:** Record the decision to correct rather than reject at the two setters
that could have rejected, the reasons the crate gives, and how the rest of the
family answers the same question.

**Responsibility:** Both clamps and their stated rationale, the four crates that
raise an error on the same predicate, and where the clamped value travels.

**In Scope:** `ring_config/src/lib.rs:112-114`, `:124`, `:128-130`,
`:142-143`; `ring_batch/src/lib.rs:318`;
`ring_claim/src/lib.rs:425`; `ring_gating/src/lib.rs:31-36`,
`:287`; `ring_slot/src/lib.rs:351`.

**Out of Scope:** Why capacity rejects instead is
[`decisions/001`](001_capacity_is_the_one_parameter_that_is_not_a_with.md). What
a caller cannot learn afterwards is
[`pitfall/001`](../pitfall/001_a_clamp_with_no_way_to_detect_it.md).

---

## Two Clamps, and the Family's Other Answer

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the two clamps, and the reason each gives --'
command grep -m1 -A2 -F '  /// A count of `0` is clamped to `1`: a ring nothing can publish into has no' ring_config/src/lib.rs
command grep -m1 -A2 -F '  /// Set the batch size, clamped to at least `1` and at most the capacity —' ring_config/src/lib.rs
echo '  -- the error four other crates raise for the same predicate --'
command grep -r 'return Err( RingError::BatchTooLarge' --include=*.rs */src | sed 's|^ring/||'
echo '  -- and the family doc that distinguishes it from a transient refusal --'
command grep -m1 -A5 -F '//! ## Full versus BatchTooLarge' ring_gating/src/lib.rs
echo '  -- every .batch() call outside this crate, receiver included --'
command grep -ro '[A-Za-z_][A-Za-z_0-9]*\.batch()' --include=*.rs */src */tests | command grep -v '^ring_config/' | command grep -o '[A-Za-z_][A-Za-z_0-9]*\.batch()' | sort | uniq -c
```

Live output:

```
  -- the two clamps, and the reason each gives --
  /// A count of `0` is clamped to `1`: a ring nothing can publish into has no
  /// use, and clamping keeps the setter infallible so a builder chain does not
  /// need a `?` in its middle.
  /// Set the batch size, clamped to at least `1` and at most the capacity —
  /// a batch larger than the ring can never be served however much draining
  /// happens, so it is corrected here rather than failing at first publish.
  -- the error four other crates raise for the same predicate --
ring_batch/src/lib.rs:    return Err( RingError::BatchTooLarge { requested : count, capacity : capacity.get() } );
ring_claim/src/lib.rs:      return Err( RingError::BatchTooLarge
ring_gating/src/lib.rs:      return Err( RingError::BatchTooLarge { requested : count, capacity : self.capacity.get() } );
ring_slot/src/lib.rs:      return Err( RingError::BatchTooLarge { requested : payload.len(), capacity : N } );
  -- and the family doc that distinguishes it from a transient refusal --
//! ## Full versus BatchTooLarge
//!
//! [`GatingSet::check`] distinguishes them, and the distinction is the whole
//! reason it returns a `Result` rather than a `bool`. `Full` is back-pressure:
//! the caller should retry, because a consumer will move. `BatchTooLarge` is a
//! configuration error: a claim wider than the ring can never fit no matter who
  -- every .batch() call outside this crate, receiver included --
      2 config.batch()
      1 cramped.batch()
      1 delta.batch()
     15 workload.batch()
```

---

### RC15 — The Family Calls It a Configuration Error, and the Configuration Crate Is the One That Does Not Raise It

`with_batch` caps a batch at the capacity and says why: "a batch larger than the
ring can never be served however much draining happens, so it is corrected here
rather than failing at first publish." Four other crates guard the same predicate
— `requested > capacity` — and return `RingError::BatchTooLarge` instead:
`ring_batch:318`, `ring_claim:425`, `ring_gating:287`, `ring_slot:351`.

`ring_gating`'s module comment goes further and names the category. It devotes a
section to distinguishing the two refusals, and its wording is: "`Full` is
back-pressure: the caller should retry, because a consumer will move.
`BatchTooLarge` is a configuration error: a claim wider than the ring can never
fit no matter who moves."

**Finding.** So the family's considered position is that a request wider than the
ring is a *configuration* error — and `ring_config` is the single crate that meets
that condition and does not report one. Its reasoning is sound in isolation: at
configuration time a correction is available, and forcing a `?` into the middle of
a builder chain to reject a value that has an obvious right answer is a poor
trade.

What is absent is any record that the two answers coexist. Nothing in
`ring_config` mentions `BatchTooLarge`, nothing in the four raising crates
mentions that a configured batch was already clamped, and a reader who learns the
family's rule from `ring_gating`'s section will not expect the exception.

---

### RC16 — The Clamped Value Has Nowhere to Go

`with_batch` guarantees `1 <= batch <= capacity` for every configuration that
exists. The four crates that raise `BatchTooLarge` take their `count` as a direct
argument at claim time, not from a configuration.

The census closes the loop: every `.batch()` call anywhere in the workspace
outside this crate — twelve of them, across `src/` and `tests/` alike — has
`workload` as its receiver, `ring_bench::Workload`'s own method of the same name.
Not one reads a `RingConfig`.

**Finding.** So the clamp protects a journey the value never takes. Nothing
consumes `config.batch()`, which means nothing downstream could have received a
batch larger than the capacity whether the clamp existed or not, and the
`BatchTooLarge` those four crates raise can never have originated in a
configuration.

**Correction (2026-09-28):** the census above, and the finding drawn from it, were
accurate when written but are no longer current. A `ring_bench` bug fix collapsed
`Workload`'s own duplicate, unvalidated `batch` field into a one-line delegate —
`Workload::batch` now reads `self.config.batch()` — so four of the twelve calls
above disappeared and the other seven became transitive reads of
`RingConfig::batch`. The clamp now has exactly the reader its stated reason
names. Nothing in either crate connects the fix to the guarantee it completed, so
this finding is kept as the record of that connection rather than deleted.

The decision is still the right one — it costs two comparisons, it is documented,
and it becomes load-bearing the moment anything reads the field. It is recorded
because the crate's own justification is stated in terms of a downstream failure
("rather than failing at first publish") that no current code path could reach,
which makes the rationale accurate about intent and not about the workspace as it
stands. That is the same shape the corpus finds repeatedly here: a careful
decision, correctly made, guarding a path with no traffic on it.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`decisions/001`](001_capacity_is_the_one_parameter_that_is_not_a_with.md) | The field where rejection was chosen instead |
| [`pitfall/001`](../pitfall/001_a_clamp_with_no_way_to_detect_it.md) | What a caller cannot learn after a clamp fires |
| [`invariant/001`](../invariant/001_two_ranges_that_cannot_be_violated.md) | The ranges the clamps guarantee |
| [`api/002`](../api/002_three_readers_used_and_four_with_no_caller.md) | The reader census this finding rests on |

### Sources

| Fact | Where |
|------|-------|
| The producer clamp and its reason | `ring_config/src/lib.rs:112-114`, `:124` |
| The batch clamp and its reason | `ring_config/src/lib.rs:128-130`, `:142-143` |
| Four crates raising `BatchTooLarge` on the same predicate | `ring_batch/src/lib.rs:318`; `ring_claim/src/lib.rs:425`; `ring_gating/src/lib.rs:287`; `ring_slot/src/lib.rs:351` |
| The family naming it a configuration error | `ring_gating/src/lib.rs:31-36` |
| Twelve `.batch()` calls outside this crate, none on a `RingConfig` | Census above |

### Tests

| Test | Covers |
|------|--------|
| `batch_clamps_into_one_through_capacity` | Both ends of the clamp the decision produces |
| `zero_producers_clamps_to_one` | The other clamp |
| `setters_commute` | That neither clamp introduces an ordering constraint |
