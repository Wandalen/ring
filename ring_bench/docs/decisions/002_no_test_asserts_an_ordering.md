# Decision: No Test Asserts an Ordering

**Status:** accepted, 2026-08-28. Rules what this crate's suite may assert about
the numbers it produces, which [`readme.md`](readme.md) records as Closed 2.

### Scope

- **Purpose**: Rule that no test in this crate may assert which candidate is fastest, and record the measured variance that turns that from a prediction into a finding.
- **Responsibility**: The obvious test that must not be written, the four options, what is asserted instead, the B3 variance measurement, and the regression gap this accepts.
- **In Scope**: Timing assertions; the eligibility-set assertions that replace them; the one coarse verdict the data does license.
- **Out of Scope**: The eligibility rule itself (→ [`../algorithm/002`](../algorithm/002_the_eligibility_filter_runs_before_the_comparison.md)); how many candidates there are (→ [`001_five_candidates_for_four_named_paths.md`](001_five_candidates_for_four_named_paths.md)).

### What forced it

**A benchmark crate whose whole output is a ranking has an obvious test to
write, and it is the one test it must not write.**

```rust
// never written here
assert_eq!( comparison.fastest().unwrap().candidate(), Candidate::DirectSpsc );
```

That assertion is a claim about wall-clock time on the machine running the
suite. It fails under load, under a different core count, under a debug profile,
and on a laptop that thermally throttles mid-run. And a flaky test **inside a
benchmark harness** does not merely annoy — it discredits precisely the
measurement the harness exists to produce. The first person to see it go red on
an unrelated change concludes the numbers are noise, and they are then right to.

### Options

| # | Shape | Consequence |
|---|-------|-------------|
| A | Assert the expected winner | Flaky by construction. Discredits the output it was meant to protect |
| B | Assert a ratio with a tolerance | Same failure, deferred. A tolerance wide enough never to flake is wide enough to assert nothing |
| C | Assert nothing about the numbers at all | The timing code could stop being written and no test would notice |
| ✅ **D** | **Assert record accounting; read the timing and assert nothing about it** | Everything deterministic is checked; the one non-deterministic quantity is exercised without being constrained |

**C is the failure mode D is shaped against.** A field nobody reads can silently
stop being populated. `a_roomy_run_keeps_everything_and_returns_it` therefore
calls `outcome.write_nanos()` and binds the result to `_elapsed` — the call is
the assertion. Reading it and asserting nothing is the only honest option:
either it is read or the code path can rot; either it is unconstrained or the
test is flaky.

### The decision

**What is asserted:** offered, reported, received, dropped, silently discarded,
refusals, ceilings, error renderings, counter totals. All deterministic. All are
what make a timing number mean anything — a comparison between two candidates
that kept different numbers of records is not a comparison.

**What is never asserted:** which candidate is fastest, how much faster,
or that any candidate is faster than any other. Not once, in any test, under any
fixture.

**What is asserted about `fastest()` instead is the *eligibility set*:**

| Fixture | Assertion |
|---|---|
| `roomy` (4096 slots, 256 records) | `fastest()` is `Some`, and whatever it returns is lossless |
| `cramped` (16 slots, 256 records) | `fastest()` is `None` — no candidate kept the workload |
| `parallel` (4 producers) | The refusal list is non-empty and every refusal is a `ProducerCeiling` |

Each is a claim about *which candidates are permitted to win*, which is
deterministic, and it is the claim that carries the crate's one load-bearing
rule: **a path that dropped records is not eligible to be fastest.** The
quickest way to finish a write phase is to refuse every record, so ranking by
time alone ranks the worst candidate first and prints a plausible number while
doing it.
→ [`algorithm/002`](../algorithm/002_the_eligibility_filter_runs_before_the_comparison.md).

### The variance this rests on, measured

This decision was made from a prediction — that an ordering assertion would be
flaky. Manual stage B3 (`tests/manual/readme.md`) measured it: ten comparisons
of the identical roomy workload in one process, 2026-08-28.

| Quantity | Measured |
|---|---|
| Distinct winners across 10 rounds | 2 (`DirectSpsc` once, `OffTheShelf` nine times) |
| Spread of `contract_ring` across rounds | 32 960 – 95 800 ns — a factor of **2.9** |
| Gap between the three leaders within a round | **1–3 %** |

**A 1–3% difference against 190% run-to-run noise.** An assertion that
`off_the_shelf` beats `direct_spsc` is not a marginal call; it is asserting a
signal two orders of magnitude below the variance it sits in. The prediction
was right and is now a number rather than an intuition.

**The ten rounds were recorded with `off_the_shelf` in the field, which only
`cargo test --features crossbeam` compiles** — `cargo test -p ring_bench`, the
default build, has five candidates, not six (→ BN14). Recomputed with
`off_the_shelf` dropped from each round, `direct_spsc` wins nine rounds and
`contract_ring` wins the tenth: still at least two distinct winners, on the
same data, in the build the default suite actually runs. The instability the
prediction was about is real in both builds; only the specific winners differ.

**What the same data does license.** Across all ten rounds three groups appear,
and only the first two never overlap:

| Group | Range across 10 rounds |
|---|---|
| `contract_ring`, `direct_spsc`, `off_the_shelf` | 30 240 – 95 800 ns |
| `mutex_queue` | 130 601 – 291 362 ns |
| `direct_mpsc` | 239 282 – 653 645 ns |

So at one producer **the three single-producer lock-free paths beat the mutex
baseline by 3–5x with no overlap in any round** — which is the verdict
this comparison actually asked for, and it is safe to state. `direct_mpsc`, the
fourth lock-free path, is a multi-producer structure driven by one producer —
its worst configuration — and its range overlaps the mutex baseline's on
239 282 – 291 362 ns (→ BN13); it is not part of this verdict. What remains
unstateable is the ordering *within* the leading group.

This distinction is why the decision is "no test asserts an ordering" rather
than "no ordering can be reported". The report prints the durations; a human
reads the separation. Automating the coarse verdict would need a tolerance
policy and a stored baseline — see *What would reverse it*.

### What it costs

**A performance regression in a candidate will not break a test here.** If
`ring_core`'s dispatch became ten times more expensive, every assertion in this
suite would still pass. That is a real gap and it is accepted: catching it needs
a stored baseline and a tolerance policy, which is a different artefact from a
behavioural suite and would come with the flakiness this ADR exists to refuse.
Filed as a known non-coverage rather than papered over — the report is the
place a regression shows, and reading it is a human step.

### What would reverse it

A stable measurement environment — pinned cores, fixed governor, a recorded
baseline — makes B viable, and at that point a ratio assertion is worth having.
Nothing in this workspace provides one, and adding a test that assumes one is
how a suite acquires a failure mode that only appears on other people's
machines.

### BN13 — The One Verdict This ADR Licenses Is Contradicted by the Table It Is Read Off

"At one producer the lock-free paths beat the mutex baseline by 3–5x with no
overlap in any round" is the single ordering claim this decision says is safe to
state. Its own evidence table disagrees:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
echo '  -- the group ranges the verdict is read off --'
awk '/Group \| Range across 10 rounds/, /^$/' tests/manual/readme.md | sed 's/^/    /'
echo '  -- do the mutex and the lock-free MPSC ranges overlap? --'
awk '
  /^\| .mutex_queue. \|/ { m = $0 }
  /^\| .direct_mpsc. \|/ { d = $0 }
  END {
    split( m, a, "|" ); split( a[ 3 ], ml, "–" )
    split( d, b, "|" ); split( b[ 3 ], dl, "–" )
    printf "    mutex_queue  %d - %d ns\n", ml[ 1 ], ml[ 2 ]
    printf "    direct_mpsc  %d - %d ns\n", dl[ 1 ], dl[ 2 ]
    printf "    overlap      %s\n", ( dl[ 1 ] + 0 <= ml[ 2 ] + 0 ? "YES, on " dl[ 1 ] + 0 " - " ml[ 2 ] + 0 " ns" : "none" )
  }
' tests/manual/readme.md
echo '  -- is the slower one lock-free? --'
printf '    locks in ring_mpsc/src : %s\n' "$( command grep -cE 'Mutex|RwLock|\.lock\(\)' ../ring_mpsc/src/lib.rs )"
printf '    locks in ring_core/src : %s\n' "$( command grep -cE 'Mutex|RwLock|\.lock\(\)' ../ring_core/src/lib.rs )"
printf '    locks in ring_bench    : %s  (the baseline itself)\n' "$( command grep -cE 'Mutex|\.lock\(\)' src/lib.rs )"
```

Live output:

```
  -- the group ranges the verdict is read off --
    | Group | Range across 10 rounds |
    |---|---|
    | `contract_ring`, `direct_spsc`, `off_the_shelf` | 30240 – 95800 ns |
    | `mutex_queue` | 130601 – 291362 ns |
    | `direct_mpsc` | 239282 – 653645 ns |
    
  -- do the mutex and the lock-free MPSC ranges overlap? --
    mutex_queue  130601 - 291362 ns
    direct_mpsc  239282 - 653645 ns
    overlap      YES, on 239282 - 291362 ns
  -- is the slower one lock-free? --
    locks in ring_mpsc/src : 0
    locks in ring_core/src : 0
    locks in ring_bench    : 15  (the baseline itself)
```

**`direct_mpsc` is a lock-free path and it is the slowest row in the table.**
`ring_mpsc` and `ring_core` contain no lock of any kind; the only `Mutex` in the
comparison is the baseline the sentence says the lock-free paths beat. At one
producer `direct_mpsc` runs 239 282 – 653 645 ns against the mutex's
130 601 – 291 362 ns — worse at both ends — by 1.8x at the low end and 2.2x at the
high — and the two ranges **overlap on
239 282 – 291 362 ns**, which is the exact thing the sentence above them says
does not happen. Whether it is slower in every individual round cannot be told
from the record at all: the per-round lines print only the three leaders
(→ BN14).

The table's own lead-in says "the two groups never overlap" and then lists
**three** groups. The claim is true of the pairing it was written about — the
three leaders against the mutex — and the sentence generalises it to "the
lock-free paths", which is a class the third row also belongs to.

**This is the crate's most consequential wrong sentence**, because this ADR's
entire structure is a refusal to state orderings *except this one*. Everything
else here is a discipline about what may not be claimed; this is the one thing
that is claimed, and it is claimed about a set the evidence splits.

The narrow verdict is intact and worth keeping: *at one producer, the three
single-producer lock-free paths beat the mutex baseline by 3–5× with no overlap*.
`direct_mpsc` is a multi-producer structure driven by one producer, which is the
configuration it is worst at, and its row is a finding rather than a
counterexample. But the finding is not stated — it is contradicted, by a
sentence that reads as a summary of the table directly above it.

The general shape: **a verdict written one row wider than its evidence reads as
a summary and functions as a claim**, and the widening happens in the sentence
that names the category rather than in the data.

```sh
cd "$(git rev-parse --show-toplevel)"
# -m2 and no -n: this file is its own subject, so an unbounded match
# also finds this command line and every copy of its own output below,
# and -n re-prefixes a fresh line number onto each earlier pass's output
command grep -m2 'three single-producer lock-free paths\|three groups appear' ring_bench/docs/decisions/002_no_test_asserts_an_ordering.md
```

Live output:

```
**What the same data does license.** Across all ten rounds three groups appear,
So at one producer **the three single-producer lock-free paths beat the mutex
```

**Disposition:** applied — the lead-in now says "three groups appear" instead
of the miscounted "two groups", and the verdict sentence narrows to "the three
single-producer lock-free paths", with `direct_mpsc`'s own overlapping range
named explicitly as the reason it is excluded. The narrow claim this ADR relies
on is now the claim the sentence actually makes.
Now prints: `three single-producer lock-free paths`

### BN14 — The Variance Evidence Was Taken in the Build the Suite Does Not Run

Every recorded round names `off_the_shelf`, which exists only behind the
`crossbeam` feature. Recompute the rounds without it:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
echo '  -- which build the recorded rounds were taken in --'
printf '    rounds naming off_the_shelf : %s of %s\n' \
  "$( command grep -c '^round [0-9]*:.*off_the_shelf=' tests/manual/readme.md )" \
  "$( command grep -c '^round [0-9]*:' tests/manual/readme.md )"
printf '    cfg sites gating that variant in src : %s\n' \
  "$( command grep -c 'cfg( feature = "crossbeam" )' src/lib.rs )"
echo '  -- recompute each round with the optional candidate removed --'
awk '
  /^round [0-9]+:/ {
    cr = ds = 0
    for ( i = 1; i <= NF; i++ )
    {
      if ( $i ~ /^contract_ring=/ ) { split( $i, p, "=" ); cr = p[ 2 ] + 0 }
      if ( $i ~ /^direct_spsc=/ )   { split( $i, p, "=" ); ds = p[ 2 ] + 0 }
    }
    w = ( ds < cr ? "DirectSpsc" : "ContractRing" )
    seen[ w ] = 1
    printf "    %-8s default-build winner = %-12s (contract_ring=%d direct_spsc=%d)\n", substr( $1 $2, 1, 7 ), w, cr, ds
  }
  END { n = 0; for ( k in seen ) n++; printf "    distinct winners without the feature : %d\n", n }
' tests/manual/readme.md
```

Live output:

```
  -- which build the recorded rounds were taken in --
    rounds naming off_the_shelf : 10 of 10
    cfg sites gating that variant in src : 6
  -- recompute each round with the optional candidate removed --
    round0:  default-build winner = DirectSpsc   (contract_ring=33881 direct_spsc=30960)
    round1:  default-build winner = DirectSpsc   (contract_ring=33040 direct_spsc=30800)
    round2:  default-build winner = DirectSpsc   (contract_ring=95800 direct_spsc=90240)
    round3:  default-build winner = DirectSpsc   (contract_ring=94921 direct_spsc=89441)
    round4:  default-build winner = DirectSpsc   (contract_ring=55760 direct_spsc=52080)
    round5:  default-build winner = DirectSpsc   (contract_ring=45640 direct_spsc=31240)
    round6:  default-build winner = DirectSpsc   (contract_ring=71041 direct_spsc=66760)
    round7:  default-build winner = DirectSpsc   (contract_ring=35200 direct_spsc=32961)
    round8:  default-build winner = DirectSpsc   (contract_ring=32960 direct_spsc=30961)
    round9:  default-build winner = ContractRing (contract_ring=33281 direct_spsc=43000)
    distinct winners without the feature : 2
```

Stage B3's prediction was "at least two distinct winners across ten rounds", and
its result — two, `DirectSpsc` once and `OffTheShelf` nine times — depends on a
candidate that is absent from `cargo test -p ring_bench`. **The evidence for a
decision governing the default suite was gathered in a build the default suite
does not compile.** Nothing in the ADR or in the stage says so; B4, two sections
below B3 in the same file, is *about* the feature being off and does not connect
back.

**And the repository runs both builds, which is the part that makes this
unresolvable rather than merely unstated.** `verb/test` — the project's own
final-verification command — invokes `cargo nextest run --all-features`, so the
six-candidate build is what the gate grades. A developer running
`cargo test -p ring_bench` gets the five-candidate one. The ADR names neither,
and its conclusion is a statement about relative timings, which is exactly the
kind of claim that can differ between the two.

**Recomputed against the default build, the conclusion survives** — and it
survives more narrowly than the record suggests. Dropping `off_the_shelf` leaves
`direct_spsc` winning nine rounds and `contract_ring` winning round 9, so the
"at least two distinct winners" prediction is still met, by exactly one round, on
a 33 281-against-43 000 margin. The instability is real in both builds.

That recomputation is what was missing, and it is cheap: the per-round durations
were recorded, so the counterfactual is arithmetic rather than a re-run. **What
the record lacked was not data but the question** — nobody asked whether the
evidence transferred to the build being shipped, because the output looked like
the output of the thing under test.

Two smaller consequences fall out of the same gap. The per-round lines print
three candidates; `mutex_queue` and `direct_mpsc` appear only in the summary
ranges, so BN13's "no overlap in any round" cannot be checked against the
recorded rounds at all — the numbers that would settle it were never printed.
And with the feature off, the three-leader group the variance argument is about
has two members, which changes "the gap between the three leaders" from a
statement about a trio to one about a pair.

The general shape: **evidence gathered under a non-default feature is evidence
about a different program**, and a feature that only *adds* a candidate looks
harmless precisely because the rows it does not touch still look right.

```sh
cd "$(git rev-parse --show-toplevel)"
# -m1 and no -n: this file is its own subject, so an unbounded match
# also finds this command line and every copy of its own output below,
# and -n re-prefixes a fresh line number onto each earlier pass's output
command grep -m1 'default build, has five candidates' ring_bench/docs/decisions/002_no_test_asserts_an_ordering.md
```

Live output:

```
default build, has five candidates, not six (→ BN14). Recomputed with
```

**Disposition:** applied — the "variance this rests on" section now states
which build the ten recorded rounds were taken in and gives the recomputed
default-build result inline (`direct_spsc` nine rounds, `contract_ring` one),
so the "at least two distinct winners" prediction is shown to hold in both
builds rather than only the one that was measured.
Now prints: `default build, has five candidates`
