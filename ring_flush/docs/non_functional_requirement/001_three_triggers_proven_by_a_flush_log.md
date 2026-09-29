# Non-Functional Requirement: Three Triggers, Proven by a Recorded Flush Log

### Scope

- **Purpose**: Carry this criterion's binary Reached condition, and account for the fact that it is one criterion bundling three independent claims plus a negative one that no other row in the acceptance table asks for.
- **Responsibility**: State the quality attribute, the measurable statement, the measurement method, and the acceptance threshold.
- **In Scope**: The three-policy criterion; the flush log's role as evidence.
- **Out of Scope**: The append-path cost ceiling (→ [The Decision Costs Nothing on the Append Path](002_the_decision_costs_nothing_on_the_append_path.md)); the log's structure (→ [The Flush Log](../data_structure/002_the_flush_log.md)).

### Quality Attribute

**Behavioural precision** — that a configured policy does exactly what its name
says, and specifically that it does *nothing else*.

This is the only row among the acceptance table's twenty-two whose criterion is
partly negative in the temporal sense: not "the output is correct" but "no
event occurred outside this set." `ring_handle`'s row is negative too, in a
different way — its negatives are compile-time absences provable by a compiler
(→ [its own criterion](../../../ring_handle/docs/non_functional_requirement/001_proven_by_code_that_must_not_compile.md)).
This one's negatives are runtime non-events, provable only by enumeration.

### Statement

**Each of the three policies — `OnFull`, `OnBarrier`, `OnBatch( n )` — fires at
exactly its stated trigger and at no other point, demonstrated by a scripted
sequence against a recorded flush log.**

Quoted from
[the acceptance table](../../../bench_harness/docs/acceptance/001_feature_reached_tests.md),
row 176, whose claiming test is `ring_flush/tests/flush_test.rs`.

**It bundles four independent claims:**

| # | Claim | Independent of |
|---|-------|----------------|
| B1 | `OnFull` fires when and only when the buffer cannot accept a record | B2, B3 |
| B2 | `OnBarrier` fires when and only when a barrier is announced | B1, B3 |
| B3 | `OnBatch( n )` fires when and only when `n` records have accumulated | B1, B2 |
| B4 | The log is a faithful record of what actually happened | **All three** — if B4 fails, B1–B3 are unverified regardless of what the test asserts |

**B4 is the criterion beneath the criterion and it is not stated in the
acceptance row.** The row mandates a log and asserts against it; nothing in the
row requires the log to be complete. A flush path that bypasses logging
satisfies every assertion while proving nothing
(→ [trigger exclusivity](../invariant/001_a_policy_fires_only_at_its_trigger.md)'s
E1 gap). The mitigation is structural: write log entries from
[`FlushOutcome`](../type/002_flush_outcome.md) rather than beside it, so a
flush that produced an outcome and no log entry is impossible by construction
rather than by discipline.

### Measurement Method

| # | Measurement | Mechanism |
|---|-------------|-----------|
| M1 | Positive case per policy | Script the trigger; assert exactly one log entry with the matching `cause` |
| M2 | **Negative case per policy** | Script a sequence that triggers the *other* two policies' conditions; assert **zero** entries |
| M3 | Cause fidelity | Assert on `FlushEntry::cause`, not merely on entry count — a flush for the wrong reason is the failure mode a count cannot see |
| M4 | Log completeness | Assert `outcome` and log entry agree for every drive call in the scenario |
| M5 | Boundary | `OnBatch( n )` at `n-1` records: assert no flush. At `n`: assert exactly one |

**M2 is the measurement that discharges the "and at no other point" clause**
and it is the one most likely to be omitted, because it asserts an absence and
absences do not feel like tests. The concrete shape: configure `OnBarrier`,
fill the buffer to capacity, drive repeatedly without announcing a barrier,
assert the log is empty. That single scenario catches
[the pitfall](../pitfall/001_on_barrier_cannot_see_the_barrier.md)'s F3 —
`OnBarrier` silently degenerating to flush-on-every-drive.

**This paragraph originally added "which no positive test can detect," and
that was measured and found false.** Injecting the degeneration turned four
tests red, two of them positive
(`tests/manual/readme.md`'s F1). The distinction that actually matters is not
positive versus negative but **firing versus not-firing**: any assertion that a
given drive returns `NotTriggered` catches it, and such assertions appear
incidentally, as setup lines in tests measuring something else. The correct
claim is narrower and still worth stating — M2 is the only measurement that
catches F3 *deliberately*. The other catchers would stop catching it the moment
their setup lines were rewritten for an unrelated reason.

**M3 was said to be what makes
[trigger exclusivity](../invariant/001_a_policy_fires_only_at_its_trigger.md)'s
V2 detectable — and V2 turns out not to be reachable.** The reasoning was: an
`OnBatch( 64 )` policy that also flushes when full produces entries at plausible
moments, so only the `cause` field distinguishes them. Injecting exactly that
defect left all 23 tests green, and the probe that followed explains why: the
count of records staged since the last flush *is* the buffer's occupancy, and
validation caps `n` at capacity, so the batch trigger fires no later than the
buffer fills. **An `OnBatch` policy never observes a full buffer**
(`tests/manual/readme.md`'s F3).

M3 stays, with a different justification. It discriminates `Shutdown` from a
policy firing on the final drain — where both produce one entry and only the
`cause` says which — and it attributes entries in a log spanning a policy
change. Neither is V2.

### Acceptance Threshold

**Binary, and Reached requires all five measurements passing simultaneously.**

Per the acceptance table's own rule — "Reached is binary. Where a criterion
below names a number, the number is the test's assertion, not a target to
approach."

| Measurement | Threshold |
|-------------|-----------|
| M1 | Exactly 1 entry, correct `cause`, for each of 3 policies |
| M2 | Exactly 0 entries, for each of 3 policies |
| M3 | `cause` matches the configured policy's trigger in every entry |
| M4 | Agreement on every drive call — no tolerance |
| M5 | 0 entries at `n-1`; exactly 1 at `n` |

**And the test file must cite `docs/feature/176_` textually.** `g3_features.sh`
does not record a crate→feature edge, though: it greps every family crate's
`tests/` tree for the id with no file-type filter, so any file under any
family crate's `tests/` — including a hand-written plan like
`tests/manual/readme.md` — satisfies it. The reachable failure is therefore a
**false positive**: the feature reported claimed because some document
somewhere in the family mentions its number, not because this crate's own
executable suite does. `ring_flush` is clean today — it is the only crate
citing 176, and it cites it from `flush_test.rs` itself — but that is a fact
about this crate, not a property the gate holds (→ `FL33` below).

### What this criterion does not cover

| Uncovered | Why it matters |
|-----------|----------------|
| Whether `OnBarrier`'s announcement corresponds to a real barrier | Unverifiable from here (→ [The Driver Surface](../api/002_the_driver_surface.md)) |
| Whether any code flushes `ring_tls` directly, bypassing this crate | Outside this crate; no gate checks it (→ [the publication-point invariant](../invariant/002_publication_point_is_designed_not_inherited.md)'s P5) |
| Whether `n` is well-chosen against `ring_batch`'s claim width | Two crates, one benchmark (→ [`pitfall/002`](../pitfall/002_two_batch_sizes_that_must_not_diverge.md)) |
| Multi-threaded interleaving | The buffer is thread-local; the scripted sequence is single-threaded by construction |

**The first two are the crate's standing blind spots** and both are the same
shape — this crate can be correct while the system around it is not, and its
own suite stays green either way.

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_flush_log.md](../data_structure/002_the_flush_log.md) | The evidence structure; B4 is why its entries must derive from the outcome |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_a_policy_fires_only_at_its_trigger.md](../invariant/001_a_policy_fires_only_at_its_trigger.md) | The same requirement as a standing restriction rather than a measurement |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [002_the_decision_costs_nothing_on_the_append_path.md](002_the_decision_costs_nothing_on_the_append_path.md) | The companion requirement, which the acceptance table does not state |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_on_barrier_cannot_see_the_barrier.md](../pitfall/001_on_barrier_cannot_see_the_barrier.md) | M2's target — F3 is what the negative case catches |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_flush_outcome.md](../type/002_flush_outcome.md) | M4's other side; its M5 is B4's structural mitigation |

### Sources

| File | Relationship |
|------|--------------|
| [`bench_harness/docs/acceptance/001_feature_reached_tests.md`](../../../bench_harness/docs/acceptance/001_feature_reached_tests.md) | This crate's row verbatim, and the binary-Reached rule |

### Tests

| File | Relationship |
|------|--------------|
| `tests/flush_test.rs` | The claiming test — 24 tests, citing `docs/feature/176_` in its first line. M1–M5 all discharged: M1/M2 by the three policy pairs, M3 by `every_entry_names_its_own_policys_trigger`, M4 by `the_log_agrees_with_every_outcome_it_recorded`, M5 by `on_batch_fires_at_the_boundary_and_not_before` |
| `tests/manual/readme.md` | F1 falsified M2's "no positive test" claim; F3 established that M3's stated target V2 is unreachable. Both corrections are above |

### FL33 — The Gate Behind the Citation Rule Reads Prose as Readily as Code, and Never Checks Which Crate Wrote It

The rule this criterion states and the gate that enforces it are not the same rule:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- what this instance says the gate does --'
awk '/^### FL/{ exit } /g3_features\.sh|test file must cite/{ printf "    %d: %s\n", NR, $0 }' \
  ring_flush/docs/non_functional_requirement/001_three_triggers_proven_by_a_flush_log.md \
  | sed -E 's/^(.{0,118}).*/\1/'
echo '  -- what the gate actually searches --'
command grep -E 'search\+=|grep -rqE' bench_harness/gate/g3_features.sh \
  | sed -E 's/^(.{0,124}).*/\1/'
echo '  -- family test trees whose markdown already satisfies that grep --'
command grep -rlE 'docs/feature/0*[0-9]+_' ring_*/tests --include='*.md' 2>/dev/null \
  | sed ' s|/tests/manual/readme.md||' | tr '\n' ' ' | sed 's/^/    /; s/$/\n/'
printf '    prose files that count as a claiming test: %s\n' \
  "$( command grep -rlE 'docs/feature/0*[0-9]+_' ring_*/tests --include='*.md' 2>/dev/null | wc -l )"
echo '  -- crates citing feature 176 today --'
command grep -rlE 'docs/feature/0*176_' ring_*/tests 2>/dev/null | sed 's/^/    /'
```

Live output:

```
  -- what this instance says the gate does --
    112: **And the test file must cite `docs/feature/176_` textually.** `g3_features.sh`
  -- what the gate actually searches --
  d="$( crate_dir "$c" )" && [ -d "$d/tests" ] && search+=( "$d/tests" )
  if [ ${#search[@]} -gt 0 ] && grep -rqE "docs/feature/0*${id}_" "${search[@]}" 2>/dev/null; then
  -- family test trees whose markdown already satisfies that grep --
    ring_barrier ring_store ring_claim ring_consume ring_core ring_cursor ring_event ring_gating ring_publish ring_spsc ring_tls ring_trace ring_wait 
    prose files that count as a claiming test: 13
  -- crates citing feature 176 today --
    ring_flush/tests/flush_test.rs
```

The instance says the citation "is how `g3_features.sh` records the crate→feature
edge," and that a test passing every measurement without it "reports the feature
unclaimed — a false negative, and the safe direction to fail in." Both halves are
wrong, and they are wrong in the same direction.

**The gate records no crate→feature edge.** Line 19 assembles its search set from
every crate the family declares, and line 26 greps that whole set for the
feature id. Feature 176 is satisfied if *anything* under *any* family crate's
`tests/` names it. Delete `flush_test.rs`'s first line and add the string to a
neighbouring crate's suite and G3 still passes; the edge it records is
family→feature, and the crate is not part of the tuple.

**And the grep carries no file-type filter**, so a sentence in a hand-written
plan is a claiming test as far as G3 is concerned. Thirteen family crates'
`tests/manual/readme.md` already carry such citations. None of those thirteen
is wrong to — a manual plan citing the feature it plans is exactly right — but
it means the gate cannot distinguish an executable claim from a written
intention, which is the distinction it exists to make.

So the reachable failure is a **false positive**: a feature reported claimed
because some document somewhere in the family mentions its number. The instance
names the opposite failure and calls the direction safe. This crate happens to
be clean — `ring_flush` is the only crate citing 176, and it cites it from its own
executable suite — but that is a fact about this crate, not a property the gate
holds.

```sh
# -mN and no -n: this file is its own subject, so an unbounded
# match also finds this command line and every copy of its own
# output below, and -n re-prefixes a fresh line number onto each
# earlier pass's output -- the stacked addresses this block carried
cd "$(git rev-parse --show-toplevel)"
F=ring_flush/docs/non_functional_requirement/001_three_triggers_proven_by_a_flush_log.md
command grep -m2 'false positive' "$F"
```

Live output:

```
**false positive**: the feature reported claimed because some document
So the reachable failure is a **false positive**: a feature reported claimed
```

**Disposition:** applied — the Acceptance Threshold section's citation
paragraph now states the gate's real mechanism (family→feature, no
file-type filter) and names the reachable failure as a false positive rather
than the false negative it previously claimed, matching what FL33 itself
demonstrates. Fixing `g3_features.sh` to record a true crate→feature edge
with a file-type filter would be the thorough fix; it changes gate behavior
for all 33 crates, not one crate's documentation, so it is named here and
left to a gate-authoring decision rather than applied in this pass. Now
prints: `false positive`

### FL34 — M4's Threshold Cannot Be Missed, Because the Implementation Derives One Side From the Other

### FL34 — M4's Threshold Cannot Be Missed, Because the Implementation Derives One Side From the Other

The measurement that guards the evidence, and the four lines that make it unfalsifiable:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- what M4 measures, and what it must reach --'
awk '/^### FL/{ exit } /^\| M4 \||^\| B4 \|/{ printf "    %s\n", $0 }' \
  ring_flush/docs/non_functional_requirement/001_three_triggers_proven_by_a_flush_log.md \
  | sed -E 's/^(.{0,132}).*/\1/'
echo '  -- how the entry is produced --'
awk -v n1="$(( $( command grep -n -m1 -F '    self.record( cause, FlushOutcome::Flushed { count } )' ring_flush/src/lib.rs | cut -d: -f1 ) + 2 ))" -v n2="$(( $( command grep -n -m1 -F '      log.entries.push( FlushEntry { policy : self.policy, cause, outcome } );' ring_flush/src/lib.rs | cut -d: -f1 ) + 2 ))" 'NR >= n1 && NR <= n2 { printf "    %d: %s\n", NR, $0 }' ring_flush/src/lib.rs
echo '  -- and every construction of a FlushEntry in the crate --'
command grep -r 'FlushEntry {' ring_flush/src ring_flush/tests | sed 's|ring_flush/||'
```

Live output:

```
  -- what M4 measures, and what it must reach --
    | B4 | The log is a faithful record of what actually happened | **All three** — if B4 fails, B1–B3 are unverified regardless of 
    | M4 | Log completeness | Assert `outcome` and log entry agree for every drive call in the scenario |
    | M4 | Agreement on every drive call — no tolerance |
  -- how the entry is produced --
    649: 
    650:   /// Derive the log entry from the outcome, so the two cannot disagree.
    651:   fn record( &mut self, cause : FlushCause, outcome : FlushOutcome ) -> FlushOutcome
    652:   {
    653:     if let Some( log ) = self.log.as_mut()
    654:     {
    655:       log.entries.push( FlushEntry { policy : self.policy, cause, outcome } );
    656:     }
    657: 
  -- and every construction of a FlushEntry in the crate --
src/lib.rs:      log.entries.push( FlushEntry { policy : self.policy, cause, outcome } );
tests/flush_test.rs:    let entry = ring_flush::FlushEntry { policy : FlushPolicy::OnFull, cause : FlushCause::Full, outcome };
```

M4 is listed as a measurement, given a mechanism, and given a threshold with "no
tolerance." What it asserts is that the log entry and the returned outcome agree.
The implementation constructs the entry *from* the outcome, in the crate's only
`FlushEntry` literal, under a comment saying so in as many words: "Derive the log
entry from the outcome, so the two cannot disagree."

That is the right implementation. Making a class of disagreement impossible is
better than measuring for it. The defect is the bookkeeping: an invariant held by
construction is entered in the Measurement Method table beside four measurements
that can genuinely fail, and given a threshold in the Acceptance Threshold table
as though passing it were evidence of something. `the_log_agrees_with_every_outcome_it_recorded`
passes today, would pass against a completely wrong `cause`, and would pass
against a log recording flushes that never happened.

**B4 is the claim that actually needed guarding and M4 does not reach it.** B4
says the log is a faithful record of *what happened*; M4 checks the entry against
the *outcome*, which is the other output of the same call. Both come from `run`,
so a common-mode error — a flush recorded as `Flushed { count }` when the push
placed fewer — produces an entry and an outcome that agree perfectly and are both
wrong. B1–B3 are read off that log, and this instance says so itself: if B4
fails, "B1–B3 are unverified regardless of what they assert."

The gap is real but narrow, and worth stating precisely rather than alarmingly:
`count` comes from `try_push_batch`'s own return value, so the failure needs the
producer to misreport. What is missing is not a test — it is the acknowledgement
that this criterion's foundation rests on a crate it does not own, in a table
whose purpose is to say what has been measured here.
