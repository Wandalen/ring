# pitfall

Two ways to be wrong about this crate. Both are one line long, both compile, and
both leave most of the test suite green.

### Overview Table

| ID | Name | The mistake |
|----|------|-------------|
| 001 | [Unwrapping the Empty Set to Zero](001_unwrapping_the_empty_set_to_zero.md) | Resolving `slowest() == None` to `Seq::ZERO` in a caller, undoing the distinction the crate exists to carry |
| 002 | [Reversing the Two Refusals](002_reversing_the_two_refusals.md) | Swapping `check`'s two `if`s, turning a permanent error into a retryable one |

### Why Both Are Cheap to Make

| Mistake | Compiles | Sequential tests | Concurrent test | Caught by |
|---------|:--------:|:----------------:|:---------------:|-----------|
| `unwrap_or( Seq::ZERO )` in a caller | ✅ | ✅ pass — it is outside the crate | ✅ pass | Nothing in this repository |
| Reversed refusals | ✅ | ❌ one test fails | ✅ pass | `the_two_failures_are_distinguished_at_the_boundary`, and manual check M4 |

The first is the more dangerous, because the failing behaviour is in a caller
this crate's suite cannot see.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| GT47 | Disposal in the family | n/a — observation | Every `Option< Seq >` disposal in the family is either a justified `map_or` or an `expect` with a stated precondition; four of the five `unwrap`-family calls name their reason, one does not. There is no bare `.unwrap()` in any of the 33 crates' production source, which is what makes the pitfall documented here hypothetical rather than latent |
| GT48 | What guards the pitfall | n/a — observation | Both ungated tests do, and only the second is load-bearing: a variant returning the capacity at sequence zero and zero after one lap passes `an_ungated_ring_has_a_full_capacity_of_headroom` and fails `an_ungated_ring_and_a_consumer_at_zero_disagree_after_one_lap` |
| GT49 | The reversal | n/a — observation | Two assertions catch it, in two tests. `a_claim_wider_than_the_ring_is_a_configuration_error` fails at its `assert_eq!` — under the reversed order `check( Seq::ZERO, 5 )` on a set with four free slots returns `Full` before the width is ever tested — and `the_two_failures_are_distinguished_at_the_boundary` fails at `check( Seq( 4 ), 5 ).unwrap_err().is_configuration()`. The second is the sharper instrument: it holds the two refusals apart on a *full* ring, where only the ordering distinguishes them |
| GT50 | A claim of zero | n/a — observation | `check( producer, 0 )` succeeds on a completely full ring — `0 > capacity` is false, and `0 > headroom` is false even when `headroom` is zero. That is correct, and it means `Ok` does not imply the ring had room, which is the one reading of the success arm a caller might make and should not |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the safe disposal of the empty set --'
command grep 'map_or( self.capacity.get()' ring_gating/src/lib.rs
echo '  -- bare unwraps in the family production source --'
command grep -rE '\.unwrap\(\)' --include=*.rs ring_*/src/ | command grep -v '//' | wc -l
echo '  -- the order of the two refusals --'
command grep -E '^    if count > ' ring_gating/src/lib.rs
echo '  -- the assertion that catches the reversal --'
command grep 'is_configuration()' ring_gating/tests/gating_test.rs
```

Live output:

```
  -- the safe disposal of the empty set --
    self.slowest().map_or( self.capacity.get(), | slowest |
  -- bare unwraps in the family production source --
0
  -- the order of the two refusals --
    if count > self.capacity.get()
    if count > self.headroom( producer )
  -- the assertion that catches the reversal --
  assert!( err.is_configuration(), "no consumer's progress can ever make this fit" );
  assert!( !RingError::Full.is_configuration(), "a retry loop must keep going on this one" );
  assert!( set.check( Seq( 4 ), 5 ).unwrap_err().is_configuration() );
```

**Both pitfalls are one line each, and only the second has a detector.** The
second arm is why the first pitfall is hypothetical family-wide: there is no bare
`.unwrap()` in any of the 33 crates' production source. Swapping the two `if`s,
by contrast, makes a too-wide claim report `Full` — retryable — and the
`is_configuration()` assertion in
[`002`](002_reversing_the_two_refusals.md)'s boundary test then fails.
