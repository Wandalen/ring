# Item: The Six of a Run

### Scope

**Purpose:** Take `Available`'s six public items one at a time and record what
each commits to.

**Responsibility:** `Available::new`, `start`, `end`, `len`, `is_empty`,
`sequences` — signature, body, and the guarantee each carries.

**In Scope:** `ring_consume/src/lib.rs:98-185`.

**Out of Scope:** `Consumer`'s eight — that is
[`002`](002_the_eight_of_a_consumer.md). The size and layout, which are
[`data_structure/001`](../data_structure/001_sixteen_and_twenty_four.md).

---

## The Six

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A88 -F '#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]' ring_consume/src/lib.rs | grep -vE '^\s*///' | grep -v '^\s*$'
```

Live output:

```
#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
pub struct Available
{
  start : Seq,
  len : u64,
}
impl Available
{
  #[ must_use ]
  pub const fn new( start : Seq, len : u64 ) -> Self
  {
    Self { start, len }
  }
  #[ must_use ]
  pub const fn start( self ) -> Seq
  {
    self.start
  }
  #[ must_use ]
  pub const fn end( self ) -> Seq
  {
    self.start.advanced_by( self.len )
  }
  #[ must_use ]
  pub const fn len( self ) -> u64
  {
    self.len
  }
  #[ must_use ]
  pub const fn is_empty( self ) -> bool
  {
    self.len == 0
  }
  pub fn sequences( self ) -> impl Iterator< Item = Seq >
  {
    ( self.start.0..self.end().0 ).map( Seq )
  }
}
```

| Item | Signature | Body |
|------|-----------|------|
| `new` | `const fn new( start : Seq, len : u64 ) -> Self` | `Self { start, len }` |
| `start` | `const fn start( self ) -> Seq` | `self.start` |
| `end` | `const fn end( self ) -> Seq` | `self.start.advanced_by( self.len )` |
| `len` | `const fn len( self ) -> u64` | `self.len` |
| `is_empty` | `const fn is_empty( self ) -> bool` | `self.len == 0` |
| `sequences` | `fn sequences( self ) -> impl Iterator< Item = Seq >` | `( self.start.0..self.end().0 ).map( Seq )` |

Five are one line. The sixth is one line. Every one takes `self` by value —
`Available` is `Copy`, so there is no borrowing anywhere in the type.

---

### CN26 — `Available::new` Is Public, and the Type Therefore Guarantees Nothing

A caller can write `Available::new( Seq( 9_000_000 ), 12 )` and hold a run that
corresponds to no ring, no producer, and no publication. The type is a pair of
integers with accessors; it carries no evidence of where it came from.

That is the right choice, and the reason is worth stating because the opposite
choice is the reflex:

**A private constructor would buy nothing.** `Available` is `Copy` and inert. It
grants no capability — holding one does not reserve a slot, block a producer, or
authorise a read. The only thing a caller can do with a fabricated one is
compute wrong numbers for itself.

**`commit` does not trust it.** The guard re-derives the live range from the
cursor and the barrier and checks `through` against *that*, never against an
`Available` the caller passes in — indeed `commit` takes a `Seq`, not an
`Available`, so a fabricated run cannot even be handed to it. A caller who
computes `bogus.end()` and commits it gets the same treatment as any other
out-of-range value: refused
([`algorithm/002`](../algorithm/002_the_two_sided_guard.md)).

**It makes the type testable.** Three of the crate's 21 tests construct
`Available` directly — `an_available_run_is_half_open` at `:47`,
`an_empty_run_starts_and_ends_in_the_same_place` at `:60`, and
`sequences_yields_exactly_the_run` at `:74` — and assert its arithmetic without
needing a cursor, a barrier, or a producer. A private constructor would force
all three to build a whole ring to test a subtraction.

The contrast with `ring_claim`'s `Claim` is instructive. `Claim` also has a
public constructor, and there it is a genuine hazard: a fabricated `Claim`
carries an obligation to publish, and publishing a range nobody claimed corrupts
the ring. `ring_claim`'s corpus records the consequences. `Available`'s public
constructor has no equivalent, and the reason is the permission/obligation
distinction that
[`data_structure/001`](../data_structure/001_sixteen_and_twenty_four.md) CN22
draws between two identically-shaped types.

**Cost:** none. Recorded because "public constructor on a concurrency type" is a
pattern worth flagging, and this is the case where it is correct.

---

### CN27 — `end` Is Computed, Not Stored, and Inherits `ring_types`' Arithmetic

```rust
pub const fn end( self ) -> Seq
{
  self.start.advanced_by( self.len )
}
```

`Available` stores a start and a length; `end` derives the third value on
demand. Storing two of three and computing the third is the right call — it
makes an inconsistent `Available` unrepresentable, which is why
`an_empty_run_starts_and_ends_in_the_same_place` can be asserted rather than
enforced.

What it inherits is `advanced_by`'s arithmetic:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A6 -F '  /// assert_eq!( Seq( 10 ).advanced_by( 0 ), Seq( 10 ) );' ring_types/src/id.rs | tail -n 5
```

Live output:

```
  #[ must_use ]
  pub const fn advanced_by( self, n : u64 ) -> Self
  {
    Self( self.0 + n )
  }
```

Plain `+`. So `end()` panics on overflow in a debug build and wraps in a release
build, which is standard Rust and entirely fine at any reachable sequence count
— `u64` at even a billion publications per second is roughly 585 years.

Two things make this worth a finding rather than a footnote.

**The sibling method documented the same arithmetic incorrectly.** `Seq::next`,
just above `advanced_by` in the same file, has an identical body and had a doc
comment claiming it "saturates in a release build". Release `+` wraps; it does
not saturate. That was filed against `ring_types` from two directions
(`ring_claim` CL34, `ring_publish` PB40) and recorded here as well because
`Available::end` is a *third* caller inheriting the arithmetic that sentence
described wrongly.

**Both halves have since been written.** `next`'s paragraph says "wraps to zero
in a release build" and keeps the 584-year figure as the reason that is
survivable. And `advanced_by` — which this entry credited as "correct by silence"
— was not correct by silence: silence is not a contract, and it is the method this
crate actually calls. It now carries an overflow note of its own, saying the
reachability argument does **not** carry over from `next`, because `n` comes from
the caller and reaches the wrap in a single call from any position. That is the
distinction this entry's own reasoning depends on: `end()` derives `n` from `len`,
a stored `usize` that came from a real cursor, so the bound is inherited rather
than checked here.

**`end()` is where an overflow would first be visible.** `start` and `len` are
stored values that came from real cursors; `end` is the only derived quantity in
the type. A ring that somehow reached the wrap point would produce a correct
`start`, a correct `len`, and an `end` that precedes its own `start` — which
`commit`'s guard would then read as a backwards commit and refuse, and
`commit_available` would store
([`invariant/002`](../invariant/002_the_cursor_only_moves_forward.md) CN12).

Neither is a live defect. The second is the concrete form of the conditional
hazard CN12 describes, and it is recorded here because this is the line where
the condition would arise.

**Cost:** none reachable — 585 years. Recorded because it is the specific
arithmetic behind CN12's "if `end` could precede `start`" and because the
family's documentation of that arithmetic is wrong one method away.

---

## The Other Four

| Item | Notes |
|------|-------|
| `start` | pure accessor; `must_use`, `const`, `Copy` self |
| `len` | pure accessor; returns `u64` where `Claim::len` returns `usize` — see [`data_structure/001`](../data_structure/001_sixteen_and_twenty_four.md) |
| `is_empty` | `self.len == 0`; clippy requires it wherever `len` exists, and here it is genuinely used — 9 call sites across the suite |
| `sequences` | takes `self` by value, so no `+ use< >` bound is needed; `Iterator`'s own `must_use` covers the return ([`api/001`](../api/001_sixteen_public_items.md) CN18) |

`sequences` is the only non-`const` item of the six, and the only one using the
`.0` field escape — possible because `Seq`'s field is `pub`
([`type/001`](../type/001_availables_const_surface.md)).

---

## Cross-References

| Definition | Instance | Relationship |
|------------|----------|--------------|
| item | [002](002_the_eight_of_a_consumer.md) | `Consumer`'s eight |
| api | [001](../api/001_sixteen_public_items.md) | the six as part of the 16-item surface |
| data_structure | [001](../data_structure/001_sixteen_and_twenty_four.md) | the layout the six accessors read |
| type | [001](../type/001_availables_const_surface.md) | what the `const` surface commits to |
| invariant | [002](../invariant/002_the_cursor_only_moves_forward.md) | the condition CN27's arithmetic would produce |

### Sources

| What | Where |
|------|-------|
| The six | `ring_consume/src/lib.rs:98-185` |
| `advanced_by` | `ring_types/src/id.rs:65-68` |
| `Seq::next`'s incorrect claim | `ring_types/src/id.rs:34-37` |
| The three direct-construction tests | `ring_consume/tests/consume_test.rs:47,60,74` |

### Tests

| Claim | Verified by |
|-------|-------------|
| All six bodies are one line | the `sed` listing above |
| `end` is derived, not stored | `Available`'s two fields are `start` and `len` |
| `advanced_by` is plain `+` | `ring_types/src/id.rs:67` |
| Three tests construct `Available` directly | `grep -n 'Available::new'` -> `:47`, `:60`, `:74` |
