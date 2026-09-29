# invariant

Three properties, two files, and a coverage shape that is worth looking at
directly. `ring_claim` promises that no two producers are ever granted the same
sequence, that a refused claim leaves the cursor exactly where it found it, and
that no granted range extends past the slowest consumer. The first is the
crate's reason to exist; the third is the one its founding decision was made to
protect; the second is the small one that makes retrying safe and without which
neither of the others is usable.

The first file takes exclusivity and the second takes the other two, because
that is how the tests group: exclusivity is verified with the gate switched off,
and the gate is verified with contention throttled down to almost nothing. Four
of the five concurrent tests sit on one side of that line and the fifth sits on
the other — and the fifth, it turns out, reaches back across it.

That split is defensible in each direction — a producer blocking on a full ring
is a producer not creating contention, and a gate that never binds cannot be
overrun — but it produces a corpus where the invariant carrying four fifths of
the test code is not the invariant that catches the bug the crate exists to
avoid. Both files end up saying some version of that, from opposite ends.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [No Two Producers Hold One Sequence](001_no_two_producers_hold_one_sequence.md) | CL23, CL24 — the four exclusivity tests running with a zero-consumer gate, the header that counts three of five and names a second broken implementation, and the partition assertion that turns O(n²) into a sort |
| 002 | [A Refused Claim Moves Nothing](002_a_refused_claim_moves_nothing.md) | CL25, CL26 — the failure that issues no atomic against the success that issues the crate's most expensive one, and why the limit is read *after* the grant |

### The Two Configurations

Laid side by side, the exclusivity tests and the gate test are near-mirror
opposites on every axis a concurrency test has:

| | Exclusivity (001) | The gate (002) |
|--|-------------------|----------------|
| Capacity | 16,384 (×3), 8,192 | **4** |
| Consumers in the `GatingSet` | **0** | 1 |
| Gate behaviour | inert — `headroom` returns capacity unconditionally | binds on essentially every call |
| Producer threads | 4, 4, 4, 3 | 3 |
| What limits progress | nothing | a consumer releasing 4,000 times, yielding after each |
| Claim calls per run | 8,000 / 2,000 / 2,000 / 3,000 | 9,000 attempted, most refused |
| Assertion shape | a partition over every range collected at the end | a comparison per grant, against a bound read live |
| Fails against the rejected design | **no** | yes |

Neither column is a mistake. Maximising contention requires removing the thing
that makes producers wait; exercising the gate requires making them wait
constantly. These are genuinely incompatible pressures, so a test serves one or
the other.

The gate test does reach across, partially: after asserting its own bound per
grant, it also sorts and dedups every range it collected, so exclusivity is
checked under a binding gate too. What it cannot check there is contiguity —
the number of sequences a run *should* produce is unknowable when claims are
refused at an unpredictable rate, so there is nothing to compare a length
against. That is the real reason the zero-consumer configuration exists: not
merely more contention, but the only configuration in which the expected total
is a constant the test can assert against.

| | Gate off (4 tests) | Gate binding (1 test) |
|--|:------------------:|:---------------------:|
| No two producers share a sequence | ✔ | ✔ |
| No sequence skipped | ✔ | **✘ — not assertable** |

### Why They Can Be Opposites Without Either Being Wrong

The two invariants are independent, and the independence is structural rather
than incidental:

| Remove | Exclusivity | The gate |
|--------|-------------|----------|
| the `GatingSet` | **still holds** — the exchange alone decides who wins | gone |
| the compare-exchange (use `fetch_add`) | **still holds** — the add is atomic | **broken** |

Exclusivity rests entirely on `compare_exchange` linearising, and the gate rests
entirely on the gate being *inside* the retry. Each mechanism holds one property
and neither holds the other, which is why the tests can be configured as
opposites without either configuration undermining what it is asserting.

The residue is small and nameable, and it is the contiguity half: nothing
verifies that a producer refused by the gate and retrying leaves **no hole** in
the sequence space. That it gets a *disjoint* range is covered — the tight-gate
test dedups. That it leaves no gap is only ever checked with the gate off.

Today both retries run the same loop body, so the two paths cannot diverge. The
day someone splits them, that is the gap.

### The Volume Inversion

The two files reach the same observation from opposite ends, and stated together
it is the most useful thing in this definition:

| | Exclusivity | The gate |
|--|-------------|----------|
| Tests asserting it | 4 concurrent + 1 exhaustive pairwise | 1 concurrent + 1 sequential |
| Lines of concurrent test code | **156** (`:306-461`) | **71** (`:462-532`) |
| Claim calls exercised | 15,000 | 9,000 attempted, capacity 4 |
| Distinguishes the rejected design from the shipped one | **no** | **yes** |

Four tests, 156 lines and 15,000 claim calls, and the whole of it passes
unchanged against the implementation the module documentation spends thirteen
lines rejecting. One test, 71 lines, capacity 4 — and it is the one that fails.

`tests/manual/readme.md § C1` exists because that is not obvious from reading
the file. It mutates the implementation and runs the suite five times, and the
value it delivers is not "the test works" but *which* test works, measured
rather than assumed.

The reading to take from this: **test volume on an invariant is evidence about
that invariant and nothing else.** These two invariants are both real, both
required, and both correctly tested — and the crate's total test weight
distributes almost inversely to which property is fragile.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order

# every gate configuration in the suite — the 0-consumer ones are the exclusivity tests
grep 'GatingSet::new' ring_claim/tests/claim_test.rs

# how many concurrent tests there actually are, against what the header claims
grep -c 'thread::scope' ring_claim/tests/claim_test.rs
command grep -m1 -A10 -F '//! ## What a sequential test cannot show' ring_claim/tests/claim_test.rs | tail -n 10

# the zero-consumer case that makes the gate inert
command grep -m1 -A7 -F '  pub fn headroom( &self, producer : Seq ) -> usize' ring_gating/src/lib.rs

# the only mutation in the crate, and the only place it can fail
grep -vE "^[[:space:]]*//" ring_claim/src/lib.rs | grep -E "compare_exchange"

# who reads the bound I2 is stated against
command grep -r '\.limit()' ring_*/src/*.rs ring_*/tests/*.rs
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
  pub fn headroom( &self, producer : Seq ) -> usize
  {
    self.slowest().map_or( self.capacity.get(), | slowest |
    {
      ring_seqno::free_slots( producer, slowest, self.capacity )
    } )
  }

      match self.cursor.compare_exchange( current, next, CLAIM_SUCCESS, GATING )
      match self.cursor.compare_exchange( current, next, CLAIM_SUCCESS, GATING )
ring_gating/src/lib.rs:  /// assert_eq!( set.limit(), Some( Seq( 8 ) ) );
ring_gating/src/lib.rs:  /// assert_eq!( set.limit(), Some( Seq( 13 ) ) );
ring_claim/tests/claim_test.rs:          let limit = consumers.limit().expect( "one consumer" );
ring_gating/tests/gating_test.rs:  let limit = set.limit().expect( "one consumer, so a limit exists" );
ring_gating/tests/gating_test.rs:  assert_eq!( ungated.limit(), None );
ring_gating/tests/gating_test.rs:  assert_eq!( set.limit(), Some( Seq( 13 ) ) );
ring_gating/tests/gating_test.rs:  let limit = set.limit().unwrap();
```

| | Value |
|--|------:|
| `GatingSet::new` calls in the suite | 20 |
| …with 0 consumers (gate inert) | **4** |
| …with 1 consumer | 15 |
| …with 2 consumers | 1 |
| `thread::scope` blocks | **5** |
| …named in the module header | 3 |
| Broken implementations the header names | **2** |
| …with a mutation check behind them | 1 |
| Capacity in the exclusivity tests | 16,384 / 8,192 |
| Capacity in the gate test | **4** |
| Tests that fail against the rejected design | **1 of 5** |
| Atomic mutations in the crate | 2, both `compare_exchange` |
| Callers of `GatingSet::limit` outside `ring_gating` | **1** — `claim_test.rs:524` |
| `loom` models in this crate | **0** |

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CL23 | `ring_claim` | n/a — coverage | Four of the five concurrent tests build their `GatingSet` with **zero consumers**, which `ring_gating::headroom` answers with the full capacity unconditionally, so the gate never refuses; the crate's two invariants are verified under configurations that are mirror opposites on every axis a concurrency bug lives on |
| CL24 | `ring_claim` | n/a — coverage | The invariant with the most test code behind it cannot detect the crate's founding defect: `fetch_add` claiming produces no duplicate grants, so 4 of 5 concurrent tests pass against it unchanged |
| CL25 | `ring_claim` | n/a — observation | The two calls that leave the cursor unmoved differ in every other respect: `claim( 1 )` on a full ring returns `Err` and issues no atomic at all, while `claim( 0 )` returns `Ok` and issues a `compare_exchange` — the cheap path is the failing one, which is the opposite of what a reader predicts |
| CL26 | `ring_claim` | n/a — observation | The contention test reads the consumer limit *after* each grant, and monotonicity is what makes that both sound and sensitive; reading it before would fail correct implementations whenever a consumer advanced in the window |

Supporting observations recorded alongside those findings, carrying no ID of their own:

| Note | Where |
|------|-------|
| The tight-gate test reaches across the split — it dedups its collected ranges, so exclusivity *is* asserted with the gate binding — but it cannot assert **contiguity** there, because refusals make the expected sequence total unknowable; that is the actual reason a zero-consumer configuration is needed, and it leaves "a gate-refused retry leaves no hole" as the one genuinely uncovered case | [001](001_no_two_producers_hold_one_sequence.md) |
| The test file's header claims "the three multi-threaded tests below" and names three; the file contains five, and one omitted test is the only concurrent coverage of `claim_up_to` | [001](001_no_two_producers_hold_one_sequence.md) |
| The same header names a *second* broken implementation — a CAS retry that recomputes the range but not the start — which is guarded by an assertion but by no mutation run, unlike the `fetch_add` design | [001](001_no_two_producers_hold_one_sequence.md) |
| Exclusivity is asserted as a partition — flatten, sort, dedup, compare lengths, then check both ends — turning an O(n²) pairwise property into two length comparisons, with the pairwise version kept single-threaded and exhaustive at 900 pairs | [001](001_no_two_producers_hold_one_sequence.md) |
| `assert!( granted > 0 )` at the end of the contention test is the assertion most concurrency tests omit — without it, a run whose loop body never executed reports success | [002](002_a_refused_claim_moves_nothing.md) |
| The crate has no `loom` model and its own test header says so, pointing at `ring_publish/tests/handshake_test.rs`; `ring_publish`'s simpler algorithm got the exhaustive model, and this crate's retry loop got a statistical test whose sensitivity was measured by hand | [002](002_a_refused_claim_moves_nothing.md) |

