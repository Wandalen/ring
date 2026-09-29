# Invariant: No Two Producers Hold One Sequence

### Scope

- **Purpose**: State the crate's multi-producer exclusivity requirement, the mechanism that holds it, and the configuration under which it is actually verified — which is not the configuration the crate runs in.
- **Responsibility**: Name the invariant precisely, show the four tests that assert it, and record that the design defect the crate exists to avoid does not violate this invariant at all.
- **In Scope**: Exclusivity between producers, and everything that establishes or tests it.
- **Out of Scope**: What a refusal must leave unchanged — see [`invariant/002`](002_a_refused_claim_moves_nothing.md).

### The Invariant

The requirement, as `src/lib.rs:20-23` states it:

> Multi-producer exclusivity is `docs/feature/172_multi_producer_claim.md`'s
> requirement, and it is the reason this crate exists rather than being two
> lines inside `ring_publish`: **no two producers may ever be granted the same
> sequence.**

Formally: for any two `Claim`s `a` and `b` returned by distinct calls on the same
`Claimer`, `a.overlaps( b )` is `false`. `Claim::overlaps` is public specifically
so this can be asserted directly — `:212-214` says so:

> The property `docs/feature/172_multi_producer_claim.md` forbids across
> producers, exposed so a test can assert it directly rather than …

### What Holds It

One line: `compare_exchange( current, next, … )` succeeds for exactly one caller
per value of `current`. Every other producer's exchange fails, adopts the winner's
value, and computes a disjoint range from it. There is no lock, no ticket, and no
per-producer state — the atomic's own linearisation *is* the exclusivity.

That has a consequence worth stating: the invariant does not depend on the gate.
Remove the `GatingSet` entirely and exclusivity still holds; producers would
simply run past the consumers. The gate enforces a *different* invariant
([`invariant/002`](002_a_refused_claim_moves_nothing.md)), and the two are
independent.

### CL23 — The Exclusivity Tests Run With the Gate Disabled

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'GatingSet::new' ring_claim/tests/claim_test.rs
```

Live output:

```
  let consumers = GatingSet::new( cap( 64 ), 1 );
  let consumers = GatingSet::new( cap( CAPACITY ), 1 );
  let consumers = GatingSet::new( cap( 4 ), 1 );
  let consumers = GatingSet::new( cap( 4 ), 1 );
  let consumers = GatingSet::new( cap( 4 ), 1 );
  let consumers = GatingSet::new( cap( 4 ), 1 );
  let consumers = GatingSet::new( cap( 4 ), 1 );
  let consumers = GatingSet::new( cap( 8 ), 1 );
  let consumers = GatingSet::new( cap( 4 ), 1 );
  let consumers = GatingSet::new( cap( 4 ), 1 );
  let consumers = GatingSet::new( cap( 8 ), 1 );
  let consumers = GatingSet::new( cap( 8 ), 2 );
  let consumers = GatingSet::new( cap( 8 ), 1 );
  let consumers = GatingSet::new( cap( 16_384 ), 0 );
  let consumers = GatingSet::new( cap( 16_384 ), 0 );
  let consumers = GatingSet::new( cap( 16_384 ), 0 );
  let consumers = GatingSet::new( cap( 8_192 ), 0 );
  let consumers = GatingSet::new( cap( CAPACITY ), 1 );
  let consumers = GatingSet::new( cap( 4 ), 1 );
  let consumers = GatingSet::new( cap( 8 ), 1 );
```

| Test | Capacity | Consumers | Gate active |
|------|---------:|----------:|:-----------:|
| `no_two_producers_are_ever_granted_the_same_sequence` | 16,384 | **0** | **no** |
| `claims_under_contention_lose_no_sequences` | 16,384 | **0** | **no** |
| `claim_up_to_under_contention_loses_no_sequences_either` | 16,384 | **0** | **no** |
| `each_producers_own_claims_stay_in_issue_order` | 8,192 | **0** | **no** |
| `no_grant_ever_passes_the_limit_under_contention` | 4 | 1 | **yes** |

Four of the five `thread::scope` tests use a `GatingSet` with **zero consumers**,
which `ring_gating::headroom` answers with the full capacity unconditionally —
so the gate never refuses. The test says why (`:326-328`):

> Capacity exceeds the total, so nothing ever gates — this test is about
> exclusivity alone, and a producer blocking on a full ring would only reduce
> the contention it is trying to create.

That is correct reasoning and it produces a real coverage shape: the crate's two
invariants are verified under **opposite** configurations — exclusivity with
contention maximised and the gate off, the gate with capacity 4 and contention
throttled by a consumer that must keep releasing.

One test spans both, and it is the fifth row. `no_grant_ever_passes_the_limit_under_contention`
checks the gate per grant *and* deduplicates every range at the end (`:538-542`):

```rust
let mut every : Vec< Seq > = all.iter().flat_map( | c | c.sequences() ).collect();
let granted = every.len();
every.sort_unstable();
every.dedup();
assert_eq!( every.len(), granted, "two producers were granted the same sequence" );
```

So exclusivity *is* asserted under a binding gate, by three producers that spend
most of their 3,000 attempts being refused. What that test cannot assert is the
other half of the property:

| | Four zero-consumer tests | The tight-gate test |
|--|--------------------------|---------------------|
| No two producers share a sequence | **✔** | **✔** |
| No sequence is skipped (contiguity) | **✔** — `TOTAL` is known exactly | **✘** — refusals make the total unknown |
| Gate active while asserting it | ✘ | **✔** |

Contiguity needs a predicted count to compare against, and a test whose claims
are refused at an unpredictable rate has none — `granted` is whatever the run
produced, so the assertion can only be self-referential. That is why the
zero-consumer configuration exists at all: it is not just "more contention", it
is the only configuration in which the *expected* number of sequences is
knowable, which is what makes `assert_eq!( every.len(), TOTAL )` possible.

So the gap is narrower than it first looks, and it is specifically about
contiguity: **no test checks that a producer refused by the gate and retrying
leaves no hole in the sequence space.** Duplicate-freedom is covered in both
configurations; contiguity is only ever covered with the gate switched off.

#### The test file's own header counts three of them

```sh
cd "$(git rev-parse --show-toplevel)"
grep -c 'thread::scope' ring_claim/tests/claim_test.rs   # → 5
command grep -m1 -A10 -F '//! ## What a sequential test cannot show' ring_claim/tests/claim_test.rs | tail -n 10
```

Live output:

```
5
//!
//! Exclusivity is a property of concurrent claims, and a single-threaded suite
//! passes it trivially. The five multi-threaded tests below are therefore the
//! ones carrying the crate: `no_two_producers_are_ever_granted_the_same_sequence`
//! collects every granted range from four threads and asserts the union is a
//! partition; `no_grant_ever_passes_the_limit_under_contention` drives claims
//! against a gate tight enough that a check-then-advance implementation
//! overruns it; `claims_under_contention_lose_no_sequences` asserts the
//! ranges are contiguous with no gap, which is the failure a naive CAS retry
//! produces when it recomputes the range but not the start;
```

`tests/claim_test.rs:14` used to open "**The three** multi-threaded tests below
are therefore the ones carrying the crate", naming three by hand while the file
had five. The two the header omitted were both in the table above:

| Named in the header (then) | Omitted (then) |
|-----------------------------|-----------------|
| `no_two_producers_are_ever_granted_the_same_sequence` | `claim_up_to_under_contention_loses_no_sequences_either` |
| `no_grant_ever_passes_the_limit_under_contention` | `each_producers_own_claims_stay_in_issue_order` |
| `claims_under_contention_lose_no_sequences` | |

An ordinary drift — the header was written when three was right, and two tests
arrived after it. It was worth recording because the sentence was not a summary
count but a *claim about which tests carry the crate*, and one of the two it
omitted covered the crate's second entry point entirely. A reader trusting the
header would have concluded `claim_up_to` had no concurrency test at all.

**Disposition:** fixed — the header now says "The five
multi-threaded tests" and names all five, including the two it previously
omitted.

The same passage carries something more useful, though, and it is easy to read
past. It names a **second** broken implementation, distinct from the `fetch_add`
one the decision documents reject:

> `claims_under_contention_lose_no_sequences` asserts the ranges are contiguous
> with no gap, which is the failure **a naive CAS retry produces when it
> recomputes the range but not the start**.

So the header identifies two ways to get this wrong — advance-then-check, and a
retry that refreshes one half of its state — and `tests/manual/readme.md § C1`
mutates for the first only. The second is guarded by an assertion rather than by
a mutation run, which is a weaker guarantee: nothing proves the contiguity
assertions would actually fail against it.

### CL24 — The Invariant With the Most Test Code Behind It Cannot Detect the Founding Defect

`tests/manual/readme.md § C1` states it flatly:

> `fetch_add` produces no duplicate grants (the add is atomic, so every producer
> still gets a distinct start), so the *only* symptom is a grant that runs past
> the gate.

So of the five concurrency tests:

| Test | Passes against the rejected `fetch_add` design |
|------|:---------------------------------------------:|
| `no_two_producers_are_ever_granted_the_same_sequence` | **✔ passes** |
| `claims_under_contention_lose_no_sequences` | **✔ passes** |
| `claim_up_to_under_contention_loses_no_sequences_either` | **✔ passes** |
| `each_producers_own_claims_stay_in_issue_order` | **✔ passes** |
| `no_grant_ever_passes_the_limit_under_contention` | ✘ fails |

Four of five tests, roughly 150 lines of the file, 8,000 grants across four
threads — and none of it distinguishes the correct implementation from the one
the module documentation spends thirteen lines rejecting. One test does, and
`§ C1` exists to prove that it does, five runs out of five.

This is not an argument that the four are wasteful. They assert a real
requirement, they would catch a genuinely different class of bug — a `Claim`
whose `len` disagreed with what was exchanged, a cursor assigned rather than
exchanged — and `§ C2` guards those separately. It is an argument that **test
volume on an invariant is not evidence about a different invariant**, and this
file is a clean example of the two being easy to confuse.

### How the Exclusivity Tests Establish the Property

They do not assert `overlaps` pairwise — that would be O(n²) over 8,000 claims.
They flatten to sequences and check a partition:

```rust
let mut every : Vec< Seq > = ranges.iter().flatten().flat_map( | c | c.sequences() ).collect();
assert_eq!( every.len(), TOTAL, "a producer lost a grant" );

every.sort_unstable();
every.dedup();
assert_eq!( every.len(), TOTAL, "two producers were granted the same sequence" );

assert_eq!( every.first().copied(), Some( Seq::ZERO ) );
assert_eq!( every.last().copied(), Some( Seq( TOTAL as u64 - 1 ) ) );
assert_eq!( claimer.claimed(), Seq( TOTAL as u64 ) );
```

Five assertions covering four distinct failures — a lost grant, a duplicate, a
skipped sequence at either end, and a cursor that disagrees with what was
handed out. `dedup` after `sort_unstable` is what turns exclusivity into a
length comparison, and the first/last checks are what turn it into
*contiguity* — the property that no sequence was silently skipped, which
duplicate-detection alone would miss.

The pairwise version does exist, single-threaded and exhaustive:
`overlap_is_symmetric_and_detects_every_shared_sequence` checks 900 range pairs
against a from-first-principles definition, which is where `overlaps` itself is
verified.

### Invariants

| File | Relationship |
|------|--------------|
| [002_a_refused_claim_moves_nothing.md](002_a_refused_claim_moves_nothing.md) | The other invariant, tested under the opposite configuration |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_the_gate_inside_the_retry.md](../algorithm/001_the_gate_inside_the_retry.md) | The exchange that establishes this property |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_compare_exchange_rather_than_fetch_add.md](../decisions/001_compare_exchange_rather_than_fetch_add.md) | The design this invariant cannot distinguish |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_sixteen_bytes_and_one_hundred_twenty_eight.md](../data_structure/001_sixteen_bytes_and_one_hundred_twenty_eight.md) | The missing `Clone` derive, which would break this without a race |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_eight_readings_of_a_range.md](../item/001_the_eight_readings_of_a_range.md) | `overlaps`, public so this invariant can be asserted directly |

### Sources

| File | Relationship |
|------|--------------|
| `ring_claim/src/lib.rs:20-23` | The requirement, and why it justifies a crate |
| `ring_claim/src/lib.rs:210-235` | `overlaps`, and its stated purpose |
| `ring_claim/src/lib.rs:436-447` | The exchange that holds it |
| `ring_gating/src/lib.rs:221-228` | The zero-consumer case that makes the gate inert |

### Tests

| File | Relationship |
|------|--------------|
| `tests/claim_test.rs:11-31` | The header — all five multi-threaded tests named, and a second broken implementation |
| `tests/claim_test.rs:112` — `overlap_is_symmetric_and_detects_every_shared_sequence` | 900 pairs against a first-principles definition |
| `tests/claim_test.rs:320` — `no_two_producers_are_ever_granted_the_same_sequence` | 8,000 grants, five assertions, gate disabled |
| `tests/claim_test.rs:356` — `claims_under_contention_lose_no_sequences` | Multi-slot claims, same partition check |
| `tests/claim_test.rs:440` — `each_producers_own_claims_stay_in_issue_order` | Per-producer monotonicity; global order deliberately not asserted |
| `tests/manual/readme.md § C1` | Which of the five actually fails against the rejected design |
