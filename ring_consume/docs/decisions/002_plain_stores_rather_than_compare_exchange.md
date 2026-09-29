# Decision: Plain Stores Rather Than Compare-Exchange

### Scope

**Purpose:** Record why the read half of the handshake uses unconditional stores
where the write half uses compare-exchange, and establish what that decision
assumes.

**Responsibility:** The atomic operations `ring_consume` performs, counted
against its siblings, and the single-consumer premise that licenses them.

**In Scope:** Every atomic operation in the four handshake crates; the module
documentation's single-consumer argument; what enforces it.

**Out of Scope:** The ordering constants themselves — that is
[`non_functional_requirement/002`](../non_functional_requirement/002_eleven_constants_and_the_one_that_is_shared.md).
The guard around the store, which is
[`algorithm/002`](../algorithm/002_the_two_sided_guard.md).

---

## The Mirror Image

```sh
cd "$(git rev-parse --show-toplevel)"
echo "crate          CAS store load fetch_add"
for c in ring_claim ring_publish ring_consume ring_barrier ring_cursor ring_gating; do
  s=$( grep -vE "^[[:space:]]*//" ring/$c/src/*.rs )
  printf '%-14s %-3s %-5s %-4s %s\n' "$c" \
    "$( echo "$s" | grep -c 'compare_exchange' )" \
    "$( echo "$s" | grep -cE '\.store\(' )" \
    "$( echo "$s" | grep -cE '\.load\(' )" \
    "$( echo "$s" | grep -c 'fetch_add' )"
done
```

Live output:

```
crate          CAS store load fetch_add
ring_claim     2   0     1    0
ring_publish   1   0     1    0
ring_consume   0   2     1    0
ring_barrier   0   0     0    0
ring_cursor    2   1     5    2
ring_gating    0   0     0    0
```

### CN8 — The Two Halves Are Exact Opposites, and Both Are Right

| | `ring_claim` | `ring_consume` |
|--|------------:|---------------:|
| `compare_exchange` | 2 | **0** |
| `store` | 0 | **2** |
| `load` | 1 | 1 |

Not a rough contrast — an exact inversion, in crates of comparable size solving
the two halves of one problem.

The reason is the number of writers. `ring_claim`'s cursor is advanced by
however many producers a ring has, so the gate check and the advance must be a
single atomic step; a `fetch_add` would advance past the gate and a
load-then-store would race. Its founding decision is precisely that argument.
`ring_consume`'s cursor has exactly one writer, so there is no race to lose:
the value it stores is derived from a state only it can change, and no other
thread will write between the read and the store.

So `ring_claim` pays two compare-exchange loops and `ring_consume` pays two
plain stores, for the same guarantee, because they face different numbers of
writers. The symmetry is the clearest single fact about the handshake and it is
recorded in neither crate.

Two further readings from the same table:

**`ring_barrier` and `ring_gating` perform zero atomic operations.** Both are
pure views over cursors someone else owns; every atomic access they cause goes
through `ring_cursor`'s `SeqCell` trait. That is why `ring_cursor` carries five
loads, and why it was, until `b7e075ca`, the one crate in either chain with an
allocation
([`non_functional_requirement/001`](../non_functional_requirement/001_what_the_read_path_costs.md)
— the read chain now scans clean, and the only remaining allocation site in
either is `ring_gating`, on the write side).

**Nobody uses `fetch_add` outside `ring_cursor`.** `ring_claim`'s founding
decision rejects it explicitly; the other three simply never reach for it. The
two in `ring_cursor` are its own, for a different purpose.

**Cost:** none. Recorded because it is the family's cleanest structural fact and
appears in no crate's documentation.

---

### CN9 — Single-Consumer Is a Premise, and It Is Enforced by Prose

The module documentation states the premise and its failure mode:

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/## Why this is single-consumer/,/does not need to know about it\./p' \
  ring_consume/src/lib.rs
```

Live output:

```
//! ## Why this is single-consumer
//!
//! One cursor, no compare-exchange, plain stores. Two consumers sharing a
//! [`Consumer`] would each advance the same cursor and each believe they had
//! read what the other did. Fanning out to several independent consumers is a
//! matter of giving each its own cursor in the gating set — which is what
//! `ring_gating::GatingSet` already is, and why this crate does not need to
//! know about it.
//!
//! Nothing in the type system enforces this. [`Consumer::new`] takes a
//! `&PaddedCursor`, so calling it twice over the *same* cursor compiles
//! cleanly — two [`Consumer`]s, each believing it owns the only view, each
//! computing an available run that overlaps the other's, each committing over
//! the other. That is the premise's exact failure mode, reachable in two
//! lines of safe code, and no check in this crate or the family reports it.
//!
//! ## Why the cursor is borrowed rather than owned
//!
//! [`Consumer::new`] takes a `&PaddedCursor` from somewhere else, and the
//! somewhere else is almost always a producer's `ring_gating::GatingSet`. That
//! is the whole mechanism: the producer decides what it may overwrite by
//! reading the cursors in its set, so a consumer whose position lived in a
//! cursor it owned privately would be invisible to the producer and gate
//! nothing. A ring wired that way runs, passes every single-threaded test, and
//! overwrites unread slots on the first lap.

#![ deny( missing_docs ) ]

use ring_barrier::Barrier;
use ring_cursor::{ PaddedCursor, SeqCell, GATING };
use ring_types::{ RingError, Seq };

/// The ordering a commit is made visible at.
///
/// `Release`, paired with the producer's `Acquire` gating read: the producer
/// must not observe this consumer's advance before the reads that justified it.
/// A `Relaxed` store here lets the producer see freed slots and overwrite them
/// while the reads that freed them are still in flight.
const COMMIT : core::sync::atomic::Ordering = core::sync::atomic::Ordering::Release;

/// A contiguous run of sequences a consumer may read.
///
/// Half-open, like `ring_claim::Claim`, and for the same reason: `end` is
/// directly the sequence to commit once the run has been read.
///
/// ```
/// use ring_consume::Available;
/// use ring_types::Seq;
///
/// let run = Available::new( Seq( 2 ), 3 );
/// assert_eq!( run.start(), Seq( 2 ) );
/// assert_eq!( run.end(), Seq( 5 ) );
/// assert_eq!( run.len(), 3 );
/// assert_eq!( run.sequences().collect::< Vec< _ > >(), vec![ Seq( 2 ), Seq( 3 ), Seq( 4 ) ] );
/// ```
#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
pub struct Available
{
  start : Seq,
  len : u64,
}

impl Available
{
  /// A run of `len` sequences beginning at `start`.
  ///
  /// ```
  /// use ring_consume::Available;
  /// use ring_types::Seq;
  /// assert!( Available::new( Seq::ZERO, 0 ).is_empty() );
  /// ```
  #[ must_use ]
  pub const fn new( start : Seq, len : u64 ) -> Self
  {
    Self { start, len }
  }

  /// The first readable sequence.
  ///
  /// ```
  /// use ring_consume::Available;
  /// use ring_types::Seq;
  /// assert_eq!( Available::new( Seq( 7 ), 2 ).start(), Seq( 7 ) );
  /// ```
  #[ must_use ]
  pub const fn start( self ) -> Seq
  {
    self.start
  }

  /// One past the last readable sequence — the value to commit.
  ///
  /// ```
  /// use ring_consume::Available;
  /// use ring_types::Seq;
  /// assert_eq!( Available::new( Seq( 7 ), 2 ).end(), Seq( 9 ) );
  /// ```
  #[ must_use ]
  pub const fn end( self ) -> Seq
  {
    self.start.advanced_by( self.len )
  }

  /// How many sequences are readable.
  ///
  /// ```
  /// use ring_consume::Available;
  /// use ring_types::Seq;
  /// assert_eq!( Available::new( Seq::ZERO, 4 ).len(), 4 );
  /// ```
  #[ must_use ]
  pub const fn len( self ) -> u64
  {
    self.len
  }

  /// Whether there is nothing to read.
  ///
  /// ```
  /// use ring_consume::Available;
  /// use ring_types::Seq;
  /// assert!( Available::new( Seq( 3 ), 0 ).is_empty() );
  /// assert!( !Available::new( Seq( 3 ), 1 ).is_empty() );
  /// ```
  #[ must_use ]
  pub const fn is_empty( self ) -> bool
  {
    self.len == 0
  }

  /// Every readable sequence, in order.
  ///
  /// ```
  /// use ring_consume::Available;
  /// use ring_types::Seq;
  ///
  /// let seen : Vec< u64 > = Available::new( Seq( 5 ), 2 ).sequences().map( | s | s.0 ).collect();
  /// assert_eq!( seen, vec![ 5, 6 ] );
  /// ```
  pub fn sequences( self ) -> impl Iterator< Item = Seq >
  {
    ( self.start.0..self.end().0 ).map( Seq )
  }
}

/// One consumer's position, and the barrier bounding how far it may read.
///
/// ```
/// use core::sync::atomic::Ordering;
/// use ring_barrier::Barrier;
/// use ring_consume::Consumer;
/// use ring_cursor::{ PaddedCursor, SeqCell };
/// use ring_types::Seq;
///
/// let published = [ PaddedCursor::default() ];
/// published[ 0 ].store( Seq( 3 ), Ordering::Release );
///
/// // In a wired ring this cursor comes from the producer's gating set.
/// let position = PaddedCursor::default();
/// let consumer = Consumer::new( &position, Barrier::over( &published ) );
///
/// assert_eq!( consumer.available().len(), 3 );
/// assert_eq!( consumer.commit( Seq( 3 ) ), Ok( Seq( 3 ) ) );
/// assert!( consumer.available().is_empty(), "caught up" );
/// assert_eq!( position.load( Ordering::Acquire ), Seq( 3 ), "the producer sees it" );
/// ```
#[ derive( Debug ) ]
pub struct Consumer< 'a >
{
  cursor : &'a PaddedCursor,
  barrier : Barrier< 'a >,
}

impl< 'a > Consumer< 'a >
{
  /// A consumer reporting into `cursor`, bounded by `barrier`.
  ///
  /// `cursor` is not reset: a consumer built over a cursor that has already
  /// advanced resumes from there, which is what makes it safe to construct one
  /// around a position the producer is already gating on.
  ///
  /// ```
  /// use core::sync::atomic::Ordering;
  /// use ring_barrier::Barrier;
  /// use ring_consume::Consumer;
  /// use ring_cursor::{ PaddedCursor, SeqCell };
  /// use ring_types::Seq;
  ///
  /// let published = [ PaddedCursor::default() ];
  /// let position = PaddedCursor::new( Seq( 7 ) );
  ///
  /// assert_eq!( Consumer::new( &position, Barrier::over( &published ) ).position(), Seq( 7 ) );
  /// ```
  #[ must_use ]
  pub const fn new( cursor : &'a PaddedCursor, barrier : Barrier< 'a > ) -> Self
  {
    Self { cursor, barrier }
  }

  /// The cursor this consumer reports into — the one a producer gates on.
  ///
  /// The producer reads this to decide what it may overwrite, which is why
  /// [`commit`] is the only thing that advances it.
  ///
  /// The returned reference is a full [`SeqCell`], so it also permits a
  /// direct `store` — which bypasses `commit`'s guard entirely, along with
  /// every guarantee this type provides. The only sound reason to call this
  /// accessor is to assert wiring identity, as the doctest below does with
  /// `ptr::eq`; never to write through it.
  ///
  /// [`commit`]: Self::commit
  ///
  /// ```
  /// use core::sync::atomic::Ordering;
  /// use ring_barrier::Barrier;
  /// use ring_consume::Consumer;
  /// use ring_cursor::{ PaddedCursor, SeqCell };
  /// use ring_types::Seq;
  ///
  /// let published = [ PaddedCursor::default() ];
  /// let position = PaddedCursor::default();
  /// let consumer = Consumer::new( &position, Barrier::over( &published ) );
  ///
  /// assert_eq!( consumer.cursor().load( Ordering::Acquire ), Seq::ZERO );
  /// assert!( core::ptr::eq( consumer.cursor(), &position ), "the very same cursor" );
  /// ```
  #[ must_use ]
  pub const fn cursor( &self ) -> &'a PaddedCursor
  {
    self.cursor
  }

  /// The barrier bounding this consumer.
  ///
  /// ```
  /// use ring_barrier::Barrier;
  /// use ring_consume::Consumer;
  /// use ring_cursor::PaddedCursor;
  ///
  /// let published = [ PaddedCursor::default(), PaddedCursor::default() ];
  /// let position = PaddedCursor::default();
  ///
  /// assert_eq!( Consumer::new( &position, Barrier::over( &published ) ).barrier().len(), 2 );
  /// ```
  #[ must_use ]
  pub const fn barrier( &self ) -> Barrier< 'a >
  {
    self.barrier
  }

  /// How far this consumer has committed.
  ///
  /// ```
  /// use ring_barrier::Barrier;
  /// use ring_consume::Consumer;
  /// use ring_cursor::PaddedCursor;
  /// use ring_types::Seq;
  ///
  /// let published = [ PaddedCursor::default() ];
  /// let position = PaddedCursor::default();
  ///
  /// assert_eq!( Consumer::new( &position, Barrier::over( &published ) ).position(), Seq::ZERO );
  /// ```
  #[ must_use ]
  pub fn position( &self ) -> Seq
  {
    self.cursor.load( GATING )
  }

  /// The run of sequences readable right now.
  ///
  /// Empty when the consumer has caught up. Never includes a claimed-but-
  /// unpublished slot, because the barrier is built over the *published*
  /// cursor — which is the whole of feature 170's first clause, and is a
  /// property of what the barrier was pointed at rather than of this function.
  ///
  /// ```
  /// use core::sync::atomic::Ordering;
  /// use ring_barrier::Barrier;
  /// use ring_consume::Consumer;
  /// use ring_cursor::{ PaddedCursor, SeqCell };
  /// use ring_types::Seq;
  ///
  /// let published = [ PaddedCursor::default() ];
  /// let position = PaddedCursor::default();
  /// let consumer = Consumer::new( &position, Barrier::over( &published ) );
  /// assert!( consumer.available().is_empty() );
  ///
  /// published[ 0 ].store( Seq( 5 ), Ordering::Release );
  /// let run = consumer.available();
  /// assert_eq!( ( run.start(), run.end() ), ( Seq::ZERO, Seq( 5 ) ) );
  /// ```
  #[ must_use ]
  pub fn available( &self ) -> Available
  {
    let position = self.position();
    let readable = self
      .barrier
      .frontier()
      .map_or( 0, | frontier | ring_seqno::pending( frontier, position ) );

    Available::new( position, readable )
  }

  /// At most `max` of what is available, for a consumer with a batch limit.
  ///
  /// ```
  /// use core::sync::atomic::Ordering;
  /// use ring_barrier::Barrier;
  /// use ring_consume::Consumer;
  /// use ring_cursor::{ PaddedCursor, SeqCell };
  /// use ring_types::Seq;
  ///
  /// let published = [ PaddedCursor::default() ];
  /// published[ 0 ].store( Seq( 9 ), Ordering::Release );
  ///
  /// let position = PaddedCursor::default();
  /// let consumer = Consumer::new( &position, Barrier::over( &published ) );
  /// assert_eq!( consumer.available_up_to( 4 ).len(), 4 );
  /// assert_eq!( consumer.available_up_to( 100 ).len(), 9, "capped by what is there" );
  /// ```
  #[ must_use ]
  pub fn available_up_to( &self, max : u64 ) -> Available
  {
    let run = self.available();
    Available::new( run.start(), run.len().min( max ) )
  }

  /// Report that everything before `through` has been read.
  ///
  /// Frees those slots for the producer, so it must be called only after the
  /// reads are done — see the module documentation on the window between
  /// [`available`] and here.
  ///
  /// [`available`]: Self::available
  ///
  /// # Errors
  ///
  /// [`RingError::Empty`] when `through` is past what is available: the
  /// consumer would be freeing slots it has not read. [`RingError::Empty`] also
  /// when `through` is behind the current position, which would re-read slots
  /// the producer has already been cleared to reuse.
  ///
  /// ```
  /// use core::sync::atomic::Ordering;
  /// use ring_barrier::Barrier;
  /// use ring_consume::Consumer;
  /// use ring_cursor::{ PaddedCursor, SeqCell };
  /// use ring_types::{ RingError, Seq };
  ///
  /// let published = [ PaddedCursor::default() ];
  /// published[ 0 ].store( Seq( 4 ), Ordering::Release );
  ///
  /// let position = PaddedCursor::default();
  /// let consumer = Consumer::new( &position, Barrier::over( &published ) );
  ///
  /// assert_eq!( consumer.commit( Seq( 5 ) ), Err( RingError::Empty ), "not read yet" );
  /// assert_eq!( consumer.commit( Seq( 3 ) ), Ok( Seq( 3 ) ) );
  /// assert_eq!( consumer.commit( Seq( 2 ) ), Err( RingError::Empty ), "backwards" );
  /// ```
  ///
  /// The ordinary call site never takes the error arm shown above: a value
  /// read from [`available`] is always still in range when it reaches
  /// `commit`, because between the two calls the run can only grow.
  ///
  /// ```
  /// use core::sync::atomic::Ordering;
  /// use ring_barrier::Barrier;
  /// use ring_consume::Consumer;
  /// use ring_cursor::{ PaddedCursor, SeqCell };
  /// use ring_types::Seq;
  ///
  /// let published = [ PaddedCursor::default() ];
  /// published[ 0 ].store( Seq( 4 ), Ordering::Release );
  ///
  /// let position = PaddedCursor::default();
  /// let consumer = Consumer::new( &position, Barrier::over( &published ) );
  ///
  /// let run = consumer.available();
  /// // ... read the slots in `run` ...
  /// assert_eq!( consumer.commit( run.end() ), Ok( Seq( 4 ) ) );
  /// ```
  pub fn commit( &self, through : Seq ) -> Result< Seq, RingError >
  {
    let run = self.available();
    if through < run.start() || through > run.end()
    {
      return Err( RingError::Empty );
    }

    self.cursor.store( through, COMMIT );
    Ok( through )
  }

  /// Commit everything currently available, and report how far that reached.
  ///
  /// Call this only when the whole available run has been read. After a
  /// partial read, use [`commit`]`( first_unread )` instead — committing
  /// everything here tells the producer that slots which were never read are
  /// free to overwrite.
  ///
  /// [`commit`]: Self::commit
  ///
  /// ```
  /// use core::sync::atomic::Ordering;
  /// use ring_barrier::Barrier;
  /// use ring_consume::Consumer;
  /// use ring_cursor::{ PaddedCursor, SeqCell };
  /// use ring_types::Seq;
  ///
  /// let published = [ PaddedCursor::default() ];
  /// published[ 0 ].store( Seq( 6 ), Ordering::Release );
  ///
  /// let position = PaddedCursor::default();
  /// let consumer = Consumer::new( &position, Barrier::over( &published ) );
  /// assert_eq!( consumer.commit_available(), Seq( 6 ) );
  /// assert_eq!( consumer.position(), Seq( 6 ) );
  /// ```
  // Duplicates `commit`'s store rather than delegating to it: `run.end()` is
  // always inside `commit`'s accepted range, so delegating would mean either
  // discarding an unreachable `Err` (`unwrap_or`, itself a smell) or changing
  // this function's return type to `Result` for an error that can never
  // occur. Two stores, kept in step by hand, was judged the smaller cost —
  // which means anything added to `commit` later (a debug assertion, a
  // counter, a trace hook) must be added here too, by hand.
  pub fn commit_available( &self ) -> Seq
  {
    let run = self.available();
    let end = run.end();
    debug_assert!( end >= run.start(), "available() must never return end < start" );
    if end != run.start()
    {
      self.cursor.store( end, COMMIT );
    }
    end
  }
}
```

Quoted:

> One cursor, no compare-exchange, plain stores. Two consumers sharing a
> [`Consumer`] would each advance the same cursor and each believe they had
> read what the other did. Fanning out to several independent consumers is a
> matter of giving each its own cursor in the gating set — which is what
> `ring_gating::GatingSet` already is, and why this crate does not need to
> know about it.

The argument is right, complete, and names the correct way to fan out. What it
cannot do is prevent the wrong way.

`Consumer` is `Sync` (auto-derived —
[`data_structure/002`](../data_structure/002_two_borrows_and_no_owned_state.md)
CN25), so `&Consumer` crosses thread boundaries freely. `Consumer` is 24 bytes
of borrows, so constructing a second one over the same cursor costs nothing and
requires no privilege. Both of these are necessary — the first for the threaded
tests, the second because a `Consumer` is meant to be a cheap view. Together
they mean:

```rust
let a = Consumer::new( &cursor, Barrier::over( &published ) );
let b = Consumer::new( &cursor, Barrier::over( &published ) );   // compiles
```

Two consumers, one cursor, no warning. Each will compute an available run
including sequences the other is about to claim as read, each will commit over
the other, and the producer will be told slots are free that nobody read. The
module documentation's exact failure mode, reachable in two lines of safe code.

What would prevent it: `Consumer::new` taking `&'a mut PaddedCursor`. That
does not work — the producer must be able to read the same cursor concurrently
(`data_structure/002` CN24), and `&mut` would forbid it. The exclusivity this
crate needs is "one writer, many readers," which Rust's borrow checker does not
express for shared atomics. So the premise genuinely cannot be enforced by the
type system as the crate is shaped, and prose is not a lazy choice but the only
remaining one.

That makes the gap a documentation problem rather than a design one, and it
sharpens what the documentation should say. The module doc explains why the
crate is single-consumer. It does not say that constructing two `Consumer`s
over one cursor is the way to violate it, that doing so compiles, or that no
check anywhere will report it. A reader who understands the premise perfectly
can still write the two lines above.

**Cost:** reachable. The premise is correct, argued, and unenforceable; the
specific two-line mistake that violates it is not named where a reader would
look.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A2 -F '//! Nothing in the type system enforces this.' ring_consume/src/lib.rs
```

Live output:

```
//! Nothing in the type system enforces this. [`Consumer::new`] takes a
//! `&PaddedCursor`, so calling it twice over the *same* cursor compiles
//! cleanly — two [`Consumer`]s, each believing it owns the only view, each
```

**Disposition:** applied — the module documentation now names the specific
two-line mistake directly: constructing two `Consumer`s over one cursor
compiles cleanly and is exactly how the single-consumer premise is violated,
with no check anywhere reporting it. The crate's 22 tests (1
`allocation_test.rs` + 21 `consume_test.rs`) plus 17 doctests re-verified
passing (`cargo test --all-features`, 2026-09-04). Now prints:
`Nothing in the type system enforces this.`

---

## What Is Correctly Absent

| Not present | Correctly so |
|-------------|--------------|
| a compare-exchange on commit | one writer; a CAS would cost an RMW to defend against a race that cannot occur |
| a `fetch_max` on the cursor | would silently permit the backwards commit `commit`'s guard refuses |
| a `loom` model | `ring_atomic` provides the seam and `ring_publish`'s handshake test uses it; a single-writer store has no interleaving to explore |
| an owned cursor to enforce exclusivity | forbids the producer's concurrent read, which is the whole mechanism |

---

## Cross-References

| Definition | Instance | Relationship |
|------------|----------|--------------|
| decisions | [001](001_two_calls_not_one.md) | the other founding decision, about the split |
| non_functional_requirement | [002](../non_functional_requirement/002_eleven_constants_and_the_one_that_is_shared.md) | the ordering the two stores carry |
| data_structure | [002](../data_structure/002_two_borrows_and_no_owned_state.md) | the `Sync` that makes CN9's two lines compile |
| invariant | [002](../invariant/002_the_cursor_only_moves_forward.md) | the property the single-writer premise protects |
| algorithm | [002](../algorithm/002_the_two_sided_guard.md) | the guard that runs before each store |

### Sources

| What | Where |
|------|-------|
| The two stores | `ring_consume/src/lib.rs:433,475` |
| The one load | `ring_consume/src/lib.rs:309` |
| The single-consumer argument | `ring_consume/src/lib.rs`, module doc |
| `ring_claim`'s two compare-exchanges | `ring_claim/src/lib.rs` |

### Tests

| Claim | Verified by |
|-------|-------------|
| Zero compare-exchange, two stores | the per-crate atomic count above |
| `ring_claim` is the exact inverse | the same count, `ring_claim` row |
| `ring_barrier`/`ring_gating` do no atomics | the same count, both rows zero |
| Two `Consumer`s over one cursor compile | `Consumer::new` takes `&PaddedCursor`; nothing is consumed |
