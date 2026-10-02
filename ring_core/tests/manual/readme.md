# ring_core manual testing plan

`tests/core_test.rs` carries the reached-test for
`docs/feature/187_optional_crossbeam_queue_backend.md` and 21 tests around it,
plus 7 doc tests. Line coverage is 100% over 120 coverable lines, **measured with
`--all-features`**. See C4, which is about the reading itself.

This crate is unlike its 32 siblings in one way that shapes every check below.
**It is the only one with a cargo feature**, so it is the only one that is two
programs rather than one. A gate that runs a single configuration is
half-blind on it, and two of the eight stages here exist because that was true
of the family's gates until this crate arrived.

The second theme is the crate's whole claim, *the backend is a build flag
rather than a rewrite*. A test that loops over backends asserts it, so the
loop's own coverage of backends matters in a way an ordinary assertion's does
not. C1 and C2 are about whether that loop, and the documentation beside it,
can fail at all.

Run from the workspace root. The code-line filter throughout is
`grep -nE <pattern> <file> | grep -vE '^[0-9]+:[[:space:]]*//'`. The number
comes from the *unfiltered* stream, because filtering first and numbering after
renumbers the surviving lines and understates every position. `ring_spsc`'s own
plan carried that defect, off by roughly 500 lines.

## C1. The suite detects a broken composition, and the doc tests do not

**This is a mutation check: it edits the source, runs the suite, and restores.**
Take a copy first. The restore is the whole procedure, not a formality.

The mutation is not invented. It is the bug this crate shipped with for one
build. `Producer::try_push`'s MPSC arm called
`ring_mpsc::Producer::push`, which **takes the record by value and returns
`Result< Seq, RingError >`**. So on a full ring the record is consumed inside
it and there is nothing left to hand back, while this crate's signature promises
`Result< (), T >`. The only way to make that compile is to panic on refusal.
The fix claims a slot *before* consuming the record.

```bash
cp ring_core/src/lib.rs /tmp/-core_orig.rs

# Mutation — in `Producer::try_push`, replace the whole `ProducerInner::Mpsc`
# arm body (the `match producer.claim() { ... }` block) with:
#
#   Ok( producer.push( record ).map( | _ | () ).expect( "mpsc refusal carries no record" ) )

cargo nextest run -p ring_core --offline --features crossbeam 2>&1 \
  | grep -E 'FAIL|Summary'
cargo test --doc -p ring_core --offline --features crossbeam 2>&1 \
  | grep -E 'test result'

cp /tmp/-core_orig.rs ring_core/src/lib.rs
cargo nextest run -p ring_core --offline --features crossbeam
```

**Expected**, measured rather than predicted:

| Suite | Result under the mutation |
|---|---|
| `core_test` (21 tests) | **4 FAIL**: `a_partial_batch_push_reports_its_count_and_consumes_the_refused_record`, `drop_newest_discards_the_incoming_record_without_an_error`, `a_refused_record_comes_back_on_every_backend`, `four_threads_publishing_through_clones_lose_nothing` |
| doc tests (7 tests) | **0 fail — all 7 pass** |

Then 21/21 after the restore.

**Result (2026-09-11): the suite has since grown by two tests. The counts above
are dated to 2026-08-28.** `the_same_program_behaves_identically_on_every_backend`
and `a_capacity_of_one_cycles_correctly_on_every_backend` (new, a capacity-1
boundary check) did not exist then. Re-running the identical mutation today
against the current 23-test suite, twice, measured two different shapes:

| Run | `core_test` summary line | FAIL rows shown |
|---|---|---|
| 1st | `22 tests run: 17 passed, 5 failed, 0 skipped` | the four original, plus `the_same_program_behaves_identically_on_every_backend` (run before the capacity-1 test was added) |
| 2nd | `19/23 tests run: 14 passed, 5 failed, 0 skipped`, with 4 tests never scheduled | the four original, plus `a_capacity_of_one_cycles_correctly_on_every_backend` |

Both runs restored clean afterward (22/22, then 23/23). Doc tests stayed
`0 fail — all 7 pass` in both.

**The two runs disagree about whether nextest completes the suite under this
mutation. That disagreement is itself the finding worth recording, not a
measurement error to average away.** The 1st run scheduled and ran every test
to completion. The 2nd stopped 4 short of the full 23 while still reporting
`0 skipped`, so those 4 were never dispatched at all rather than skipped
by a filter. **5 FAIL is stable across both runs**, but which tests appear in
that 5 is not. `the_same_program_behaves_identically_on_every_backend`
appeared in the 1st and not the 2nd (not reached). The new capacity-1 test,
once it existed, appeared in the 2nd. Read neither run's FAIL list as the
exhaustive set of tests this mutation breaks. Each is only a same-day lower
bound of 5. It confirms the original finding survives at the current suite size,
without pinning down which 5 of 23 a future run will name.

**The zero is the finding.** Every doc example on this crate could have shipped
against a `try_push` that panics on any full multi-producer ring. C2 explains
why, and it is not an oversight that more examples would fix.

A secondary reading from the same runs:

- The original recording says `nextest`'s summary was `16/21 tests run: 12
  passed, 4 failed` and treats **4 as a lower bound** because nextest "stops
  scheduling after the first failure." The 2026-09-11 runs show that framing
  is incomplete rather than wrong. One run completed all 23 tests (`0`
  unscheduled) and the other stopped 4 short. Same recipe, same day,
  two different scheduling outcomes. Whatever governs it is not visible from
  this file alone. **Read the day's own summary line rather than assuming
  either shape**, and treat any single FAIL-count as a lower bound unless that
  line's own arithmetic (`run` total equals `passed + failed + skipped`) says
  the suite actually completed.
- `four_threads_publishing_through_clones_lose_nothing` fails at **30.015s**
  in the original recording, **30.024s** and **30.013s** in the two
  2026-09-11 runs. That is consistently "not instantly and not never," never
  the same figure twice, which is what a 30s deadline guard should produce.

## C2. What the doc examples reach

Read the seven examples and classify them. The command extracts each block with
its line number, so you can check the classification against the source rather
than take it on trust:

```bash
awk '/^[[:space:]]*(\/\/\/|\/\/!) ```/{ if(!inb){inb=1; start=NR; body=""} else {inb=0; print "block @" start ":"; print body} ; next } inb{ sub(/^[[:space:]]*(\/\/\/|\/\/!) ?/,""); body = body "    " $0 "\n" }' ring_core/src/lib.rs
```

**Expected:** seven blocks. Originally at lines 52, 79, 142, 339, 406, 443, 548,
and now at **63, 90, 155, 359, 426, 464, 574**. The
`Fix(decision_121_link_pointed_at_docsrs_not_the_ruling)` module-doc-comment
correction (2026-09-11) shifted them by adding lines before the first block. Re-run the
command above rather than trusting either list if `src/lib.rs`'s module doc
comment has changed again since.

| Reached | Verdict | Evidence |
|---|---|---|
| SPSC push/pop | **yes** | every behavioural block |
| Saturation and refusal | **yes** | @339 fills 2 slots and asserts `Err( 3 )`; @406 the batch form |
| Construction-time policy refusal | **yes** | @142 |
| Backend selection | **yes** | @79 |
| **MPSC push path** | **no** | @79 builds an MPSC ring but only reads `.backend()` and never pushes |
| **Crossbeam, at all** | **no** | zero blocks name it |
| Wrap | no | no block drains and refills |
| Contention | no | no block spawns a thread |

The MPSC row explains C1's zero. **`Producer::try_push`'s own doc example
exercises exactly one of its three arms.** That is not a gap to close by adding
examples. A doc test is prose that has to stay readable, and the example on
`try_push` is there to show the caller what a refusal looks like, which one
backend demonstrates as well as three. Backend coverage is
`the_same_program_behaves_identically_on_every_backend`'s job, and the division
is deliberate. But it means **the doc tests must never be cited as evidence
that this crate's backends work.** They are evidence that its API reads
well.

## C3. Both feature configurations are built by something

No test can check this. It is a property of which commands the gates run, and
until this crate arrived the answer was *one configuration*.

```bash
grep -nE '^[^#]*cargo [a-z]+ .*(--all-features|--no-default-features)' \
  bench_harness/gate/*.sh
```

**Expected:** three hits. `g1_coverage.sh:25` with `--all-features`,
`g2_docs.sh:48` with `--all-features`, and `g2_docs.sh:58` with
`--no-default-features`.

Anchored to `cargo <subcommand>` and to a non-`#` line start, not to the flag
name. The obvious form, grepping for `all-features` alone, returns **five**
here, because the amendment those three lines belong to explains itself in two
comments that name the flags. A check that counts its own prose drifts upward,
and upward reads as extra safety.

**Before this crate, the answer was one hit**, and both halves were wrong:

- **G1** (coverage) ran tarpaulin with no feature flags at all. See C4 for what
  that produced.
- **G2** (docs / `cargo check`) ran `--all-features` only. `--all-features`
  compiles the `#[ cfg( feature = ... ) ]` arms and skips every
  `#[ cfg( not( ... ) ) ]` one, so the default build was compiled by no gate. A
  default build could stop compiling entirely and all six gates would still
  report REACHED.

Confirm the negative directly. This must fail if the default build is broken,
and it is the check G2 now runs:

```bash
RUSTFLAGS="-D warnings" cargo check -p ring_core --all-targets --no-default-features --offline
```

**Expected:** clean. The default build's unique code is currently test-only
(`every_backend` has a `#[ cfg( not( feature = "crossbeam" ) ) ]` twin), which
is why `--all-targets` is not optional here. Without it the check
compiles the library, finds no `cfg(not(...))` code in it, and passes while
never looking at the half that has some.

## C4. The coverage figure depends on the feature set, and one reading is a miscount

```bash
cargo tarpaulin -p ring_core --offline --skip-clean --out Stdout --engine llvm 2>&1 \
  | grep -E 'ring_core/src'
cargo tarpaulin -p ring_core --offline --features crossbeam --skip-clean --out Stdout --engine llvm 2>&1 \
  | grep -E 'ring_core/src'
```

**Expected:** `93/119` for the first and `120/120` for the second, **for the same
test suite**. That is a 21.85-point swing with nothing tested differently.

The 26-line difference is not untested code. Print the lines the first run
names and every one of them is a `Storage::Crossbeam` / `ProducerInner::Crossbeam`
/ `ConsumerInner::Crossbeam` arm, i.e. code the default build's compiler
removed. **Tarpaulin counts `cfg`-eliminated lines as uncovered rather than
omitting them from the denominator.** Verified stable: repeated runs give the
same figure, and dropping `--skip-clean` does not change it, so it is a real
property of the tool rather than a stale-artifact effect.

So `--all-features` is the honest reading, the one where every counted
line is also a line the binary contains, and G1 now passes it. This is a fix
to the machinery, and it was worth making rather than annotating. The goal for
this workstream is 100% across the family, and a gate that reports 78.2% for a
crate with nothing untested makes that number unreadable.

**A second, smaller artifact of the same kind, which could be mistaken for a
gap.** The crate briefly read `120/121`, one line short, and the line was
`else`, the bare keyword on its own line under the family's brace style, where `llvm-cov` opens a region nothing can execute. Both arms of that
`if` were hit, 23 and 16 times. It is now a `match`, which is what the other 32
crates use:

```bash
grep -cE '^[[:space:]]*else[[:space:]]*$' ring_*/src/lib.rs | grep -v ':0'
```

**Expected:** no output. Any crate this prints will read one line short of 100%
for a reason that has nothing to do with its tests.

## C5. This crate adds no atomic and no unsafe

A correctness requirement, not hygiene. `ring_spsc` asserts **zero read-modify-writes across
a run**, and every publish through this crate reaches it. A counter added here
would break that assertion with nothing in `ring_spsc`'s own dependency tree to
blame for it.

```bash
grep -nE '\b(Atomic[A-Za-z]*|unsafe|Ordering::|fetch_[a-z]+|compare_exchange)\b' \
  ring_core/src/lib.rs | grep -vE '^[0-9]+:[[:space:]]*//'
grep -c 'ring_stats' ring_core/Cargo.toml
```

**Expected:** no output from the first, and `0` from the second. Line 59
mentions `AtomicUsize` in prose and is correctly filtered. It was originally
line 48, shifted by the
`Fix(decision_121_link_pointed_at_docsrs_not_the_ruling)` module-doc-comment
correction, 2026-09-11. `ring_stats` (feature 185) is deliberately not a
dependency, because instrumentation is where the counter would come from.

The filter's anchoring matters here more than usual: this crate's module
documentation argues about atomics at length, and an unanchored grep reports
that argument as an implementation.

## C6. The API diverges from `ring_handle`'s specified one

```bash
grep -nE '&self|&mut self' ring_handle/docs/api/001_producer_surface.md
grep -nE 'pub fn (try_push|try_push_batch|free_capacity|is_closed)' ring_core/src/lib.rs
```

**Expected:** four hits in api/001 (lines 28–31), and three in `src/lib.rs`
(373, 441, 499). `is_closed` is absent, so it does not appear. The lines were
originally 353, 420, 478, shifted by the
`Fix(decision_121_link_pointed_at_docsrs_not_the_ruling)` module-doc-comment
correction (2026-09-11).

| Point | `ring_handle` api/001 | `ring_core` | |
|---|---|---|---|
| `try_push` receiver | `&self` | `&mut self` | diverges |
| `try_push_batch` receiver | `&self` | `&mut self` | diverges |
| `free_capacity` receiver | `&self` | `&self` | **agrees** |
| Refusal type | `Full< T >` | `T` | diverges |
| `is_closed` | present | **absent** | settled, see below |

**The receiver divergence is exactly the two mutating methods, and that is not
a coincidence.** The check was written expecting a blanket `&self` → `&mut self`
difference, and running it showed `free_capacity` matching the spec. The two
that differ are the two that reach a backend producer, and `ring_spsc`'s
requires exclusive access. The one that agrees only reads. So api/001's `&self`
is more than a style choice this crate ignored. Honouring it needs interior
mutability or a `Copy` producer at every backend, which is S6's problem to
solve, not a typo to correct.

`is_closed` is the one that is settled. Liveness is `ring_shutdown`'s (feature
184), and a handle-local copy of that flag is the failure `ring_shutdown`
exists to prevent, so this crate has no flag to read.

The receiver and refusal-type divergences are open and belong to S6, where
`ring_handle` is implemented. This stage exists so they are found by running a
command rather than by someone noticing during review.

## C7. `free_capacity`'s binding contract is not tested, and cannot be here

The module documentation's table says `free_capacity` is **binding at SPSC,
advisory at MPSC and crossbeam**, and calls this the crate's sharpest hazard,
one signature over two contracts with no compiler error between them.

`free_capacity_never_overstates_the_room_available` tests the direction that is
testable from one thread, that a reported `n` never exceeds the room actually
available. It does not test the binding claim, because the binding claim is
*"and at SPSC, `n` pushes will therefore succeed"*, whose only failure mode
requires a second thread taking the room in between. A single-threaded test
cannot distinguish binding from advisory, because both satisfy it.

```bash
grep -nE '^fn .*free_capacity' ring_core/tests/core_test.rs
```

**Expected:** exactly one hit, `free_capacity_never_overstates_the_room_available`
at line 619. It was originally 547. The suite has grown since, and independently
the crate's own module doc comment moved by ~20 lines under
`Fix(decision_121_link_pointed_at_docsrs_not_the_ruling)`, 2026-09-11. Track
the line number by re-running the command, not by memory, since it has already
drifted once. No test is named for the binding contract, because there is not
one.

Anchored to `^fn `, not to the identifier. The bare form returns six hits here,
three of which are the doc comments arguing about the hazard. A stage whose
expected count includes its own explanatory prose cannot distinguish a test
being added from a paragraph being added.

What stands in for it is structural rather than behavioural. SPSC is the backend
where `try_clone` returns `None`, so no second producer can exist to invalidate
the reading. `try_clone_refuses_at_spsc_and_permits_elsewhere` is what
guards the contract, and it guards it by construction rather than by
observation. This is recorded here because a reader looking for a `free_capacity`
binding test will not find one and should not conclude it was forgotten.

## C8. The concurrency test fails rather than hangs

A retry-on-refusal loop with no bound is an unbounded spin the moment the ring
stops releasing slots, and a hung test in CI reports nothing at all.

Both loops in `four_threads_publishing_through_clones_lose_nothing` are bounded
by a 30s deadline. Their bounds differ, and the difference is the check:

- The **consumer** loop refreshes its deadline **on progress**, whenever a
  drain moved at least one record. That is correct, because it is waiting on
  work that arrives incrementally.
- The **producer** loop computes its deadline **once, outside the loop, and
  never refreshes it**. This first refreshed it per iteration, which pushes the
  deadline forward faster than time passes. The assertion could never fire, and
  the guard silently became the unbounded spin it exists to prevent.

```bash
grep -n 'deadline' ring_core/tests/core_test.rs
awk 'NR>=804 && NR<=822 && /deadline[[:space:]]*=[^=]/ { print NR": "$0 }' \
  ring_core/tests/core_test.rs
```

**Expected:** six hits from the first: 803 (producer, computed before the
loop), 809 (the comment recording why), 818 (producer assertion), 827 (consumer
init), 834 (consumer refresh, guarded by `received.len() > before`), 839
(consumer assertion). Originally 715/721/730/739/747/752. Both the crate's
test suite and its module doc comment grew since, and re-running the first
command is how a reader confirms today's numbers rather than trusting either
list.

**Result (2026-09-11): the range in the second command was stale and had
silently gone vacuous.** The line numbers above shifted, but the awk range
`NR>=727 && NR<=734` was never updated alongside them. That range now falls
inside `every_public_type_is_debuggable`, a different test with no `deadline`
variable at all. So the command printed nothing for a reason that had stopped
being "the producer's loop is correct" and become "this range contains no
`deadline` token of any kind." Both directions were confirmed before fixing it.
Injecting a real `deadline =` reassignment at the consumer's refresh line
(834) still produced no output from the *old* range (727–734), proving it was
blind to a real change nearby. The corrected range (804–822) prints nothing
against the healthy file and does catch an injected reassignment placed
inside the producer's own `while let` body. The range is now **804–822**, the
producer's `while let` and its body. It was corrected alongside the line
numbers above so the two cannot drift apart silently a second time.

**The second command must print nothing.** Any *assignment* to `deadline`
inside the producer's `while let` body is the defect above, restored. A count
alone cannot catch it. The broken version had six hits too, just one of them
in the wrong place. That is why the range check is the real test and the
count is only orientation for it. A range check that has drifted off its own
target is worse than a count that miscounts. It still reports the same
reassuring "prints nothing," for a reason that has quietly stopped being the
reason it was written for. This is defect shape 4 from the summary below
("a prediction the command cannot reach"), found here rather than only in C9.

Under C1's mutation this test failed at **30.015s** with its own message,
rather than hanging, which is the evidence that the surviving guard fires. It
is the only observation of the deadline actually expiring, and it comes free
with C1, which is why the two stages are worth running in that order.

## C9. The dependency edges are the ones the docs claim

[`docs/integration/001`](../../docs/integration/001_family_dependency_seam.md)
accounts for six in-house edges, one external edge behind four flags, and four
crates that are *pointedly absent*. Every one of those claims is prose, and prose
about a dependency list goes stale the moment someone adds a line.

```bash
cd ring_core
grep -cE '^(ring_stats|ring_shutdown|ring_handle) ' Cargo.toml
cargo tree -p ring_core -e normal --depth 1
cargo tree -p ring_core -e normal --depth 1 --features crossbeam
cargo tree -p ring_core -e features --features crossbeam
cargo tree -p ring_core -e normal --invert
```

The third command is `-e features`, not a second `--depth 1`. The result below
shows why the depth-limited form was the wrong instrument for check 3.

**Expected**, written before running:

1. **0**, the three absent crates. Confirmed already. The grep anchors to
   column 0 so the prose in this very file cannot inflate it, which is defect
   shape 1 from the summary below applied in advance rather than after the fact.
2. **7 lines**, the crate plus its six in-house dependencies, and
   **no `crossbeam-queue`**. The default build must not pull the optional edge.
3. **8 lines**, the same plus `crossbeam-queue`, and beneath it nothing from
   `crossbeam-utils` or `std`, since `default-features = false`.
4. **1 line**, because nothing depends on `ring_core` yet. `ring_handle`,
   `ring_factory`, and `ring_bench` are S6–S8 work. When they land this line
   count rises, and the *direction* is what matters. Edges point into this
   crate, never out of it.

**Result (2026-08-28): 1, 2, and 4 correct. 3 is wrong, twice over.**

`crossbeam-utils v0.8.22` **is** pulled, `default-features = false`
notwithstanding. `crossbeam-queue` depends on it unconditionally for
`CachePadded` and `Backoff`, which `ArrayQueue` needs in every configuration.
Two probes established what the flag actually does:

```bash
cargo tree -p ring_core -e features --features crossbeam    # here
cargo tree -e features                                      # a scratch crate, plain `crossbeam-queue = "0.3"`
```

| | `crossbeam-queue` features | `crossbeam-utils` features | packages |
|---|---|---|---|
| default | `default`, `std`, `alloc` | **`std`** | 2 |
| ours | `alloc` | **none** | 2 |

So the flag removes a *feature*, not a crate. The benefit is real, since
`crossbeam-utils/std` is the thread-parking machinery a spin-only queue never
calls. But it is not the benefit the docs claimed, and
`docs/integration/001` and `docs/invariant/001` were both corrected.

**The sharper failure is that the command could not have tested the claim
either way.** `--depth 1` shows one level, and the prediction was about level
two. Had `crossbeam-utils` been absent as predicted, the stage would have
printed 8 lines, matched, and passed. It would have recorded a verified claim
it never looked at. A
prediction the command cannot reach is worse than a wrong one, because a wrong
one fails.

## Run Record

| Date | Stages | Outcome |
|---|---|---|
| 2026-08-28 | C1–C9 | 9/9 run. Six stages were wrong as first written and were corrected against real output rather than the output being explained away; two of those corrections changed machinery (`g1_coverage.sh`, `g2_docs.sh`) and one changed a test (the producer's deadline guard, which could never fire). C1 measured 4 integration failures against **0 of 7 doc-test failures** under the shipped MPSC bug, which is the crate's sharpest finding: the doc examples reach one of `try_push`'s three arms. C4 found the same crate reading 78.2% and 100% for one suite, 26 lines apart, and the low reading counting code the compiler had deleted. The crate's own citation of feature 187 named a file that does not exist and was corrected to `187_optional_crossbeam_queue_backend.md` |

### Per-stage detail

| Stage | Run | Correct as first written | Result |
|---|---|---|---|
| C1 | yes | yes | 4 integration FAIL, **0 of 7 doc tests fail**; 21/21 after restore |
| C2 | yes | yes | 7 blocks at 52/79/142/339/406/443/548; MPSC push path and crossbeam reached by none |
| C3 | yes | **no** | grep counted its own comments, giving 5 hits, not 3. Re-anchored to `cargo <sub>` |
| C4 | yes | **no** | expected one figure, got two: 93/119 vs 120/120, 26 lines `cfg`-removed |
| C5 | yes | yes | zero atomics, zero unsafe, `ring_stats` absent |
| C6 | yes | **no** | `free_capacity` was predicted to diverge; it agrees. Divergence is the two mutating methods only |
| C7 | yes | **no** | "hits in that test only" was false, with 6 hits, half of them prose. Re-anchored to `^fn ` |
| C8 | yes | **no** | predicted five hits, found six. A count cannot catch this defect at all, so it needed a range check |
| C9 | yes | **no** | 3 of 4 predictions held; `crossbeam-utils` is pulled regardless of `default-features`, and `--depth 1` could not have seen it either way |

**Six of nine stages were wrong as first written. Three of them changed code,
machinery, or documentation.**

- **C4** expected one coverage figure. Running it produced two, 22 points apart,
  and the smaller turned out to be counting lines the compiler had deleted. As
  written the stage would have recorded a coverage gap that does not exist.
- **C3** was written as a check that the gates cover both configurations. They
  did not. The stage found a hole in the machinery rather than in the crate,
  and `g1_coverage.sh` and `g2_docs.sh` were both amended before it could pass.
  Then its own grep proved wrong on top of that.
- **C8** was written to document a guard. Reading the guard closely enough to
  write the stage is what exposed that the producer half refreshed its deadline
  every iteration and so could never fire. Then the stage's expected count was
  wrong. Worse, a count was the wrong instrument, because the broken version
  had six hits too.
- **C6 and C7** were both wrong in the same direction. Each stated an
  expectation as a blanket ("the receiver diverges", "hits in that test only")
  that measurement narrowed. C6's narrowing is the more interesting of the two, because the
  narrowed fact explains *why* the divergence exists.

Four defect shapes recur across this family's plans and are worth naming, since
every failure here is one of them:

1. **A grep that counts its own prose** (C3, C7). The paragraph explaining a
   check contains the string the check greps for, so the count drifts upward,
   and upward reads as extra safety. Anchor to syntax, never to an identifier.
   C9 is the first stage written with this shape already in mind. Its
   column-anchored `^(ring_stats|...)` is deliberately immune to the sentence
   naming all three crates two lines above it.
2. **A count where a position is what matters** (C8). The defect and the fix
   have the same number of hits.
3. **A blanket expectation that measurement narrows** (C4, C6, C7, C9). Being
   nearly right is what makes these survive review.
4. **A prediction the command cannot reach** (C9, new). The stage asserts
   something at depth 2 and runs a command limited to depth 1. This shape is
   strictly worse than the other three, because all of those *fail* when they
   are wrong. This one **passes either way**. Had the prediction been correct,
   the stage would have printed the expected line count, matched, and recorded a
   claim it never examined. Check that each command can observe the thing its
   expectation is about, and not only that the expectation is right.

This is the third crate in a row where executing a manual plan's own commands
found defects that writing them did not: `ring_mpsc` (3 of 9 stages),
`ring_spsc` (3 checks), now 6 of 9. **The rate is not falling**, which is the
argument for the discipline rather than against it. The commands are the check,
and the prose is only a record of having run them.

One further defect surfaced during this crate's work and was fixed at the leaf
rather than here. Adding `RingError::PolicyUnsupported` to `ring_types` left its
`Display` arm unexercised, and G1 reported `ring_types/src/error.rs 16/17` on
the next run. `ring_types`' own roster test now carries that incident. The
roster is maintained by hand, and because the enum is `#[ non_exhaustive ]` it
cannot be made to fail the build from outside the crate. G1 is what catches it.
