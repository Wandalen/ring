# Data Structure: The Workload Description

### Scope

- **Purpose**: Define the record every candidate is driven by, and record why two of its numbers — both called "producers" — are set by one setter and cannot be given different values.
- **Responsibility**: State the fields, the construction rules, and the four degenerate values that are refused rather than clamped.
- **In Scope**: The five fields; the tie between `Workload::producers` and `RingConfig::producers`; `batch`'s fold into `config` alone (no second representation left to tie, see BN9/BN10); the accumulator-semantics axis (`cells`, `semantics`); `records_of`'s disjoint ranges.
- **Out of Scope**: `RingConfig`'s own validation, which is `ring_config`'s and clamps rather than refuses; what a candidate does with the description (→ [`algorithm/001`](../algorithm/001_one_workload_through_six_runners.md)).

### Abstract

Five fields describing one measurement run, held together by the rule that no
setter may move half of a tied pair. Two of the fields are called "producers" and
mean different things; the type exists partly to make it impossible to give them
different values. Two more — `cells` and `semantics` — describe a second axis:
how many accumulator cells a run's drained records fold into, and whether a
repeat write to one cell overwrites or sums.

### Structure

```rust
pub struct Workload
{
  config : RingConfig,
  producers : usize,
  records_per_producer : usize,
  cells : usize,
  semantics : AccumulatorSemantics,
}
```

**Defaults: one producer, 1024 records, batch 1, one accumulator cell, `Set`
semantics.** Deliberately modest — this crate's own suite runs them, and a
suite that takes a second per case stops being run. `batch` is not a field of
this struct — see BN9/BN10 below — so its default of 1 is `RingConfig::new`'s,
not `Workload::new`'s.

| Field | Meaning | Derived |
|---|---|---|
| `config` | The ring every candidate is built from | `capacity()`, `batch()` |
| `producers` | How many threads publish concurrently | `offered()` = `producers × records_per_producer` |
| `records_per_producer` | How many records each thread publishes | `records_of( i )`, disjoint per producer |
| `cells` | How many accumulator cells a run's drained records fold into | `AccumulatorTable::cells()` has this length |
| `semantics` | Whether a repeat write to a cell overwrites (`Set`) or sums (`Delta`) | Read once per drained record, in `run`'s accumulator fold |

#### The two numbers called "producers"

**`RingConfig` has its own `producers` field and it means something else.** It
selects the *backend* — one producer picks SPSC, more picks MPSC — rather than
the thread count. Two independent numbers under one name is a trap with an
obvious failure: a four-thread workload driving a ring configured
single-producer, which is either a data race the type system happens to prevent
or a silently wrong measurement.

**So there is no setter that moves one without the other:**

```rust
pub fn with_producers( mut self, producers : usize ) -> Result< Self, WorkloadError >
{
  if producers == 0 { return Err( WorkloadError::ZeroProducers ); }
  self.producers = producers;
  self.config = self.config.with_producers( producers );   // ← both, always
  Ok( self )
}
```

**`batch` is tied the same way and for a weaker reason**, worth stating so the
symmetry is not mistaken for necessity. Nothing breaks if the config's batch and
the workload's batch differ — `ring_core` reads the config's field in no path
this crate reaches. They are tied because a `Workload` is meant to be a single
description, and a second batch number that nothing reads is a field waiting to
be read wrongly later.

**This is the mirror image of the trap
[`ring_factory/docs/pitfall/001`](../../../ring_factory/docs/pitfall/001_the_criterion_grades_the_clamped_value.md)
records.** There, a caller's requested value is silently corrected and the
corrected one is graded. Here, the risk was two values that could legitimately
differ and would then be compared against each other. Both are failures of *one
concept with two representations*, and both are closed the same way — by making
the second representation unreachable.

### Validation

**Four degenerate values are refused, not clamped.** Three of them degrade into
a *working* run rather than an obviously broken one, which is what makes
refusal the right response for those three:

| Value | Refused as | What it would have produced |
|---|---|---|
| `producers == 0` | `WorkloadError::ZeroProducers` | Nothing writes. **Every candidate ties at zero nanoseconds**, and the report is a table of zeroes that looks like a result |
| `records_per_producer == 0` | `WorkloadError::ZeroRecords` | The same tie, reached differently |
| `batch == 0` | `WorkloadError::ZeroBatch` | Staging never reaches a flush, so the staged candidate publishes nothing and the unstaged ones are unaffected. **A comparison in which one arm is inert** |
| `cells == 0` | `WorkloadError::ZeroCells` | Not a working-looking run at all: `destination_of` computes `producer % workload.cells()`, so the first drained record panics on division by zero |

**`batch == 0` is the one that justifies the policy for the first three.** The
other two produce a table of zeroes, which a reader would question. A zero
batch produces a table in which five rows are normal and one is zero, which
reads as a finding about the staged path — an actual, specific, wrong
conclusion rather than an obviously broken output.

**`cells == 0` is refused for the opposite reason.** It has no plausible-looking
wrong answer to guard against — left unrefused, it does not degrade at all, it
panics before a table exists. Refusing it upfront turns a division-by-zero
panic buried inside the accumulator fold into a typed, immediate
`WorkloadError` raised at construction time.

**`RingConfig`'s own setters clamp instead**, and this crate does not attempt to
override that: `with_batch( 0 )` on a config floors to 1. The refusal here
happens before the config is touched, so the two policies never both apply to
one value. `cells` and `semantics` have no `RingConfig` counterpart to clamp
against — the accumulator axis is this crate's own, not `ring_config`'s.

**`capacity` is validated by `RingConfig::new` and is the one field with no
degenerate case handled here** — a non-power-of-two is refused loudly upstream,
before a `Workload` can exist.

### Operations

Three builder setters — `with_producers`, `with_records`, `with_batch` — each
returning `Result< Self, WorkloadError >` and each maintaining the ties above,
plus the accessors every runner reads: `config`, `producers`,
`records_per_producer`, `batch`, `capacity`, `offered`, and `records_of`.

#### `records_of` is disjoint

```rust
pub fn records_of( &self, producer : usize ) -> Range< Record >
```

Producer 0 publishes `0..records_per_producer`, producer 1 the next block, and
so on — **so a drained record identifies its writer.** Nothing in this crate
asserts on the values yet. The property exists so that an ordering question
(did producer 2's records arrive in order? did any interleave?) can be asked of
a run already recorded, rather than requiring a new harness.

`Record` is `u64`: large enough to carry an identity across `producers ×
records_per_producer` values, small enough that the measurement is of the
publication protocol rather than of `memcpy`.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_one_workload_through_six_runners.md](../algorithm/001_one_workload_through_six_runners.md) | Every step that reads this record |

### Data Structures

| File | Relationship |
|------|--------------|
| [002_three_counts_that_are_not_interchangeable.md](002_three_counts_that_are_not_interchangeable.md) | What a run against this description produces |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_the_door_caps_what_the_structure_does_not.md](../pitfall/001_the_door_caps_what_the_structure_does_not.md) | Why `producers` above 1 excludes candidates rather than reconfiguring them |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/002_one_candidate_through_one_run.md](../lifecycle/002_one_candidate_through_one_run.md) | The states a candidate passes through under this description |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_candidate.md](../type/001_candidate.md) | `admits`, which reads `producers` |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_config/src/lib.rs`](../../../ring_config/src/lib.rs) | `RingConfig`'s five fields and its clamping setters |
| [`ring_factory/docs/pitfall/001`](../../../ring_factory/docs/pitfall/001_the_criterion_grades_the_clamped_value.md) | The mirror-image trap — a requested value corrected before it is graded |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/bench_test.rs`](../../tests/bench_test.rs) | `a_workload_refuses_every_degenerate_dimension` asserts all four refusals and their renderings; `the_producer_count_sets_the_backend_it_needs` asserts the tie on both `producers` and `batch`, and the disjointness of `records_of`; `the_accumulator_axis_defaults_to_set_and_one_cell` pins the `cells`/`semantics` defaults above without perturbing the other four |

### BN9 — The Batch Is Not Tied the Same Way, and Both Assertions of the Tie Were Handed the Fixture Where It Holds

The section above says `batch` "is tied the same way and for a weaker reason".
Set the two setters side by side:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
echo '  -- what each setter does with the number it is given --'
awk '/pub fn with_producers\( mut self, producers/, /^  \}/' src/lib.rs | command grep -E 'self\.' | sed 's/^/    Workload   /'
awk '/pub fn with_batch\( mut self, batch/, /^  \}/'      src/lib.rs | command grep -E 'self\.' | sed 's/^/    Workload   /'
awk '/pub const fn with_producers\( mut self, producers/, /^  \}/' ../ring_config/src/lib.rs | command grep -E 'self\.' | sed 's/^/    RingConfig /'
awk '/pub const fn with_batch\( mut self, batch/, /^  \}/'         ../ring_config/src/lib.rs | command grep -E 'self\.|let capped' | sed 's/^/    RingConfig /'
echo '  -- and what the two constructors set --'
awk '/pub const fn new\( config : RingConfig \) -> Self/, /^  \}/' src/lib.rs | command grep 'Self {' | sed 's/^/    Workload::new   /'
command grep -n 'batch : 1,' ../ring_config/src/lib.rs | sed 's/^/    RingConfig::new /' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
echo '  -- both assertions of the tie, and the fixture each was handed --'
command grep -n -B12 'workload.config().batch()' tests/bench_test.rs \
  | command grep -E 'let workload = |config\(\).batch|batch\(\), 32' | sed 's/^/    /' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
echo '  -- and what Workload::batch() reads today --'
awk '/pub const fn batch\( &self \) -> usize/, /^  \}/' src/lib.rs | sed 's/^/    /'
echo '  -- what each fixture asks for, and what its config carries --'
awk '
  /^fn (roomy|cramped|parallel)\(/ { n = $2; sub( /\(.*/, "", n ); cap = 0; b = 32; next }
  n != "" && /RingConfig::new\(/ { cap = $0; sub( /.*RingConfig::new\( /, "", cap ); sub( / \).*/, "", cap ) }
  n != "" && /with_batch\(/      { b   = $0; sub( /.*with_batch\( /, "", b );        sub( / \).*/, "", b ) }
  n != "" && /^\}/ { c = ( b > cap ? cap : b )
                     printf "    %-9s with_batch( %-4s ) -> config carries %-4s  %s\n", n, b, c, ( b == c ? "" : "<-- clamped" )
                     n = "" }
' tests/bench_test.rs
```

Live output:

```
  -- what each setter does with the number it is given --
    Workload       self.producers = producers;
    Workload       self.config = self.config.with_producers( producers );
    Workload       self.config = self.config.with_batch( batch );
    RingConfig     self.producers = if producers == 0 { 1 } else { producers };
    RingConfig     let capped = if batch > self.capacity.get() { self.capacity.get() } else { batch };
    RingConfig     self.batch = if capped == 0 { 1 } else { capped };
  -- and what the two constructors set --
    Workload::new       Self { config, producers : 1, records_per_producer : 1024, cells : 1, semantics : AccumulatorSemantics::Set }
    RingConfig::new 75:        batch : 1,
  -- both assertions of the tie, and the fixture each was handed --
      let workload = parallel();
      assert_eq!( workload.batch(), 32 );
      assert_eq!( workload.config().batch(), 32 );
      let workload = roomy();
        workload.config().batch(),
          workload.config().batch(),
  -- and what Workload::batch() reads today --
      pub const fn batch( &self ) -> usize
      {
        self.config.batch()
      }
  -- what each fixture asks for, and what its config carries --
    roomy     with_batch( 32   ) -> config carries 32    
    cramped   with_batch( 32   ) -> config carries 16    <-- clamped
    parallel  with_batch( 32   ) -> config carries 32    
```

**`with_producers` wrote the same value to both fields. `with_batch` did not.**
It wrote the caller's value to `self.batch` and the *clamped* value to
`self.config`, because `RingConfig::with_batch` caps at the capacity. The two
setters looked symmetric and were not: `RingConfig::with_producers` clamps only
zero, which `Workload::with_producers` has already refused, so the clamp can
never fire; `RingConfig::with_batch` clamps at the capacity, which
`Workload::with_batch` did not check at all.

**`Workload::new` breaks the tie before any setter runs.** It is `const`, it
sets `batch : 32`, and it does not touch the config it was handed —
`RingConfig::new` having set `batch : 1`. So every `Workload` starts life with
the pair split 32-against-1, and only a subsequent `with_batch` call brings them
into agreement, and only when the capacity is large enough to allow it.

**Two tests assert the tie, and both were handed a 4096-slot fixture.**
`the_producer_count_sets_the_backend_it_needs` uses `parallel()`;
`the_flush_relay_is_unreachable_while_the_batch_ties_the_buffer` uses `roomy()`.
`cramped()` — 16 slots, batch 32 — is defined in the same file and used by three
other tests, and the tie is 32-against-16 on it. Neither assertion has ever been
run against it.

The Validation section above says the two policies "never both apply to one
value", and for `batch == 0` that is exactly right — the refusal happens first,
so the floor is unreachable. For `batch > capacity` **both policies do apply**:
this crate accepts the value and `ring_config` corrects it, which is the
condition the sentence was written to rule out. Only the zero case was
considered, because only the zero case has an error variant to notice it by.

**Disposition:** applied — the split is closed by removing the near side of it.
`Workload` no longer carries a `batch` field: `with_batch` writes only
`self.config`, and `batch()` reads that back, so what this crate reports and what
`ring_config` corrected are the same number by construction rather than by
agreement. `for batch > capacity` both policies still apply — this crate accepts
the value and `ring_config` clamps it — but there is no longer a second copy for
the correction to disagree with, so the Validation sentence is now true as a
statement about what a caller can observe. The tie is asserted on the fixture
that could break it: `the_batch_reported_is_the_batch_the_config_carries` runs
`roomy`, `cramped` and `parallel`, checks `batch() == config().batch()` and
`batch() <= capacity()` on each, and additionally pins `cramped().batch() == 16`
so the clamp itself is what the test is looking at. It was proven able to fail by
making `batch()` return a constant 32, which reddened both it and
`a_cramped_run_drops_and_the_drop_is_counted_from_the_drain`. What this does not
buy: `Workload::with_batch` still accepts a batch larger than the capacity
silently rather than returning a `WorkloadError`, so the correction is still a
clamp a caller can be surprised by — it is now merely one they can see. Now
prints: `cramped   with_batch( 32   ) -> config carries 16`

### BN10 — The Second Representation Is Reachable, and Nothing in the Family Reads It

The section above closes with: *"both are closed the same way — by making the
second representation unreachable."* Measure it:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- every read of RingConfig::batch in the family, src and tests --'
command grep -r '\.batch()' ring_*/src/ ring_*/tests/ --include=*.rs \
  | command grep -v '^ring_config/' | command grep 'config()\.batch()\|cfg\.batch()' | sed 's/^/    /'
printf '    outside ring_config and ring_bench : %s\n' \
  "$( command grep -rn '\.batch()' ring_*/src/ ring_*/tests/ --include=*.rs 2>/dev/null \
      | command grep -cv '^\(ring_config\|ring_bench\)/' )"
echo '  -- inside ring_config, every one of them a doctest line --'
command grep '\.batch()' ring_config/src/lib.rs | sed 's/^/    /'
echo '  -- what the field is documented to guarantee --'
command grep 'always between one and the capacity inclusive' ring_config/src/lib.rs | sed 's/^/    /'
echo '  -- and what the crate downstream of it tells its readers --'
command grep 'clamped .batch.' ring_factory/src/lib.rs | sed 's/^/    /'
```

Live output:

```
  -- every read of RingConfig::batch in the family, src and tests --
    ring_bench/tests/bench_test.rs:    assert_eq!(workload.config().batch(), 32);
    ring_bench/tests/bench_test.rs:        workload.config().batch(),
    ring_bench/tests/bench_test.rs:            workload.config().batch(),
    outside ring_config and ring_bench : 0
  -- inside ring_config, every one of them a doctest line --
        /// assert_eq!(cfg.with_batch(0).batch(), 1);
        /// assert_eq!(cfg.with_batch(8).batch(), 8);
        /// assert_eq!(cfg.with_batch(999).batch(), 16);
        /// assert_eq!(RingConfig::new(8).unwrap().batch(), 1);
  -- what the field is documented to guarantee --
        /// The batch size, always between one and the capacity inclusive.
  -- and what the crate downstream of it tells its readers --
    //! `RingConfig` has already validated `capacity` and clamped `batch` and
```

`workload.config()` is a public `const fn` returning a `Copy` of the config, and
`RingConfig::batch()` is a public accessor, so the second representation was
reachable by anyone. The two callers that reached it were the two assertions from
BN9 — both in this crate's own suite, both checking that it equalled the first
representation.

**Outside `ring_config` and `ring_bench`, the field had zero readers in all 33
crates**, in source and in tests. Inside `ring_config` every occurrence is a
doctest line demonstrating the clamp. So the accessor exists, the clamp exists,
the invariant is documented — *"always between one and the capacity inclusive"* —
and the only code that has ever asked for the value is a pair of assertions
written to confirm it matches a different field.

**`ring_factory`'s module documentation names the clamp as a precondition it
relies on**: "`RingConfig` has already validated `capacity` and clamped `batch`".
It then builds rings without reading `batch`. A guarantee cited by a consumer
that does not consume it is not load-bearing; it is a description of the
producer, filed under the reader.

That is what the doc above got right for the wrong reason. It defended the tie by
saying "`ring_core` reads the config's field in no path this crate reaches" — and
the measurement is stronger than the claim: *nothing* reads it, anywhere. Which
is also why BN9's split is currently invisible. The tie was maintained to prevent
"a second batch number that nothing reads" from being "a field waiting to be read
wrongly later", the tie does not hold, and the field is waiting exactly as
described.

The general shape: **a field whose only readers are the assertions that it agrees
with another field has no independent purpose**, and the agreement those
assertions check is the one property nothing downstream depends on.

**Disposition:** applied — the second representation was deleted rather than
reconciled. `Workload` has no `batch` field; `with_batch` writes only
`self.config`, and `batch()` reads it back, so the value every caller sees is the
one `RingConfig::with_batch` clamped. That inverts the finding's closing
observation: the config's copy is no longer the one nothing reads, it is now the
only one there is, and the clamp it applies is load-bearing on every fixture
rather than latent. It is load-bearing immediately — under the split, the
`cramped` fixture drove the staged candidate with an unclamped 32 into a 16-slot
ring, which `ring_flush` refused on the first `free_capacity` check and never
recovered from; that stall is gone. The tie is now asserted where it could break:
`the_batch_reported_is_the_batch_the_config_carries` runs all three fixtures,
including `cramped`, rather than the two where a batch of 32 clamps to itself,
and was proven able to fail. What this does not buy: `RingConfig::batch()` still
has no reader outside this crate and `ring_config`'s own doctests, so the
guarantee `ring_factory`'s module doc cites remains one it does not consume — a
separate finding this change does not touch. Now prints: `with_batch( 32   ) -> config carries 16`
