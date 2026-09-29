# Invariant: A Policy Fires Only at Its Trigger

### Scope

- **Purpose**: State the negative half of this crate's acceptance criterion — each policy fires *and at no other point* — and account for why that half is the one nothing naturally observes.
- **Responsibility**: Fix the restriction, name what enforces it, and enumerate the ways it is violated silently.
- **In Scope**: Trigger exclusivity for all three policies; the observability this restriction forces.
- **Out of Scope**: Whether each trigger is *correct* for a workload, which is a tuning question; the sequencing after the decision (→ [Sequencing Seal, Drain and Reset](../algorithm/002_sequencing_seal_drain_reset.md)).

### Invariant Statement

**Each of the three policies causes a flush at exactly its stated trigger, and
at no other point in the program's execution.**

- `OnFull` — when the buffer has no room for the record being appended, and only then.
- `OnBarrier` — when the driver is told the barrier is reached, and only then.
- `OnBatch( n )` — when the count of records accumulated since the last flush reaches `n`, and only then.

The
[acceptance table](../../../bench_harness/docs/acceptance/001_feature_reached_tests.md)
states both halves in one sentence: each policy "fires at exactly its stated
trigger **and at no other point**, asserted by a scripted sequence with a
recorded flush log."

**The two halves are not equally hard.** That a policy fires when it should is
a positive claim: arrange the trigger, observe the flush. That it fires *only*
then is a claim about every moment the trigger did not occur, which cannot be
observed by watching for something — it requires a record of everything that
did happen, compared against what should have.

### Enforcement Mechanism

| # | Mechanism | Covers | Gap |
|---|-----------|--------|-----|
| E1 | The recorded flush log (→ [The Flush Log](../data_structure/002_the_flush_log.md)) | Every flush that occurred, with its cause | Only what the log records. An unrecorded flush path is invisible to the mechanism designed to catch it |
| E2 | The scripted sequence in `ring_flush/tests/flush_test.rs` | The specific interleavings the test author thought of | Any sequence not scripted |
| E3 | A single flush entry point in this crate | Flushes routed through this crate | **Nothing prevents a caller flushing `ring_tls` directly** — the primitives are public, by `ring_tls`'s deliberate design |
| E4 | `FlushOutcome` reporting what happened (→ [Flush Outcome](../type/002_flush_outcome.md)) | A caller who checks the return | A caller who ignores it |

**E3 is the structural hole and it is not this crate's to close.**
[`ring_tls`'s consolidator read surface](../../../ring_tls/docs/api/002_consolidator_read_surface.md)
exposes seal, drain and reset as three separate public operations, precisely so
that this crate can sequence them. That same publicity means any code may call
them in any order without consulting a policy at all. The invariant above
constrains *this crate's* behaviour; it cannot constrain the buffer's.

**So the invariant is exactly as strong as the convention that flushes go
through here** — which is the same shape as
[`ring_handle`'s export-boundary gap](../../../ring_handle/docs/integration/002_on_the_export_surface.md),
and has the same answer: a gate would have to check it, and no gate does.

### Violation Consequences

| # | Violation | Detected by | Consequence |
|---|-----------|-------------|-------------|
| V1 | `OnFull` also flushes on a size threshold "for safety" | E1, if the log records causes rather than just occurrences | Latency becomes bimodal; the benchmark measures a policy nobody configured |
| V2 | `OnBatch( n )` fires at `n` *or* on full | ~~**Nothing.**~~ **Unreachable — see below** | ~~The measured throughput of `OnBatch` silently includes `OnFull`'s behaviour~~ |
| V3 | A caller invokes `ring_tls`'s drain directly | Nothing here. E3's gap | Records reach the ring outside any policy — the exact state this crate's "If Missing" clause describes |
| V4 | `OnBarrier` also flushes on a timer, because the barrier signal was unreliable | E2 only if scripted | Publication order becomes timing-dependent, defeating [the publication-point invariant](002_publication_point_is_designed_not_inherited.md) |
| V5 | A flush occurs during `Drop` | `dropping_a_driver_with_records_staged_publishes_nothing` | A buffer flushes after the test's last assertion; the log is checked before the entry appears |

### V2 was called the most dangerous violation, and it cannot happen

**This section said V2 was "both the most tempting and the least detectable,"
and the argument was sound right up to its conclusion.** An `OnBatch( 64 )`
policy on a buffer that fills at 50 records can never reach its trigger, so
flushing on full is the obvious fix, and it silently merges two policies the
benchmark is trying to compare.

The resolution this section proposed is the right one: **the configuration is
invalid, not the policy's behaviour**, which makes V2 a validation question
rather than a behavioural one (→ [Flush Policy](../type/001_flush_policy.md)'s
N2). What it did not notice is that once that validation exists, V2 stops being
a violation anyone can commit.

The chain: N2 caps `n` at the buffer's capacity, so `len >= n` holds no later
than `len == capacity`; and once it holds, the flush empties the buffer. **An
`OnBatch` policy therefore never observes a full buffer**, and the "or on full"
clause has no state in which it can fire. Confirmed by injecting exactly that
defect and finding all 23 tests still green — then by a second probe asserting
`since_flush == buffer.len()` at every trigger evaluation
(`tests/manual/readme.md`'s F3).

**That second probe cost the counter its existence.** `since_flush` was a
`usize` incremented on every `append`, and the probe found it equal to
`buffer.len()` at every evaluation without exception — so it was deleted, and
`OnBatch( n )` now reads `buffer.len() >= n` directly
(→ [`algorithm/001`](../algorithm/001_evaluating_a_policy_at_an_append.md)'s
removed Step 3). The probe was aimed at V2 and hit something else; there is no
counter in this crate to be wrong about.

**Two things follow, and neither is a test.** V2 needs no detection mechanism,
which is why no test in `flush_test.rs` targets it and why
`the_batch_trigger_arrives_no_later_than_the_buffer_fills` asserts the
*unreachability* instead of the violation. And the reason
[`nfr/001`](../non_functional_requirement/001_three_triggers_proven_by_a_flush_log.md)
gives for its M3 — that the `cause` field exists to catch V2 — is wrong;
corrected there.

**V5 is the reason this invariant has to mention `Drop` at all, and it is now
detected.** A flush in a destructor is invisible to a test that asserts and then
returns, and it is exactly what a well-meaning implementation adds to avoid
losing records at teardown. The detection is not subtle — construct a driver,
stage records, drop it without a final drain, then ask the *consumer* whether
anything arrived. The consumer outlives the driver, so the observation happens
after the destructor rather than before it, which is the whole difficulty this
row described. Whether the final drain is a policy decision or a shutdown
obligation is settled in
[From Configuration to the Final Drain](../lifecycle/002_from_configuration_to_the_final_drain.md);
this invariant only requires that it not be a *silent* one.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_evaluating_a_policy_at_an_append.md](../algorithm/001_evaluating_a_policy_at_an_append.md) | The evaluation this invariant constrains — its step 4 is where V1 and V2 would be introduced |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_flush_log.md](../data_structure/002_the_flush_log.md) | E1 — the mechanism this invariant forces into existence, and its own costs |

### Invariants

| File | Relationship |
|------|--------------|
| [002_publication_point_is_designed_not_inherited.md](002_publication_point_is_designed_not_inherited.md) | The companion restriction; V3 and V4 violate both at once |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_three_triggers_proven_by_a_flush_log.md](../non_functional_requirement/001_three_triggers_proven_by_a_flush_log.md) | This invariant stated as a measurable criterion with a threshold |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_on_barrier_cannot_see_the_barrier.md](../pitfall/001_on_barrier_cannot_see_the_barrier.md) | Why V4's "the barrier signal was unreliable" is a realistic premise rather than a hypothetical |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_flush_policy.md](../type/001_flush_policy.md) | Where V2's resolution belongs — as a validation rule, not a behavioural fallback |
| [../type/002_flush_outcome.md](../type/002_flush_outcome.md) | E4 — what a caller would have to check for the invariant to be observable at runtime |

### Sources

| File | Relationship |
|------|--------------|
| [`bench_harness/docs/acceptance/001_feature_reached_tests.md`](../../../bench_harness/docs/acceptance/001_feature_reached_tests.md) | Row 176 — the "and at no other point" clause this invariant restates, and the flush log it mandates |
| [`ring_tls/docs/api/002_consolidator_read_surface.md`](../../../ring_tls/docs/api/002_consolidator_read_surface.md) | E3's gap — the three primitives are public by design, so direct calls cannot be prevented from here |

### Tests

| File | Relationship |
|------|--------------|
| `tests/flush_test.rs` | E1 and E2, with causes recorded — `every_entry_names_its_own_policys_trigger`. V1 would be caught by it. **V2 turns out to be unreachable**, so no test can catch it and none needs to: see the correction to V2's row above, and `tests/manual/readme.md`'s F3 for the injection that established it |

### FL21 — A Deletion Rests on a Fault Injection Against a Suite Twelve Tests Smaller Than Today's

The evidence that removed the batch counter is quoted with its sample size:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the suite the experiment ran against, as recorded --'
for f in ring_flush/tests/manual/readme.md ring_flush/docs/*/[0-9][0-9][0-9]_*.md
do
  awk '/^### FL/{ exit } /all 23 tests|23 tests run/{ printf "%s:%d: %s\n", FILENAME, NR, substr( $0, 1, 60 ) }' "$f"
done | sed 's|ring_flush/||'
echo '  -- the suite today --'
printf '    #[test] functions across tests/: %s\n' \
  "$( command grep -rhc '^#\[ *test *\]' ring_flush/tests/*.rs | paste -sd+ | bc )"
echo '  -- and whether any of the three records says when it was measured --'
command grep -r 'Result (2026' ring_flush/tests/manual/readme.md | sed -E 's/^(.{0,86}).*/\1/'
```

Live output:

```
  -- the suite the experiment ran against, as recorded --
tests/manual/readme.md:53: Summary  16/23 tests run: 12 passed, 4 failed
tests/manual/readme.md:57: it was undercounted.** `16/23 tests run` is nextest stopping
tests/manual/readme.md:149: Summary [0.023s] 23 tests run: 23 passed, 0 skipped
tests/manual/readme.md:169: Summary [0.041s] 23 tests run: 23 passed, 0 skipped
docs/invariant/001_a_policy_fires_only_at_its_trigger.md:80: defect and finding all 23 tests still green — then by a seco
docs/non_functional_requirement/001_three_triggers_proven_by_a_flush_log.md:85: defect left all 23 tests green, and the probe that followed 
  -- the suite today --
    #[test] functions across tests/: 36
  -- and whether any of the three records says when it was measured --
**Result (2026-08-28): red, and by more than predicted — four failures.**
**Result (2026-08-28): holds.**
**Result (2026-08-28): WRONG. All 23 tests passed with the defect present.**
**Result (2026-08-28): holds.**
**Result (2026-08-28): holds.**
**Result (2026-08-28): fails to compile, and the prediction named the wrong
```

Thirty-six tests now; the experiment saw twenty-three. Two doc instances and the
manual test plan all carry the twenty-three, and only the manual plan carries a
date.

**The conclusion is probably still right and the evidence no longer supports it
at the strength quoted.** "All 23 tests still green with the defect present"
means the suite could not distinguish `OnBatch( n )` from `OnBatch( n ) or full`.
Twelve tests have been added since, several of them about batch behaviour, and
whether *those* would catch the injected defect is unmeasured. The sentence in
this instance does not say "as of a suite of twenty-three"; it reads as a
standing property of the crate.

**A negative result is the kind that decays.** "The suite caught it" stays true
forever — a test that failed once will fail again. "The suite did not catch it"
is a statement about coverage at a moment, and coverage is the thing most likely
to have changed since. This instance cites the durable-sounding half of a
perishable result, and it did so at exactly the point where a counter was deleted
on the strength of it.

The reusable shape: **fault injection produces a result with a shelf life, and
nothing in the corpus records an expiry.** The manual plan dates its run, which
is the right instinct and the wrong half — a date tells a reader when, not
whether the answer would still be the same.

### FL22 — Seven Instances Record the Ownership Half-Close and the Two Rows About That Hole Are Not Among Them

E3 and V3 state the gap without qualification:

```sh
# -mN and no -n: this file is its own subject, so an unbounded
# match also finds this command line and every copy of its own
# output below, and -n re-prefixes a fresh line number onto each
# earlier pass's output -- the stacked addresses this block carried
cd "$(git rev-parse --show-toplevel)"
echo '  -- the two rows --'
command grep -m2 '^| E3 \|^| V3 ' ring_flush/docs/invariant/001_a_policy_fires_only_at_its_trigger.md \
  | sed -E 's/^(.{0,118}).*/\1/'
echo '  -- instances recording that the buffer is taken by value --'
for f in ring_flush/docs/*/[0-9][0-9][0-9]_*.md
do
  case "$f" in */invariant/001_*) continue ;; esac
  awk '/^### FL/{ exit } /[Tt]akes the (buffer|`TlsBuffer`) \*{0,2}by value/{ print FILENAME; exit }' "$f"
done | sed 's|ring_flush/docs/|    |' | sort
echo '  -- and how the sibling instance phrases the same hole --'
command grep 'shrinks from' ring_flush/docs/integration/002_a_decision_on_the_export_surface.md \
  | sed -E 's/^(.{0,110}).*/\1/'
```

Live output:

```
  -- the two rows --
| E3 | A single flush entry point in this crate | Flushes routed through this crate | **Nothing prevents a caller flus
| V3 | A caller invokes `ring_tls`'s drain directly | Nothing here. E3's gap | Records reach the ring outside any poli
  -- instances recording that the buffer is taken by value --
    algorithm/002_sequencing_seal_drain_reset.md
    api/001_the_policy_surface.md
    api/002_the_driver_surface.md
    integration/001_two_dependencies_and_the_barrier_it_cannot_see.md
    integration/002_a_decision_on_the_export_surface.md
    lifecycle/003_buffer_state_through_a_flush.md
    type/001_flush_policy.md
  -- and how the sibling instance phrases the same hole --
So X5's obligation shrinks from "any buffer, at any time" to "a buffer the
```

E3 says "**Nothing prevents a caller flushing `ring_tls` directly**." V3 says a
caller invoking that drain is detected by "Nothing here." Both are unqualified,
and both are about a buffer this crate holds.

**For a buffer this crate holds, the thing they describe is unrepresentable.**
`Flusher::new` takes the `TlsBuffer` by value, so after binding there is no
borrow on which `drain()` could be called and no accessor that yields one. The
reachable version is narrower: a consumer keeps a *second*, unbound buffer and
publishes from it — which `integration/002` states exactly, in the words "X5's
obligation shrinks from any buffer, at any time to a buffer the consumer
deliberately withheld."

**The by-value fact is not obscure in this crate.** Seven other instances record
it, each as a consequence for something of their own: the sealing step has no
epoch swap, the policy surface has one fewer error case, the driver surface has
its lifetime, the dependency seam half-closes, the export hole shrinks, the
buffer's state machine loses a transition, and the policy type keeps its traits.
It is among the most repeated facts in this corpus, and it did not reach the two
rows whose entire subject is the hole it partly closed.

The reusable shape: **a fact propagates to the documents that follow from it and
not to the ones it falsifies.** Every instance that mentions by-value ownership
mentions it as a reason for something. The rows that needed updating are the ones
where it is a reason *against* something already written, and nothing walks that
direction.
