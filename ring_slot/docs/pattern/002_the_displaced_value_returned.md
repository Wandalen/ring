# Pattern: The Displaced Value Returned

### Scope

**Purpose:** Record the convention that `set` hands back what it replaced, that
the reason worth keeping it for is nowhere written while the sentence that is
written names a crate structurally unable to call it, and that the one consumer
reading the value reads it for that unwritten reason — behind an assertion
release builds remove.

**Responsibility:** `TypedSlot::set`'s return value: why it exists, who reads it,
and what the four shipped call sites do with it.

**In Scope:** `set`'s doc comment and body in `ring_slot/src/lib.rs`; the
four `set` call sites in shipped library code; `ring_overflow/src/lib.rs`.
Addressed by name rather than by line — SL40's own disposition inserted comments
at three of the four sites, which moved every address this section used to
carry.

**Out of Scope:** That `set` carries no `#[ must_use ]` while six other functions
do is [`api/001`](../api/001_ten_functions_six_const_seven_must_use.md) SL17 — this
instance is about what the value *means*, not about the attribute. That the same
justification is also unreachable — `ring_core` refuses `DropOldest` at
construction, so the named policy cannot run at all — is that instance's SL18;
SL39 below takes the complementary half, which is what the return value is
*actually* for. The `write`-side refusal is
[`algorithm/001`](../algorithm/001_write_is_a_bound_check_and_a_copy.md).

---

## The Convention

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/^  \/\/\/ Place `value` in the slot, returning whatever it held before\.$/,/^  \/\/\/ ```$/p;/^  pub fn set( &mut self, value : T ) -> Option< T >$/,/^    self\.0\.replace( value )$/p' ring_slot/src/lib.rs
```

Live output:

```
  /// Place `value` in the slot, returning whatever it held before.
  ///
  /// Returning the displaced value rather than dropping it keeps the door open
  /// for a future evict-oldest policy to hand a caller what it evicted instead
  /// of losing it silently — but `ring_overflow` does not depend on this crate
  /// and cannot reach this return value today. The one reader that binds it now
  /// is `ring_core`'s `debug_assert`, confirming a freshly claimed slot came
  /// back empty rather than handing anything back to a caller.
  ///
  /// Deliberately **not** `#[ must_use ]`, unlike [`TypedSlot::take`]: on a ring
  /// the displaced value belongs to a lap the consumer already finished, so
  /// dropping it is the ordinary case rather than a lost record. `core` marks
  /// neither `Option::replace` nor `Option::take` for the same kind of reason.
  ///
  /// ```
  pub fn set( &mut self, value : T ) -> Option< T >
  {
    self.0.replace( value )
```

`Option::replace`, surfaced unchanged. The convention is not universal within the
crate — every mutator it has, by what it returns:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '--- the mutators, by return type ---'
command grep -E '^  (pub )?fn (set|clear|write)\(' ring_slot/src/lib.rs
echo '--- and the test that binds three displaced values in a row ---'
command grep -m1 -A8 -F 'fn setting_over_a_value_returns_the_displaced_one()' ring_slot/tests/slot_test.rs \
  | command grep -E 'assert'
```

Live output:

```
--- the mutators, by return type ---
  fn clear( &mut self );
  pub fn set( &mut self, value : T ) -> Option< T >
  fn clear( &mut self )
  pub fn write( &mut self, payload : &[ u8 ] ) -> Result< (), RingError >
  fn clear( &mut self )
--- and the test that binds three displaced values in a row ---
  assert_eq!( slot.set( 1u8 ), None );
  assert_eq!( slot.set( 2u8 ), Some( 1 ) );
  assert_eq!( slot.set( 3u8 ), Some( 2 ) );
  assert_eq!( slot.get(), Some( &3 ) );
```

`clear` drops what it held and returns `()` — declared once on the trait and
implemented twice — and `BytesSlot::write` returns a `Result< (), RingError >`,
so a payload overwritten by it is gone with nothing handed back. Exactly one of
the crate's mutators hands anything back, and the test above binds its return on
three consecutive calls.

---

### SL39 — The Real Reason to Keep the Return Value Is Not the Documented One

The doc comment gives one justification and one beneficiary: `ring_overflow`'s
evict-oldest policy, handing the caller what it evicted. `ring_overflow` cannot
do that:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '--- does ring_overflow depend on ring_slot? ---'
command grep 'ring_slot' ring_overflow/Cargo.toml ring_overflow/src/lib.rs 2>/dev/null \
  || echo '  no dependency, no mention'
echo '--- and what it actually offers ---'
command grep -E '^pub fn |^pub enum ' ring_overflow/src/lib.rs
```

Live output:

```
--- does ring_overflow depend on ring_slot? ---
  no dependency, no mention
--- and what it actually offers ---
pub enum Resolution
pub fn resolve( policy : OverflowPolicy, stats : &RingStats ) -> Result< Resolution, RingError >
```

No dependency edge, no mention of the crate, and a surface that is one pure
function from a policy and a statistics snapshot to a `Resolution` enum.
`ring_overflow` decides *what should happen* on a full ring; it never holds a
slot, never evicts anything, and has nothing to hand back.

**Finding.** The justification is stale in the strongest sense — not merely
out of date, but naming a division of labour the family does not have.
`ring_overflow` is a policy crate at Tier 1, structurally beside `ring_slot`
rather than above it; the eviction it names is performed in `ring_core`, which
does depend on both.

Two things follow. The reason to keep the return value is real and unstated: the
value is the only evidence a caller can obtain that a slot it believed empty was
not, which SL40 shows is exactly what its one reader uses it for. And the comment
as written sends a reader to a crate where they will find nothing — the same
failure mode recorded elsewhere in this family for `ring_atomic`'s `loom` seam
and `ring_claim`'s `cursor()`, where a doc comment names a consumer that does not
exist. It is worth correcting here because `set`'s signature is otherwise
unexplained: `Option::replace` returning the old value is idiomatic, and without
a reason a reader will assume the return is incidental and discard it.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_slot
command grep -F 'cannot reach this return value today' src/lib.rs
```

Live output:

```
    /// and cannot reach this return value today. The one reader that binds it now
```

**Disposition:** applied — `set`'s doc comment no longer sends a reader to
`ring_overflow` for a policy it cannot reach; it now says so directly and names
the real, current reader instead: `ring_core`'s `debug_assert`, which uses the
return value as the only evidence a freshly claimed slot was not already
occupied — the unstated reason this finding surfaces.
Now prints: `cannot reach this return value today`

---

### SL40 — Four Call Sites, Four Handlings, and the Only Reader Gates a Protocol Check Behind `debug_assert!`

Every `set` in shipped library code across the family:

```sh
cd "$(git rev-parse --show-toplevel)"
# sorted: grep is shimmed to a parallel ugrep here, so hits arrive in completion order
command grep -r '\.set( ' ring_*/src/*.rs | command grep -vE ':[[:space:]]*//' | LC_ALL=C sort
```

Live output:

```
ring_core/src/lib.rs:            let displaced = reserved.set( record );
ring_event/src/lib.rs:    slot.set( self );
ring_mpsc/src/lib.rs:    reserved.set( value );
ring_spsc/src/lib.rs:    drop( reservation.set( record ) );
```

Four call sites doing the same thing — placing a value into a slot the caller has
just claimed — and four different treatments of the identical return: a bare
discard, another bare discard, an explicit `drop`, and a binding. The addresses
are deliberately not quoted: this finding's own disposition inserted comments at
three of the four, which moved every line number the census used to print.

The one binding is the family's only check that a claimed slot was actually
empty:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -B1 -A5 -F '          Ok( mut reserved ) =>' ring_core/src/lib.rs
```

Live output:

```
        {
          Ok( mut reserved ) =>
          {
            let displaced = reserved.set( record );
            debug_assert!( displaced.is_none(), "a claimed slot held a record" );
            Ok( () )
          }
```

**Finding.** "A claimed slot held a record" is a protocol violation, not a
recoverable condition: it means the claim machinery handed out a slot the
consumer had not released, so a live record was silently destroyed by the `set`
that overwrote it. `ring_core` is the only crate that looks, and it looks through
`debug_assert!`, which is compiled out under the release profile the benches and
any shipping build use. Where the violation would be cheapest to catch —
in production, at the moment of the overwrite, with the displaced record still in
hand — nothing catches it.

This finding was written believing the other three sites could each make the same
check for the cost of one comparison. They cannot, and the disposition below
records the measurement that says so. `ring_spsc`'s `drop( … )` is the most
pointed of the three: it names the return value explicitly, which means the
author saw it, and then discards it. Under the workspace's lint table that `drop`
is not even required — `unused_results` is not enabled, so the two bare discards
compile without a warning too
([`api/001`](../api/001_ten_functions_six_const_seven_must_use.md) SL17 records
that `set` carries no `#[ must_use ]` either, so nothing anywhere pushes a caller
to look).

The fix this finding proposed was at this crate rather than at the four
consumers: `#[ must_use = "the displaced value is the only evidence the slot was
not empty" ]` on `set`, turning all three discards into compile-time decisions
and leaving `ring_core`'s binding untouched. SL17 measured that proposal against
the whole family and declined it for `set` while applying it to `take`. What was
applied here instead is the thing the census could not see: each of the four
sites now says, in the source, which of three different contracts it is under.

```sh
cd "$(git rev-parse --show-toplevel)"
echo '--- ring_core: binds it and asserts ---'
command grep -m1 -A1 -F 'let displaced = reserved.set( record );' ring_core/src/lib.rs
echo '--- ring_mpsc: drops it, and says why it may not assert ---'
command grep -m1 -B7 -F '    reserved.set( value );' ring_mpsc/src/lib.rs | command grep -m1 -A2 -F 'this is not the same'
echo '--- ring_spsc: drops it, and says why ---'
command grep -m1 -B2 -F '    drop( reservation.set( record ) );' ring_spsc/src/lib.rs | command grep -m1 -F 'committed by the consumer'
echo '--- ring_event: the trait contract that requires discarding ---'
command grep -m1 -B6 -F '    slot.set( self );' ring_event/src/lib.rs | command grep -m1 -A1 -F 'documented contract'
```

Live output:

```
--- ring_core: binds it and asserts ---
            let displaced = reserved.set( record );
            debug_assert!( displaced.is_none(), "a claimed slot held a record" );
--- ring_mpsc: drops it, and says why it may not assert ---
    // The displaced value is dropped, deliberately, and this is not the same
    // situation as `ring_core`'s identical line — which asserts the slot was
    // empty. It can assert that because its own consumer always drains with
--- ring_spsc: drops it, and says why ---
    // The slot's previous occupant, if any, was committed by the consumer a lap
--- ring_event: the trait contract that requires discarding ---
    // Discarding the displaced value is `fill`'s documented contract — "write
    // `self` into `slot`, replacing whatever it held" — not an oversight. A
```

**Disposition:** applied — the premise, not the remedy. The census was right that
four sites treat one return four ways and that nothing said why; it was wrong
that the difference is stylistic and wrong that the other three could each make
`ring_core`'s check. There are three contracts here, not one, and each site now
names its own. `ring_core` binds and asserts, and is entitled to: its own
consumer always drains with `TypedSlot::take`, so a claimed slot really is empty.
`ring_mpsc` hands out a `Batch`, and a consumer reading through `get`/`peek`
leaves the record in place, so on any lap after the first the slot legitimately
still holds one — its new comment says exactly that, and that dropping the
displaced value is what bounds the ring's storage. `ring_spsc` already carried
the equivalent sentence. `ring_event`'s `fill` discards by documented contract —
"replacing whatever it held" — because the trait exists to give both shapes one
signature and `BytesSlot` has nothing to hand back. The unifying `debug_assert!`
is declined on measurement rather than on taste: adding it to `ring_mpsc` and
`ring_spsc` and running their suites fails 2 of 31 and 4 of 28 respectively, on
correct code — `every_record_written_is_destroyed_exactly_once`,
`a_ring_smaller_than_the_traffic_still_loses_nothing`,
`a_record_taken_through_get_mut_leaves_its_slot_empty_across_a_wrap` — because an
overwriting ring reaching lap 2 is the design, not a protocol violation. The
`#[ must_use ]` on `set` is declined separately and for a different reason, at
[`api/001`](../api/001_ten_functions_six_const_seven_must_use.md) SL17.
Now prints: `// The displaced value is dropped, deliberately, and this is not the same`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`pattern/001`](001_one_trait_two_shapes.md) | The other convention this crate establishes |
| [`api/001`](../api/001_ten_functions_six_const_seven_must_use.md) | The `must_use` census, and `set`'s absence from it |
| [`item/001`](../item/001_the_four_of_a_typed_slot.md) | `set` among the four, and its inherent-only reachability |
| [`lifecycle/001`](../lifecycle/001_a_slot_across_one_publish.md) | The claim-then-set sequence these four sites sit in |
| [`invariant/001`](../invariant/001_a_read_returns_what_was_written.md) | The write-side property `set` upholds without a `Result` |

### Sources

Addressed by content rather than by line number throughout — SL40's own
disposition inserted comments at three of the four call sites, which moved every
address this table used to carry.

| Fact | Where |
|------|-------|
| `set`'s signature and its stated reason | the doc line opening `/// Returning the displaced value rather than dropping it` |
| The four shipped call sites | the doc-comment-filtered census quoted above |
| The one protocol check | the `debug_assert!` beside `let displaced = reserved.set( record );` in `ring_core` |
| Why the other three may not make it | the comments above each of the three discards, quoted above |
| `ring_overflow` has no edge to `ring_slot` | `ring_overflow/Cargo.toml`, `ring_overflow/src/lib.rs` |
| `ring_overflow`'s actual surface | `command grep -E '^pub fn \|^pub enum '` over that file, quoted above |
| Exactly one mutator hands anything back | the three-mutator signature census, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `setting_over_a_value_returns_the_displaced_one` | Three chained `set` calls, each return asserted |
| `a_typed_slot_round_trips_its_value` | `set` onto empty returning `None` |
| `a_typed_slot_holds_non_copy_payloads` | The displaced value as an owned `String`, not a copy |
| `a_slot_holding_a_default_value_is_still_occupied` | A displaced value that compares equal to a fresh one |
| `ring_mpsc` — `every_record_written_is_destroyed_exactly_once` | Not this crate's test, but the one that fails when `ring_core`'s assertion is copied to `ring_mpsc` — the measurement behind the disposition's decline |
| `ring_spsc` — `a_ring_smaller_than_the_traffic_still_loses_nothing` | The same, on the other overwriting ring: a slot occupied on lap 2 is the design |
