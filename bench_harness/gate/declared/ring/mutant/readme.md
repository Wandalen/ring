# mutant

Historical defects, each recorded with the exact edit that reinstates it.
`g12_mutation.sh` applies one at a time, runs its crate's suite, and requires
the suite to go red. Then it puts the file back and checks that it came back
byte-identically.

| File | Responsibility |
|------|-----------------|
| `b2_eligibility_filter.mutant` | Removes the filter that keeps a lossy path from being reported fastest |
| `b5_counter_mapping.mutant` | Reinstates the `reported`-for-`received` counter mapping the crate calls its most expensive lesson |
| `f1_barrier_guard.mutant` | Drops the `at_barrier` guard, so `OnBarrier` fires on every drive |
| `s1_closing_batch_count.mutant` | Flips the sign on the closing partial batch, putting `reported` below `received` |
| `s2_flush_log_empty_polarity.mutant` | Makes `FlushLog::is_empty` answer `true` always, which every assertion on it expected |
| `s3_handle_consumer_empty_polarity.mutant` | The same, for `ring_handle`'s `Consumer::is_empty` |
| `s4_mpsc_consumer_empty_polarity.mutant` | The same, for `ring_mpsc`'s `Consumer::is_empty` |
| `s5_registry_empty_polarity.mutant` | The same, for `Registry::is_empty` |
| `s6_trace_empty_polarity.mutant` | The same, for `Trace::is_empty` |
| `s7_batch_budget_bound.mutant` | Spends one attempt past the budget, which costs one more record |
| `s8_batch_unproductive_break.mutant` | Inverts the early exit, stopping on the productive attempt instead |
| `s9_batch_attempt_counter.mutant` | Freezes the attempt counter at zero, leaving only the break to end the loop |
| `s10_free_capacity_binding.mutant` | Answers zero room forever, which the advisory contract permits and the binding one does not |
| `s11_committed_watermark.mutant` | Returns the default sequence instead of reading the consumer cursor |
| `s12_vanished_in_ring_term.mutant` | Subtracts the in-ring term that `vanished` should add |
| `s13_tick_lost_subtraction.mutant` | Divides instead of subtracts a `Tick`'s loss count, panicking once a batch moves nothing |
| `s14_bytesslot_eq_always_true.mutant` | Answers `true` from `BytesSlot::eq` regardless of payload |

**Two provenances, and the id prefix carries which.** `B` and `F` entries come
from manual test rounds, recorded in `ring_bench/tests/manual/readme.md` and
`ring_flush/tests/manual/readme.md`. Each was paid for by a wrong answer
somebody got first. `S` entries came from `gate/mutant_survey.sh`, which
proposed them mechanically before they had cost anyone anything. Those records
are the source;
these files are the executable form of them, and the prose in each explains what
its round found rather than repeating the mechanism.

Keep the distinction visible. A gate that replays only defects people tripped
over cannot defend a crate nobody has been unlucky with yet. The survey entries
did not require the bad luck, and they outnumber the manual ones several times
over, from crates nobody had been unlucky with at all.

## The one-polarity rule

S2 through S6 and S11 are one defect in six places, and it is the same defect
B5 records in a different shape.

**A predicate or quantity asserted only where it equals its degenerate value is
indistinguishable from a constant returning that value.** Five crates asserted
`is_empty()` and never `!is_empty()`; `ring_mpsc` asserted `committed()` three
times and expected `Seq::ZERO` all three. Each suite looked thorough, with ten
assertion sites in `ring_flush` alone, and each was measuring the same point
over and over.

B5 is this rule at a different arity: it relates `reported` to `received` and
was asserted only on a fixture where the two were equal. `is_empty` relates a
length to zero and was asserted only where they coincided. Both pass for a
reason unconnected to the code under test, and neither is visible from a green
board. That is the whole reason this directory exists.

The fixes are all one line, and all of them land at a point where the test
already had the opposite state in hand and walked past it. None of these six
needed a new fixture. `ring_trace`'s
`clearing_empties_the_log_without_switching_it_off` recorded an entry, cleared
it, and asserted empty. It never asserted the entry had been there, so it could
not tell a working `clear` from a log that was never written to.

A seventh crate, `ring_store`, carried what looks like the same defect and does
not: its `is_empty` is unconditionally `false` for every value the type can
hold. That one is in `../accepted/`, and telling the two apart is the judgement
this directory and that one exist to record.

## Format

A `key: value` header, then the two blocks the probe swaps between markers.
Everything before the first `---` line that is not a key is a comment.

```
id: B2
crate: ring_bench
file: src/lib.rs
round: 2026-08-28
round-failures: 5
detector: a_path_that_dropped_records_is_not_eligible_to_be_fastest

--- from
<the exact lines as they stand in the crate today>
--- to
<the exact lines the defect had>
--- end
```

`round` is the date the defect was reinstated by hand, and it is required. An
undated probe is a claim rather than a record, the same bar G4 holds manual runs
to. `round-failures` is how many tests that mutation actually fails, measured
under `--no-fail-fast`.

Neither is asserted against. The gate prints the recorded count beside the
observed one so erosion stays visible, but a floor would fail on a legitimate
test consolidation, and the claim worth defending is binary: reinstate the
defect, the suite notices.

The `--no-fail-fast` in that definition matters, and F1 is why. nextest
stops at the first failures by default, so the count it reports under a mutation
depends on scheduling rather than on the suite. F1's round recorded four and its
own output says `16/23 tests run`, a run cut short. The true figure is six.
Building this gate is what forced the question, because a number worth printing
every run is a number that has to mean the same thing every run.

`detector` names the test written to catch this deliberately. The gate reports
it when a mutant leaves the suite green, so the failure message names what
stopped working rather than only which probe slipped through. In two of the three manual
rounds the deliberate detector was not the only thing that caught the defect,
and in B5 it did not catch it at all until it was fixed. Every survey round is
the opposite by construction. The survey only proposes a mutation *because*
nothing caught it, so its detector is always a test written afterwards, and
always the only thing standing between the defect and a green board.

## Adding one

The `from` block must match its target **exactly once**. Zero matches means the
declaration went stale when the code moved. A mutation that edits nothing
leaves the suite green for the most misleading possible reason, so the gate
treats it as a failure rather than a skip. More than one match means the probe
would hit a site no round recorded.

```bash
# the gate parses and checks every declaration before running a single test,
# so a stale `from` block fails in seconds rather than after four suite runs
GATE_FAMILY=ring bash bench_harness/gate/g12_mutation.sh
```

A new mutant needs a round behind it. Reinstate the defect by hand first, run
the crate's suite, and record what actually failed. The point of the count is
that someone watched it, and every one of the manual three failed differently
from its own prediction.

For a survey round the order is inverted, and the discipline has to be stated
because the obvious sequence is wrong. The survey has already proved the
mutation survives; that half is measured, not assumed. What remains is to add
the assertion **while the mutation is still in place**, confirm the suite goes
red naming the intended test, restore, and confirm green. A fix written
against an unmutated tree proves only that it compiles. A survey round also
reinstates the survivors it rules *unkillable* and confirms they stay green. An acceptance is a
claim about what no test can do, and it needs the same evidence as a fix.

Then the whole family is swept a second time, which is what closes the round.
The by-hand checks above confirm each ruling one at a time, in the shape the
person doing them expected. The re-sweep confirms all of them at once,
mechanically, in the shape the tool finds them, and it does not depend on anyone
remembering which crates were touched. That is worth the tens of minutes a warm
sweep takes.
