# Pattern: A Refusal Is a Row

### Scope

- **Purpose**: Record that a candidate excluded by the workload appears in the result with its reason, never as a missing row, and that this is what makes `Comparison::run` infallible.
- **Responsibility**: State the problem, the structure that solves it, the accounting identity it guarantees, where it applies, and what it costs.
- **In Scope**: `Comparison::refusals`; the accounting identity; the report's refusal lines.
- **Out of Scope**: Which refusals exist (→ [`type/002`](../type/002_run_error.md)); why the producer ceilings are where they are (→ [`pitfall/001`](../pitfall/001_the_door_caps_what_the_structure_does_not.md)).

### Problem

**At four producers, four of six candidates cannot run with the `crossbeam`
feature on — three of five without it.** Only `mutex_queue` and `direct_mpsc`
survive either way. Three obvious responses, two of them wrong:

| Response | What a reader sees |
|---|---|
| Propagate the first refusal as an `Err` | No comparison at all, because one candidate has a ceiling. **The most restrictive candidate vetoes the whole run** |
| Skip the refused candidates | A two-row table. Reads as "these were the candidates", and the reader has no way to know four (three without the `crossbeam` feature) are missing |
| ✅ **Collect them** | A two-row table plus the refusal lines — four with the feature on, three without — saying which paths could not be reached and why |

**The middle one is the trap.** A shorter table is not obviously shorter — a
reader who does not already know there are six candidates sees a complete
comparison of two. And at four producers the refusal list is **the larger and
more interesting half of the output**: two rows of measurement against four
lines saying the in-house ring is unreachable at this producer count by every
route this crate built.

**The count itself was got wrong first.** This document said "three of six"
until the report was read by a human for the first time
(`tests/manual/readme.md` B1) — the automated suite asserts the accounting
identity `outcomes + refusals == ALL`, which holds at any split and therefore
never contradicted the wrong number.

### Solution

```rust
pub struct Comparison
{
  workload : Workload,
  outcomes : Vec< Outcome >,
  refusals : Vec< RunError >,
}

pub fn run( workload : Workload ) -> Self   // ← no Result
```

**`Comparison::run` is infallible because a refusal is data, not an error.** The
loop over `Candidate::ALL` sorts each result into one of the two vectors and
returns unconditionally. A comparison in which *nothing* ran is a `Comparison`
with an empty `outcomes` and a full `refusals` — still a result, and a more
informative one than an `Err` carrying whichever refusal happened to come first.

#### The accounting identity

```text
outcomes().len() + refusals().len() == Candidate::ALL.len()
```

**Every candidate is accounted for exactly once, and it is asserted rather than
assumed.** The identity is what turns "the table looks complete" into a checkable
property: a candidate silently dropped from the loop, or counted twice, breaks
it.

It also survives the cargo feature. `Candidate::ALL` is `const` and the feature
switches which constant, so the identity holds at five and at six without a
runtime count.
→ [`type/001`](../type/001_candidate.md).

#### Rendering

Each refusal gets a line, produced by `RunError`'s own `Display`:

```text
refused: contract_ring admits 1 producer(s), asked for 4
```

**The line is self-explaining** — candidate, ceiling, request — so the refusal
list does not require the reader to know each candidate's bound. That is why
`ProducerCeiling` carries both numbers rather than just the candidate.

### Applicability

| Condition | Why it matters |
|---|---|
| The set of things attempted is **fixed and knowable** | `Candidate::ALL` is a `const` array, so "how many should there be" has an answer the identity can be asserted against. A dynamically-discovered set has nothing to compare a short table to |
| A refusal is **evidence**, not noise | Here the four refusal lines at four producers are the finding. Where a refusal means "not applicable, move on", collecting it is clutter |
| Attempts are **independent** | Each candidate's refusal says nothing about the next one's viability, so there is no reason for the first to veto the rest |
| A partial result is **still useful** | Two measured rows plus four explained absences beat an `Err`. Where a partial result would mislead, propagate instead |

The second row is the discriminator. This pattern's whole value is that it turns
absence into output; where the absence is uninteresting, it buys a longer table
and nothing else.

### Consequences

**A four-producer run is a comparison *and* a statement about the Contract.**
The refusal lines — four with the `crossbeam` feature on, three without — are
the measurement that comparing candidates "under the same producer counts" is
unsatisfiable through one door — stated as output rather than as a paragraph
in a design document.
→ [`pitfall/001`](../pitfall/001_the_door_caps_what_the_structure_does_not.md).

**The cost is that a caller wanting only the successful runs has to filter**,
and a caller that forgets to look at `refusals()` sees the same short table the
skipping design would have produced. Mitigated in the one place it matters —
`report()` prints the refusals — and not otherwise: a type that made the
refusals impossible to ignore (say, an `Either` per candidate) would complicate
every reader of `outcomes()` to protect against a mistake only a report makes.

**This pattern is why `run` is public.** A caller who wants exactly one
candidate gets a `Result` and handles it; a caller who wants the comparison gets
both vectors. Two shapes for two questions, rather than one shape that answers
neither well.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_run_surface.md](../api/001_the_run_surface.md) | The fallible/infallible pair this pattern explains |
| [../api/002_the_report_surface.md](../api/002_the_report_surface.md) | Where the refusal lines are emitted |

### Patterns

| File | Relationship |
|------|--------------|
| [001_the_measurement_is_a_value.md](001_the_measurement_is_a_value.md) | The same instinct applied to what did happen |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_the_door_caps_what_the_structure_does_not.md](../pitfall/001_the_door_caps_what_the_structure_does_not.md) | C1, whose mitigation this pattern is |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_candidate.md](../type/001_candidate.md) | `ALL`, and why the identity survives the cargo feature |
| [../type/002_run_error.md](../type/002_run_error.md) | What a refusal carries, and why both numbers |

### Sources

| File | Relationship |
|------|--------------|
| [`../readme.md`](../readme.md) | "a number for one candidate in isolation says nothing" — which applies equally to a table missing three |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/bench_test.rs`](../../tests/bench_test.rs) | `a_comparison_lists_refusals_rather_than_shortening_the_table` asserts the accounting identity and that every refusal at four producers is a `ProducerCeiling`; `the_report_names_every_candidate_and_every_refusal` asserts the rendering |

### BN39 — The Document That Records Getting This Count Wrong Still Carries the Old Count

The Problem section opens on a number and the Consequences section closes on a
different one, about the same run:

> **At four producers, four of six candidates cannot run**  …  A two-row table
> plus **four lines** saying which paths could not be reached and why

> **The three refusal lines** are the measurement that comparing candidates
> "under the same producer counts" is unsatisfiable through one door

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
printf -- '--- the same document, four places (prose only, before the findings) ---\n'
awk '/^### BN/{ exit } { print NR ":" $0 }' docs/pattern/002_a_refusal_is_a_row.md \
  | command grep -E 'four of six|four lines saying|five and at six|three refusal lines' \
  | sed -E 's/^(.{0,114}).*/\1/' | sed 's/^/  /'
printf -- '--- the two ALL declarations, and their lengths ---\n'
awk '/cfg\( *feature = "crossbeam"|cfg\( *not\( *feature = "crossbeam"/{ tag = $0; sub( /^ */, "", tag ) }
     /pub const ALL/{ inb = 1; c = 0; print "  " tag }
     inb && /Self::/{ c++ }
     inb && /\];/{ print "    entries : " c; inb = 0 }' src/lib.rs
printf -- '--- which candidates carry a ceiling ---\n'
awk '/fn producer_ceiling/, /^  \}/' src/lib.rs | command grep -E 'Self::' | sed 's/^ */  /'
```

Live output:

```
--- the same document, four places (prose only, before the findings) ---
  12:**At four producers, four of six candidates cannot run with the `crossbeam`
  66:switches which constant, so the identity holds at five and at six without a
--- the two ALL declarations, and their lengths ---
  #[ cfg( feature = "crossbeam" ) ]
    entries : 6
  #[ cfg( not( feature = "crossbeam" ) ) ]
    entries : 5
--- which candidates carry a ceiling ---
  Self::MutexQueue | Self::DirectMpsc => None,
  Self::ContractRing | Self::TlsOverRing | Self::DirectSpsc => Some( 1 ),
  Self::OffTheShelf => Some( 1 ),
```

Both numbers are right, for different builds, and the document states which one
neither time. `Candidate::ALL` is declared twice under opposite `cfg`s: six
entries with `--features crossbeam`, five without. Four candidates carry a
producer ceiling of one under the feature and three without it, so a
four-producer run refuses four in one build and three in the other, and the
survivors — `mutex_queue` and `direct_mpsc`, the two with no ceiling — are the
same either way. The Problem section is describing the six-candidate build; the
Consequences section is describing the five-candidate one.

**What makes this a finding is where the document already says all of this.**
The accounting-identity section, between the two, is explicit that both builds
exist and that this is fine: *"the identity holds at five and at six without a
runtime count."* The document knows the count is build-dependent, says so once,
and then hard-codes a different build's answer on either side of the sentence
that says so.

And the Problem section carries its own account of how the number went wrong
before — *"This document said 'three of six' until the report was read by a
human"* — with the correct diagnosis attached: the identity holds at any split,
so no assertion contradicted it. The correction was then applied to the two
sentences a human had just read and not to the third, which is the same failure
one paragraph later, in the paragraph that describes the failure.

The general shape: **a correction driven by reading one build's output lands
only where that output was read.** The record of the correction does not make
the rest of the document a place anyone re-checked; it makes it the place that
looks already-checked.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
# -m2 and no -n: this file is its own subject, so an unbounded match
# also finds this command line and every copy of its own output below,
# and -n re-prefixes a fresh line number onto each earlier pass's output
command grep -m2 'feature on, three without' docs/pattern/002_a_refusal_is_a_row.md
```

Live output:

```
| ✅ **Collect them** | A two-row table plus the refusal lines — four with the feature on, three without — saying which paths could not be reached and why |
The refusal lines — four with the `crossbeam` feature on, three without — are
```

**Disposition:** applied — the Problem section's opening claim, its own table
row, and the Consequences section now all state both counts with their build
attribution (four under `crossbeam`, three without), matching the style the
accounting-identity section already used. The "three of six" self-correction
story in the Problem section is untouched — it is history, not a live claim —
and BN40 (not this crate's finding to disposition) is left as found.
Now prints: `feature on, three without`

### BN40 — The Test Named for This Pattern Passes at Any Refusal Count From One to Four

The document diagnoses precisely why the wrong count survived: the suite asserts
`outcomes + refusals == ALL`, which *"holds at any split and therefore never
contradicted the wrong number."* The implied fix — assert the split — has not
been made, and the test the Tests table names as this pattern's own is the one
that does not make it.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
printf -- '--- every count assertion in the whole suite ---\n'
command grep -E 'outcomes\(\)\.len\(\)|refusals\(\)\.len\(\)' tests/bench_test.rs \
  | sed -E 's/^(.{0,104}).*/\1/' | sed 's/^/  /'
printf '  of those, compared against an integer literal : %s\n' \
  "$( command grep -cE '(outcomes|refusals)\(\)\.len\(\)\s*,\s*[0-9]' tests/bench_test.rs )"
printf -- '--- what the named test pins ---\n'
awk '/fn a_comparison_lists_refusals_rather_than_shortening_the_table/, /^\}/' tests/bench_test.rs \
  | command grep -E 'assert|matches!|for refusal' | sed 's/^ */  /' | sed -E 's/^(.{0,104}).*/\1/'
printf -- '--- candidate variants it names, against the two ALL lengths ---\n'
printf '  named by the test     : %s\n' \
  "$( awk '/fn a_comparison_lists_refusals_rather_than_shortening_the_table/,/^\}/' tests/bench_test.rs \
      | command grep -oE 'Candidate::[A-Za-z]+' | command grep -v 'Candidate::ALL' | sort -u | tr '\n' ' ' )"
printf '  ALL, crossbeam / not  : %s\n' \
  "$( awk '/pub const ALL/{ inb = 1; c = 0 } inb && /Self::/{ c++ } inb && /\];/{ printf "%s ", c; inb = 0 }' src/lib.rs )"
```

Live output:

```
--- every count assertion in the whole suite ---
    assert_eq!( roomy_run.outcomes().len(), Candidate::ALL.len() );
      comparison.outcomes().len() + comparison.refusals().len(),
    assert_eq!( first.outcomes().len(), second.outcomes().len() );
  of those, compared against an integer literal : 0
--- what the named test pins ---
  assert!( ran.contains( &Candidate::MutexQueue ) );
  assert!( ran.contains( &Candidate::DirectMpsc ) );
  assert!( !ran.contains( &Candidate::ContractRing ) );
  assert_eq!
  for refusal in comparison.refusals()
  assert!
  matches!( refusal, RunError::ProducerCeiling { requested : 4, ceiling : 1, .. } ),
--- candidate variants it names, against the two ALL lengths ---
  named by the test     : Candidate::ContractRing Candidate::DirectMpsc Candidate::MutexQueue 
  ALL, crossbeam / not  : 6 5 
```

Three count assertions exist in the file and none compares to a literal: two
compare to `Candidate::ALL.len()` and one compares two runs to each other. The
suite is build-agnostic by construction — correct for the identity, and the
reason the identity is worth asserting — but it means no test anywhere knows how
many refusals a four-producer run should produce.

The named test then pins less than its name suggests. It asserts that
`mutex_queue` and `direct_mpsc` ran, that `contract_ring` did not, the identity,
and that every refusal it *does* find is a `ProducerCeiling { requested : 4,
ceiling : 1 }`. `tls_over_ring`, `direct_spsc` and `off_the_shelf` are named
nowhere in it. Each is free to run or be refused, and the trailing `for refusal`
loop is a per-item shape check that iterates whatever is there — so the test
holds for one refusal, or two, or three, or four.

**The failure mode it is named against is the one it does not exclude.** "Lists
refusals rather than shortening the table" is a claim about a count; the test
establishes that the table is not *empty* of refusals and that the refusals
present are well-formed. A change that gave `tls_over_ring` no ceiling would move
it from the refusal list into the outcome table, shorten the refusal list by one,
and pass — which is the shortening, arriving by the one route the identity cannot
see, in the test written to prevent it.

