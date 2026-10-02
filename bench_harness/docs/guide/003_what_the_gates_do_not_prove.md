# Guide: what the gates do not prove

### Scope

- **Purpose**: Record that a crate at 6/6 gates and 100% line coverage had zero defence against the single defect its own documentation calls its most expensive, and state exactly which gap each gate leaves open.
- **Responsibility**: Stop a green gate board from being read as a completeness claim it was never able to make.
- **In Scope**: The B5 measurement; the two independent reasons it slipped; the per-gate limits; the named next constraint.
- **Out of Scope**: How to run the gates (→ [`001`](001_running_the_verdicts_yourself.md)); what they did establish (→ [`002`](002_the_four_verdicts.md)).

**This is the most transferable thing this effort produced, and it is a
negative result.** Read it before quoting the green board.

### The measurement

Manual stage B5 took `ring_bench` at its verified state: 6/6 gates, 18
behavioural tests, 3 doctests, **100% line coverage over 298 lines**. It then
reinstated the crate's original headline defect verbatim:

```rust
  stats.record_claim( reported as u64 );   // was `received`
  stats.record_publish( reported as u64 );
  stats.record_consume( reported as u64 );
  stats.record_drop( workload.config().overflow(), ( offered - reported ) as u64 );
```

This is [verdict 1](002_the_four_verdicts.md#verdict-1--ok-is-not-evidence-a-record-was-kept)
put back. It counts the write API's own successes instead of what the drain
produced. On the cramped fixture it maps 240 discarded records into the slot
counters, which reads as **240 leaked slots on a 16-slot ring**. That is an
impossible state, stated by the crate's own statistics.

**Every test passed.** 18/18 and 3/3, green.

### Two independent reasons it slipped

Either one alone would have been enough. That both were present is what makes
the result worth recording.

| Test | Why it missed |
|---|---|
| `the_counters_are_the_runs_own_totals`, **the test named after the job** | Runs `MutexQueue`, where `reported == received == 16` on the cramped fixture. The two quantities it exists to distinguish are *equal* under its own fixture, so the mapping is invisible to it |
| `a_dropnewest_ring_reports_successes_it_did_not_keep` | Uses `ContractRing`, the one candidate where they differ, but asserts on `Outcome`'s fields, never on `outcome.stats()`. The mutation touches only the counter mapping |

**One test had the right assertions on the wrong input. The other had the right
input and never looked at the object.**

**The fixture choice is the deeper trap.** Picking `MutexQueue` for the counter
test was reasonable. It is the simplest candidate, and a test of "the counters
are the run's own totals" has no obvious reason to prefer a complicated one. It
is nevertheless precisely wrong, because **a test that distinguishes two
quantities must run on an input where they differ.** That property appears
nowhere in the test's name, its assertions, or its coverage figure.

**Coverage measures which lines ran, not which values were checked.** Both
mutated lines executed, under both tests, on every run. Neither test looked at
what they produced. The 100% was true and meant nothing here.

**Closed:** `a_dropnewest_ring_reports_successes_it_did_not_keep` now pins all
four counters and `in_flight()` on `ContractRing`. Re-probed under the same
mutation: red, one failure, the intended one. **The fix was found by hand, not
by any gate**, and no gate would have found it.

### What each gate does and does not prove

| Gate | Proves | Does not prove |
|---|---|---|
| **G1** coverage | Every line under `src/` executed at least once | That any value it produced was checked. **This is B5** |
| **G2** docs | Every crate exports something and documents it | That the prose is true. G2 matches on a skeleton phrase; it cannot read |
| **G3** features | Each of the 22 features is cited by name in some test | That the test tests the feature. A citation is a string in a comment |
| **G4** manual | Every crate has a dated run record | That the run happened, or that its predictions were checked. **Three of `ring_bench`'s five predictions were wrong. G4 grades the file, not the outcome** |
| **G5** exports | No dependency escapes the five Contract crates | That the Contract is sufficient. [Verdict 2](002_the_four_verdicts.md#verdict-2--the-contract-door-caps-what-the-structures-do-not) is a case where it is not |
| **G6** unsafe | `unsafe` appears only in the three allowlisted crates | That it is correct there. No gate reads it |
| **G12** mutation | Fifteen recorded defects, reinstated, each still turn the suite red | That the suite catches a defect nobody has recorded. It replays a list, and the list is only as good as the rounds that wrote it. Eleven of the fifteen, though, were written by a machine rather than by bad luck; see § The sweep |
| **G13** freshness | Every crate was swept clean against the exact source and tests it carries now | That the sweep found everything. It grades the *date* on a ruling, never the ruling. A crate can be perfectly fresh and still blind to whatever no mutation operator generates |

**The pattern across the row is the same.** Each gate checks a *proxy* that is
cheap and mechanical, and each proxy is separated from the property it stands in
for by exactly one step of judgement. That is not a defect in the gates. A gate
that required judgement would not be a gate. It is a statement about what a
green board is worth.

**The honest summary of the 6/6 this was written against:** the family is
built, every line runs, every feature is claimed, and someone ran it by hand and
wrote down what happened. None of that is a correctness claim, and B5 is the
proof that it isn't. The board reads 8/8 now. The seventh gate is the one that
replays B5, and the eighth is the one that notices when the seventh's list has
gone out of date. So the summary is two gates less true than it was, and the
sentence above still stands for the other six.

Note what the eighth gate is and is not. It does not check any more of the code
than the seventh did. It checks that the seventh was checking *this* code.
Every row in the table above is a proxy one step of judgement away from the
property it stands for, and G13 adds no new proxy; it puts an expiry date on two
that already existed. That is a smaller claim than the others and worth stating
as such, because a board of eight reads like more assurance than a board of
seven, and here it is the same assurance with one fewer way to quietly stop
being true.

### The named next constraint, now built as G12

**A mutation gate.** Reintroduce a known defect; require the suite to go red.
Built 2026-08-29 as `g12_mutation.sh` and declared for the ring family, which
now grades 7/7. It is **G12** rather than the G7 the earlier implementation plan predicted,
because the orbital family's determinism gate had taken G7 in the
meantime.

| Source | Defect | Under G12 |
|---|---|---|
| `ring_bench` B5 | `reported` mapped into the slot counters in place of `received` | red, 1 failure |
| `ring_bench` B2 | The eligibility filter dropped, so a path that discarded records can be called fastest | red, 5 failures |
| `ring_flush` F1 | The `at_barrier` guard dropped, so `OnBarrier` fires on every drive | red, 6 failures |

**Two corrections this table carries.** Its B2 row previously described "the
`Flushed` arm of `drain_final`". That finding is real, but it is not B2 and it
is not a mutation at all. It is the coverage gap recorded in `ring_bench`'s own
surprise table, and § The second-order lesson below has been rewritten to stop
attributing it to a stage it did not come from. B2 is *"does the suite go red
when the eligibility filter is removed"*, per
`ring_bench/tests/manual/readme.md` § B2.
And F1's failure count was recorded as four. Building the gate forced the
question of what that number meant. The answer was that nextest stops at the
first failures by default, so four was how many had run rather than how many
catch it. Under `--no-fail-fast` it is six.

**The earlier implementation plan's own reservation about this gate was real and is worth keeping.** A
gate added after the fact against defects already fixed does grade the fix
rather than the suite. For B5 that is literally true, since the gate passes
only because a test was repaired in response to the round that found it. What
it buys is retention, not discovery. A human habit and a written record held
those three defences, and now a verdict holds them. The remaining honesty lives
in G12's own non-vacuity rules. A declaration that stops matching its target
fails rather than skipping, and a mutation the suite ignores fails rather than
passing quietly. Both were demonstrated firing before the gate was declared.

### The second-order lesson

**This one is not from a mutation round at all**, which is why it generalises
furthest. During implementation, both `ring_bench` fixtures used 256 records
with batch 32, which divides exactly, so the staged candidate's closing
`drain_final` had nothing left to do and its `Flushed` arm was never reached.
**Coverage caught that one; no assertion did.** A 250/32 fixture was added
(`ring_bench/tests/bench_test.rs`) and the line went green.

So coverage found a hole that assertions missed, and assertions were the only
thing that could find the hole coverage missed (B5), **in the same crate.**
Neither instrument dominates the other, and a project that runs only one of them
will have blind spots shaped exactly like the one it skipped.

**Note which instrument is which, because it inverts between the two.** The
`drain_final` gap was a line that never executed, so the 100% figure was the
thing that moved when it was fixed. B5 was a line that executed on every single
run and was never checked, so the 100% figure did not move at all. It was
already 100% before, during, and after. That is the whole reason G12 exists.
It is the only gate on the board whose verdict can change while every other
number stays exactly where it was.

### The lesson repeated on purpose, in this crate

Everything above was paid for by defects that escaped first. On 2026-08-29
`gate/mutant_survey.sh` produced the same finding deliberately instead, and the
target was `bench_harness` itself. It was chosen because task 129 had *just*
certified it at 100% line coverage over 63 lines, so a clean result would have
proved nothing and a dirty one would prove a great deal.

It came back dirty. Of 48 mutations, **three survived**: the `^` in the
workload's mixing function became `|` and then `&`, and the `+` in the `Mixed`
archetype's width schedule became `*`. All twelve tests stayed green through
every one. The tests asserted that the sequence is *reproducible* and
*seed-dependent*. Every bitwise operator satisfies both, so nothing pinned
the operation itself. The workload generator behind every benchmark comparison
this crate exists to produce could have been changed silently, and every past
run would have become incomparable to every future one without a single test
failing.

**That is B5 again, in the crate built to grade B5's family, found by a tool
rather than by being burned.** Coverage was 100% before the survey and 100%
after; the only number that moved was the survivor count, from three to zero
(T13 pins the mixing as XOR by unmixing back to the seed, T14 pins the width
schedule against `Uniform`). The inversion the section above describes is not a
historical curiosity about `ring_bench`. It reproduces on demand, in new code,
written by someone who had just read the warning.

In practice, **G1's 100% means every line ran, and nothing more.**
Treating it as "every line is defended" is the single most expensive mistake
this effort has recorded, and it has now been made twice.

→ [`gate/readme.md`](../../gate/readme.md) § The one thing here that is not a gate

### The third repetition, and what the survey learned about itself

The survey was then pointed at `ring_bench`, the crate where B2 and B5 were
found. It was chosen because a crate with a proven history of coverage-blind
defects is where a discovery tool should have to prove itself. 180 mutations, 22 survivors.

**Eighteen of the twenty-two were the tool lying.** They were all one function
behind `#[ cfg( feature = "crossbeam" ) ]`, which is off by default.
`cargo mutants` mutates source text and runs the suite with default features, so
a mutation inside a `cfg`-disabled body compiles as dead code and passes. It
passes always, for every mutation, however good the tests are. The suite does cover
that function once the feature is on. All eighteen were noise, and they buried
the two findings that were not.

**That is this crate's own baseline pathology, reproduced inside the tool built
to escape it.** The `gate/readme.md` opening states it. A gate that cannot tell
*not started* from *finished* measures nothing. A survey that cannot tell *not
compiled* from *not defended* is the same failure wearing different clothes,
and it was written by someone who had that sentence in front of him. Fixed by
surveying with `--all-features`; the re-run returned **3** survivors from the
same 180. The number that changed was not the crate's quality.

**The real finding was the exact-division trap, for the third time.** A mutation
flipped the sign on `MutexQueue`'s closing partial-batch commit, putting
`reported` 26 below `received`. That is a run claiming it wrote fewer records
than the drain gave back. All 18 tests passed. The assertion that catches it exactly,
`received <= reported`, runs across every candidate on the 256/32 fixture, where
the batch divides and the remainder is zero. The 250-record fixture has the
remainder and asserted only `is_lossless()` and `received()`, reading `reported`
on one hand-picked candidate.

| Occurrence | How the rule was broken |
|---|---|
| B5 | By fixture. The candidate chosen had `reported` and `received` equal |
| `drain_final` | By dimension. The batch always divided, so the tail was never reached |
| **S1** | By both at once. The assertion relating the two lived on the fixture that made them equal, and the fixture that could separate them never made the assertion |

**Each one breaks the same rule. A test that relates two quantities must run
where they can differ.** S1 is the sharpest instance because the 250-record
fixture *was itself added to close a blind spot*, and arrived carrying a
narrower one. Writing a fixture to defeat an arithmetic accident does not
protect the assertions that never run on it.

Closed by moving `conserved()` into that fixture's own candidate loop, and
recorded as G12's `S1`. **It is the first entry on that list that nobody had to
be unlucky to find.** The three remaining survivors are all timing, all
undefendable without a clock, and are
[recorded as accepted](../../../ring_bench/tests/bench_test.rs) rather than left
looking like work outstanding.

### The sweep: what thirty-three crates said at once

The survey had then been run on two crates, both of them `ring_bench`-shaped
benchmarks with a known history. The earlier implementation plan named that as the closing
constraint: *thirty-one of thirty-three crates have never been surveyed, and
the two that were are the least representative sample available.* Sweeping the
family on 2026-08-30 cost **39 minutes** and returned **29 survivors across 13
crates**; the other 20 were clean, and `ring_bench` was correctly absent, its
three rulings subtracted rather than re-reported.

Every survivor was ruled and every ruling verified: **11 fixed, 18 accepted.**
G12 now replays 15 defects instead of 4.

**The one-polarity rule, six more times.** Five crates asserted `is_empty()` and
never `!is_empty()`; `ring_mpsc` asserted `committed()` three times and expected
`Seq::ZERO` all three. A predicate asserted at one polarity is indistinguishable
from the constant it returns there. That is B5's rule at a different arity, so
the tally across this document is now ten occurrences of one mistake. None of
the six needed a new fixture. Every fix landed at a point where the test already
had the opposite state in hand and walked past it. `ring_trace`'s
`clearing_empties_the_log_without_switching_it_off` recorded an entry, cleared
it, and asserted empty. It never asserted the entry had been there, so it could
not distinguish a working `clear` from a log that was never written to.

**A test cannot catch a mutation it does not straddle.** This one is new, and
`ring_core::free_capacity` is the case. The suite asserted
`assert_eq!( claimed == 0, producer.is_full() )`, which reads like a strong
cross-check and is not one. `is_full` is *defined* as `free_capacity() == 0`, so
mutating `free_capacity` moves both sides of the comparison together. The
assertion sat entirely on one side of the defect. Its companion assertion, that
`free_capacity` never overstates, is satisfied by a constant zero. Understating
is permitted, and zero is the largest understatement available.

The crate's own module table calls this API *"one signature over two contracts:
binding at SPSC, advisory elsewhere."* The suite tested the advisory contract
against all three backends and the binding contract against none, at 100% line
coverage. **Coverage of a crate is not coverage of its contracts**, and where a
document names two, a suite testing one is half-done at full marks.

**Retrying is not always free.** `ring_poll`'s retry loop looked untestable from
a single thread. Nothing changes between attempts, so a repeat can only find
the same answer, which is the crate's own stated design. That reasoning holds
for `push_within` and `recv_within` and is false for `push_batch_within`, because
`try_push_batch` pulls a record before it can know there is room and drops the
one it cannot place. Each failed attempt destroys a record, so a budget of N
against a full ring costs N records, not one. The loss is visible in the
caller's iterator and nowhere else. `try_push` hands the record back; `try_push_batch` eats it.
Three survivors became defects on that one distinction, and eight became
acceptances.

**Both directions were proved, not asserted.** Every fix was verified by
reinstating its mutation, confirming the suite went red naming the intended
test, restoring, and confirming green. Every acceptance claiming *no test can
kill this* was verified the same way in reverse: reinstated, and confirmed to
leave the suite green. An acceptance is a claim about what is impossible and
deserves the evidence a fix gets. Otherwise `accepted/` becomes the place
things go to be excused.

**What the sweep says about the sweep.** Sixteen of the eighteen acceptances are
equivalent mutants, where no input separates the mutation from the original. The
survey cannot tell those from defects and never will; it reports the mutation
and stops. So a 29-survivor result was never a 29-defect result, and the number
that matters is the split, not the survivor count. Only a human reading each one
produces that split. The tool narrowed **1,160 mutations to 29**, a 97.5%
catch rate, in 39 minutes. It decided none of them.

### Guides

| File | Relationship |
|------|--------------|
| [001_running_the_verdicts_yourself.md](001_running_the_verdicts_yourself.md) | The commands that produce the 7/7 this instance qualifies |
| [002_the_four_verdicts.md](002_the_four_verdicts.md) | Verdict 1 is the defect B5 reinstated |

### Pitfalls

| File | Relationship |
|------|--------------|
| [`ring_bench/docs/pitfall/003`](../../../ring_bench/docs/pitfall/003_ok_is_not_kept_and_the_verdict_inverts.md) | § The Mitigation Was Undefended for Longer Than the Coverage Suggested, the primary record of B5 |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_gate_non_vacuity.md](../invariant/001_gate_non_vacuity.md) | The one thing the gates *do* guarantee about themselves: they failed first |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_bench/tests/manual/readme.md`](../../../ring_bench/tests/manual/readme.md) | Stages B1–B5 as run, with the three wrong predictions |
| [`ring_bench/tests/bench_test.rs`](../../../ring_bench/tests/bench_test.rs) | The two tests in the table, and the assertions that closed the hole |
