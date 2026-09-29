# API: The Report Surface

### Scope

- **Purpose**: Define the one rendering operation, and record the three things it states rather than omits.
- **Responsibility**: State the signature, the layout, and the absence-reporting rules.
- **In Scope**: `Comparison::report`; its columns; the refusal lines; the `fastest lossless: none` line.
- **Out of Scope**: What the numbers mean (→ [`data_structure/002`](../data_structure/002_three_counts_that_are_not_interchangeable.md)); machine-readable output, which does not exist.

### Abstract

One rendering operation, returning a `String` and printing nothing. Its content
is defined by what it refuses to leave out: the overflow policy, the refused
candidates, and the case where no candidate is eligible to win.

### Operations

```rust
pub fn report( &self ) -> String
```

**Returns a `String`; prints nothing.** The caller decides where it goes — a
test asserts on substrings of it, a binary prints it, a future harness could
write it to a file. A `report` that printed would be untestable without
capturing stdout, and the one assertion this crate can make about its output is
that the output *contains* what it claims to.
→ [`pattern/001`](../pattern/001_the_measurement_is_a_value.md).

#### Layout

```text
1 producer(s) x 256 records, batch 32, capacity 4096, overflow DropNewest
candidate          offered  reported  received  dropped   silent     write ns
mutex_queue            256       256       256        0        0        41231
contract_ring          256       256       256        0        0        18904
…
refused: contract_ring admits 1 producer(s), asked for 4
fastest lossless: direct_spsc
```

| Element | Rule |
|---|---|
| Header line | Every field of the workload, **including the overflow policy** |
| One row per outcome | In `Candidate::ALL` order, never sorted by time |
| One line per refusal | The `Display` of the `RunError`, which names the candidate and both numbers |
| Trailing verdict | The fastest lossless candidate, **or an explicit statement that there is none** |

#### Three absences are stated, not omitted

**1. The overflow policy is in the header.** It is the field that decides
whether `reported` and `received` can diverge, so a table read without it is a
table whose `silent` column has no explanation. It is also the field a caller is
least likely to have set deliberately — `RingConfig::new( n )` supplies
`DropNewest` silently — which is exactly why the report says it out loud.
→ [`pitfall/003`](../pitfall/003_ok_is_not_kept_and_the_verdict_inverts.md).

**2. A refused candidate gets a line, not a missing row.** A shorter table looks
like a comparison of fewer candidates; a table plus a refusal list says which
paths could not be reached at this producer count and why. At four producers
that list is the more interesting half of the output.
→ [`pattern/002`](../pattern/002_a_refusal_is_a_row.md).

**3. `fastest lossless: none — every candidate dropped records`** is printed
when nothing is eligible. An omitted line reads as a formatting gap; a stated
absence reads as a finding, and on a workload larger than the capacity it is the
finding.

**What is not stated: per-row eligibility.** The row loop renders every
outcome's `write_nanos()` in `Candidate::ALL` order with no reference to
`Outcome::is_lossless` — an ineligible candidate's row is identical in shape
to an eligible one, same columns, same time, no marker. The only place
eligibility appears is the trailing verdict line, so on a workload where
every candidate is ineligible the table shows a column of times with nothing
in it saying none of them count (→ BN7).

#### The `silent` column

`silently_discarded` — `reported - received` — gets its own column rather than
being folded into `dropped`. The two answer different questions: `dropped` is
how many records the workload lost, `silent` is how many the *path claimed to
have taken* and then did not have. Under `OverflowPolicy::Fail` the column is
zeroes and costs eight characters; under the default it is the difference
between a path that applies back-pressure and one that absorbs.

### Error Handling

**`report` cannot fail.** It takes `&self`, allocates a `String`, and returns it;
there is no `Result`, no panic path, and no configuration it can be handed that
it will refuse. Everything that *could* have failed already did so before a
`Comparison` existed.

| Condition | How it appears in the output |
|---|---|
| A candidate was refused | A `refused:` line, rendered by `RunError`'s own `Display` — the error was captured at run time and is data by the time this function sees it |
| No candidate is eligible to win | The explicit `fastest lossless: none` line, not an omission and not an error |
| Nothing ran at all | A header, a column heading, and a full refusal list. Still a valid report |

The one hazard is on the reading side rather than the writing side: a caller who
asserts on substrings is coupled to the exact wording, and the four assertions in
`the_report_names_every_candidate_and_every_refusal` are the record of which
strings are load-bearing.

### Compatibility Guarantees

**Not machine-readable.** No CSV, no JSON, no serialisation. This crate's
deliverable is a comparison a human reads and acts on, and every field behind
the report is already a public accessor — a consumer who wants structured output
has `Comparison::outcomes()` and can shape it however their tooling needs.
Adding a second output format would be a second thing to keep in step with the
first.

**Not a ranking.** The rows are in candidate order. The only ordering computed
anywhere is `fastest()`'s single minimum, and that is filtered first.
→ [`decisions/002`](../decisions/002_no_test_asserts_an_ordering.md).

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_the_eligibility_filter_runs_before_the_comparison.md](../algorithm/002_the_eligibility_filter_runs_before_the_comparison.md) | The trailing verdict line's source, including the `None` case |

### APIs

| File | Relationship |
|------|--------------|
| [001_the_run_surface.md](001_the_run_surface.md) | The accessors this reads |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_three_counts_that_are_not_interchangeable.md](../data_structure/002_three_counts_that_are_not_interchangeable.md) | The six numeric columns |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_the_measurement_is_a_value.md](../pattern/001_the_measurement_is_a_value.md) | Why this returns rather than prints |
| [../pattern/002_a_refusal_is_a_row.md](../pattern/002_a_refusal_is_a_row.md) | Absence 2 |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_run_error.md](../type/002_run_error.md) | The `Display` impl each refusal line is produced by |

### Sources

| File | Relationship |
|------|--------------|
| [`../readme.md`](../readme.md) | "The comparison is the deliverable" — this function is that deliverable's shape |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/bench_test.rs`](../../tests/bench_test.rs) | `the_report_names_every_candidate_and_every_refusal` asserts every candidate name appears, the refusal line appears at four producers, the header carries the capacity and the producer count, and the `none` verdict appears on the cramped fixture |

### BN7 — The Table Renders the Layout the Crate Rejected, and the Filter Reaches Only the Last Line

`fastest()` filters before it minimises. `report()` does not filter at all:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
echo '  -- does the row loop know about eligibility? --'
awk '/pub fn report\( &self \) -> String/, /^  \}/' src/lib.rs \
  | command grep -E 'is_lossless|fastest\(\)|for outcome|write_nanos' | sed 's/^/    /'
echo '  -- every assertion the report test makes --'
awk '/fn the_report_names_every_candidate_and_every_refusal/, /^\}/' tests/bench_test.rs \
  | command grep -oE 'contains\( "[^"]*" \)|contains\( candidate.name\(\) \)' | sed 's/^/    /'
```

Live output:

```
  -- does the row loop know about eligibility? --
        for outcome in &self.outcomes
            outcome.write_nanos(),
        match self.fastest()
  -- every assertion the report test makes --
    contains( candidate.name() )
    contains( "fastest lossless: " )
    contains( "capacity 4096" )
    contains( "every candidate dropped records" )
    contains( "refused: contract_ring" )
    contains( "4 producer(s) x 256 records" )
    contains( "refused: contract_ring: " )
    contains( "refused: tls_over_ring: " )
```

The row loop is `for outcome in &self.outcomes` and it renders `write_nanos()`
for every one of them. `is_lossless` appears nowhere in the function; the only
call to `fastest()` is at line 44 of it, producing the single trailing verdict
line. **So an ineligible candidate gets a full row, with its time, unmarked, and
the reader learns it was ineligible from a sentence forty characters below the
number.**

[`algorithm/002`](../algorithm/002_the_eligibility_filter_runs_before_the_comparison.md)
enumerates the three orderings this crate rejected. The third is *"minimise on
time and let the reader consult the `dropped` column"*, and its verdict on that
option is: **"This is the failure the crate was built with. The column was right
and the ranking was read anyway."** That sentence describes this table. The
`dropped` and `silent` columns are right, the `write ns` column is right, and
nothing marks which rows the crate itself considers uncomparable.

On the cramped fixture the effect is total: every row is ineligible, every row
still shows a time, and the fastest number in the column belongs to whichever
path discarded records most efficiently. **The verdict line says none is
eligible; the table above it does not.**

The suite cannot see this. Its six assertions — listed above — are all substring
checks on names, on the header, on the refusal line and on the verdict line.
**Not one reads a number out of the table body**, so the rendering of the column
the crate exists to produce is asserted by nothing.

The fix is a column or a marker, not a redesign: the data is already there, and
`Outcome::is_lossless` is already public. The general shape, and the reason this
is worth a finding rather than a nitpick: **a rule enforced in the API and not in
the rendering is enforced against the caller who reads the accessors and not
against the caller who reads the output** — and for this crate the output *is*
the deliverable.

```sh
cd "$(git rev-parse --show-toplevel)"
# -m1 and no -n: this file is its own subject, so an unbounded match
# also finds this command line and every copy of its own output below,
# and -n re-prefixes a fresh line number onto each earlier pass's output
command grep -m1 'is not stated: per-row eligibility' ring_bench/docs/api/002_the_report_surface.md
```

Live output:

```
**What is not stated: per-row eligibility.** The row loop renders every
```

**Disposition:** declined — a code change (a column or marker keyed off
`Outcome::is_lossless`) is the fix this finding itself names, and implementing
new report output is beyond this documentation-disposition pass's own scope
absent explicit user authorization. What is applied: the doc's own "stated,
not omitted" section no longer implies three absences is the complete list —
it now names this fourth, unstated one directly, so the doc itself is no
longer misleading about its own completeness.
Now prints: `is not stated: per-row eligibility`

### BN8 — The Header Prints a Batch and a Capacity That `RingConfig` Says Cannot Coexist

The header line reads two of its five fields through different paths:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
echo '  -- the header line, and the accessors it mixes --'
awk '/producer\(s\) x \{\} records, batch/, /self.workload.config\(\).overflow\(\),/' src/lib.rs | sed 's/^/    /'
echo '  -- both read the config --'
awk '/pub fn capacity\( &self \) -> usize/, /^  \}/' src/lib.rs | sed 's/^/    /'
awk '/pub const fn batch\( &self \) -> usize/, /^  \}/' src/lib.rs | sed 's/^/    /'
echo '  -- RingConfig on what those two fields may be --'
command grep 'always between one and the capacity inclusive' ../ring_config/src/lib.rs | sed 's/^/    /'
echo '  -- what the header prints, per fixture --'
awk '
  /^fn (roomy|cramped|parallel)\(/ { n = $2; sub( /\(.*/, "", n ); cap = 0; b = 32; next }
  n != "" && /RingConfig::new\(/ { cap = $0; sub( /.*RingConfig::new\( /, "", cap ); sub( / \).*/, "", cap ) }
  n != "" && /with_batch\(/      { b   = $0; sub( /.*with_batch\( /, "", b );        sub( / \).*/, "", b ) }
  n != "" && /^\}/ { c = ( b > cap ? cap : b )
                     printf "    %-9s header: \"batch %s, capacity %s\"   asked for batch %s   %s\n", n, c, cap, b, ( b == c ? "" : "<-- clamped before the header sees it" )
                     n = "" }
' tests/bench_test.rs
```

Live output:

```
  -- the header line, and the accessors it mixes --
          "{} producer(s) x {} records, batch {}, capacity {}, overflow {:?}",
          self.workload.producers(),
          self.workload.records_per_producer(),
          self.workload.batch(),
          self.workload.capacity(),
          self.workload.config().overflow(),
  -- both read the config --
      pub fn capacity( &self ) -> usize
      {
        self.config.capacity().get()
      }
      pub const fn batch( &self ) -> usize
      {
        self.config.batch()
      }
  -- RingConfig on what those two fields may be --
      /// The batch size, always between one and the capacity inclusive.
  -- what the header prints, per fixture --
    roomy     header: "batch 32, capacity 4096"   asked for batch 32   
    cramped   header: "batch 16, capacity 16"   asked for batch 32   <-- clamped before the header sees it
    parallel  header: "batch 32, capacity 4096"   asked for batch 32   
```

`capacity` is `self.config.capacity().get()` — read through the config, after
validation. `batch` **was** `self.batch` — a `Workload` field of its own, which
`with_batch` wrote unclamped. The header put the two side by side.

`RingConfig::batch`'s own doc says the field is **"always between one and the
capacity inclusive"**, and `RingConfig::with_batch` enforces it by clamping. So
the pair the header prints for the cramped fixture — `batch 32, capacity 16` —
was a configuration `RingConfig` will not hold. The report stated it anyway,
because it did not read the batch from the config.

**This was harmless for a reason that was itself a finding**
(→ [`data_structure/001`](../data_structure/001_the_workload_description.md)):
the clamped copy had no reader. Every runner used `workload.batch()`, so 32 was
what actually happened and the header described the run correctly. The config's
16 was written, carried to the builder, and consulted by nobody.

Which left the report making a true statement in a form its own configuration
type declares invalid, and a second number travelling alongside it that would
contradict it if anything ever looked. The suite asserted `capacity 4096` on the
roomy fixture and `4 producer(s) x 256 records` on the parallel one; it made no
assertion about the header on the cramped fixture, which was the only one where
the two numbers disagreed.

The general shape: **a struct that mirrors a field of another struct owns a
second copy with its own rules**, and the moment one of the two applies a
correction the other does not, every consumer has to know which copy it is
holding. Here there were three consumers and they did not agree — the runners took
the unclamped one, the builder took the clamped one, and the header printed the
unclamped one next to a field it read from the clamped one.

**Disposition:** applied — the mirror is gone. `Workload` no longer holds a
`batch` field; `Workload::batch()` returns `self.config.batch()`, so the three
consumers now read one value that has been through `RingConfig::with_batch`'s
clamp. The header prints a pair its own configuration type will hold, and it
prints it for the cramped fixture without special-casing anything. The change is
not cosmetic: because every runner reads `batch()`, the staged candidate was
publishing a 32-record batch into a 16-slot ring, failing `Flusher`'s
`free_capacity` pre-check on the first drive, and stalling with a permanently
full buffer — `a_cramped_run_drops_and_the_drop_is_counted_from_the_drain`
asserted `reported() == 0` and read that stall as a measurement. With the clamped
16 one flush fits exactly; that test now asserts 16 and says why it changed.
`the_batch_reported_is_the_batch_the_config_carries` pins the tie on all three
fixtures including `cramped`, the one that could break it, and was proven able to
fail by making `batch()` return a constant 32. What this does not buy: the header
is still a `format!` string with no assertion on the cramped fixture's rendering,
so a future field added to it can still go unchecked. Now prints: `cramped   header: "batch 16, capacity 16"`
