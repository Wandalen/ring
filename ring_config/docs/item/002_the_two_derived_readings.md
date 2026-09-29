# Item: The Two Derived Readings

### Scope

**Purpose:** Read the record's two computed members as named things — what each
returns, who calls it, and what the first one's own documentation says about how
many of them there are.

**Responsibility:** `is_multi_producer` and `is_tick_safe`: their bodies, their
call censuses, the miscount in one doc, and the feature the uncalled one cites.

**In Scope:** `ring_config/src/lib.rs:209-224`, `:226-241`;
`ring_types/src/policy.rs:60-88`.

**Out of Scope:** The five stored fields these two are computed from are
[`item/001`](001_five_fields_and_the_one_another_crate_keeps_a_copy_of.md). The
declarations as attributed surface are
[`api/001`](../api/001_twelve_functions_eleven_of_them_const.md).

---

## Both Readings, and Everyone Who Asks

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the two derived readings, declaration and body --'
command grep -A 2 'pub const fn is_multi_producer\|pub const fn is_tick_safe' ring_config/src/lib.rs
echo '  -- and the claim the first one makes about how many there are --'
command grep -m1 -A2 -F '  /// A derived reading, and the one a factory branches on:' ring_config/src/lib.rs
echo '  -- every non-doctest call to either, in src/ or tests/, anywhere --'
command grep -r 'is_multi_producer()\|is_tick_safe()' --include=*.rs */src */tests | command grep -v '^ring_config/src' | command grep -v '///\|//!' | sed -E 's#^(/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/division/[^/]+|/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/substrate/[^/]+|module|ring|spike)/##'
echo '  -- what is_tick_safe delegates to --'
command grep -A 7 'pub const fn is_non_blocking' ring_types/src/policy.rs
```

Live output:

```
  -- the two derived readings, declaration and body --
  pub const fn is_multi_producer( &self ) -> bool
  {
    self.producers > 1
--
  pub const fn is_tick_safe( &self ) -> bool
  {
    self.wait.is_non_blocking()
  -- and the claim the first one makes about how many there are --
  /// A derived reading, and the one a factory branches on: a single-producer
  /// ring must not pay for a synchronisation it does not need, per
  /// `docs/feature/172_multi_producer_claim.md`.
  -- every non-doctest call to either, in src/ or tests/, anywhere --
ring_core/src/lib.rs:    let storage = match config.is_multi_producer()
ring_bench/tests/bench_test.rs:  assert!( workload.config().is_multi_producer() );
ring_bench/tests/bench_test.rs:  assert!( workload.config().is_multi_producer(), "the config asks for the MPSC backend" );
ring_config/tests/config_test.rs:  assert!( !RingConfig::new( 8 ).unwrap().with_producers( 0 ).is_multi_producer() );
ring_config/tests/config_test.rs:  assert!( !cfg.is_multi_producer() );
ring_config/tests/config_test.rs:  assert!( !cfg.with_producers( 1 ).is_multi_producer() );
ring_config/tests/config_test.rs:  assert!( cfg.with_producers( 2 ).is_multi_producer() );
ring_config/tests/config_test.rs:  assert!( cfg.with_producers( 64 ).is_multi_producer() );
ring_config/tests/config_test.rs:      cfg.with_wait( kind ).is_tick_safe(),
ring_config/tests/config_test.rs:  assert!( cfg.with_wait( WaitKind::None ).is_tick_safe() );
ring_config/tests/config_test.rs:  assert!( !cfg.with_wait( WaitKind::Spin ).is_tick_safe() );
ring_core/tests/core_test.rs:  assert!( !one.is_multi_producer() );
  -- what is_tick_safe delegates to --
  pub const fn is_non_blocking( self ) -> bool
  {
    match self
    {
      Self::None => true,
      Self::Spin | Self::Yield | Self::Park => false,
    }
  }
```

---

### RC27 — The Doc Says "The One Derived Reading in the Record" and There Are Two

`is_multi_producer` opens its documentation at `:211` with "The one derived
reading in the record, and the one a factory branches on". `is_tick_safe` is
declared at `:238`, twenty-seven lines below that sentence, in the same `impl`
block, computed the same way — a `const fn` over `&self` returning a `bool` from
one field.

**Finding.** The count is wrong, and the direction it is wrong in matters. It
appears on the reading that everything uses and it undercounts by exactly the one
that nothing uses.

A reader who takes the sentence at face value has been told the record has one
derived reading; the second is below the fold in a 242-line file, and its own doc
gives no hint that it belongs to a pair. The two are then documented as if neither
had a sibling, and the crate has no sentence anywhere describing them as a group
or saying what distinguishes them — one reads a count and compares it, the other
reads an enum and delegates to that enum's own predicate.

Both clauses of the sentence are true of `is_multi_producer` and only the second
is exclusive to it. Deleting three words fixes it: "A derived reading, and the one
a factory branches on."

The doc comment now avoids the exclusivity claim:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A2 -F '  /// A derived reading, and the one a factory branches on:' ring_config/src/lib.rs
```

Live output:

```
  /// A derived reading, and the one a factory branches on: a single-producer
  /// ring must not pay for a synchronisation it does not need, per
  /// `docs/feature/172_multi_producer_claim.md`.
```

**Disposition:** applied — `is_multi_producer`'s doc comment in `src/lib.rs`
now opens with "A derived reading" rather than "The one derived reading in
the record", dropping the exclusivity claim this instance's own Finding says
is false while keeping the factory-branches-on clause that is true. Now
prints: `A derived reading, and the one a factory branches on: a single-producer`

---

### RC28 — `is_tick_safe` Is Called Only by the Test That Tests It

The census covers every `src/` and `tests/` directory under `/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/`, `ring/`,
`/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/` and `/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/substrate/` — the whole crate tree, not one family.
`is_multi_producer` is called from production once — the backend selection in
`ring_core/src/lib.rs`, the `match config.is_multi_producer()` line printed
above — and asserted from two other crates' suites, twice in
`ring_bench/tests/bench_test.rs` and once in `ring_core/tests/core_test.rs`.

`is_tick_safe` appears three times, all three in `ring_config`'s own
`config_test.rs`. No production caller in any crate. No other crate's test.

All seven sites are addressed by content rather than by line, because the seven
line numbers this finding once carried had every one of them drifted — between
twelve and seventy-eight lines — while not one of the sites themselves moved
file or changed text. The crate count in the opening sentence was wrong in a
second way, independent of drift: it named the ring family's thirty-three while
the recipe has always swept whatever tree it was pointed at, which is now five
roots and every crate under them.

**Finding.** The predicate is correct, `const`, `must_use`, exercised over all
four `WaitKind` variants, and asked by nothing.

"Exercised over" is weaker than the "exhaustively tested" this finding first
claimed, and the difference is visible in the census above.
`tick_safety_is_exactly_non_blocking_waiting` loops `WaitKind::ALL` and asserts
`cfg.with_wait( kind ).is_tick_safe()` equals `kind.is_non_blocking()` — but
`is_tick_safe`'s entire body is `self.wait.is_non_blocking()`, so that loop
compares the delegate against itself and holds for all four variants whatever
the delegate answers. Only the two lines after it supply an expected value from
outside: `None` is tick-safe, `Spin` is not. `Yield` and `Park` have their
answer graded by the function under test.

What makes it worth recording rather than filing as one more unused accessor is
what it guards: a restriction — that code reachable from inside a tick may only
call fallible, non-parking operations — that no crate in the tree currently
exposes a surface for. Nothing calls `is_tick_safe`, and no crate exposes a
tick-path surface today, so the accessor is a query nobody asks rather than an
enforcement of anything.

A queryable `is_tick_safe()` is inherently the ask-and-remember form of that
restriction: it requires a caller to remember to check, and the census is what
happens when nobody remembers. A structural restriction — a narrower surface a
tick-bound caller simply cannot escape — would need no predicate at all; if this
capability is ever built that way, this member's role becomes diagnostic, a
thing a debug view prints, rather than a gate.

Its wording is also looser than the vocabulary crate's. `ring_config` says "true
only when waiting cannot park"; `WaitKind::is_non_blocking`, which it delegates
to, is an exhaustive `match` — `Self::None => true`, the other three arms
`false` — and documents itself as "true for exactly `WaitKind::None`". It read
`matches!( self, Self::None )` when this instance was filed; the rewrite is a
recorded fix, carrying its own `Fix(…)`/`Root cause`/`Pitfall` block, for a
`matches!` that would have answered a fifth variant `false` without anything
forcing a second look. Both forms are accurate — the implication runs one way —
but the
looser phrasing suggests the reading excludes only `Park`, and it also excludes
`Spin`, whose own doc says it "burns a core".

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`item/001`](001_five_fields_and_the_one_another_crate_keeps_a_copy_of.md) | The stored fields these two read |
| [`api/002`](../api/002_three_readers_used_and_four_with_no_caller.md) | The same call census taken across the whole reader surface |
| [`pattern/002`](../pattern/002_a_derived_reading_instead_of_a_stored_field.md) | Why these are computed rather than stored |
| [`integration/001`](../integration/001_eleven_declarers_and_four_that_build_on_it.md) | `ring_core`, the one crate that calls either from production |

### Sources

| Fact | Where |
|------|-------|
| `is_multi_producer`'s doc, body and the miscount | `ring_config/src/lib.rs:209-224` |
| `is_tick_safe`'s doc and body | `ring_config/src/lib.rs:226-241` |
| The one production caller | `ring_core/src/lib.rs:174` |
| Every call to either, in `src/` and `tests/` | Census above |
| What `is_tick_safe` delegates to | `ring_types/src/policy.rs:60-88` |
| `Spin` burning a core | `ring_types/src/policy.rs:24` |

### Tests

| Test | Covers |
|------|--------|
| `multi_producer_is_derived_from_the_count` | The reading a factory branches on, at four counts |
| `tick_safety_is_exactly_non_blocking_waiting` | All four `WaitKind` variants against the delegate; `None` and `Spin` against a value from outside |
| `every_named_field_is_carried` | The two fields both readings are computed from |
