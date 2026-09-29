# non_functional_requirement

Two requirements about *how* this crate performs rather than what it computes —
one it has always met and layers carefully, one it broke for the whole time
anybody was looking at it and now meets.

### Overview Table

| ID | Name | Requirement | Met? |
|----|------|-------------|:----:|
| 001 | [The Layout Claim Is Testable](001_the_layout_claim_is_testable.md) | The cache-line claim must be checkable in a unit test, on any machine, without contention | ✅ |
| 002 | [The Gating Read Allocates Nothing](002_the_gating_read_allocates_nothing.md) | A gating read on a producer's admission path allocates nothing | ✅ since `b7e075ca` |

### The Two Are the Same Problem From Opposite Sides

001 is about a claim that is **hard to measure and easy to verify structurally**:
throughput under contention cannot go in a unit test, so the claim is decomposed
until three of its four layers are compile-time or deterministic facts.

002 was the reverse — a defect that was **easy to see and hard to price**. The
allocation was one visible line; whether it cost anything depended on whether the
optimiser removed it and on how often the retry loop it sat in actually retried.
Neither was ever measured, and the line was deleted by another crate's change
before anybody got round to it.

| | 001 | 002 |
|---|---|---|
| The fact | Verified five ways | Was verified by reading one line; the line is gone |
| The consequence | Unmeasured (no benchmark harness yet) | Unmeasured, and now unmeasurable — no before-state survives |
| Confidence the fact is right | High | High, and it is now a past-tense fact |
| Confidence it matters | Assumed | **Never established** |
| A test that would catch a regression | `cursor_test.rs`, three assertions | `allocation_test.rs`, four call shapes and a control arm |

**They no longer end at the same missing instrument.** There is still no benchmark
harness, so the padding's benefit has no number and 001 is written so that when
one arrives its result will be interpretable. 002's instrument turned out to
exist all along — a test-only counting allocator, which the document had argued
was forbidden and was not (→ CU35).

### The Honest Summary

The crate's layout claims are among the best-verified in the family. Its
allocation behaviour is now verified too, by a test that was written years' worth
of readings later than the finding asking for it — and the one allocation the
crate used to make sat in the loop condition of a lock-free CAS retry, the
position where an allocation does the most damage if it does any at all. What it
actually cost there was never measured and cannot be now.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the requirement this crate meets, and the test that says so --'
command grep 'fn a_padded_cursor_occupies_exactly_one_cache_line' ring_cursor/tests/cursor_test.rs
echo '  -- the requirement it used to break, and the test that now pins it --'
command grep 'fn the_gating_fold_allocates_nothing_at_every_arity' ring_cursor/tests/allocation_test.rs
printf '    call shapes asserted allocation-free: %s\n' \
  "$( tr '\n' ' ' < ring_cursor/tests/allocation_test.rs | tr -s ' ' \
      | command grep -oE '\( 0, 0 \), "[^"]+"' | wc -l )"
printf '    assertions that a deliberate allocation is seen: %s\n' \
  "$( command grep -c 'control_calls >= 1' ring_cursor/tests/allocation_test.rs )"
echo '  -- and the line that used to break it --'
printf '    Vec< Seq > in src/lib.rs: %s\n' \
  "$( command grep -c 'Vec< Seq >' ring_cursor/src/lib.rs )"
```

Live output:

```
  -- the requirement this crate meets, and the test that says so --
fn a_padded_cursor_occupies_exactly_one_cache_line()
  -- the requirement it used to break, and the test that now pins it --
fn the_gating_fold_allocates_nothing_at_every_arity()
    call shapes asserted allocation-free: 4
    assertions that a deliberate allocation is seen: 1
  -- and the line that used to break it --
    Vec< Seq > in src/lib.rs: 0
```

**Both requirements are met and both have a test.** The control arm is the part
worth reading twice: a counting allocator that is silently not installed reports
zero for everything, and zero is the answer the four assertions above are looking
for, so a broken instrument and a clean result are the same output
(→ [`002`](002_the_gating_read_allocates_nothing.md)).

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CU33 | The met requirement | n/a — observation | The layout claim is met and its evidence is three assertions in two tests. Nothing in this crate's suite measures the property the padding exists to buy — throughput that does not fall as core count rises — which lives in `ring_bench` and is a different crate's requirement |
| CU34 | `tests/cursor_test.rs` | n/a — coverage | That file still holds zero occurrences of `alloc`, and for as long as it was the crate's only test file neither requirement here had an allocation-observing test. The gap is closed by a second file rather than by that one: `tests/allocation_test.rs` |
| CU35 | `GlobalAlloc` | **wrong doc** | 002 recorded that the measurement was unavailable by policy — `unsafe` confined by gate G6 to `ring_spsc`, `ring_mpsc` and `ring_core`. `g6_unsafe.sh` scans `"$MODULE/$c/src"` and nothing else, so a test-only opt-out was never within the gate's reach and the policy was never the obstacle it was named as |
| CU36 | `ring_flush/tests/append_cost_test.rs:9-11` | **wrong doc** | That file named the allowlist as `ring_atomic`, `ring_store`, `ring_slot` and `ring_align` — an earlier ruling's list, every name of which a later ruling replaced. Fixed at the source; the file now names the live three and states G6's scan path itself |
| CU53 | The `tests/` unsafe allowance | n/a — observation | G6 permits a test-only counting allocator in every crate. Four took it — `ring_cursor`, `ring_barrier`, `ring_claim`, `ring_consume` — and `ring_flush` declined it on the ground that spreading the opt-out defeats the gate's purpose even outside its scan path. No decision rules between the two positions |
