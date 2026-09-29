# Lifecycle: A Resolution Computed, Matched, and Discarded

### Scope

**Purpose:** Record the span of existence of the one value this crate produces —
where a `Resolution` is born, how long it lives, and where it dies.

**Responsibility:** The value's production lifetime, the sites that bind it to a
name, and the lifetime of the input it is derived from.

**In Scope:** `ring_core/src/lib.rs:180`, `:207`, `:244`, `:411-414`;
`ring_overflow/tests/overflow_test.rs:38`, `:272`, `:293`.

**Out of Scope:** Which derives the type carries and that only the suite exercises
them is [`type/001`](../type/001_six_derives_on_a_fieldless_enum.md) § OV33. The
crate's own lifecycle as a work item is
[`lifecycle/002`](002_implemented_tested_and_still_planned.md).

---

## Born, Matched, Gone

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the whole life of the value in production: one expression --'
command grep -m1 -A3 -F '      Err( record ) => match would_resolve( self.overflow )' ring_core/src/lib.rs
echo '  -- every site binding a Resolution to a name, across producer and consumer --'
command grep -r 'let .*= *would_resolve\|let .*= *resolve(\|let .*: *Resolution\|let a = Resolution' --include=*.rs ring_overflow ring_core | sed 's|ring/||'
echo '  -- how many of those sit outside the test file --'
command grep -r 'let .*= *would_resolve\|let .*= *resolve(\|let .*: *Resolution\|let a = Resolution' --include=*.rs ring_overflow/src ring_core/src | wc -l
echo '  -- where the policy is set, its getter, and any assignment after construction --'
command grep 'overflow : config.overflow()\|self\.overflow *= *[^=]\|fn overflow' ring_core/src/lib.rs
```

Live output:

```
  -- the whole life of the value in production: one expression --
      Err( record ) => match would_resolve( self.overflow )
      {
        Resolution::DroppedIncoming => Ok( () ),
        Resolution::EvictedOldest | Resolution::Refused => Err( record ),
  -- every site binding a Resolution to a name, across producer and consumer --
ring_overflow/tests/overflow_test.rs:    let resolution = would_resolve( policy );
ring_overflow/tests/overflow_test.rs:    let resolution = would_resolve( policy );
ring_overflow/tests/overflow_test.rs:    let _ = resolve( policy, &stats );
ring_overflow/tests/overflow_test.rs:  for _ in 0..3 { let _ = resolve( OverflowPolicy::DropNewest, &stats ); }
ring_overflow/tests/overflow_test.rs:  for _ in 0..2 { let _ = resolve( OverflowPolicy::DropOldest, &stats ); }
ring_overflow/tests/overflow_test.rs:  let _ = resolve( OverflowPolicy::Fail, &stats );
ring_overflow/tests/overflow_test.rs:    let _ = would_resolve( policy );
ring_overflow/tests/overflow_test.rs:  let a = Resolution::EvictedOldest;
ring_overflow/tests/overflow_test.rs:    let outcome = resolve( policy, &stats );
  -- how many of those sit outside the test file --
0
  -- where the policy is set, its getter, and any assignment after construction --
    Ok( Self { storage, overflow : config.overflow() } )
        overflow : config.overflow(),
  pub const fn overflow( &self ) -> OverflowPolicy
```

---

### OV45 — In Production the Value Never Outlives the Expression That Produced It

`would_resolve( self.overflow )` appears as a `match` scrutinee. The value it
returns is compared against two patterns covering all three variants and is gone
at the closing brace. It is never bound to a name, never stored in a field,
never returned, never passed on.

Nine sites in the workspace bind a `Resolution` to a name and all nine are in
`ring_overflow/tests/overflow_test.rs`. Across both crates' `src/` the count is
zero.

**Finding.** So the type has no lifecycle to speak of, and that is the useful thing
to know about it. There is no `Drop` impl and nothing to run; the value cannot be
shared, cannot be observed stale, cannot be seen torn by a second thread, and
cannot be read after the state it describes has moved on — because it does not
exist long enough for any of those to be possible.

Which places the whole of the crate's concurrency exposure on the other half of
the call. `resolve` writes a counter that outlives the process's every
`Resolution` and is read by other threads with no ordering guarantee
([`nfr/002`](../non_functional_requirement/002_a_core_only_crate_that_never_says_so.md)
§ OV52). The value is ephemeral; its side effect is not. A reader reasoning about
this crate's thread-safety should be looking at the counter, and the return type is
what draws the eye.

The one exception is `a_resolution_is_a_plain_comparable_value`, where a
`Resolution` is copied to a second binding and then lives inside a `HashMap` across
three loop iterations — the only place in the workspace one survives its producing
expression, and it is a test asserting that it can.

---

### OV46 — The Input Is Fixed at Construction and the Output Is Recomputed Per Event

`Producer::overflow` is set once, from `config.overflow()`, at each of the two
construction sites. The census finds a getter and no assignment anywhere after
that — the field is private, has no setter, and every later `Producer` is built by
copying the value forward.

So `self.overflow` cannot change for the life of the producer, `would_resolve` is
a pure total function, and `would_resolve( self.overflow )` therefore yields the
same variant on every call a given producer will ever make. It is recomputed on
each full-ring event anyway.

**Finding.** Nothing is wrong here — the recomputation costs 0.68 ns
([`nfr/001`](../non_functional_requirement/001_seven_times_the_cost_on_the_half_nobody_ships.md)
§ OV49) on a path that only runs when a push has already failed, and caching it
would trade that for a second field to keep consistent.

What is worth recording is where the alternative would have led. `Resolution` and
`OverflowPolicy` are both one byte
([`data_structure/001`](../data_structure/001_one_byte_and_two_hundred_fifty_three_spare_niches.md)),
so a producer could store the resolved variant instead of the policy at no size
cost, and the mapping would then be evaluated once, at construction, from a value
the config already holds.

That is the shape in which `would_resolve`'s `const`-ness would finally do
something. The split exists to keep the mapping compile-time evaluable, and
nothing in the workspace evaluates it at compile time
([`workaround/001`](../workaround/001_the_recorder_forecloses_const.md) § OV38) —
the one caller positioned to take up that property is the one caller there is, and
it computes at the latest moment instead of the earliest. Not a defect; a closed
loop worth naming, because it identifies the single edit that would justify a
property the crate is already paying a duplicated mapping to preserve.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`workaround/001`](../workaround/001_the_recorder_forecloses_const.md) | The `const`-ness this lifetime never uses |
| [`nfr/002`](../non_functional_requirement/002_a_core_only_crate_that_never_says_so.md) | The side effect that does outlive the value |
| [`type/001`](../type/001_six_derives_on_a_fieldless_enum.md) | The derives that permit a longer life |
| [`integration/001`](../integration/001_one_consumer_one_import_one_site.md) | The one site the value crosses |

### Sources

| Fact | Where |
|------|-------|
| The value's entire production lifetime | `ring_core/src/lib.rs:411-414` |
| Nine name-bindings, all in the test file | Census above |
| The policy set at construction, with no setter | `ring_core/src/lib.rs:180`, `:207`, `:244` |
| The one place a `Resolution` outlives its expression | `ring_overflow/tests/overflow_test.rs:272-282` |

### Tests

| Test | Covers |
|------|--------|
| `a_resolution_is_a_plain_comparable_value` | The only life longer than one expression |
| `resolve_agrees_with_would_resolve` | That the mapping is stable across calls |
| `would_resolve_touches_no_counters` | That the short-lived half leaves nothing behind |
