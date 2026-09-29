# Pitfall: The Run That Panics in Debug and Wraps in Release

### Scope

**Purpose:** Record that `run` reaching the top of the sequence space panics in
debug and wraps in release, that `ring_types` documents the release behaviour as
saturation when it is wraparound, and that the wrap produces exactly the silent
monotonicity violation the same doc comment says it was designed to avoid.

**Responsibility:** `run`'s addition, the `Seq` arithmetic under it, and the
build-configuration split in what happens at `u64::MAX`.

**In Scope:** `ring_index/src/lib.rs:118-122`;
`ring_types/src/id.rs:32-68`; `ring_mpsc/src/lib.rs:220-222`.

**Out of Scope:** `of` at the same inputs is safe and is
[`algorithm/001`](../algorithm/001_one_and_of_a_mask.md) IX2. The duplicate fold
is [`pitfall/002`](002_the_second_fold_nobody_noticed.md).

---

## The Same Call, Two Answers

`run( Seq( u64::MAX ), 2, cap 1024 )` asks for two slots starting at the last
sequence a `u64` can hold. Under `cargo run --release` and `cargo run`
respectively:

```
--- (4) the fold near the end of the sequence space ---   [RELEASE]
  of( u64::MAX, cap 1024 )     = SlotIndex(1023)
  of( u64::MAX - 1, cap 1024 ) = SlotIndex(1022)
  run( u64::MAX, 2 )           = [SlotIndex(1023), SlotIndex(0)]

--- (4) the fold near the end of the sequence space ---   [DEBUG]
  of( u64::MAX, cap 1024 )     = SlotIndex(1023)
  of( u64::MAX - 1, cap 1024 ) = SlotIndex(1022)

thread 'main' panicked at ring_types/src/id.rs:67:11:
attempt to add with overflow
  run( u64::MAX, 2 )           = PANICKED (advanced_by overflowed)
```

`of` gives the same answer in both builds, at every input, because masking
neither carries nor overflows. `run` does not, because it adds first.

---

### IX16 — The Overflow Is Inherited From a `+` in Another Crate

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -F '  ( 0..count as u64 ).map( | n | of( start.advanced_by( n ), capacity ) ).collect()' ring_index/src/lib.rs
command grep -m1 -A6 -F '  /// assert_eq!( Seq( 10 ).advanced_by( 0 ), Seq( 10 ) );' ring_types/src/id.rs | tail -n 5
```

Live output:

```
  ( 0..count as u64 ).map( | n | of( start.advanced_by( n ), capacity ) ).collect()
  #[ must_use ]
  pub const fn advanced_by( self, n : u64 ) -> Self
  {
    Self( self.0 + n )
  }
```

**Finding.** `run` performs `count` additions before it performs any fold, and
each one is a plain `+` on `u64` in `ring_types`. Not `checked_add`, not
`wrapping_add`, not `saturating_add` — the bare operator, whose behaviour is
defined by the build profile rather than by the code.

That places the hazard outside the crate that exhibits it. `ring_index` cannot
choose the overflow behaviour of `run` without changing `advanced_by`, and
`advanced_by` is `ring_types`' to change. What `ring_index` *could* do — and
does not — is say that `run` inherits it. `run`'s doc comment discusses batch
claims and wrapping-at-most-once; it has no `# Panics` section.

---

### IX17 — The Release Behaviour Is Wraparound, and `ring_types` Documents It as Saturation

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A5 -F '  /// The next sequence after this one.' ring_types/src/id.rs
```

Live output:

```
  /// The next sequence after this one.
  ///
  /// Panics on overflow in a debug build and wraps to zero in a release
  /// build — the standard `u64` addition behaviour. A wrapped `Seq` would
  /// silently invert every gate comparison in the family, which is why the
  /// non-wrapping argument has to hold: at 10⁹ publications per second a
```

Rust's `+` on `u64` does not saturate in release. It wraps:

```
  u64::MAX + 1, as Rust's `+` computes it in release : 0
  if it saturated, it would be                       : 18446744073709551615
  the two agree                                      : false
  and the slot each would fold to at capacity 1024   : 0 vs 1023
```

**Finding.** Three claims in that comment, and the middle one was false in a way
that inverted the other two.

"Saturates in a release build" was wrong — release-mode integer overflow wraps.
"Which is the standard `u64` addition behaviour" was right about the *mechanism*
and wrong about what it produces. And "deliberately not wrapping, since a
wrapped `Seq` would silently violate the monotonicity every gate in the family
relies on" named precisely the failure the code actually has: a release build
does wrap, a wrapped `Seq` does silently violate monotonicity, and every gate in
the family does rely on it.

The `run` output above is the demonstration rather than an argument about it.
`run( u64::MAX, 2 )` returns `[SlotIndex(1023), SlotIndex(0)]` in release. Slot
0 can only come from a sequence that folded to 0, and the only sequence two
steps into the run is `u64::MAX + 1`. Had the addition saturated, the second
element would be `SlotIndex(1023)` again — the same slot, repeated, which is
what saturation means here. It is not. The value wrapped to zero.

This was `ring_types`' comment, not this crate's, and correcting it was that
crate's to do. It has since been done, and in the shape this entry's own last
paragraph argued for. `next`'s comment now says "wraps to zero in a release
build" and keeps the 584-year figure as the reason that is survivable rather than
as a claim about the operator. And `advanced_by` — the method `run` actually
calls, and the one that had no overflow note at all — now carries one stating
that the reachability argument does **not** carry over from `next`: `n` comes
from the caller and reaches the wrap in a single call from any position.

That is the distinction this entry exists to make. It was recorded here because
`run` is where the wraparound becomes observable: `next` is called all over the
family and never near the boundary, while `run` reaches `start + ( count - 1 )`
in one call and will cross it for any sufficiently large `start`. What remains open is
this crate's own half — `run` still carries no `# Panics` section of its own, and
no test drives it across the boundary.

**Disposition:** declined — the false claim is a doc comment on `Seq::next` in
`ring_types/src/id.rs`, a different crate's file outside this crate's
own `src/`, `docs/`, `Cargo.toml`. `ring_index` neither wrote nor can correct
that comment from here; `run`'s own doc comment makes no saturation claim of
its own for this finding to fix.

---

### IX18 — `u64::MAX` Is Not a Hypothetical Input; the Family Reserves It

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rn 'u64::MAX' ring_*/src/*.rs | command grep -vE ':[0-9]+: *//[^/!]' \
  | sed 's|\.rs:[0-9]*:|.rs:  |'
```

Live output:

```
ring_atomic/src/lib.rs:    /// `u64`, and the addition wraps: `fetch_add( u64::MAX )` moves the cursor
ring_index/src/lib.rs:  /// from `ring_mpsc::UNSTAMPED`, the family's `Seq( u64::MAX )` sentinel,
ring_mpsc/src/lib.rs:  /// assert_eq!( ring_mpsc::UNSTAMPED, Seq( u64::MAX ) );
ring_mpsc/src/lib.rs:  pub const UNSTAMPED : Seq = Seq( u64::MAX );
ring_trace/src/lib.rs:    /// `ring_mpsc::UNSTAMPED` (`Seq(u64::MAX)`). See `pitfall/001` TR41.
```

**Finding.** `ring_mpsc` reserves `Seq( u64::MAX )` as its "this slot has never
been published" sentinel, and publishes it. So the exact value at which `Seq`
arithmetic changes behaviour by build profile is a named public constant that a
caller can hold, pass around, and — with nothing stopping them — hand to
`advanced_by`, `next`, or `run`.

Nothing connects the two facts. `UNSTAMPED`'s doc does not mention that
advancing it overflows; `advanced_by` has no `# Panics`; `run` has no `# Panics`;
and the two crates do not depend on each other in either direction, so no
compiler check or test could notice the pairing.

The practical exposure is small — a real ring would need `2^64` publications to
reach the boundary honestly, and `ring_types`' own test computes that as a
number of years — but the sentinel makes it reachable in one step rather than
`2^64`, from a constant the API invites you to use.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A5 -F '/// Feature 167'"'"'s central claim: the sequence never wraps. Asserted at the one' ring_types/tests/types_test.rs
```

Live output:

```
/// Feature 167's central claim: the sequence never wraps. Asserted at the one
/// place it could — a `u64` counting publications does not reach its ceiling in
/// any reachable workload, and the type deliberately offers no wrapping op.
#[ test ]
fn seq_does_not_wrap_within_any_reachable_workload()
{
```

The test proves the boundary is unreachable *by counting*, which is true and is
the case that is not the hazard. It says nothing about reaching it by
construction, which `UNSTAMPED` does in a single expression.

And its own doc comment closes the loop back to IX17: "the type deliberately
offers no wrapping op." The type offers `next` and `advanced_by`, both of which
wrap in release, because a bare `+` on `u64` is a wrapping op in release. The
claim, the comment on `next`, and the test's premise all describe a type that
saturates. Nothing in the family does.

`run` now documents the boundary this finding names, and the first of the two
"(to create)" tests in this file's own Tests table is written — the second
(`Seq( u64::MAX ).next()` in release) stays "(to create)": it asserts
`ring_types`' behaviour, a different crate outside this pass's scope.

`2>/dev/null` on the second command below drops cargo's own build-progress
chatter (compile timing and the hashed binary path both vary run to run),
leaving only the test's own deterministic stdout:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A6 -F '/// # Panics' ring_index/src/lib.rs
cargo test -p ring_index --test index_test run_overflows_at_the_top_of_the_sequence_space 2>/dev/null
```

Live output:

```
/// # Panics
///
/// In a debug build, if `start.0 + ( count - 1 ) as u64` overflows `u64` for
/// `count > 0` — the last step `run` actually takes, reachable in one step
/// from `ring_mpsc::UNSTAMPED`, the family's `Seq( u64::MAX )` sentinel,
/// rather than only after `2^64` publications. In a release build the
/// addition wraps instead of panicking, which silently produces a slot index

running 1 test
test run_overflows_at_the_top_of_the_sequence_space - should panic ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 11 filtered out; finished in 0.00s
```

**Disposition:** applied — added a `# Panics` section to `run`'s doc comment
in `src/lib.rs`, naming the debug-build overflow, the release-build wrap, and
`ring_mpsc::UNSTAMPED` as the one-step path to the boundary; added
`#[ should_panic ]` test `run_overflows_at_the_top_of_the_sequence_space` to
`tests/index_test.rs`, pinning the debug-mode panic as an asserted fact rather
than only a fact demonstrated in this file's probe output. The crate's 11
unit/integration tests plus 3 doctests re-verified passing (`cargo test -p
ring_index --all-features`, 2026-09-04). Now prints:
`test run_overflows_at_the_top_of_the_sequence_space - should panic ... ok`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`algorithm/001`](../algorithm/001_one_and_of_a_mask.md) | `of` at the same inputs, where masking is total |
| [`algorithm/002`](../algorithm/002_a_run_is_the_fold_applied_count_times.md) | The loop that performs the additions |
| [`invariant/002`](../invariant/002_the_fold_is_total_and_the_run_is_not.md) | The totality claim stated as a property and where it stops |
| [`item/002`](../item/002_the_two_that_nothing_calls.md) | `run` read on its own |

### Sources

| Fact | Where |
|------|-------|
| `run`'s addition | `ring_index/src/lib.rs:121` |
| `advanced_by`'s bare `+` | `ring_types/src/id.rs:64-68` |
| The saturation claim | `ring_types/src/id.rs:34-37` |
| Debug panic and release wrap | Probe, both profiles, quoted above |
| `u64::MAX + 1` wrapping rather than saturating | `rustc -O` probe, quoted above |
| The reserved sentinel | `ring_mpsc/src/lib.rs:220-222` |
| The unreachable-by-counting test | `ring_types/tests/types_test.rs:44-57` |

### Tests

| Test | Covers |
|------|--------|
| `ring_types::seq_does_not_wrap_within_any_reachable_workload` | The boundary as unreachable by counting — the case that is not the hazard; declared in `ring_types/tests/`, not here |
| `run_overflows_at_the_top_of_the_sequence_space` | `run` at `Seq( u64::MAX )`, asserting the debug panic with `#[ should_panic ]` |
| `run_does_not_overflow_one_below_the_top_of_the_sequence_space` | One `Seq` below the boundary still succeeds — pins `start + ( count - 1 )`, not `start + count`, as the real edge |
| *(to create)* | `Seq( u64::MAX ).next()` in release, asserting whichever behaviour `ring_types` decides to actually have |
