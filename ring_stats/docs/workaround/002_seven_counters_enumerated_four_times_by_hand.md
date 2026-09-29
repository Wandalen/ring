# Workaround: Seven Counters, Enumerated Four Times by Hand

### Scope

**Purpose:** Record the workaround `reset` uses for Rust having no field iteration, the
three other hand-maintained enumerations of the same seven counters, and the check the
family applies to exactly this problem elsewhere and not here.

**Responsibility:** `reset`'s array of references, the struct fields, `new`'s
initialisers, the reset test's assertions, and the compiler-checked `match` that covers
three of the seven.

**In Scope:** `RingStats::COUNTERS`, `RingStats::counters`, the `const` size
assertion, `RingStats::reset` and the two `match policy` sites in
`ring_stats/src/lib.rs`; `reset_returns_every_counter_to_the_fresh_state` and
`a_snapshot_reaches_every_counter_and_a_reset_clears_every_one` in
`ring_stats/tests/stats_test.rs`; `OverflowPolicy::ALL` in
`ring_types/src/policy.rs`.

**Out of Scope:** What the reset window looks like from another thread is
[`lifecycle/001`](../lifecycle/001_zero_to_reset_to_zero.md). Why `reset` cannot take
`&mut self` is
[`api/001`](../api/001_fourteen_methods_and_no_exclusive_borrow.md) § ST5.

---

## Four Lists of Seven, and One the Compiler Checks

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the workaround: no field iteration, so the counters are walked as an array of references --'
command grep -m1 -A6 -F '  const fn counters( &self ) -> [ &AtomicU64; Self::COUNTERS ]' ring_stats/src/lib.rs
echo '  -- and the constant plus assertion that now make the compiler check its length --'
command grep -m1 -F '  pub const COUNTERS : usize = 7;' ring_stats/src/lib.rs
command grep -m1 -B6 -A3 -F 'const _ : () = assert!' ring_stats/src/lib.rs
echo '  -- every place the seven counters are enumerated by hand --'
printf '    struct fields                  %s\n' "$( command grep -m1 -B1 -A8 -F 'pub struct RingStats' ring_stats/src/lib.rs | command grep -c 'AtomicU64' || true )"
printf '    new() initialisers             %s\n' "$( command grep -m1 -A10 -F '    Self' ring_stats/src/lib.rs | command grep -c 'AtomicU64::new' || true )"
printf "    counters()'s array             %s\n" "$( command grep -m1 -A4 -F '  const fn counters( &self ) -> [ &AtomicU64; Self::COUNTERS ]' ring_stats/src/lib.rs | command grep -o '&self\.[a-z_]*' | wc -l )"
printf '    assertions in the reset test   %s\n' "$( command grep -m1 -A25 -F 'fn reset_returns_every_counter_to_the_fresh_state()' ring_stats/tests/stats_test.rs | command grep -c 'assert_eq!( stats\.' || true )"
echo '  -- against the three the compiler does check --'
command grep 'match policy' ring_stats/src/lib.rs
command grep 'pub const ALL' ring_types/src/policy.rs
```

Live output:

```
  -- the workaround: no field iteration, so the counters are walked as an array of references --
  const fn counters( &self ) -> [ &AtomicU64; Self::COUNTERS ]
  {
    [
      &self.claimed, &self.published, &self.consumed,
      &self.dropped_newest, &self.dropped_oldest, &self.failed, &self.wait_nanos,
    ]
  }
  -- and the constant plus assertion that now make the compiler check its length --
  pub const COUNTERS : usize = 7;
// The struct is `COUNTERS` counters and nothing else, checked at compile time.
// Every field is an `AtomicU64`, so the type's size is exactly the count times
// one counter's. This is the line an eighth field fails on, and failing here is
// the point: before it, a counter added to the struct but left out of `reset`'s
// array was cleared by nothing, read plausibly, stayed monotone, and failed no
// test in the suite.
const _ : () = assert!
(
  core::mem::size_of::< RingStats >() == RingStats::COUNTERS * core::mem::size_of::< AtomicU64 >()
);
  -- every place the seven counters are enumerated by hand --
    struct fields                  7
    new() initialisers             7
    counters()'s array             7
    assertions in the reset test   7
  -- against the three the compiler does check --
    let counter = match policy
    let counter = match policy
  pub const ALL : [ Self; 4 ] = [ Self::Spin, Self::Yield, Self::Park, Self::None ];
  pub const ALL : [ Self; 3 ] = [ Self::DropNewest, Self::DropOldest, Self::Fail ];
```

---

### ST51 — Adding a Counter Requires Four Manual Edits and the Compiler Names None of Them

Rust has no way to iterate a struct's fields, so the crate does the idiomatic thing:
build an array of references to all seven counters and loop over it. That is the
correct workaround, and it is the whole of the constraint — there is no better version
available in the language.

Its consequence is that the seven counters are written out four separate times: as
struct fields, as `new`'s initialisers, as the array, and as the reset test's
assertions. Every list is exactly seven and every list is maintained by hand.

**Finding.** Add an eighth counter and the compiler insisted on two of the four —
the struct field and the `new` initialiser, because a missing field is a hard error.
It said nothing about the other two. A counter absent from the array survived
every reset silently, and the test that would catch it is itself an explicit list of
seven assertions that would also need the eighth added by hand.

The failure was quiet in the worst way. `reset`'s stated purpose is that a recycled
ring "does not carry the previous world's numbers"; a counter missing from the array
carries them forever, reads plausibly, and stays monotone. Nothing in the crate's
tests would have gone red.

Three of the seven counters were exempt. `record_drop` and `dropped` each dispatch
through an exhaustive `match policy` on a three-variant enum, so adding a fourth
`OverflowPolicy` variant produces two compile errors naming both sites exactly. Within
one struct, the drop counters were protected by exhaustiveness and the other four were
protected by nothing.

**Disposition:** applied — the array moved out of `reset` into
`RingStats::counters`, whose return type is `[ &AtomicU64; Self::COUNTERS ]`, and a
compile-time `assert!` pins `size_of::< RingStats >()` to `COUNTERS` times one
counter's size. Adding an eighth field now fails the size assertion; raising `COUNTERS`
to eight to satisfy it then fails the array's declared length, and that second error
names the array by line. Both were proven to fire rather than assumed: an eighth field
produced `error[E0080]: evaluation panicked: assertion failed: core::mem::size_of::<RingStats>() ==`, and removing one entry from the array produced `error[E0308]: expected an array with a size of 7, found one with a size of 6`. The manual edits the
language requires are unchanged in number; what changed is that three of the four lists
are now enforced by the compiler rather than two. The fourth — the reset test's seven
hand-written assertions — is covered instead by
`a_snapshot_reaches_every_counter_and_a_reset_clears_every_one`, which compares the
whole value against `Default::default()` after a reset rather than naming seven fields,
so a counter `reset` misses cannot be missed there too. Now prints: `  core::mem::size_of::< RingStats >() == RingStats::COUNTERS * core::mem::size_of::< AtomicU64 >()`

---

### ST52 — The Family's Answer to This Exists, Is Documented, and Is Not Applied Here

`OverflowPolicy::ALL` is itself a hand-maintained array, and `ring_types` knows it —
`assert_eq!( OverflowPolicy::ALL.len(), 3 )` appears both as a doctest on the enum and
as an assertion in `types_test.rs`, and the same pattern covers `WaitKind::ALL` at four.
The crate documents the reasoning in its own corpus, including the observation that
adding a fifth variant compiles while `ALL.len()` stays four.

So the convention is established one crate away and has a name, a rationale, and tests:
where a set must be written out by hand, publish the set as a constant and assert its
length.

**Finding.** `ring_stats` had four hand-written sets of seven and applied the convention
to none of them. There was no `RingStats::COUNTERS`, no length assertion, and nothing
that related the array to the struct's field count. The one list in the crate
that was checked at all was checked by a mechanism the crate did not choose —
`OverflowPolicy`'s exhaustiveness, inherited from `ring_types`.

The convention is now applied, in a form one rung stronger than `ring_types`' own.
`OverflowPolicy::ALL`'s length is asserted at runtime, in a doctest and a test;
`RingStats::COUNTERS` is checked at *compile* time, by a `const` assertion on the
struct's size and by the declared length of the array `counters` returns. The
difference is available here and not there because every field of `RingStats` has the
same type, so its size determines the count — an enum's variants offer no equivalent.

What the convention still does not remove is the manual edit itself, which the language
requires. It converts the silent failure into a loud one — which is what `ring_types`
did for exactly this problem, and what
[`workaround/001`](001_a_floor_that_absorbs_more_than_it_was_built_for.md) § ST50 found
missing in the crate's other defensive construct: the guard is present, the check that
the guard still covers what it claims is not.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`workaround/001`](001_a_floor_that_absorbs_more_than_it_was_built_for.md) | The crate's other workaround, and its own missing check |
| [`lifecycle/001`](../lifecycle/001_zero_to_reset_to_zero.md) | The array's traversal order, observed from another thread |
| [`api/001`](../api/001_fourteen_methods_and_no_exclusive_borrow.md) | Why `reset` must store seven times rather than assign once |
| [`type/002`](../type/002_seven_counters_and_one_width.md) | The seven fields the four lists enumerate |

### Sources

| Fact | Where |
|------|-------|
| The array of references, and the loop that stores through it | Census above |
| `COUNTERS` and the `const` size assertion | Census above |
| Four hand-maintained lists of seven | Census above |
| The two exhaustive `match policy` sites | Census above |
| `OverflowPolicy::ALL` as a hand-maintained set | Census above |
| The length assertions the family applies | `ring_types/tests/types_test.rs:141`, `:182` |
| The convention documented | `ring_types/docs/non_functional_requirement/002_the_enum_sets_are_closed_and_asserted.md` |

### Tests

| Test | Covers |
|------|--------|
| `reset_returns_every_counter_to_the_fresh_state` | Seven assertions, listed by hand |
| `a_fresh_set_is_all_zero` | `new`'s seven initialisers, via the readers |
| `a_drop_lands_under_its_own_policy_only` | The three counters the compiler already protects |
| `a_snapshot_reaches_every_counter_and_a_reset_clears_every_one` | Seven distinct values, each traced through a snapshot, then compared whole against the default after a reset |
| `RingStats::COUNTERS`'s own doctest | That the constant is seven, so a raised constant is visible in the docs as well as the array |
