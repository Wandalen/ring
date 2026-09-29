# Item: The Eight Readings of a Range

### Scope

- **Purpose**: Take `Claim`'s eight public items one at a time and record what each costs, what each guarantees, and the two places where the shape was decided by something outside this crate.
- **Responsibility**: Tabulate the eight, explain why exactly seven are `const fn` and why the eighth cannot be, and show that the one non-obvious guard in `overlaps` is not covered by its own doctest.
- **In Scope**: `Claim::new`, `start`, `end`, `len`, `is_empty`, `contains`, `sequences`, `overlaps`.
- **Out of Scope**: `Claimer`'s seven — see [`item/002`](002_the_seven_of_the_claimer.md). The census across both types is [`api/001`](../api/001_seventeen_items_and_nothing_that_drops_silently.md).

### The Eight

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A127 -F 'impl Claim' ring_claim/src/lib.rs
```

Live output:

```
impl Claim
{
  /// A claim of `len` sequences beginning at `start`.
  ///
  /// Public because `ring_publish` and the test suites of both crates need to
  /// construct one directly; a producer obtains real claims from
  /// [`Claimer::claim`], which is the only path that establishes exclusivity.
  ///
  /// ```
  /// use ring_claim::Claim;
  /// use ring_types::Seq;
  /// assert_eq!( Claim::new( Seq::ZERO, 0 ).len(), 0 );
  /// ```
  // No `#[ must_use ]` here: `Claim` itself already carries one *with a
  // message*, and a bare attribute on the constructor would only shadow it
  // with a less informative warning.
  pub const fn new( start : Seq, len : usize ) -> Self
  {
    Self { start, len }
  }

  /// The first sequence in the range.
  ///
  /// ```
  /// use ring_claim::Claim;
  /// use ring_types::Seq;
  /// assert_eq!( Claim::new( Seq( 9 ), 2 ).start(), Seq( 9 ) );
  /// ```
  #[ must_use ]
  pub const fn start( self ) -> Seq
  {
    self.start
  }

  /// One past the last sequence in the range.
  ///
  /// ```
  /// use ring_claim::Claim;
  /// use ring_types::Seq;
  /// assert_eq!( Claim::new( Seq( 9 ), 2 ).end(), Seq( 11 ) );
  /// ```
  #[ must_use ]
  pub const fn end( self ) -> Seq
  {
    self.start.advanced_by( self.len as u64 )
  }

  /// How many sequences the range covers.
  ///
  /// ```
  /// use ring_claim::Claim;
  /// use ring_types::Seq;
  /// assert_eq!( Claim::new( Seq::ZERO, 5 ).len(), 5 );
  /// ```
  #[ must_use ]
  pub const fn len( self ) -> usize
  {
    self.len
  }

  /// Whether the range covers nothing.
  ///
  /// ```
  /// use ring_claim::Claim;
  /// use ring_types::Seq;
  /// assert!( Claim::new( Seq( 3 ), 0 ).is_empty() );
  /// ```
  #[ must_use ]
  pub const fn is_empty( self ) -> bool
  {
    self.len == 0
  }

  /// Whether `seq` falls inside the range.
  ///
  /// ```
  /// use ring_claim::Claim;
  /// use ring_types::Seq;
  ///
  /// let claim = Claim::new( Seq( 4 ), 2 );
  /// assert!( claim.contains( Seq( 4 ) ) );
  /// assert!( claim.contains( Seq( 5 ) ) );
  /// assert!( !claim.contains( Seq( 6 ) ), "half-open" );
  /// assert!( !claim.contains( Seq( 3 ) ) );
  /// ```
  #[ must_use ]
  pub const fn contains( self, seq : Seq ) -> bool
  {
    // Compared as raw `u64` rather than through `Seq`'s operators: `PartialOrd`
    // is not callable in a `const fn`, and reaching through the newtype for two
    // comparisons is the entire cost of having this one at compile time.
    seq.0 >= self.start.0 && seq.0 < self.end().0
  }

  /// Every sequence in the range, in order.
  ///
  /// ```
  /// use ring_claim::Claim;
  /// use ring_types::Seq;
  ///
  /// let seen : Vec< u64 > = Claim::new( Seq( 2 ), 3 ).sequences().map( | s | s.0 ).collect();
  /// assert_eq!( seen, vec![ 2, 3, 4 ] );
  /// ```
  pub fn sequences( self ) -> impl Iterator< Item = Seq >
  {
    ( self.start.0..self.end().0 ).map( Seq )
  }

  /// Whether this range shares any sequence with `other`.
  ///
  /// The property `docs/feature/172_multi_producer_claim.md` forbids across
  /// producers, exposed so a test can assert it directly rather than
  /// reconstructing the comparison at every call site.
  ///
  /// ```
  /// use ring_claim::Claim;
  /// use ring_types::Seq;
  ///
  /// let first = Claim::new( Seq( 0 ), 4 );
  /// assert!( !first.overlaps( Claim::new( Seq( 4 ), 4 ) ), "adjacent, not overlapping" );
  /// assert!( first.overlaps( Claim::new( Seq( 3 ), 4 ) ) );
  /// assert!( !first.overlaps( Claim::new( Seq( 0 ), 0 ) ), "an empty claim covers nothing" );
  /// ```
  #[ must_use ]
  pub const fn overlaps( self, other : Self ) -> bool
  {
    // Raw `u64` comparisons for the same reason `contains` uses them, and with
    // more at stake: this predicate is the one the exclusivity tests assert
```

| Item | Line | `const` | `must_use` | Reads a field | Computes |
|------|-----:|:-------:|:----------:|:-------------:|----------|
| `new( start, len )` | 118 | ✔ | — (type's) | — | nothing |
| `start()` | 131 | ✔ | ✔ | `start` | nothing |
| `end()` | 144 | ✔ | ✔ | both | one add |
| `len()` | 157 | ✔ | ✔ | `len` | nothing |
| `is_empty()` | 170 | ✔ | ✔ | `len` | one compare against a literal |
| `contains( seq )` | 188 | **✔** — was ✘ | ✔ | both | two raw `u64` compares |
| `sequences()` | 205 | **✘** | — (`Iterator`'s) | both | a range, mapped |
| `overlaps( other )` | 226 | **✔** — was ✘ | ✔ | all four | two `is_empty`, two compares |

Every one of the eight takes `self` **by value**. Not one takes `&self`. That is
the correct choice for a 16-byte `Copy` type — a reference would be half the
payload and add an indirection — and it is applied without exception, which is
worth noting because the sibling type does the opposite just as uniformly
([`item/002`](002_the_seven_of_the_claimer.md)).

Only `end()` does arithmetic that can be observed. The other seven are field
reads, comparisons, or one iterator construction.

### CL27 — The `const` Line Is a Choice Here, and a Sibling Crate Made It Differently

Seven of eight are `const fn` and one is not. Five of eight were `const` when
this was written, and the split looked arbitrary until the mechanism was
located:

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'derive\|pub const fn' ring_types/src/id.rs | head
```

Live output:

```
//! publications for the lifetime of a ring, and the slot index derived from it.
/// What *does* wrap is the [`SlotIndex`] derived from it, which is a different
#[ derive( Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default ) ]
  pub const fn next( self ) -> Self
  pub const fn advanced_by( self, n : u64 ) -> Self
  pub const fn distance_to( self, later : Self ) -> u64
#[ derive( Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default ) ]
  pub const fn get( self ) -> usize
```

`Seq` (`ring_types/src/id.rs:24`) derives its ordering:

```rust
#[ derive( Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default ) ]
```

and provides arithmetic as inherent methods:

```rust
pub const fn next( self ) -> Self            // :44
pub const fn advanced_by( self, n : u64 ) -> Self   // :57
pub const fn distance_to( self, later : Self ) -> u64  // :74
```

That is the whole explanation. Trait methods are not callable from a `const fn`
on stable, and `>=` / `<` on `Seq` are `PartialOrd` calls. So, as the crate
stood when this was written:

| Item | Used | Const-callable |
|------|------|:--------------:|
| `end()` | `Seq::advanced_by` — an **inherent** `const fn` | ✔ |
| `is_empty()` | `usize == 0` — a primitive compare, not a trait call | ✔ |
| `contains()` | `seq >= self.start`, `seq < self.end()` — **`PartialOrd`** | ✘ — ✔ today |
| `overlaps()` | four `Seq` compares | ✘ — ✔ today |
| `sequences()` | `Range::map`, returning `impl Iterator` | ✘ |

The two ✘ that became ✔ did so by the escape the rest of this finding
describes; `sequences` is the only one no escape reaches.

So far this reads as a constraint imposed by `ring_types`. It is not — `Seq` is
`pub struct Seq( pub u64 )`, so the field is reachable from any crate, and
comparing `.0` directly is a primitive comparison that `const fn` accepts.

`ring_batch` does exactly that, and gets `const` on both:

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/^  pub const fn contains( &self, seq : Seq ) -> bool$/,/^  }$/p;/^  pub const fn overlaps( &self, other : &Self ) -> bool$/,/^  }$/p' ring_batch/src/lib.rs
```

Live output:

```
  pub const fn contains( &self, seq : Seq ) -> bool
  {
    seq.0 >= self.start.0 && seq.0 < self.end().0
  }
  pub const fn overlaps( &self, other : &Self ) -> bool
  {
    !self.is_empty() && !other.is_empty()
      && self.start.0 < other.end().0 && other.start.0 < self.end().0
  }
```

```rust
pub const fn contains( &self, seq : Seq ) -> bool
{
  seq.0 >= self.start.0 && seq.0 < self.end().0
}

pub const fn overlaps( &self, other : &Self ) -> bool
{
  !self.is_empty() && !other.is_empty()
    && self.start.0 < other.end().0 && other.start.0 < self.end().0
}
```

Same predicate, same guards, same half-open convention — and `const`, because
the comparison is on `u64` rather than on `Seq`:

| | `ring_claim::Claim` — as written | `ring_batch::BatchClaim` |
|--|----------------------------------|--------------------------|
| `contains` | `seq >= self.start` — `PartialOrd` | `seq.0 >= self.start.0` — primitive |
| `const` | ✘ | **✔** |
| `overlaps` | `self.start < other.end()` | `self.start.0 < other.end().0` |
| `const` | ✘ | **✔** |
| `const fn` count | 5 of 8 | **7 of 8** |

So the boundary was a decision made *here*, and the lower-tier crate made the
opposite one and came out ahead by two. Nothing in `ring_types` had to change.

Which shape is better is genuinely arguable. Reaching through `.0` bypasses the
newtype — `Seq`'s whole purpose is that it is not a bare `u64` and cannot be
confused with a `SlotIndex`
(`ring_types/src/id.rs:4-7`) — so `ring_claim`'s version was the one that
respected the abstraction, and `ring_batch`'s bought `const` by punching through
it. The cost of respecting it was two `const fn`s the family did not then use in
a `const` context.

`ring_claim` now takes the same escape, by value rather than by reference:

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/^  pub const fn contains( self, seq : Seq ) -> bool$/,/^  }$/p;/^  pub const fn overlaps( self, other : Self ) -> bool$/,/^  }$/p' ring_claim/src/lib.rs
```

Live output:

```
  pub const fn contains( self, seq : Seq ) -> bool
  {
    // Compared as raw `u64` rather than through `Seq`'s operators: `PartialOrd`
    // is not callable in a `const fn`, and reaching through the newtype for two
    // comparisons is the entire cost of having this one at compile time.
    seq.0 >= self.start.0 && seq.0 < self.end().0
  }
  pub const fn overlaps( self, other : Self ) -> bool
  {
    // Raw `u64` comparisons for the same reason `contains` uses them, and with
    // more at stake: this predicate is the one the exclusivity tests assert
    // with, so having it answerable at compile time is worth reaching through
    // the newtype for.
    !self.is_empty() && !other.is_empty()
      && self.start.0 < other.end().0
      && other.start.0 < self.end().0
  }
```

**Disposition:** applied — `ring_claim` took the same `.0` escape, so both
crates now reach 7 of 8 and both columns above read as the right-hand one does.
What the fix did *not* copy is the silence — `Claim::contains` and
`Claim::overlaps` each carry a comment naming `PartialOrd`'s unavailability in a
`const fn` as the reason they compare raw `u64`, which is what makes the trade
readable at the call site rather than only here. Now prints: `pub const fn contains( self, seq : Seq ) -> bool`

What is worth recording is that the trade was not written down in either crate
when this was found. Neither mentioned the other, neither said why it compared
the way it did, and a reader of either would reasonably have concluded the shape
was forced. `ring_batch` still says nothing; `ring_claim` now does.

### CL28 — `overlaps`'s Empty Guards Are Load-Bearing, and Its Doctest Misses Them

```rust
pub const fn overlaps( self, other : Self ) -> bool
{
  // Raw `u64` comparisons for the same reason `contains` uses them, and with
  // more at stake: this predicate is the one the exclusivity tests assert
  // with, so having it answerable at compile time is worth reaching through
  // the newtype for.
  !self.is_empty() && !other.is_empty()
    && self.start.0 < other.end().0
    && other.start.0 < self.end().0
}
```

The `const` and the `.0` are CL27's subject, not this one's. What matters here
is the two `is_empty()` calls, which look like defensive noise in front of a
standard
half-open interval test. They are not — drop them and the function returns
`true` for a pair that shares nothing:

| `self` | `other` | Without the guards | Correct |
|--------|---------|:------------------:|:-------:|
| `(0, 0)` | `(0, 4)` | `0 < 4 && 0 < 0` → **false** | false |
| **`(2, 0)`** | **`(0, 4)`** | `2 < 4 && 0 < 2` → **true** | **false** |

An empty claim *at the boundary* is caught by the arithmetic. An empty claim
strictly *inside* another range is not: it has a start the other range contains
and an `end()` equal to that start, so both strict comparisons pass and a range
covering zero sequences reports overlapping four.

Now the doctest (`:220-223`):

```rust
let first = Claim::new( Seq( 0 ), 4 );
assert!( !first.overlaps( Claim::new( Seq( 4 ), 4 ) ), "adjacent, not overlapping" );
assert!( first.overlaps( Claim::new( Seq( 3 ), 4 ) ) );
assert!( !first.overlaps( Claim::new( Seq( 0 ), 0 ) ), "an empty claim covers nothing" );
```

The third line is labelled as the empty case, and it is the **first** row of the
table above — the one the guards do not affect. Delete both `is_empty()` calls
and all three assertions still pass. The doctest that documents the guard does
not exercise it.

What does exercise it is `overlap_is_symmetric_and_detects_every_shared_sequence`,
which enumerates `a_len in 0..5` and `b_len in 0..5` across starts `0..6` and
compares against a definition built from `sequences` and `contains`:

```rust
let shares = a.sequences().any( | s | b.contains( s ) );
assert_eq!( a.overlaps( b ), shares, "{a:?} vs {b:?}" );
```

Reimplementing both variants against that definition and counting where they
part gives the size of the gap exactly:

| | Guarded (shipped) | Unguarded |
|--|:-----------------:|:---------:|
| Doctest assertions passed | 3 / 3 | **3 / 3** |
| 900-pair test — pairs where the verdict differs from `shares` | 0 | **52** |

Fifty-two of nine hundred, and the lowest-numbered is `(0, 2)` against `(1, 0)`
— an empty claim one sequence into a two-wide range. So the guard is covered,
by the exhaustive test, 900 pairs away from the code it protects, and not by the
three-line example sitting directly above it.

The fix is one character in the doctest: `Seq( 0 )` → `Seq( 2 )` on the last
line, which turns the assertion that currently documents the guard into one that
also tests it. Worth doing precisely because the doctest is what a reader sees
first, and it currently reads as though it covers the case.

### Why `sequences` Is the Only One Without an Explicit Attribute

Seven of the eight are annotated; two are not — `new` and `sequences` — and both
omissions are deliberate for the same reason. The source states it only for the
first (`:115-117`):

> No `#[ must_use ]` here: `Claim` itself already carries one *with a message*,
> and a bare attribute on the constructor would only shadow it with a less
> informative warning.

`sequences` gets the same treatment silently: its return type is `impl Iterator`,
and `Iterator` is `#[ must_use = "iterators are lazy and do nothing unless
consumed" ]` in `core`. Return-position `impl Trait` inherits it, so calling
`sequences()` and discarding the result already warns:

```
warning: unused implementer of `Iterator` that must be used
  = note: iterators are lazy and do nothing unless consumed
```

So the rule holds without exception across the type: **every item's result is
protected, and the two carrying no attribute are exactly the two whose return
type carries a better one.** The same rule extends to `Claimer`
([`item/002`](002_the_seven_of_the_claimer.md)) and is counted whole in
[`api/001`](../api/001_seventeen_items_and_nothing_that_drops_silently.md).

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_seventeen_items_and_nothing_that_drops_silently.md](../api/001_seventeen_items_and_nothing_that_drops_silently.md) | The census across both types |
| [../api/002_the_two_constructors_of_a_range.md](../api/002_the_two_constructors_of_a_range.md) | `Claim::new` against `Claimer::claim` |

### Items

| File | Relationship |
|------|--------------|
| [002_the_seven_of_the_claimer.md](002_the_seven_of_the_claimer.md) | The other type's seven, taking `&self` throughout |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_sixteen_bytes_and_one_hundred_twenty_eight.md](../data_structure/001_sixteen_bytes_and_one_hundred_twenty_eight.md) | Why sixteen bytes makes `self`-by-value right |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_no_two_producers_hold_one_sequence.md](../invariant/001_no_two_producers_hold_one_sequence.md) | The property `overlaps` exists to let a test assert |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_the_half_open_range_as_a_value.md](../pattern/002_the_half_open_range_as_a_value.md) | The half-open convention these eight implement |

### Sources

| File | Relationship |
|------|--------------|
| `ring_claim/src/lib.rs:102-236` | The eight, in full |
| `ring_claim/src/lib.rs:115-117` | Why the constructor carries no attribute |
| `ring_types/src/id.rs:24-25` | The derived ordering, and the `pub u64` that makes the alternative available |
| `ring_types/src/id.rs:44-85` | The three inherent `const fn`s that make `end` `const` |
| `ring_batch/src/lib.rs:145-148,185-189` | The same two predicates, `const`, comparing `.0` |

### Tests

| File | Relationship |
|------|--------------|
| `tests/claim_test.rs:55` — `a_claim_is_half_open` | `start`/`end`/`len` together |
| `tests/claim_test.rs:69` — `an_empty_claim_contains_nothing…` | `is_empty` and `contains` at zero width |
| `tests/claim_test.rs:82` — `sequences_yields_exactly_the_range` | `sequences` against an explicit list |
| `tests/claim_test.rs:112` — `overlap_is_symmetric_and_detects_every_shared_sequence` | The 900 pairs that cover the empty guard |
