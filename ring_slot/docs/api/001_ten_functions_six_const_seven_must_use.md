# API: Ten Functions, Six `const`, Seven `must_use`

### Scope

**Purpose:** Record the crate's whole public function surface as a census, which
functions carry `#[ must_use ]`, and why the two returning an owned value are
split — one now marked, one deliberately not.

**Responsibility:** The ten `pub fn` items on `TypedSlot` and `BytesSlot`, and
the attributes applied to them.

**In Scope:** Every `pub fn` and `pub const fn` in `ring_slot/src/lib.rs`
and the `#[ must_use ]` attributes above them — enumerated by the recipe below
rather than by line number, since both move whenever the file does.

**Out of Scope:** The `Slot` trait's two methods are
[`api/002`](002_a_trait_with_two_methods.md) — they are trait items, not
inherent `pub fn`, and none of the counts here include them. Whether the four
non-`const` functions *could* be `const` is
[`workaround/001`](../workaround/001_four_functions_that_could_be_const.md). SL18
below takes only the half of `set`'s justification that is checkable against the
family as it stands — the named policy is refused at construction, so it cannot
run; what the return value is *actually* for, and why it is worth keeping
regardless, is
[`pattern/002`](../pattern/002_the_displaced_value_returned.md) SL39.

---

## The Whole Surface

```sh
cd "$(git rev-parse --show-toplevel)"
grep '^\s*pub \(const \)\?fn ' ring_slot/src/lib.rs
```

Live output:

```
  pub const fn empty() -> Self
  pub fn set( &mut self, value : T ) -> Option< T >
  pub const fn get( &self ) -> Option< &T >
  pub fn take( &mut self ) -> Option< T >
  pub const fn empty() -> Self
  pub const fn capacity( &self ) -> usize
  pub const fn len( &self ) -> usize
  pub const fn is_empty( &self ) -> bool
  pub fn write( &mut self, payload : &[ u8 ] ) -> Result< (), RingError >
  pub fn read( &self ) -> &[ u8 ]
```

Ten functions, four on `TypedSlot` and six on `BytesSlot`. The census:

```sh
cd "$(git rev-parse --show-toplevel)"
body=$( grep -vE '^[[:space:]]*//' ring_slot/src/lib.rs )
printf 'pub fn        %d\n' "$( printf '%s\n' "$body" | grep -c '^\s*pub \(const \)\?fn ' )"
printf 'pub const fn  %d\n' "$( printf '%s\n' "$body" | grep -c '^\s*pub const fn ' )"
printf 'must_use      %d\n' "$( printf '%s\n' "$body" | grep -c 'must_use' )"
printf 'must_use msg  %d\n' "$( printf '%s\n' "$body" | grep -c 'must_use =' )"
```

Live output:

```
pub fn        10
pub const fn  6
must_use      7
must_use msg  1
```

Six of ten are `const`; seven of ten carry `#[ must_use ]`; one of the seven
carries a custom message. The comment filter is load-bearing in this command —
without it, the doctest lines that call these functions inflate every count.

The six and the seven are *not* the same set, and the one message belongs to the
seventh — `take`, added by SL17's disposition below, which is also why the
`must_use msg` line reads `1` rather than the `0` this document recorded when
the finding was raised.

---

### SL17 — `must_use` Marks the Five Cheapest Functions and Skips Both That Return an Owned Value

Interleaving the attribute lines with the signatures shows the split directly:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -n 'must_use\|^\s*pub \(const \)\?fn ' ring_slot/src/lib.rs | grep -vE ':[[:space:]]*///' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
```

Live output:

```
  #[ must_use ]
  pub const fn empty() -> Self
  pub fn set( &mut self, value : T ) -> Option< T >
  pub const fn get( &self ) -> Option< &T >
  #[ must_use = "this is the only way a payload leaves the slot; dropping it here destroys the record" ]
  pub fn take( &mut self ) -> Option< T >
  #[ must_use ]
  pub const fn empty() -> Self
  #[ must_use ]
  pub const fn capacity( &self ) -> usize
  #[ must_use ]
  pub const fn len( &self ) -> usize
  #[ must_use ]
  pub const fn is_empty( &self ) -> bool
  pub fn write( &mut self, payload : &[ u8 ] ) -> Result< (), RingError >
  #[ must_use ]
  pub fn read( &self ) -> &[ u8 ]
```

| Function | `must_use`? | What ignoring the result costs |
|----------|:-----------:|--------------------------------|
| `TypedSlot::empty` | ✔ | A constructed value — recomputable for free |
| `TypedSlot::set` | ✘ | The displaced value, dropped — usually the intent; see the disposition |
| `TypedSlot::get` | ✘ | A borrow — nothing |
| `TypedSlot::take` | ✔ | **The taken value, dropped — the only exit the payload has** |
| `BytesSlot::empty` | ✔ | A constructed value — recomputable for free |
| `BytesSlot::capacity` | ✔ | A `const` read of `N` |
| `BytesSlot::len` | ✔ | A field read |
| `BytesSlot::is_empty` | ✔ | An integer comparison |
| `BytesSlot::write` | ✘ | Nothing — `Result` is `#[ must_use ]` by its own type |
| `BytesSlot::read` | ✔ | A slice range |

Three of the four omissions are correct. `get` returns a borrow, so discarding it
is a no-op the compiler already erases; `write` returns `Result`, which carries
`#[ must_use ]` on the type and so warns without help.

The remaining two are the whole finding. `set` and `take` each return
`Option< T >` by value, and for a non-`Copy` `T` that `Option` is the *only*
remaining handle on the payload. Discarding it destroys the value, and nothing
says so:

```sh
cd "$(git rev-parse --show-toplevel)"
# sorted: grep is shimmed to a parallel ugrep here, so hits arrive in completion order
grep -rn '^\s*[a-z_]*\.\(set\|take\)(' ring_*/src/*.rs ring_*/tests/*.rs 2>/dev/null \
  | grep -vE ':[[:space:]]*///?' | grep -v 'let \|= ' | sort | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
```

Live output:

```
ring_store/tests/buffer_test.rs:    slot.set( i * 2 );
ring_store/tests/buffer_test.rs:    slot.set( 1 );
ring_event/src/lib.rs:    slot.set( self );
ring_event/tests/event_test.rs:  typed.set( 5u16 );
ring_event/tests/event_test.rs:  typed.set( 1 );
ring_mpsc/src/lib.rs:    reserved.set( value );
ring_mpsc/tests/mpsc_test.rs:    reserved.set( 42 );
ring_mpsc/tests/mpsc_test.rs:                  reserved.set( value );
ring_mpsc/tests/mpsc_test.rs:                  reserved.set( record );
ring_mpsc/tests/mpsc_test.rs:    first.set( 1 );
ring_mpsc/tests/mpsc_test.rs:    second.set( 2 );
ring_mpsc/tests/mpsc_test.rs:    third.set( 3 );
ring_slot/tests/slot_test.rs:  slot.set( 5u16 );
ring_slot/tests/slot_test.rs:  slot.set( 0u32 );
ring_slot/tests/slot_test.rs:  text.set( String::new() );
ring_slot/tests/slot_test.rs:  typed.set( 9u32 );
ring_slot/tests/slot_test.rs:  slot.set( CountsItsOwnDrop );
```

**Correction (2026-09-28):** this section previously read "fifteen" discarding
call sites and "sixteen" family-wide. The recipe above has been re-run:
`ring_mpsc`'s tests have gained a discarding `set` not present before
(`reserved.set( record )`), so the census now reads seventeen lines and the
family-wide total — this census plus the one bound call site SL18 shows
separately — is eighteen. Every "fifteen"/"sixteen" elsewhere in this document
has been updated to "seventeen"/"eighteen" to match.

Every hit is a `set`. Not one is a `take` — the census pattern covers both, and
`take`'s result is bound at every call site in the family. That asymmetry is the
whole disposition below: the two functions look alike in the signature column and
are used in opposite ways.

`RUSTFLAGS="-D warnings"` and `cargo clippy` are both silent on all seventeen. The
workspace lints table sets `missing_docs`, `unsafe-code = "deny"` and four
others, and neither `unused_results` nor `clippy::must_use_candidate` is among
them — so the attribute on the function is the only mechanism available.

**Finding.** The `must_use` set was inverted relative to consequence: it covered
the five functions whose results are pure, cheap, and recomputable — where
ignoring the result wastes a cycle — and omitted the two whose results are owned
values, where ignoring the result can destroy data.

But "can" is doing real work in that sentence, and the two functions differ on
it. `take` has no other exit: the value it returns is the payload leaving the
slot, so discarding it always loses a record. `set`'s return is a *displaced*
value, and on a ring the slot it displaces belongs to a lap the consumer already
finished — dropping it is what bounds the ring's storage. That is why seventeen
call sites drop it and none of them is a defect.

`core` draws the same line, and drew it first:

```sh
cd "$(git rev-parse --show-toplevel)"
# `TypedSlot::set` wraps `Option::replace`; `TypedSlot::take` wraps `Option::take`.
# Discard all three and ask the compiler which one it objects to.
printf 'fn main() { let mut a : Option< u32 > = Some( 1 ); a.take(); a.replace( 2 ); std::mem::replace( &mut a, None ); }\n' > /tmp/-std_mustuse_probe.rs
rustc --crate-name probe --edition 2021 --emit=metadata -o /dev/null /tmp/-std_mustuse_probe.rs 2>&1 \
  | command grep -oE 'unused return value of `[^`]+`' | LC_ALL=C sort
rm -f /tmp/-std_mustuse_probe.rs
```

Live output:

```
unused return value of `std::mem::replace`
```

One of the three. `Option::take` and `Option::replace` — the two calls this
crate's `take` and `set` are one-line wrappers around — are both unmarked in
`core`, and `mem::replace`, which has no receiver to leave the value in, is
marked. The distinction `core` is drawing is not "returns an owned value"; it is
"is this the value's only remaining handle".

By that test `TypedSlot::take` is on `mem::replace`'s side and `TypedSlot::set`
is on `Option::replace`'s — with one difference that pushes `take` further than
its `core` counterpart: `Option` has `as_ref`, `unwrap`, `map` and a dozen other
exits, and a `TypedSlot` has exactly one.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -B1 -F '  pub fn take( &mut self ) -> Option< T >' ring_slot/src/lib.rs
```

Live output:

```
  #[ must_use = "this is the only way a payload leaves the slot; dropping it here destroys the record" ]
  pub fn take( &mut self ) -> Option< T >
```

**Disposition:** applied — for `take` only; declined for `set`. `take` now carries
`#[ must_use ]` with the message above, and it cost nothing: the census recipe
in this section finds seventeen discarding call sites and every one is a `set`, so
the attribute landed without a single `let _ =` anywhere in the family. Removing
it again is a compile error, not a warning — the workspace builds under
`RUSTFLAGS="-D warnings"` — which was verified by adding one discarding
`probe.take();` to `slot_test.rs` and watching `cargo nextest run -p ring_slot`
refuse to compile with `unused return value of TypedSlot::<T>::take that must be
used`. `set` is declined on the evidence above: `core` leaves the identical call
unmarked, the seventeen discards are the intended use rather than a backlog of
defects, and marking it would turn each into a `let _ =` that documents nothing.
The two shipped-code discards SL40 reads for a different reason are now
commented at their own sites rather than annotated here.
Now prints: `must_use msg  1`

---

### SL18 — `set`'s Stated Justification Names a Consumer That Cannot Reach It

The return value SL17 is about has a documented reason to exist, and the reason
names a specific crate:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '--- what `set` names as its reason ---'
command grep -m1 -A2 -F '  /// Returning the displaced value rather than dropping it keeps the door open' ring_slot/src/lib.rs
echo '--- ring_overflow deps, in full ---'
grep 'ring_' ring_overflow/Cargo.toml | tail -2
echo '--- ring_core, the only crate with both, on DropOldest ---'
command grep -m1 -A3 -F '    if config.overflow() == OverflowPolicy::DropOldest' ring_core/src/lib.rs
```

Live output:

```
--- what `set` names as its reason ---
  /// Returning the displaced value rather than dropping it keeps the door open
  /// for a future evict-oldest policy to hand a caller what it evicted instead
  /// of losing it silently — but `ring_overflow` does not depend on this crate
--- ring_overflow deps, in full ---
ring_types = { path = "../ring_types" }
ring_stats = { path = "../ring_stats" }
--- ring_core, the only crate with both, on DropOldest ---
    if config.overflow() == OverflowPolicy::DropOldest
    {
      return Err( RingError::PolicyUnsupported );
    }
```

Three facts, in order. `ring_overflow` does not depend on `ring_slot` — it
depends on `ring_types` and `ring_stats` and nothing else, so it cannot call
`set` at all; its `Resolution::EvictedOldest` is a verdict enum, not a data path.
`ring_core` is the only crate in the family depending on both, and it *refuses*
`DropOldest` at construction with `PolicyUnsupported`. So the evict-oldest policy
that the return value exists to serve is, today, unreachable.

The one call site in the family that does bind the return value binds it for the
opposite purpose:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r 'let .* = .*\.set(' ring_*/src/*.rs | grep -vE ':[[:space:]]*///?'
command grep -m1 -A2 -F '            let displaced = reserved.set( record );' ring_core/src/lib.rs
```

Live output:

```
ring_core/src/lib.rs:            let displaced = reserved.set( record );
            let displaced = reserved.set( record );
            debug_assert!( displaced.is_none(), "a claimed slot held a record" );
            Ok( () )
```

`ring_core` reads the displaced value to assert it is `None` — a sanity check
that the claim protocol handed out an empty slot. It is not handing anything
back to a caller; it is confirming there was nothing to hand back.

The claim is not confined to the source. The test that covers the displaced
value repeats it:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A4 -F '/// Setting over an occupied slot returns the displaced value rather than' ring_slot/tests/slot_test.rs
```

Live output:

```
/// Setting over an occupied slot returns the displaced value rather than
/// dropping it — what lets an evict-oldest policy hand back what it evicted
/// instead of losing it silently.
#[ test ]
fn setting_over_a_value_returns_the_displaced_one()
```

Same sentence, same justification, one crate — so a reader who checks the test
for confirmation finds the claim restated rather than tested. The test asserts
the return value; it cannot assert the reason.

**Finding.** The return type is right and its documented justification is stale.
`set` returning `Option< T >` is the correct signature — it is `Option::replace`'s
own, it costs nothing, and it keeps the eviction door open — but the sentence
explaining *why* points at a crate that structurally cannot use it and a policy
the family currently rejects.

A reader auditing `TypedSlot::set` to see whether the return value earns its
place follows the pointer to `ring_overflow`, finds no `ring_slot` dependency,
and is left without an answer. The honest version of the sentence names
`ring_core`'s `debug_assert` as the current user and the evict-oldest policy as
the intended one.

That is not an argument for `#[ must_use ]` on `set`, and reading it as one is
the mistake SL17's disposition corrects: one crate looks at the value out of
eighteen call sites, and the other seventeen drop it correctly. A signature worth
keeping and an attribute worth adding are different questions.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_slot
command grep -F 'confirming a freshly claimed slot came' src/lib.rs
```

Live output:

```
    /// is `ring_core`'s `debug_assert`, confirming a freshly claimed slot came
```

**Disposition:** applied — `set`'s doc comment no longer points a reader at
`ring_overflow`, which cannot reach this return value at all; it now names
`ring_core`'s `debug_assert` as the one binding call site today and states
the evict-oldest policy as the door the signature keeps open rather than the
consumer it currently serves.
Now prints: `confirming a freshly claimed slot came`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`api/002`](002_a_trait_with_two_methods.md) | The two trait methods, which none of these counts include |
| [`workaround/001`](../workaround/001_four_functions_that_could_be_const.md) | The four non-`const` functions, all four of which could be |
| [`item/001`](../item/001_the_four_of_a_typed_slot.md) | `set` and `take` one at a time, with what each returns |
| [`pattern/002`](../pattern/002_the_displaced_value_returned.md) | Why returning the displaced value is the family's shape |

### Sources

Addressed by content rather than by line number throughout, because every
census in this document moves its own citations when it is acted on — SL17's
disposition added one attribute and shifted eight of the ten signatures.

| Fact | Where |
|------|-------|
| The ten signatures | `ring_slot/src/lib.rs` — `command grep -n '  pub fn '`, plus the four `Slot` impls |
| The seven `must_use` attributes | `ring_slot/src/lib.rs` — `command grep -c '#\[ must_use'` |
| `set`'s stated justification | `ring_slot/src/lib.rs` — `/// Returning the displaced value rather than dropping it keeps the door open` |
| `set`'s stated reason for carrying no attribute | `ring_slot/src/lib.rs` — the doc line opening `/// Deliberately **not**` |
| `take`'s custom message | `ring_slot/src/lib.rs` — the one `#[ must_use = "…" ]` in the crate |
| `ring_overflow`'s dependencies | `ring_overflow/Cargo.toml` — `[dependencies]` |
| `DropOldest` refused at construction | `ring_core/src/lib.rs` — `OverflowPolicy::DropOldest =>` in `Ring::new` |
| The only binding call site | `ring_core/src/lib.rs` — the `debug_assert!` next to `slot.set(` |
| The workspace lint set | `Cargo.toml` — `[workspace.lints.rust]`, `[workspace.lints.clippy]` |
| Discards compiling clean | Release probe under `RUSTFLAGS="-D warnings"`, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `setting_over_a_value_returns_the_displaced_one` | That the value `set` returns is the previous one |
| `a_typed_slot_round_trips_its_value` | `take`'s return at both states, `Some` then `None` |
| `a_typed_slot_holds_non_copy_payloads` | `take` returning a `String` — the case where discarding it destroys data |
| — | The attribute on `take` is pinned by the compiler, not by a test: removing it and adding one discarding `probe.take();` is the falsification, and the workspace builds under `-D warnings`, so the pin is a build failure rather than a test failure |
