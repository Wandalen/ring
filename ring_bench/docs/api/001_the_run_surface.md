# API: The Run Surface

### Scope

- **Purpose**: Define the two entry points — one candidate at a time, or all of them — and record why the second is infallible while the first is not.
- **Responsibility**: State the signatures, the failure modes, the accessors, and the ordering guarantees.
- **In Scope**: `run`, `Comparison::run`, and the accessors on both result types.
- **Out of Scope**: Rendering (→ [`api/002`](002_the_report_surface.md)); what happens inside a run (→ [`algorithm/001`](../algorithm/001_one_workload_through_six_runners.md)).

### Abstract

Two entry points: one candidate at a time, returning a `Result`, and all of them
at once, returning a `Comparison` that cannot fail. The asymmetry is the whole
design — a single candidate can be refused, but a comparison reports refusals
rather than suffering them.

### Operations

```rust
pub fn run( candidate : Candidate, workload : &Workload ) -> Result< Outcome, RunError >;

impl Comparison
{
  pub fn run( workload : Workload ) -> Self;   // ← infallible
}
```

**One is fallible and the other is not, and the asymmetry is the design.** A
single candidate can be refused — by its own producer ceiling, or by a
dependency rejecting the configuration. A comparison cannot be refused, because
a refusal is one of the things it reports:

```rust
pub struct Comparison { workload : Workload, outcomes : Vec< Outcome >, refusals : Vec< RunError > }
```

**Every candidate in `Candidate::ALL` lands in exactly one of the two vectors**,
so `outcomes().len() + refusals().len() == Candidate::ALL.len()` always. A
comparison in which nothing could run is a `Comparison` with an empty
`outcomes` and a full `refusals` — still a result, and a more informative one
than an `Err`.
→ [`pattern/002`](../pattern/002_a_refusal_is_a_row.md).

### Error Handling

| Variant | Raised by | When |
|---|---|---|
| `ProducerCeiling` | this crate | The workload's producer count exceeds the candidate's ceiling. Checked **before** anything is built, so a refusal costs no allocation |
| `Build` | `ring_factory` | The configuration is refused by the factory — currently `OverflowPolicy::DropOldest` |
| `Ring` | `ring_core` | The same refusal, reached by the staged candidate, which builds its ring below the factory |
| `Flush` | `ring_flush` | The flush policy is refused. **Currently unreachable** — see below |

**`Build` and `Ring` being separate variants is not redundancy.** They are two
routes to the same refusal, and the variant says which route: through the
factory it arrives as `Build`, and the staged candidate — which cannot use the
factory, because `Flusher::new` needs a `ring_core::Producer` the Contract
cannot hand it — arrives as `Ring`. The distinction is asserted rather than
noted, so the day the Contract composes, a test changes.
→ [`integration/001`](../integration/001_declared_edges_and_the_three_that_were_missing.md).

**`Flush` is unreachable and deliberately kept.** `run_tls_over_ring` builds its
`TlsBuffer` with `workload.batch()` slots and binds
`FlushPolicy::OnBatch( workload.batch() )`, so both of `ring_flush`'s binding
refusals — a zero batch, and a batch above the buffer's capacity — are excluded
by construction: `Workload` refuses a zero batch, and `n > n` is false. The
variant stays because the tie is one edit from being broken and nothing in
either type enforces it; replacing it with an `expect` would turn a future
configuration mistake into a panic inside a measurement.

### Accessors

**On `Outcome`** — every field is read-only and every derived quantity is a
method, so the three counts cannot be recombined by a caller into a judgement
this crate does not make:

`candidate`, `producers`, `offered`, `reported`, `received`, `dropped`,
`silently_discarded`, `write_nanos`, `stats`, `is_lossless`, `conserved`.

**On `Comparison`** — `workload`, `outcomes`, `refusals`, `conserved`,
`silently_discarded`, `fastest`, `report`.

`Comparison::conserved` and `Comparison::silently_discarded` aggregate their
per-outcome namesakes. They exist at this grain because **a mixed run is the
dangerous one**: some candidates report their drops and some absorb them, so a
ranking read off the API's own count is ordered wrong rather than merely
imprecise. One boolean at the comparison level says whether that hazard is
present in this table.

### Ordering

`Candidate::ALL` is a fixed-order constant and `Comparison::run` iterates it, so
`outcomes` is in candidate order and two runs of the same workload produce
tables whose rows correspond. Nothing sorts by time — the report is a table, not
a ranking, and `fastest()` is the only place an ordering is computed at all.

### Compatibility Guarantees

| Guarantee | Holds because |
|---|---|
| `outcomes().len() + refusals().len() == Candidate::ALL.len()` | `Comparison::run` sorts every candidate into exactly one vector and returns unconditionally |
| Row order is stable across runs of the same workload | `Candidate::ALL` is a fixed-order `const` and nothing sorts by time |
| The identity survives the cargo feature | The feature switches which `const` array `ALL` is, so the count is right at five and at six without a runtime check |
| No `#[ non_exhaustive ]` anywhere on this surface | `publish = false` in a workspace that builds as a unit — there is no out-of-tree consumer the guarantee could protect, and it would cost the ability to construct the unreachable `Flush` variant in a test |

#### The re-export that is not here

**This crate does not re-export `RingConfig`**, even though every `Workload`
needs one. `ring_factory` already re-exports it, this crate depends on
`ring_factory`, and a consumer of `ring_bench` is by definition inside the
workspace — there is no Contract-bound caller who would be unable to name it. A
second re-export would be a second place for the same name to arrive from.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_one_workload_through_six_runners.md](../algorithm/001_one_workload_through_six_runners.md) | What `run` does between the ceiling check and the `Outcome` |
| [../algorithm/002_the_eligibility_filter_runs_before_the_comparison.md](../algorithm/002_the_eligibility_filter_runs_before_the_comparison.md) | `fastest`'s two steps |

### APIs

| File | Relationship |
|------|--------------|
| [002_the_report_surface.md](002_the_report_surface.md) | The other surface, which reads these accessors |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_workload_description.md](../data_structure/001_the_workload_description.md) | The argument |
| [../data_structure/002_three_counts_that_are_not_interchangeable.md](../data_structure/002_three_counts_that_are_not_interchangeable.md) | The result |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_candidate.md](../type/001_candidate.md) | `Candidate::ALL`, and the ceiling `run` checks first |
| [../type/002_run_error.md](../type/002_run_error.md) | The four refusals, one of which is unreachable |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_a_refusal_is_a_row.md](../pattern/002_a_refusal_is_a_row.md) | Why `Comparison::run` is infallible |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_flush/src/lib.rs`](../../../ring_flush/src/lib.rs) | `ConfigError`'s two variants, and `Flusher::new`'s argument list |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/bench_test.rs`](../../tests/bench_test.rs) | `a_comparison_lists_refusals_rather_than_shortening_the_table` asserts the accounting identity; `a_policy_refusal_names_the_crate_that_refused` asserts the `Build`/`Ring` distinction; `the_flush_relay_is_unreachable_while_the_batch_ties_the_buffer` documents and constructs the unreachable variant |

### BN5 — "Accounted For Exactly Once" Is Checked as a Sum of Two Lengths

The Compatibility Guarantee above is asserted, and the assertion is weaker than
the sentence it carries:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
echo '  -- the assertion, and what it compares --'
awk '/fn a_comparison_lists_refusals_rather_than_shortening_the_table/, /^\}/' tests/bench_test.rs \
  | command grep -E 'outcomes\(\)|refusals\(\)|ALL.len|accounted' | sed 's/^/    /'
echo '  -- how many hand-written literals that constant has --'
printf '    declarations of ALL : %s\n' "$( command grep -c 'pub const ALL' src/lib.rs )"
echo '  -- and how the two vectors are filled --'
awk '/pub fn run\( workload : Workload \) -> Self/, /^  \}/' src/lib.rs | sed 's/^/    /'
```

Live output:

```
  -- the assertion, and what it compares --
      let ran : Vec< Candidate > = comparison.outcomes().iter().map( Outcome::candidate ).collect();
        comparison.outcomes().len() + comparison.refusals().len(),
        Candidate::ALL.len(),
        "every candidate is accounted for exactly once",
      for refusal in comparison.refusals()
  -- how many hand-written literals that constant has --
    declarations of ALL : 2
  -- and how the two vectors are filled --
      pub fn run( workload : Workload ) -> Self
      {
        let mut outcomes = Vec::new();
        let mut refusals = Vec::new();
    
        for candidate in Candidate::ALL
        {
          match run( *candidate, &workload )
          {
            Ok( outcome ) => outcomes.push( outcome ),
            Err( refusal ) => refusals.push( refusal ),
          }
        }
    
        Self { workload, outcomes, refusals }
      }
```

`outcomes().len() + refusals().len() == Candidate::ALL.len()` is a statement
about two totals. The message attached to it — "every candidate is accounted for
exactly once" — is a statement about multiplicity, and no sum of two lengths can
see multiplicity. A `Candidate::ALL` that listed `DirectSpsc` twice would produce
two outcomes for it, a length of six against a constant of six, and the
assertion would pass while reporting the same candidate on two rows.

**That is not an idle hypothetical, because `ALL` is not one list.** It is two
hand-written literals under opposite `cfg`s with no assertion that they agree
(→ [`workaround/002`](../workaround/002_a_feature_that_cannot_be_negated_at_the_use_site.md)'s
BN52). The `len()` on the right-hand side is read from whichever literal the
build selected, so a duplicate introduced into either copy moves both sides of
the comparison by one. **The constant is not an independent check on the
computation; it is the same source the computation walked.**

The three `ran.contains( .. )` lines above it are membership tests, which have
the same blind spot for the same reason — `contains` on a `Vec` says at least
one, never exactly one.

Making it say what it means costs one line: collect the candidates from both
vectors, sort, and compare against a deduplicated `ALL`. The general shape:
**an accounting identity checked by counting cannot distinguish a partition from
a multiset**, and the message on the assertion is where that difference gets
claimed anyway.

### BN6 — The Test Named for the Unreachable Variant Compares a Number Neither Side Uses

`Flush` is documented as unreachable because the buffer's capacity and the flush
trigger are the same number. The test named for that tie compares two other
numbers:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
echo '  -- the two lines the tie actually lives on --'
command grep -n 'TlsBuffer::< Record >::with_capacity\|FlushPolicy::OnBatch' src/lib.rs \
  | command grep -v '///' | sed 's/^/    /' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
echo '  -- what the test compares instead --'
awk '/fn the_flush_relay_is_unreachable_while_the_batch_ties_the_buffer/, /^\}/' tests/bench_test.rs \
  | command grep -nE 'roomy|workload\.|same number' | sed 's/^/    /' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
echo '  -- RingConfig::with_batch clamps to capacity; Workload::with_batch does not --'
command grep -n 'let capped = if batch >' ../ring_config/src/lib.rs | sed 's/^/    ring_config  /' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
command grep -n 'self.batch = batch;' src/lib.rs | sed 's/^/    ring_bench   /' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
echo '  -- so, per fixture --'
awk '
  /^fn (roomy|cramped|parallel)\(/ { n = $2; sub( /\(.*/, "", n ); cap = 0; b = 32; next }
  n != "" && /RingConfig::new\(/ { cap = $0; sub( /.*RingConfig::new\( /, "", cap ); sub( / \).*/, "", cap ) }
  n != "" && /with_batch\(/      { b   = $0; sub( /.*with_batch\( /, "", b );        sub( / \).*/, "", b ) }
  n != "" && /^\}/ { c = ( b > cap ? cap : b )
                     printf "    %-9s capacity %-5s batch() = %-4s config().batch() = %-4s %s\n", n, cap, b, c, ( b == c ? "equal" : "DIVERGENT" )
                     n = "" }
' tests/bench_test.rs
```

Live output:

```
  -- the two lines the tie actually lives on --
      let buffer = TlsBuffer::< Record >::with_capacity( workload.batch() );
      let mut flusher = Flusher::new( buffer, producer, FlushPolicy::OnBatch( workload.batch() ) )
  -- what the test compares instead --
      let workload = roomy();
        workload.batch(),
        workload.config().batch(),
        "the buffer's capacity and the flush trigger are the same number",
  -- RingConfig::with_batch clamps to capacity; Workload::with_batch does not --
    ring_config  142:    let capped = if batch > self.capacity.get() { self.capacity.get() } else { batch };
  -- so, per fixture --
    roomy     capacity 4096  batch() = 32   config().batch() = 32   equal
    cramped   capacity 16    batch() = 32   config().batch() = 16   DIVERGENT
    parallel  capacity 4096  batch() = 32   config().batch() = 32   equal
```

**The tie is between lines 780 and 781, and it is the same expression written
twice.** `TlsBuffer::with_capacity( workload.batch() )` and
`FlushPolicy::OnBatch( workload.batch() )` are three tokens apart on adjacent
lines; `n > n` is false, so `ring_flush`'s capacity refusal cannot fire.

The test asserts `workload.batch() == workload.config().batch()` — and
`workload.config().batch()` is a **third** number, read by neither line. It is
the clamped copy `Workload::with_batch` writes into the config as a side effect,
and no runner ever reads it. So the assertion whose message says "the buffer's
capacity and the flush trigger are the same number" reads neither the buffer's
capacity nor the flush trigger.

**And it passes because of which fixture it was handed.** `roomy()` has 4096
slots, so the clamp is inert and the two numbers agree. `cramped()` has 16 slots
and a batch of 32, so `config().batch()` is 16 while `batch()` is 32 — this
assertion fails on a fixture defined eleven lines below it in the same file, and
its failure message would report a tie that is still, in fact, intact.

The variant is correctly kept and the reasoning in the Error Handling section
above is correct. What is not correct is that the crate believes it has a
regression test for the tie. It has a test that will break when an unrelated
clamp starts applying, and will not break when the tie it names is broken —
because breaking the tie means editing one of two adjacent occurrences of
`workload.batch()`, and nothing in the suite reads either of them.

The general shape: **a test named after an invariant is not a test of that
invariant**, and where the invariant is "these two expressions are identical",
the only thing that can check it is a comparison of those two expressions.
