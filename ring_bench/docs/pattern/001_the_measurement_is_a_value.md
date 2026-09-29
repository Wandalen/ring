# Pattern: The Measurement Is a Value

### Scope

- **Purpose**: Record that every quantity this crate produces is a field with an accessor, that nothing in the library prints, and what that buys.
- **Responsibility**: State the problem, the structure that solves it, where the pattern earns its cost, and what it costs.
- **In Scope**: `Outcome` and `Comparison` as returned data; `report()` returning a `String`; the absence of any `println!`.
- **Out of Scope**: Which numbers exist (→ [`data_structure/002`](../data_structure/002_three_counts_that_are_not_interchangeable.md)); the report's layout (→ [`api/002`](../api/002_the_report_surface.md)).

### Problem

**A benchmark harness's natural shape is a program that prints a table.** That
shape has three properties this crate cannot afford:

| Force | Consequence for a printing harness |
|---|---|
| The output must be **testable** | Asserting on a table means capturing stdout, which is global state a parallel test runner shares |
| The numbers must be **recombinable** | A printed table is a string; a consumer who wants `silently_discarded` as a fraction has to parse it back |
| The judgement must be **auditable** | If `fastest` is computed while formatting, the eligibility filter is a formatting detail rather than a rule |

The third is the decisive one. This crate's one load-bearing rule — a path that
dropped records is not eligible to be fastest — has to live somewhere a test can
reach it.

### Solution

**Nothing in `src/lib.rs` prints.** No `println!`, no `eprintln!`, no `dbg!`:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -cE 'println!|eprintln!|dbg!' ring_bench/src/lib.rs || true
```

Live output:

```
0
```

Three layers, each returning:

| Layer | Returns | Consumer |
|---|---|---|
| `run` | `Result< Outcome, RunError >` | one candidate, one workload |
| `Comparison::run` | `Comparison` — outcomes and refusals | the deliverable |
| `Comparison::report` | `String` | whoever wants to print it |

**`report` returning a `String` rather than printing is the pattern's hinge.**
A test asserts on substrings of it; a binary prints it; a future harness writes
it to a file. All three work, and the assertion needs no stdout capture.

### Applicability

| Condition | Why it matters |
|---|---|
| The output carries a **judgement**, not just numbers | If nothing is decided while formatting, printing costs little. Here `fastest` applies an eligibility rule, and a rule reachable only through a rendered table is a rule no test can isolate |
| The numbers will be **recombined** by someone | A consumer wanting `silently_discarded` as a fraction of `offered` gets an accessor rather than a parser |
| The suite is the **primary consumer** | Stdout capture is global state a parallel runner shares; a returned value has no such contention |
| A human still has to read it eventually | The pattern does not remove the need to print — it moves the print to the edge, where `report() -> String` meets whatever prints it |

The first row is the one to check. A harness whose output is purely descriptive —
six numbers, no verdict — can print them and lose nothing; this crate cannot,
because its one load-bearing rule lives in the same code path.

### Consequences

**Every derived judgement is a method on a type**, so each is separately
reachable and separately testable: `dropped`, `silently_discarded`,
`is_lossless`, `conserved`, `fastest`. A harness that computed these inline
while formatting would have exactly one testable surface — the whole table —
and a defect in any one of them would present as a wrong character in a string.

**The counts cannot be recombined into a judgement this crate does not make.**
The fields are private and the accessors are read-only, so a caller can compute
their own ratios but cannot, for instance, mutate `received` to make a candidate
eligible. That matters less for a library nobody outside the workspace consumes
than it does for what it forces during authoring: every judgement had to be
given a name and a definition, and `conserved` being the wrong *kind* of
judgement was only visible once it had one.
→ [`pitfall/003`](../pitfall/003_ok_is_not_kept_and_the_verdict_inverts.md).

**`examples/comparison.rs` is the binary, and it is not in `src/`.** Run via
`cargo run -p ring_bench --all-features --example comparison`, it is the one
place the family's measurement is shown to a human — twelve `println!`s over
three workloads, reached by one documented command (→ BN37). `src/lib.rs`
itself still prints nothing, which is the half of this pattern that holds
exactly: the example is a caller of the returned values, not a hole in the
pattern. What is narrower than the original claim: the report's formatting is
exercised by substring assertions *and* by the example, but the example
asserts nothing, so a formatting regression there is caught only by a human
reading `cargo run`'s output, not by any test.

**The `write_nanos` corollary:** a value nobody reads can stop being written.
The suite therefore *reads* `write_nanos` and asserts nothing about it — the
only honest treatment of a non-deterministic quantity that must nonetheless keep
being produced.
→ [`decisions/002`](../decisions/002_no_test_asserts_an_ordering.md).

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_run_surface.md](../api/001_the_run_surface.md) | The three returning layers |
| [../api/002_the_report_surface.md](../api/002_the_report_surface.md) | The one that returns text, and still returns rather than prints |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_three_counts_that_are_not_interchangeable.md](../data_structure/002_three_counts_that_are_not_interchangeable.md) | The fields this pattern keeps addressable |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_no_test_asserts_an_ordering.md](../decisions/002_no_test_asserts_an_ordering.md) | What the testability this pattern buys is and is not used for |

### Patterns

| File | Relationship |
|------|--------------|
| [002_a_refusal_is_a_row.md](002_a_refusal_is_a_row.md) | The same instinct applied to what did *not* happen |

### Sources

| File | Relationship |
|------|--------------|
| [`../readme.md`](../readme.md) | "The comparison is the deliverable" — a deliverable is a value, not a side effect |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/bench_test.rs`](../../tests/bench_test.rs) | Every test in the file is an instance: each asserts on returned values, and no test captures output |

### BN37 — The Cost This Pattern Books as Unpaid Was Paid, in a File the Documentation Does Not Know Exists

The Consequences section states the price of returning a value instead of
printing one, and states it as a fact about the crate:

> **The cost is that there is no binary.** Nothing in this crate runs a
> comparison and shows it to a human; the suite is the only caller.

Both sentences are false, and have been since before this instance was written.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
printf 'what is beside src/                   : %s\n' "$( ls src/ examples/ | tr '\n' ' ' )"
printf 'println! in examples/comparison.rs    : %s\n' "$( command grep -c 'println!' examples/comparison.rs )"
printf 'println! in src/lib.rs                : %s\n' "$( command grep -c 'println!' src/lib.rs )"
printf 'the command it documents              : %s\n' "$( command grep -m1 -o 'cargo run -p ring_bench.*comparison' examples/comparison.rs )"
printf -- '--- its own module doc, on why it exists ---\n'
command grep -m1 -A19 -F '//! The ring family'"'"'s write-path comparison, run and printed.' examples/comparison.rs | command grep -E '^//!' | command grep -E 'shown to a human|documentation errors|throwaway' | sed 's|^//! *|  |'
printf -- '--- who names it ---\n'
printf 'docs/ files, generated index excluded : %s\n' "$( find docs -name '*.md' -not -path 'docs/definition/*' | wc -l )"
printf -- '--- of those, naming it outside their own findings tables ---\n'
n=0
for f in $( find docs -name '*.md' -not -path 'docs/definition/*' | sort ); do
  if awk '/^### BN|^### Findings Recorded Here/{ exit } { print }' "$f" \
     | command grep -q 'examples/comparison\|--example comparison'; then printf '  %s\n' "${f#docs/}"; n=$(( n + 1 )); fi
done
printf '  documents that do                   : %s\n' "$n"
printf -- '--- and naming it anywhere at all, findings included ---\n'
printf '  documents that do                   : %s\n' \
  "$( command grep -rl 'examples/comparison\|--example comparison' docs/ | sed 's|^docs/||' | sort | tr '\n' ' ' )"
printf 'outside docs/, naming it              : %s\n' "$( command grep -rln 'examples/comparison\|--example comparison' readme.md Cargo.toml 2>/dev/null | tr '\n' ' ' )"
```

Live output:

```
what is beside src/                   : examples/: comparison.rs  src/: lib.rs 
println! in examples/comparison.rs    : 12
println! in src/lib.rs                : 0
the command it documents              : cargo run -p ring_bench --all-features --example comparison
--- its own module doc, on why it exists ---
  This is the one place the family's measurement is shown to a human. The
  documentation errors (`tests/manual/readme.md` B1). This example exists so
  that reading is a command rather than a throwaway probe.
--- who names it ---
docs/ files, generated index excluded : 41
--- of those, naming it outside their own findings tables ---
  pattern/001_the_measurement_is_a_value.md
  documents that do                   : 1
--- and naming it anywhere at all, findings included ---
  documents that do                   : definition/readme.md pattern/001_the_measurement_is_a_value.md pattern/readme.md type/001_candidate.md 
outside docs/, naming it              : readme.md 
```

`examples/comparison.rs` is the binary. It runs three workloads, prints four
sections through twelve `println!`s, and is reached by one documented command.
`src/lib.rs` has none — that half of the pattern holds exactly, which is the
part worth keeping.

What makes this a finding rather than a stale sentence is the example's own
reason for existing. Its module doc says it is *"the one place the family's
measurement is shown to a human"*, that reading it end to end *"exposed four
documentation errors"*, and that it exists *"so that reading is a command rather
than a throwaway probe."* That is a description of the gap this instance lists
as open, written by whoever closed it. The two texts are about the same absence;
one says it is deliberate and unpaid, the other says it was paid and what it
bought.

The mechanism is visible in the last four lines. Of the forty-one documents
under `docs/` outside the generated index, **zero** name the example in their
own prose. Four name it at all, and every one of the four does so in a findings
section or a findings table: this finding, BN46 — which arrived at the same file
from the other direction, through a `name()` string — and the two tables that
index them. Outside `docs/` it is named by `readme.md`, which no instance reads.
A documentation set organised by definition has no definition whose subject is
*what runs*, so a new executable is registered in the onboarding readme, which
no instance reads, and every instance that reasoned from its absence keeps
reasoning from it. `item/` catalogues declarations, `api/` catalogues entry
points, `lifecycle/` catalogues phases; a file that is none of those three is
invisible to all of them.

The general shape, and the reason this is recorded here rather than fixed in
passing: **a documentation set that partitions by kind of claim cannot see an
artifact that makes no claim.** The example asserts nothing about the API. It
just runs, and running is not a category any of the thirteen definitions owns.

```sh
cd "$(git rev-parse --show-toplevel)"
# -m1 and no -n: this file is its own subject, so an unbounded match
# also finds this command line and every copy of its own output below,
# and -n re-prefixes a fresh line number onto each earlier pass's output
command grep -m1 'is the binary, and it is not in' ring_bench/docs/pattern/001_the_measurement_is_a_value.md
```

Live output:

```
**`examples/comparison.rs` is the binary, and it is not in `src/`.** Run via
```

**Disposition:** applied — the Consequences section no longer claims there is
no binary; it names `examples/comparison.rs`, the command that runs it, and
narrows the formatting-coverage claim to what is actually true: exercised by
assertions and by a human reading `cargo run`'s output, not solely by
assertions. The documentation-set blind spot BN37 also names (no definition
type owns "what runs") is recorded as a known gap rather than fixed, since
closing it would mean adding a new doc definition type — out of scope for a
disposition pass.
Now prints: `is the binary, and it is not in`

### BN38 — The One Place the Crate Prints, Prints a Conclusion It Did Not Compute

This pattern's premise is that a judgement belongs to whoever holds the numbers,
and that formatting is an edge concern: `Comparison` returns rows, `fastest`
returns an `Option`, and nothing in `src/` decides what the rows *mean*. The
example is that edge. It is also the only code in the crate that states a
verdict, and the verdict is a string literal.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
printf -- '--- everything stability() computes ---\n'
awk '/^fn stability/, /^\}/' examples/comparison.rs \
  | command grep -nE 'let mut |= u128::|winners\.push|let ratio|name\(\) ==' \
  | sed -E 's/^([0-9]+:)\s*/  \1 /' | sed -E 's/^(.{0,92}).*/\1/' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
printf -- '--- the comment on what it throws away ---\n'
awk '/^fn stability/, /^\}/' examples/comparison.rs | command grep -E '^\s*//' | sed 's/^ */  /'
printf -- '--- the verdict it prints ---\n'
awk '/^fn stability/, /^\}/' examples/comparison.rs \
  | sed -n '/reading: one candidate/,/decisions\/002/p' | sed 's/^/  /'
printf -- '--- the verdict names three quantities; here is each one in the function ---\n'
for w in contract_ring MutexQueue mutex overlap; do
  printf '  %-14s : %s\n' "$w" "$( awk '/^fn stability/,/^\}/' examples/comparison.rs | command grep -c "$w" )"
done
```

Live output:

```
--- everything stability() computes ---
   let mut winners : Vec< &'static str > = Vec::new();
   let mut lowest = u128::MAX;
   let mut highest = 0;
   winners.push( fastest.candidate().name() );
   if outcome.candidate().name() == "contract_ring"
   lowest = u128::min( lowest, outcome.write_nanos() );
   highest = u128::max( highest, outcome.write_nanos() );
   let ratio = if lowest > 0 { highest as f64 / lowest as f64 } else { f64::NAN };
--- the comment on what it throws away ---
  // Track one candidate's own spread across identical runs. Whichever
  // candidate is slowest is uninteresting here; the point is the range.
--- the verdict it prints ---
      "\n  reading: one candidate's run-to-run spread is far larger than the gap\n\
       \x20          between the leading candidates in any single run. That is why no\n\
       \x20          test in this crate asserts which candidate is fastest. The coarse\n\
       \x20          verdict below is docs/decisions/002's own recorded finding, not\n\
--- the verdict names three quantities; here is each one in the function ---
  contract_ring  : 2
  MutexQueue     : 0
  mutex          : 1
  overlap        : 1
```

`stability()` computes three things: the set of distinct winners across ten
identical runs, the minimum and maximum of the one candidate whose name string
matches `"contract_ring"`, and their ratio. It then prints a sentence about *the
mutex baseline* — asserting a multiple ("several times") it never divides and a
disjointness ("no overlap") it never tests, about a candidate it never reads.
The word `mutex` occurs once in the whole function and `overlap` once, both
inside the literal being printed.

The discard is deliberate and documented. Immediately above the loop that fills
the spread, a comment states the design: *"Whichever candidate is slowest is
uninteresting here; the point is the range."* That is correct about the range —
and *which candidate is slowest* is the entire subject of the paragraph the same
function prints thirteen lines later.

Two things follow, and they point in opposite directions.

The narrow one is that this is the same claim [`decisions/002`](../decisions/002_no_test_asserts_an_ordering.md)
licenses and its own evidence table refutes, recorded as BN13 — the mutex
baseline is not slowest in every row of that table. Here it has become program
output, printed beneath live numbers that give it no support, to the one reader
the crate directs at a command.

The broad one is about this pattern. Keeping judgement out of `src/` did not
delete the judgement; it relocated it to the only file the pattern does not
govern, where it is a `println!` argument rather than a function with a return
type, and where nothing — no test, no gate, no reviewer reading `docs/` — is
looking. The pattern's stated benefit is that a conclusion becomes assertable
because it is a value. Its unstated corollary is that a conclusion which stays
prose stays unassertable, and prose is exactly what the edge is made of.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep "own recorded finding, not" ring_bench/examples/comparison.rs
```

Live output:

```
     \x20          verdict below is docs/decisions/002's own recorded finding, not\n\
```

**Disposition:** applied — the printed verdict now says explicitly that it is
`docs/decisions/002`'s own recorded finding rather than something `stability()`
computed, and narrows to "the three single-producer lock-free paths" with
`direct_mpsc` named as excluded — matching decisions/002's own BN13 correction,
so the two texts agree instead of one silently restating the other's
now-fixed overclaim. What is not fixed: `stability()` still computes only
`contract_ring`'s own spread and prints a sentence about the mutex baseline;
separating "what this run measured" from "what the family's decision record
says" would change the function's output shape, which is beyond what a
documentation-disposition pass edits.
Now prints: `own recorded finding, not`

