# Type: Run Error

### Scope

- **Purpose**: Define the four ways a candidate can fail to run, establish that only one of them is this crate's own, and record the variant that is unreachable by construction and kept anyway.
- **Responsibility**: State the variants, the derives, the rendering, and the validation rules.
- **In Scope**: `ProducerCeiling`, `Build`, `Ring`, `Flush`; the `Display` and `Error` impls; why two variants relay the same underlying refusal.
- **Out of Scope**: `WorkloadError`, which refuses a *description* rather than a run (→ [`data_structure/001`](../data_structure/001_the_workload_description.md)); the dependencies' own error types.

### Definition

```rust
pub enum RunError
{
  ProducerCeiling { candidate : Candidate, requested : usize, ceiling : usize },
  Build( BuildError ),      // ring_factory
  Ring( RingError ),        // ring_types, raised by ring_core
  Flush( ConfigError ),     // ring_flush
}
```

| Variant | Owner | Reachable |
|---|---|---|
| `ProducerCeiling` | **this crate** | yes — the only refusal this crate decides |
| `Build` | `ring_factory` | yes — `OverflowPolicy::DropOldest` |
| `Ring` | `ring_core` | yes — the same policy, reached by the staged candidate |
| `Flush` | `ring_flush` | **no** — excluded by construction, kept deliberately |

**One of four is this crate's.** The other three are relays: the refusal is
re-rendered, never re-decided, so a dependency that later accepts a
configuration it currently refuses needs no change here.

#### `ProducerCeiling` carries both numbers

```rust
"contract_ring admits 1 producer(s), asked for 8"
```

The candidate, the ceiling, and the request. **The two numbers together are
what make the refusal actionable** — "refused" says the run did not happen,
"admits 1, asked for 4" says what to change. It is also the line the report
prints for an excluded candidate, so the refusal list is self-explaining rather
than requiring the reader to know each candidate's bound.
→ [`pattern/002`](../pattern/002_a_refusal_is_a_row.md).

#### `Build` and `Ring` are the same refusal by two routes

**Not redundancy.** `OverflowPolicy::DropOldest` is refused by `ring_core`,
because evicting an unread record contradicts the exactly-once delivery both
in-house backends guarantee. Two candidates reach that refusal differently:

| Candidate | Route | Variant |
|---|---|---|
| `ContractRing` | `ring_factory::build`, which wraps the refusal in its own `BuildError::Unsupported` | `Build` |
| `TlsOverRing` | `ring_core::Ring::new` directly, because `Flusher::new` needs a `ring_core::Producer` the Contract cannot hand it | `Ring` |

**The variant is therefore evidence about the Contract**, not just about the
policy: the staged candidate arrives as `Ring` precisely because it could not
use the door. The day `ring_flush` and `ring_factory` compose, that candidate's
refusal becomes a `Build` and a test changes — which is how the gap stays
visible rather than becoming a paragraph nobody rereads.
→ [`integration/001`](../integration/001_declared_edges_and_the_three_that_were_missing.md).

#### Rendering

`Display` for all four; `core::error::Error` so the whole thing folds into a
`Box< dyn Error >`. The three relay variants forward to their inner error's own
`Display` rather than prefixing it — a refusal that reads
`"factory error: policy refused"` says twice what `"policy refused"` says once,
and the variant is already in the `Debug` output for anyone who needs the route.

**`ring_flush::ConfigError` implemented neither trait until this crate needed
it.** `RunError`'s `Display` would not compile, which is how the gap was found:
it was invisible while `ring_flush`'s only consumer was its own suite, where
`Debug` suffices. Fixed in `ring_flush` with a covering test rather than worked
around here.
→ [`integration/001`](../integration/001_declared_edges_and_the_three_that_were_missing.md).

#### Derives

`Debug, Clone, Copy, PartialEq, Eq`. **`Copy` is available only because all
three wrapped errors are themselves `Copy`** — none carries a `String` or an
allocation. That is a property of the dependencies, not a decision here, and it
would be lost the moment any of them added a message field; the derive would
then fail to compile, which is the right outcome.

`PartialEq` because the suite asserts whole errors by value:

```rust
assert_eq!( refused, RunError::ProducerCeiling { candidate : Candidate::ContractRing, requested : 4, ceiling : 1 } );
```

**No `#[ non_exhaustive ]`.** This crate is `publish = false` inside a workspace
that builds as a unit, so the compatibility guarantee has no consumer that
could need it — the sole reason to omit it. The attribute would not have cost
the unreachable `Flush` variant's constructibility in a test: `#[ non_exhaustive ]`
on an enum blocks downstream *matching* without a wildcard arm, not
construction, and this crate's own suite uses `matches!`/`assert_eq!`
throughout rather than an exhaustive `match`, so even that cost would have
been zero here (→ BN48).

### Validation

`RunError` has no constructor and no validating function — each variant is built
by whichever crate raises it. What is validated is *which variants can
legitimately occur*, and every rule is enforced by construction rather than by a
runtime check:

| Variant | Constructed by | What keeps it honest |
|---|---|---|
| `ProducerCeiling` | this crate, in `run` | the ceiling guard, checked before anything is allocated |
| `Build` | relayed from `ring_factory` | the factory's own refusal, re-rendered and never re-decided |
| `Ring` | relayed from `ring_core` | the same refusal, reached by the staged candidate's direct route |
| `Flush` | nothing in production | the batch/capacity tie described below |

The `Copy` derive above is the second of these checks and the only one the
compiler enforces on this crate's behalf: it holds only while every wrapped error
stays allocation-free, and stops compiling the moment one of them does not.

#### `Flush` is unreachable and kept

`run_tls_over_ring` builds its `TlsBuffer` with `workload.batch()` slots and
binds `FlushPolicy::OnBatch( workload.batch() )`. `ring_flush`'s two binding
refusals are:

| `ConfigError` | Excluded by |
|---|---|
| `ZeroBatch` | `Workload::with_batch` refusing zero |
| `BatchExceedsCapacity` | `n > n` being false |

**Kept rather than removed, and not replaced by an `expect`.** The tie between
the buffer's capacity and the flush trigger is one edit from being broken and
nothing in either type enforces it. An `expect` would turn a future
configuration mistake into a **panic inside a measurement** — the worst place
for one, since a partially-completed comparison is worse than a refused
candidate. The variant is constructed directly in a test, so its `Display` arm
is covered without a workload that produces it.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_run_surface.md](../api/001_the_run_surface.md) | Where each variant is raised, and why `Comparison::run` never returns one |
| [../api/002_the_report_surface.md](../api/002_the_report_surface.md) | The refusal lines this type's `Display` produces |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_declared_edges_and_the_three_that_were_missing.md](../integration/001_declared_edges_and_the_three_that_were_missing.md) | The composition gap the `Build`/`Ring` split is evidence of, and the `Display` impl this type forced into `ring_flush` |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_a_refusal_is_a_row.md](../pattern/002_a_refusal_is_a_row.md) | Why a refusal is collected rather than propagated |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_the_door_caps_what_the_structure_does_not.md](../pitfall/001_the_door_caps_what_the_structure_does_not.md) | What `ProducerCeiling` is reporting, and whose limit it actually is |

### Types

| File | Relationship |
|------|--------------|
| [001_candidate.md](001_candidate.md) | Carried by value in `ProducerCeiling` |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_factory/src/lib.rs`](../../../ring_factory/src/lib.rs) | `BuildError`, and its `Unsupported` variant relaying `ring_core` |
| [`ring_flush/src/lib.rs`](../../../ring_flush/src/lib.rs) | `ConfigError`'s two variants and the `Display`/`Error` impls added for this crate |
| [`ring_types/src/lib.rs`](../../../ring_types/src/lib.rs) | `RingError`, the shared refusal vocabulary |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/bench_test.rs`](../../tests/bench_test.rs) | `every_error_renders` covers all four `Display` arms and the `Box< dyn Error >` fold; `a_policy_refusal_names_the_crate_that_refused` asserts the `Build`/`Ring` split; `the_flush_relay_is_unreachable_while_the_batch_ties_the_buffer` constructs the unreachable variant and records why it stays |

### BN47 — The Test That Promises Chaining Boxes the One Variant With Nothing to Chain

`### Rendering` states the fold: *"`Display` for all four; `core::error::Error`
so the whole thing folds into a `Box< dyn Error >`."* The Tests table says
`every_error_renders` *"covers all four `Display` arms and the `Box< dyn Error >`
fold"*, and that test's own doc comment is stronger still.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
printf -- '--- the promise ---\n'
command grep -n -B 2 'fn every_error_renders' tests/bench_test.rs | sed 's/^/  /' | sed -E 's/^(.{0,96}).*/\1/' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
printf -- '--- what it constructs, and what it does with each ---\n'
awk '/fn every_error_renders/, /^\}/' tests/bench_test.rs \
  | command grep -nE 'let [a-z_]+ (=|:)|Box|to_string|source\(' \
  | sed -E 's/^([0-9]+:)\s*/  \1 /' | sed -E 's/^(.{0,94}).*/\1/' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
printf -- '--- the Error impl behind the fold ---\n'
command grep -n -A 1 'impl core::error::Error for RunError' src/lib.rs | sed 's/^/  /' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
printf '  fn source( in ring_bench           : %s\n' "$( command grep -c 'fn source(' src/lib.rs )"
printf '  fn source( across ring_*/src : %s\n' \
  "$( command grep -rh 'fn source(' ../ring_*/src/ | wc -l )"
```

Live output:

```
--- the promise ---
  /// holding a `Box< dyn Error >` learns nothing beyond what `Display` says.
  #[ test ]
  fn every_error_renders()
--- what it constructs, and what it does with each ---
   let ceiling = RunError::ProducerCeiling
   assert_eq!( ceiling.to_string(), "direct_spsc admits 1 producer(s), asked for 8" );
   let evicting = Workload::new
   let build = run( Candidate::ContractRing, &evicting ).unwrap_err();
   let ring = run( Candidate::TlsOverRing, &evicting ).unwrap_err();
   assert!( build.to_string().contains( "polic" ), "{build}" );
   assert!( ring.to_string().contains( "polic" ), "{ring}" );
   let boxed : Box< dyn core::error::Error > = Box::new( ceiling );
   assert!( boxed.to_string().contains( "direct_spsc" ) );
--- the Error impl behind the fold ---
  impl core::error::Error for RunError {}
  
  fn source( in ring_bench           : 0
  fn source( across ring_*/src : 0
```

The test boxes `ceiling` — the `ProducerCeiling` variant, the one this crate
decides itself and the only one of the four that wraps nothing. `build`, `ring`
and `relay` are each checked with `to_string()` and never boxed. `source()` is
called nowhere, and it exists nowhere: `impl core::error::Error for RunError {}`
is empty, as is every other `Error` impl in the family.

So *"and chains"* names something no error in this family does. Through a
`Box< dyn Error >`, a `RunError::Build( BuildError::Unsupported )` reports
`source() == None`, and its `Display` — by this document's own deliberate choice
— forwards to the inner error unprefixed, on the reasoning that *"a refusal that
reads `factory error: policy refused` says twice what `policy refused` says
once."*

**Those two decisions are individually right and jointly remove the route.** Not
prefixing is correct when a human reads the message. Not implementing `source` is
consistent with the rest of the family. Together they mean a caller holding the
trait object has no way — not the text, not the chain — to learn that a
`ring_factory` refusal was involved. The document names the escape hatch, and it
is the only one left: *"the variant is already in the `Debug` output for anyone
who needs the route."*

The finding is the word in the test, not the design. A test named for a property
the crate does not have is a claim nobody will re-check, and this one sits on the
single variant for which the property is vacuous.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
command grep -B5 'fn every_error_renders' tests/bench_test.rs
```

Live output:

```
/// Every error this crate can produce renders for a human. None of them
/// chains: `core::error::Error::source()` is unimplemented family-wide
/// (`RunError` and its three wrapped error types alike), so a caller
/// holding a `Box< dyn Error >` learns nothing beyond what `Display` says.
#[ test ]
fn every_error_renders()
```

**Disposition:** applied — `every_error_renders`'s own doc comment no longer
claims "and chains"; it now states the true shape (no `source()` anywhere in
the family) so the property the test's name implies matches what the test
actually establishes. This document's own "### Rendering" section and Tests
table were already accurate and needed no change — the false claim lived only
in the test's doc comment, which this crate's own source owns.
Now prints: `None of them`

### BN48 — The Second Reason Given for Omitting `#[ non_exhaustive ]` Is Not a Property of `#[ non_exhaustive ]`

`#### Derives` gives two reasons:

> **No `#[ non_exhaustive ]`.** This crate is `publish = false` inside a
> workspace that builds as a unit, so the compatibility guarantee has no consumer
> that could need it, and **it would cost the ability to construct the
> unreachable `Flush` variant in a test.**

The first reason is sound and checkable — `publish = false` is on line 6 of the
manifest. The second describes an effect the attribute does not have, and the
family contains the counterexample.

```sh
cd "$(git rev-parse --show-toplevel)"
printf -- '--- non_exhaustive attributes in the family ---\n'
command grep -r '^#\[ non_exhaustive \]' ring_*/src/ | sed 's|^ring/||' | sed 's/^/  /'
command grep -A 2 '^#\[ non_exhaustive \]' ring_types/src/error.rs | sed 's/^/  /'
printf -- '--- crates other than ring_types that construct one of its variants ---\n'
command grep -rl 'Err( RingError::' ring_*/src/ | command grep -v ring_types \
  | sed 's|^ring/||' | sed 's/^/  /'
printf -- '--- two such sites, verbatim ---\n'
command grep -r 'return Err( RingError::Full );' ring_batch/src/lib.rs | sed 's/^/  /'
command grep -r 'self.frontier().ok_or( RingError::Empty )' ring_barrier/src/lib.rs | sed 's/^/  /'
printf -- '--- and from downstream test crates ---\n'
printf '  RingError:: sites under ring_*/tests/ : %s\n' \
  "$( command grep -rh 'RingError::' ring_*/tests/ | wc -l )"
printf -- '--- what the attribute would actually cost here ---\n'
printf '  exhaustive match on a RunError value in the suite : %s\n' \
  "$( command grep -cE 'match .*RunError' ring_bench/tests/bench_test.rs )"
printf '  publish key in ring_bench/Cargo.toml             : %s\n' \
  "$( command grep -m1 '^publish' ring_bench/Cargo.toml )"
```

Live output:

```
--- non_exhaustive attributes in the family ---
  ring_testkit/src/lib.rs:#[ non_exhaustive ]
  ring_types/src/error.rs:#[ non_exhaustive ]
  #[ non_exhaustive ]
  pub enum RingError
  {
--- crates other than ring_types that construct one of its variants ---
  ring_batch/src/lib.rs
  ring_claim/src/lib.rs
  ring_consume/src/lib.rs
  ring_core/src/lib.rs
  ring_gating/src/lib.rs
  ring_overflow/src/lib.rs
  ring_shutdown/src/lib.rs
  ring_slot/src/lib.rs
  ring_spsc/src/lib.rs
  ring_tls/src/lib.rs
  ring_wait/src/lib.rs
--- two such sites, verbatim ---
      return Err( RingError::Full );
      self.frontier().ok_or( RingError::Empty )
--- and from downstream test crates ---
  RingError:: sites under ring_*/tests/ : 126
--- what the attribute would actually cost here ---
  exhaustive match on a RunError value in the suite : 0
  publish key in ring_bench/Cargo.toml             : publish = false
```

`#[ non_exhaustive ]` on an *enum* prevents a downstream crate from matching it
without a wildcard arm. It does not prevent constructing its existing variants.
`ring_types::RingError` carries the attribute — one of the family's two, beside
`ring_testkit::Anomaly` — and eleven other crates construct its variants directly
in ordinary `Err( … )` returns, with `ring_barrier` doing the same through
`ok_or`. So do all 126 sites in the family's `tests/` directories, which are
separate crates by definition.

So the sentence would still be true with the attribute applied: `RunError::Flush(
ring_flush::ConfigError::ZeroBatch )` constructs exactly as it does now. What
`#[ non_exhaustive ]` would actually cost is the exhaustive `match`, and the suite
performs none — it uses `matches!` and `assert_eq!` throughout. The real cost
here is zero, and the real reason to omit it is the one already given first.

**Why this is worth a row rather than a deletion.** The two reasons are offered
as a pair, and the false one is the concrete, memorable one — a specific test, a
specific variant. A reader deciding whether to add `#[ non_exhaustive ]` to some
other type in this family will carry away "it blocks construction in tests",
which is wrong in a direction that discourages using it where it would help. The
attribute's live uses in the family are both a few directories away and both
demonstrate the opposite.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
# -m1 and no -n: this file is its own subject, so an unbounded match
# also finds this command line and every copy of its own output below,
# and -n re-prefixes a fresh line number onto each earlier pass's output
command grep -m1 'sole reason to omit it' docs/type/002_run_error.md
```

Live output:

```
could need it — the sole reason to omit it. The attribute would not have cost
```

**Disposition:** applied — `#### Derives` now gives one reason for omitting
`#[ non_exhaustive ]` (`publish = false`, no consumer needs the guarantee) and
states plainly that the attribute would not have blocked constructing `Flush`
in a test — it blocks unmatched downstream `match`es, which this crate's own
suite doesn't write. The false second reason is gone rather than merely
qualified, since the finding shows it was never true at any tier.
Now prints: `sole reason to omit it`

**Correction (2026-09-20):** two supporting measurements above were wrong, and
neither touches the defect this row fixed.

The first was an exclusivity claim in three places: the recipe's own label read
`--- the only non_exhaustive in the family ---` and printed **two** files
directly beneath itself, the prose read "carries the attribute — the family's
only one", and the closing paragraph read "the attribute's one live use in the
family is three directories away". `ring_testkit::Anomaly` acquired the
attribute after the prose was written; the recipe, which globs rather than
counts, kept printing both files and nobody read the label against the output
one line below it. The label now names what it lists rather than how many, so
the next change to the family's marked-enum roster cannot falsify it.

The second is the more useful one to record. The `tests/` site count appeared
as **three different numbers for one measurement**: `107` in the prose, `120`
in the recorded Live output, and `126` from re-running the recipe today. A
recorded output block only keeps pace with the tree when someone re-runs it —
it is as perishable as the prose around it, just one generation fresher. Both
now read 126, taken from a verbatim re-run of the block above rather than from
the recorded copy; the rest of that block reproduced byte-identical, including
the eleven-crate list, the two verbatim sites, `exhaustive match on a RunError
value in the suite : 0`, and `publish = false`.

What survives is the whole of the finding and all of its arithmetic that
matters: `#[ non_exhaustive ]` still does not prevent constructing existing
variants, eleven crates plus every `tests/` site still do exactly that against
a marked enum, this suite still writes no exhaustive `match` on a `RunError`,
and the real cost of the attribute here is still zero. The **Disposition**
above and the `#### Derives` edit it records are untouched — the false second
reason it removed was never about how many enums carry the attribute or how
many test sites name `RingError`.

