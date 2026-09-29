# item

Forty-seven declarations: seven nouns and forty verbs. The catalogue splits
on that line, and both halves report the same thing from opposite ends — **this
crate's public surface is overwhelmingly a way of reading a result that has
already happened.**

Twenty-five of the forty verbs are `const fn`. Only six functions in the
whole crate run a candidate, and only two of those six spawn a thread.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Seven Nouns, and the One That Is Copied but Never Compared](001_seven_nouns_and_the_one_never_compared.md) | BN25, BN26 — what the vocabulary can and cannot be asked, and whose derive lists this crate's own depend on |
| 002 | [Forty Verbs, Twenty-Five of Them `const`](002_forty_verbs_twenty_five_of_them_const.md) | BN27, BN28 — the accessor half, and the one doc sentence that describes all six runners and is true of four |

### The Forty-Seven, by Kind

The forty-seven are the seven `pub struct`/`pub enum`/`pub type` and the
forty `pub fn`, inherent or free. The two `Display` impls and the two
`Error` impls are reachable public behaviour but are trait methods rather than
declarations of this crate's own, so they close the table as four rows rather
than counting toward the total.

| Kind | Declaration | Derives / shape | Notes |
|------|-------------|-----------------|-------|
| noun | `pub type Record = u64` | inherited | An alias, not a type. The only noun this crate does not define |
| noun | `pub enum WorkloadError` | `Debug, Clone, Copy, PartialEq, Eq` | 4 unit variants, one per rejected dimension |
| noun | `pub struct Workload` | `Debug, Clone, Copy` | 4 private fields. **The only noun that is `Copy` without `PartialEq`** |
| noun | `pub enum Candidate` | `Debug, Clone, Copy, PartialEq, Eq` | 5 unit variants; 6 with `crossbeam` |
| noun | `pub struct Outcome` | `Debug` | 7 private fields, one of them a `RingStats` |
| noun | `pub enum RunError` | `Debug, Clone, Copy, PartialEq, Eq` | 4 variants; 3 of them wrap another crate's error |
| noun | `pub struct Comparison` | `Debug` | 3 private fields, two of them `Vec` |
| verb | `Workload::new` | `const` → `Self` | The only constructor that cannot fail |
| verb | `Workload::with_producers` | → `Result< Self, WorkloadError >` | Sets `producers` **and** `config`, so the two cannot diverge |
| verb | `Workload::with_records_per_producer` | → `Result< Self, WorkloadError >` | Touches one field |
| verb | `Workload::with_batch` | → `Result< Self, WorkloadError >` | Sets `batch` **and** `config.batch` |
| verb | `Workload::with_cells` | → `Result< Self, WorkloadError >` | Errors on zero; `producer % cells` picks the destination |
| verb | `Workload::with_semantics` | `const` → `Self` | The only `with_*` setter that cannot fail |
| verb | `Workload::config` | `const` → `RingConfig` | By `Copy` |
| verb | `Workload::producers` | `const` → `usize` | |
| verb | `Workload::records_per_producer` | `const` → `usize` | |
| verb | `Workload::cells` | `const` → `usize` | How many destination cells drained records fold into |
| verb | `Workload::semantics` | `const` → `AccumulatorSemantics` | How a repeated write to the same cell resolves |
| verb | `Workload::batch` | `const` → `usize` | Read by three of the five default runners |
| verb | `Workload::capacity` | → `usize` | Forwards to `RingConfig::capacity` |
| verb | `Workload::offered` | `const` → `usize` | `producers * records_per_producer` |
| verb | `Workload::records_of` | → `Range< Record >` | The only verb producing an iterator |
| verb | `Candidate::name` | `const` → `&'static str` | The identity every report line is keyed on |
| verb | `Candidate::producer_ceiling` | `const` → `Option< usize >` | Carries a six-row doc table |
| verb | `Candidate::admits` | `const` → `bool` | Derived from `producer_ceiling`, so the two cannot disagree |
| verb | `AccumulatorTable::cells` | → `&[ i64 ]` | The accumulated value in each cell, in cell order |
| verb | `Outcome::candidate` | `const` → `Candidate` | |
| verb | `Outcome::producers` | `const` → `usize` | One of the workload's four dimensions, kept |
| verb | `Outcome::offered` | `const` → `usize` | The second kept |
| verb | `Outcome::reported` | `const` → `usize` | What the write API said |
| verb | `Outcome::received` | `const` → `usize` | What the drain produced |
| verb | `Outcome::dropped` | `const` → `usize` | `offered - received` |
| verb | `Outcome::silently_discarded` | `const` → `usize` | `reported - received` |
| verb | `Outcome::write_nanos` | `const` → `u128` | The one value the docs say never to assert on |
| verb | `Outcome::stats` | `const` → `&RingStats` | The only borrow of a dependency's type |
| verb | `Outcome::table` | `const` → `&AccumulatorTable` | Built once, after the drain, by replaying every record through `Workload::semantics` |
| verb | `Outcome::is_lossless` | `const` → `bool` | `received == offered` |
| verb | `Outcome::conserved` | `const` → `bool` | `reported == received` |
| verb | `run` | → `Result< Outcome, RunError >` | Free. The only verb that dispatches on `Candidate` |
| verb | `Comparison::run` | → `Self` | Free `run` over `Candidate::ALL`, sorting into two vectors |
| verb | `Comparison::workload` | `const` → `&Workload` | The only way back to the four dimensions |
| verb | `Comparison::outcomes` | → `&[ Outcome ]` | |
| verb | `Comparison::refusals` | → `&[ RunError ]` | **No candidate column** — see [`001`](001_seven_nouns_and_the_one_never_compared.md) |
| verb | `Comparison::conserved` | → `bool` | Folds `Outcome::conserved` |
| verb | `Comparison::silently_discarded` | → `usize` | Sums `Outcome::silently_discarded` |
| verb | `Comparison::fastest` | → `Option< &Outcome >` | Filter then `min_by_key`; the crate's only ordering |
| verb | `Comparison::report` | → `String` | The one verb producing something a human reads |
| trait | `WorkloadError::fmt` | `impl Display` | |
| trait | `WorkloadError` | `impl core::error::Error` | Empty body — no `source` |
| trait | `RunError::fmt` | `impl Display` | Forwards to the inner error for three of four variants |
| trait | `RunError` | `impl core::error::Error` | Empty body — no `source`, so the three wrapped errors do not chain |

**Both `Error` impls are empty**, so a `RunError::Build( … )` renders its inner
message and then the chain stops — a consumer walking `source()` never reaches
the `ring_factory` error it is holding.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
printf 'nouns (struct/enum/type): %s\n' "$( command grep -cE '^pub (struct|enum|type) ' src/lib.rs )"
printf 'public verbs:             %s\n' "$( command grep -cE '^ +pub (const )?fn |^pub fn ' src/lib.rs )"
printf '  of them const:          %s\n' "$( command grep -cE '^ +pub const fn ' src/lib.rs )"
printf 'private fns:              %s\n' "$( command grep -cE '^fn ' src/lib.rs )"
printf 'impl blocks:              %s\n' "$( command grep -cE '^impl ' src/lib.rs )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BN25 | `Comparison::refusals` | **latent hazard** | Three of `RunError`'s four variants carry no candidate, so a refusal that is not a producer-ceiling refusal cannot be attributed — and the test whose name says the report names every refusal only ever constructs the fourth |
| BN26 | `RunError`'s derive list | **latent hazard** | `RunError` is `Copy` only because three other crates' error types are, one of which reserves the right to add variants with `#[ non_exhaustive ]`, and neither side records the coupling |
| BN27 | the forty verbs | n/a — observation | Twenty-five are `const fn` accessors on a finished value; six functions run a candidate and two of those six spawn a thread, so the multi-producer comparison this crate performs is two candidates wide in the default build |
| BN28 | `Workload::with_batch` | **misleading doc** | Its doc says every candidate honours the batch and warns that a batch only one arm observed would measure batching against nothing; two of the five default candidates never read it |
