# Pitfall: Reversing the Two Refusals

### Scope

- **Purpose**: Trace how far a reversal of `check`'s two tests gets before anything in the repository notices.
- **Responsibility**: Show the reversed body, walk the suite against it, identify the single assertion that catches it, and assess whether the crate's structural check is the right defence.
- **In Scope**: The consequences of the swap.
- **Out of Scope**: Why the order is what it is — see [`algorithm/002`](../algorithm/002_check_orders_its_two_refusals.md).

### The Mistake

```rust
// Reversed — headroom tested before capacity.
pub fn check( &self, producer : Seq, count : usize ) -> Result< (), RingError >
{
  if count > self.headroom( producer )
  {
    return Err( RingError::Full );
  }
  if count > self.capacity.get()
  {
    return Err( RingError::BatchTooLarge { requested : count, capacity : self.capacity.get() } );
  }
  Ok( () )
}
```

Nothing about it looks wrong. Both refusals are still present, both still carry
correct data, and the second `if` is even *more* obviously reachable to a reader
scanning top-down — the cheap test now comes second, which reads like an
optimisation.

It is wrong because `headroom <= capacity` always holds, so every `count`
exceeding capacity fails the *first* test too. The second `if` becomes
unreachable for exactly the inputs it was written for.

### How Far It Gets

| Stage | Result |
|-------|--------|
| `cargo build` | ✅ passes |
| `cargo clippy --all-targets --all-features -- -D warnings` | ✅ passes — no lint fires on a reachable-but-dead branch of this shape |
| `cargo test --doc -p ring_gating` | ❌ **fails** — `check`'s own doctest asserts `too_wide.is_configuration()` |
| `cargo nextest run -p ring_gating` | ❌ fails — one test |
| Every other crate's suite | ✅ passes — nothing outside calls `check` |

Two failures, both inside this crate, and one of them is a doctest rather than a
test. That doctest is worth naming, because it is doing real work:

```rust
// ring_gating/src/lib.rs:277-281
let too_wide = set.check( Seq::ZERO, 5 ).unwrap_err();
assert!( too_wide.is_configuration(), "never retry this one" );

assert_eq!( set.check( Seq( 3 ), 2 ), Err( RingError::Full ) );
assert!( !RingError::Full.is_configuration(), "but do retry this one" );
```

A documentation example that fails under a reversal is a better defence than most
tests, because it is the thing a reader consults to learn the contract.

### The Single Assertion in the Test Suite

```rust
// tests/gating_test.rs:241-250
fn the_two_failures_are_distinguished_at_the_boundary()
{
  let set = set_at( 4, &[ 0 ] );

  assert_eq!( set.check( Seq( 4 ), 4 ), Err( RingError::Full ) );
  assert!( set.check( Seq( 4 ), 5 ).unwrap_err().is_configuration() );
}
```

The second line is what catches the overlap case. Walking every `check`
assertion in the crate against the reversed body:

| Assertion site | Input | Reversed returns | Verdict |
|----------------|-------|------------------|:-------:|
| `:222-229` — wider than the ring | `check( ZERO, 5 )`, headroom 4 | `Full` | ❌ caught — asserts the exact `BatchTooLarge` variant |
| `:232-238` — merely does not fit yet | `check( Seq( 3 ), 2 )`, headroom 1 | `Full` | ✅ passes — `Full` is correct either way |
| `:241-250` — both, at the overlap | `check( Seq( 4 ), 5 )`, headroom 0 | `Full` | ❌ caught — asserts `is_configuration()` |
| `:253-275` — the 1,920-state sweep | every state | `Full` where `BatchTooLarge` was due | ✅ passes |
| `:278-287` — a zero-length claim | `check( Seq( 4 ), 0 )` | `Ok` | ✅ passes |
| `check`'s doctest (`:277-281`) | `check( ZERO, 5 )`, headroom 4 | `Full` | ❌ caught |

Three of six. The three that pass do so for the same reason in different
disguises: each asks a question whose answer the reversal does not change.

**The largest test in the file is not among the three.** The 1,920-state sweep
exercises the reversed code path 1,920 times and reports success every time,
because it only ever asks whether the result is `Ok` — `RingError::Full` and
`RingError::BatchTooLarge` are equally not-`Ok`.

That is worth stating plainly: **the most thorough test in the crate is blind to
this bug by construction**, and the two that catch it are three-line assertions.

### Why the Structural Check Is the Right Defence

The crate's manual plan adds check M4, and its rationale is exact:

> `the_two_failures_are_distinguished_at_the_boundary` asserts the outcome; this
> check asserts the structure that produces it, because a `headroom`-first
> implementation can be made to pass that one test by special case.

```sh
cd "$(git rev-parse --show-toplevel)"
grep -vE "^[[:space:]]*(///|//!)" ring_gating/src/lib.rs \
  | grep -E "RingError::(BatchTooLarge|Full)"
```

Live output:

```
      return Err( RingError::BatchTooLarge { requested : count, capacity : self.capacity.get() } );
      return Err( RingError::Full );
```

**Expected:** two hits, `BatchTooLarge` on the lower line number.

The argument generalises. A property whose entire content is *"A is written
before B"* is checked by reading the order. An outcome test can always be
satisfied by a special case — here, a reversed `check` with an added
`if count > capacity` guard at the top would pass all six tests and the doctest,
while being the same bug with a patch over the one input anyone enumerated.

The structural check has no such hole, and it costs one `grep`.

### What Would Make It Cheaper Still

| Option | Cost | Effect |
|--------|------|--------|
| Run M1–M4 in CI | A shell step | Turns four manual reads into build failures |
| A test asserting `check` returns `BatchTooLarge` for *every* over-wide count across a headroom sweep | ~6 lines | Closes the special-case hole from the outcome side |
| Extend the 1,920-state sweep to compare the *error variant*, not just `is_ok()` | One assertion | Makes the largest test see the bug |

The third is the highest value for the least change: the sweep already visits
every relevant state and already computes both readings — it simply discards the
part that distinguishes them. Recorded rather than applied; a test change belongs
to a run with its own verification.

### GT49 — Two Assertions Catch the Reversal

```
a_claim_wider_than_the_ring_is_a_configuration_error
      check( Seq::ZERO, 5 ) on 4 free slots -> reversed: Full, assert_eq! fails
the_two_failures_are_distinguished_at_the_boundary
      check( Seq( 4 ), 5 )  on a full ring -> reversed: Full, is_configuration fails
```

The reversal is a one-line edit that turns a permanent error into a retryable
one. What would notice is worth counting precisely, and the two that would do so
are not equally sharp.

**Finding.** Two assertions catch it, in two tests. `a_claim_wider_than_the_ring_is_a_configuration_error` fails at its `assert_eq!` — under the reversed order `check( Seq::ZERO, 5 )` on a set with four free slots returns `Full` before the width is ever tested — and `the_two_failures_are_distinguished_at_the_boundary` fails at `check( Seq( 4 ), 5 ).unwrap_err().is_configuration()`. The second is the sharper instrument: it holds the two refusals apart on a *full* ring, where only the ordering distinguishes them

---

### GT50 — A Claim of Zero Is Always Admitted

```
check( producer, 0 ) on a completely full ring
  0 > capacity  -> false
  0 > headroom  -> false, even when headroom == 0
  => Ok( () )
```

Correct, and worth stating: the success arm means "this claim is admissible",
not "the ring has room". On a full ring those differ for exactly one value.

**Finding.** `check( producer, 0 )` succeeds on a completely full ring — `0 > capacity` is false, and `0 > headroom` is false even when `headroom` is zero. That is correct, and it means `Ok` does not imply the ring had room, which is the one reading of the success arm a caller might make and should not

---


### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_check_orders_its_two_refusals.md](../algorithm/002_check_orders_its_two_refusals.md) | Why the order is load-bearing |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_a_result_rather_than_a_bool.md](../decisions/002_a_result_rather_than_a_bool.md) | Why the two refusals are distinguished at all, and who does not call `check` |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_three_gating_readings.md](../item/001_the_three_gating_readings.md) | `check`'s outcome table |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_the_gate_must_never_over_report.md](../non_functional_requirement/002_the_gate_must_never_over_report.md) | The other place a test's name outruns its assertion |

### Pitfalls

| File | Relationship |
|------|--------------|
| [001_unwrapping_the_empty_set_to_zero.md](001_unwrapping_the_empty_set_to_zero.md) | The caller-side one-line mistake |

### Sources

| File | Relationship |
|------|--------------|
| `ring_gating/src/lib.rs:283-294` | `check`, in the correct order |
| `ring_gating/src/lib.rs:277-281` | The doctest that catches the reversal |
| `ring_types/src/error.rs:110-159` | The two predicates the distinction feeds |
| `tests/manual/readme.md` § M4 | The structural check, and its rationale |

### Tests

| File | Relationship |
|------|--------------|
| `tests/gating_test.rs:222-229` | Caught — asserts the exact variant |
| `tests/gating_test.rs:241-250` | Caught — the overlap case |
| `tests/gating_test.rs:253-275` | Not caught — compares `is_ok()` only |
