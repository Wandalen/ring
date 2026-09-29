# API: The Two Constructors of a Range

### Scope

- **Purpose**: Account for the two fallible signatures — the only two ways a `Claim` is legitimately obtained — their shared return type, and the error contract that tells a caller which failures are worth retrying.
- **Responsibility**: Place both signatures against the family's 39 fallible ones, establish where the two error variants come from and what distinguishes them, and record where the contract is stated more than once.
- **In Scope**: `Claimer::claim`, `Claimer::claim_up_to`, `Result< Claim, RingError >`, and the two variants they produce.
- **Out of Scope**: The loops behind the signatures — see [`algorithm/002`](../algorithm/002_two_loops_that_disagree_at_zero.md).

### The Two Signatures

```rust
pub fn claim( &self, count : usize ) -> Result< Claim, RingError >         // :388
pub fn claim_up_to( &self, max : usize ) -> Result< Claim, RingError >     // :442
```

Identical types, opposite promises: `claim` grants exactly `count` or nothing;
`claim_up_to` grants between one and `max`. Nothing in the signature
distinguishes them, which is why the parameter is named `count` in one and `max`
in the other — the name is the only in-signature signal of which contract
applies.

### Against Every Fallible Signature in the Family

```sh
cd "$(git rev-parse --show-toplevel)"
grep -rhE '^\s*pub (const )?fn .*-> *Result<' ring_*/src/*.rs \
  | sed 's/.*-> *//' | sed 's/.*, *//;s/ *>.*//' | sort | uniq -c | sort -rn
```

Live output:

```
     21 RingError
      4 WorkloadError
      3 Violation
      3 T
      3 Anomaly
      2 BuildError
      1 Seq
      1 RunError
      1 Refusal< T
```

| Error type | Signatures | Where |
|------------|-----------:|-------|
| `RingError` | 21 | the family's one shared error type — **both of this crate's are here** |
| a crate-local error | 13 | harness, debug and factory crates |
| a generic `T` | 3 | passthroughs |
| `Seq` | 1 | `ring_publish::try_publish` |

This crate sits in the majority. Its Tier 5 counterpart is the outlier:
`ring_publish` returns `Result< Seq, Seq >` because its only
non-success outcome is *not yet*, and no `RingError` variant names that
([`ring_publish/docs/api/002`](../../../ring_publish/docs/api/002_a_result_whose_error_is_not_an_error.md)).
Claiming's non-success outcomes are ordinary failures — no room, or a request no
ring of this size could ever satisfy — so the shared type fits.

### CL13 — Two Variants, and the Distinction Is a Retry Instruction

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'RingError::' ring_claim/src/lib.rs | grep -v '///'
grep 'is_configuration\|is_transient' ring_types/src/error.rs
```

Live output:

```
      return Err( RingError::BatchTooLarge
    Err( RingError::Full )
    Err( RingError::Full )
/// assert!( !e.is_configuration() );
  /// assert!( RingError::CapacityZero.is_configuration() );
  /// assert!( RingError::CapacityNotPowerOfTwo( 3 ).is_configuration() );
  /// assert!( !RingError::Full.is_configuration() );
  //   second look. `is_transient` just below carried the identical shape.
  pub const fn is_configuration( self ) -> bool
  /// assert!( RingError::Full.is_transient() );
  /// assert!( RingError::Empty.is_transient() );
  /// assert!( !RingError::Closed.is_transient() );
  // Fix(ring_error_classification_not_exhaustive): same shape as `is_configuration`
  // Root cause: see `is_configuration` above — an exhaustive match was available
  pub const fn is_transient( self ) -> bool
```

| Variant | Produced by | Carries | Retry? |
|---------|-------------|---------|--------|
| `RingError::BatchTooLarge { requested, capacity }` | `claim` only, `:425-429` | both numbers | **never** — no consumer's progress can fix it |
| `RingError::Full` | both, at the loop exit | nothing | **yes** — it is back-pressure |

`ring_types::error` classifies these with two predicates, `is_configuration` and
`is_transient`, and the whole point of the split is that a retry loop must
branch on it. Spinning on `BatchTooLarge` is an infinite loop by construction:
the condition is `count > capacity`, and neither operand changes.

The contract is stated in this crate three times, in three registers:

| Where | Form |
|-------|------|
| `src/lib.rs:394-399` | `claim`'s `# Errors` section, in prose |
| `src/lib.rs:448-450` | `claim_up_to`'s, stating the *absence* of `BatchTooLarge` |
| `ring_gating:246-252` | `GatingSet::check`'s, in almost the same words |

The third is the interesting one. `GatingSet::check` produces the same two
variants from the same two conditions and explains them with the same
distinction — *"a configuration error that no consumer's progress can fix, so a
retry loop must stop rather than spin"* against this crate's *"a configuration
error no consumer's progress can fix, so a retry loop must stop"*. The wording
is nearly identical because the argument is the same argument, written twice in
two crates, for a function this crate cannot call
([`integration/002`](../integration/002_four_predicates_and_the_one_that_is_called.md) § CL5).

### CL14 — Four of Seventeen Public Items Touch an Atomic

```sh
cd "$(git rev-parse --show-toplevel)"
grep -E 'self\.cursor\.(load|compare_exchange)|self\.consumers\.headroom' \
  ring_claim/src/lib.rs | grep -v '///'
```

Live output:

```
    self.cursor.load( GATING )
    self.consumers.headroom( self.claimed() )
    while count <= self.consumers.headroom( current )
      match self.cursor.compare_exchange( current, next, CLAIM_SUCCESS, GATING )
    while let granted @ 1.. = max.min( self.consumers.headroom( current ) )
      match self.cursor.compare_exchange( current, next, CLAIM_SUCCESS, GATING )
```

| Group | Items | Atomic operations |
|-------|------:|-------------------|
| `Claim` — the type and its eight methods | 9 | **none** |
| `Claimer` — the type itself | 1 | none |
| `Claimer` — `new`, `cursor`, `consumers` | 3 | none |
| `Claimer` — `claimed`, `headroom` | 2 | one `load` each |
| `Claimer` — `claim`, `claim_up_to` | 2 | one or more `compare_exchange` |

Thirteen of seventeen items perform no atomic operation at all, and the type
that gives the crate its name — `Claim` — performs none anywhere. That is the
shape the crate's testability depends on: the range arithmetic is a pure value
type with six single-threaded tests over it
(`tests/claim_test.rs:42-123`), and the concurrency lives in two methods with
five `thread::scope` blocks aimed at them.

It also explains why `Claim` is `Copy` and `Claimer` is not. A `Claim` is a pair
of integers describing a range; copying one costs nothing and shares nothing. A
`Claimer` owns a 64-byte cursor other threads are exchanging against, and
copying one would fork the cursor — two producers each advancing their own
private copy, granting the same sequences forever
([`data_structure/001`](../data_structure/001_sixteen_bytes_and_one_hundred_twenty_eight.md)).

### What a Caller Writes

The two signatures compose with `ring_wait` rather than embedding a wait, which
`src/lib.rs:25-35` states as a requirement:

> A claim that waited internally would force the wait strategy into this crate
> and make `WaitKind::None` unimplementable above it.
>
> The caller that wants to wait composes: `ring_wait::for_space`, then
> [`Claimer::claim`].

So a spinning producer, a parking producer and a tick-path producer that must
never block all call the same two functions and differ only in what they do with
`Err( RingError::Full )`. There is no `claim_blocking`, no `WaitKind` parameter,
and no feature flag — the branch lives at the call site, which is the only place
that knows which of the three it is.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_two_loops_that_disagree_at_zero.md](../algorithm/002_two_loops_that_disagree_at_zero.md) | The two bodies behind the two signatures |

### APIs

| File | Relationship |
|------|--------------|
| [001_seventeen_items_and_nothing_that_drops_silently.md](001_seventeen_items_and_nothing_that_drops_silently.md) | The full surface these two sit on |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_sixteen_bytes_and_one_hundred_twenty_eight.md](../data_structure/001_sixteen_bytes_and_one_hundred_twenty_eight.md) | Why one of the two types is `Copy` and the other cannot be |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_four_predicates_and_the_one_that_is_called.md](../integration/002_four_predicates_and_the_one_that_is_called.md) | The function whose `# Errors` section is this one written twice |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_a_seq_a_usize_and_three_casts.md](../type/001_a_seq_a_usize_and_three_casts.md) | `Seq`, `usize`, and the boundary between them |

### Sources

| File | Relationship |
|------|--------------|
| `ring_claim/src/lib.rs:25-35` | Why neither signature takes a wait strategy |
| `ring_claim/src/lib.rs:387-421` | `claim`'s doc, `# Errors`, and signature |
| `ring_claim/src/lib.rs:450-480` | `claim_up_to`'s, and the variant it never produces |
| `ring_types/src/error.rs` | `RingError`, `is_configuration`, `is_transient` |
| `ring_gating/src/lib.rs:247-294` | The same two variants, from the same two conditions, in another crate |
| `ring_publish/src/lib.rs:138-165` | The Tier 5 signature that went the other way |

### Tests

| File | Relationship |
|------|--------------|
| `tests/claim_test.rs:186` — `a_claim_wider_than_the_ring_is_a_configuration_error` | `BatchTooLarge`, with both payload fields asserted |
| `tests/claim_test.rs:197` — `a_claim_that_does_not_fit_yet_is_back_pressure` | `Full`, and the classification that distinguishes it |
| `tests/claim_test.rs:249` — `claim_up_to_never_reports_batch_too_large` | The variant the second signature cannot produce |
