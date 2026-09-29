# API: Shared Reference Everywhere and What It Forces

### Scope

**Purpose:** Record that no method in the crate takes `&mut self`, establish that
the justification for that choice is given for one mutator and withheld from the
other, and demonstrate what the undocumented one permits a second holder to do.

**Responsibility:** The receiver census, `record`'s stated reason, `clear`'s
absent one, the type-level statement `ring_stats` makes and this crate does not,
and a contended measurement of `clear` against `record`.

**In Scope:** `ring_trace/src/lib.rs:238-241`, `:344-366`;
`ring_stats/src/lib.rs:53-54`; `ring_trace/tests/trace_test.rs`.

**Out of Scope:** The surface as a whole is
[`api/001`](001_three_types_nine_attributes_and_one_private_item.md). Why the
enabled flag is frozen is
[`decisions/001`](../decisions/001_the_flag_is_fixed_at_construction.md).

---

## Two Mutators, One Explanation

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- record: a shared reference, and the reason for it --'
command grep -m1 -A4 -F '  /// Record one operation, or do nothing if disabled.' ring_trace/src/lib.rs
echo '  -- clear: the same receiver, no reason given --'
command grep -m1 -A1 -F '  /// Discard every entry, keeping the enabled state.' ring_trace/src/lib.rs
command grep -m1 -A3 -F '  pub fn clear( &mut self )' ring_trace/src/lib.rs
echo '  -- what the neighbour states once, at the type --'
command grep -m1 -A1 -F '/// Every method takes `&self`, so a single instance is shared by every producer' ring_stats/src/lib.rs
echo '  -- and which tests run clear against anything else --'
command grep 'clear()' ring_trace/tests/trace_test.rs
command grep 'thread::scope' ring_trace/tests/trace_test.rs
```

Live output:

```
  -- record: a shared reference, and the reason for it --
  /// Record one operation, or do nothing if disabled.
  ///
  /// Takes `&self`, not `&mut self`, for the same reason `ring_stats` does: the
  /// producers holding this concurrently cannot each have a unique reference.
  ///
  -- clear: the same receiver, no reason given --
  /// Discard every entry, keeping the enabled state.
  ///
  pub fn clear( &mut self )
  {
    self.entries_guard().clear();
  }
  -- what the neighbour states once, at the type --
/// Every method takes `&self`, so a single instance is shared by every producer
/// and consumer of its ring.
  -- and which tests run clear against anything else --
  trace.clear();
  trace.clear();
  trace.clear();
  thread::scope( | scope |
  thread::scope( | scope |
```

## What a Second Holder Can Do

```rust
// -tr_probe/src/bin/shared_clear.rs
// One producer recording 200,000 operations; a second holder of the same
// `&Trace` calling `clear` in a loop. Both are safe, neither blocks the other
// for long, and `record` returns `()`.
std::thread::scope( | scope |
{
  scope.spawn( move ||
  {
    while !done.load( Ordering::Relaxed ) { trace.clear(); clears.fetch_add( 1, Ordering::Relaxed ); }
  } );
  scope.spawn( move ||
  {
    for i in 0..RECORDS { trace.record( TraceOp::Publish, Seq( i ), 1 ); }
    done.store( true, Ordering::Relaxed );
  } );
} );
```

Two runs, `--release`:

```
  producer recorded:        200000
  another holder cleared:   157336 times
  entries surviving:        115
  producer's own signal:    record returns ()
  the trace still says:     enabled true  empty false

  producer recorded:        200000
  another holder cleared:   281556 times
  entries surviving:        0
  producer's own signal:    record returns ()
  the trace still says:     enabled true  empty true
```

---

### TR7 — The Crate Froze the Flag to Prevent a Silent Hole and Left `clear` Open

`Trace`'s doc gives a specific reason for making `enabled` immutable after
construction: "A trace that could be switched on mid-run would produce a log with
a silent hole at the front, which reads exactly like a run where nothing happened
early — the one misreading a diagnostic tool must not invite." The argument is
right, and the flag is a private field with no setter, so it holds.

`clear` produces the same hole through the door next to it. It takes `&self`, so
any holder of a shared reference can call it, and the probe shows what that
means: a producer recording 200,000 operations alongside one clearing holder ends
with 115 entries in one run and **zero** in the other — literally "a run where
nothing happened". `record` returns `()`, so the producer cannot tell. `is_empty`
then reports true and `is_enabled` reports true, which is the state a reader would
naturally interpret as a correctly-configured trace that saw no traffic.

**Finding.** The two cases are not equally likely — nothing in the family calls
`clear` today, since nothing calls this crate at all — but they are the same
hazard and only one is closed. The cheapest honest repair is a paragraph on
`clear` making the reach explicit: that it is reachable from every holder of a
`&Trace`, that it erases work other holders recorded, and that a cleared trace is
indistinguishable from an idle one. The stronger repair is to give `clear` a
receiver that a shared holder does not have — `&mut self`, reachable only by
whoever owns the trace or can prove exclusivity — which costs nothing at the one
call site in the tests and forecloses the case entirely.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A3 -F '  pub fn clear( &mut self )' ring_trace/src/lib.rs
```

Live output:

```
  pub fn clear( &mut self )
  {
    self.entries_guard().clear();
  }
```

**Disposition:** applied — took the stronger repair: `clear` now takes `&mut self`
instead of `&self`, so a second holder of a shared `&Trace` can no longer call it
at all, and a doc paragraph on `clear` states the reach and the `record`-returns-`()`
consequence directly (see the module's `TR7` cross-reference in its source). The
probe above still documents the hazard as it existed under the old `&self`
receiver; it can no longer compile against the current signature, which is the
fix working as intended. The two single-threaded test call sites (`clearing_empties_the_log_without_switching_it_off`,
`clearing_a_disabled_trace_is_harmless`) were updated to bind `let mut trace`.
Verified via `cargo test -p ring_trace --all-features`, 2026-09-04 — ring_trace's
20 unit tests plus 9 doctests all pass. Now prints:
`pub fn clear( &mut self )`

---

### TR8 — Uniform Interior Mutability Stated Once, on One Method

The receiver census is eleven `&self`, one `self`, and zero `&mut self`. That is
a design decision about the whole type: `Trace` is uniformly interior-mutable,
and any holder of a shared reference can do anything the type does. The
justification appears once, in the middle of `record`'s documentation, phrased as
a fact about that method.

`ring_stats` — the crate `record`'s own sentence points at — states it at the
type instead: "Every method takes `&self`, so a single instance is shared by
every producer and consumer of its ring." Two lines, above the struct, before
any method. A reader arriving at `clear` there already knows
the rule and can ask the right question about it; a reader arriving at `clear`
here has met the rule only if they read `record` first.

**Finding.** Placement is the whole of it. Moving one sentence from `record`'s
doc to `Trace`'s, and keeping `record`'s specific note about producers, would
give this crate the shape the neighbour already has and would make TR7's gap
visible at the point where a reader can act on it — because a type-level
statement that every method takes `&self` invites exactly the question of what
`clear` therefore permits. The suite reinforces the placement problem: `clear` is
called at three lines, all single-threaded, while both `thread::scope` tests
record only.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`decisions/001`](../decisions/001_the_flag_is_fixed_at_construction.md) | The hole this one closes, and the argument for it |
| [`lifecycle/001`](../lifecycle/001_three_states_and_the_two_ways_out_of_one.md) | Clearing against disabling, as states |
| [`api/001`](001_three_types_nine_attributes_and_one_private_item.md) | The surface these receivers belong to |
| [`invariant/002`](../invariant/002_the_order_is_the_thing_a_counter_lacks.md) | The other property a concurrent `clear` would break |

### Sources

| Fact | Where |
|------|-------|
| `record`'s stated reason for `&self` | `ring_trace/src/lib.rs:238-241` |
| `clear`'s receiver, now `&mut self` (TR7 applied) | `ring_trace/src/lib.rs:344-366` |
| The flag-freezing argument | `ring_trace/src/lib.rs:184-188` |
| `ring_stats`' type-level statement | `ring_stats/src/lib.rs:53-54` |
| `clear` called only single-threaded | Census above |
| 200,000 records reduced to 115 and to 0 | Probe above, two runs |

### Tests

| Test | Covers |
|------|--------|
| `clearing_empties_the_log_without_switching_it_off` | `clear`'s single-threaded contract |
| `clearing_a_disabled_trace_is_harmless` | `clear` against the other state |
| `concurrent_recorders_lose_no_entry` | The contention case that omits `clear` |
