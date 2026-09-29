# lifecycle

Two lifecycles: a sequence's, and the consumer's. They are unrelated in length —
a sequence passes through four states in microseconds, a `Consumer` may be built
and dropped a thousand times around a cursor that outlives all of them — and they
share one property that decides both.

Neither has any state of its own. A sequence's state is a pair of comparisons
against two cursors; a `Consumer`'s is nothing at all. Everything durable lives
in the borrowed cursor, which is why a `Consumer` is disposable and why the one
transition that matters is invisible.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [A Sequence From Published to Committed](001_a_sequence_from_published_to_committed.md) | CN30, CN31 — three observable transitions and the load-bearing one that is not, and commit as an irreversible act nothing calls irreversible |
| 002 | [The Consumer Over a Ring's Life](002_the_consumer_over_a_rings_life.md) | CN32, CN33 — disposability the test suite structurally depends on, and a wrap point four crates assume away |

### The Two, Side by Side

| | 001 — a sequence | 002 — a consumer |
|--|------------------|------------------|
| States | unpublished → available → read → committed | construct → poll → advance → drop |
| Duration | microseconds | arbitrary; often shorter than one poll |
| State held | none — inferred from two cursors | none — two borrows |
| Unobservable step | **read** | none |
| Irreversible step | **commit** | none |
| Verified by | `consume_test.rs:97`, `:109` | the fresh-per-case sweep at `:261` |

### The Transition Nobody Can See

Of a sequence's four states, three are observable from inside this crate:
unpublished (the frontier has not reached it), available (it lies between
position and frontier), committed (the position has passed it). The fourth —
*read* — happens in the caller's buffer, which this crate never touches.

Every hazard in the crate is that one gap wearing different clothes: committing
before reading, `commit_available` after a partial read, and never committing at
all are all "the crate believes a claim it cannot check." That is irreducible —
a primitive that could verify the read would have to own the buffer, making it a
different crate — and the module documentation frames it correctly, once, in a
place neither `commit`'s nor `commit_available`'s own documentation references.

### Disposability, and What It Actually Requires

A `Consumer` can be built and dropped freely: construction initialises nothing,
destruction releases nothing, and the next one over the same cursor resumes
exactly where the last stopped. The test suite depends on this structurally —
the 900-case sweep builds a fresh `Consumer` per case.

The subtlety is that a fresh `Consumer` is *not* enough to reset. The sweep also
builds a fresh `PaddedCursor` per case, because an accepted commit moves the
position and the state lives one level down. `tests/manual/readme.md § N6` exists
precisely because the first draft reused one consumer across the inner loop and
was silently testing different cases than it asserted.

So disposability is load-bearing in two directions at once, and it is exactly why
`Consumer::new` not resetting the cursor is correct: a constructor that zeroed it
would tell every producer the consumer had read nothing.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# the four states' tests: two cursors and two comparisons
grep -A10 'pub fn available(' ring_consume/src/lib.rs

# the irreversible step, and the guard that forbids going back
grep -A10 'pub fn commit(' ring_consume/src/lib.rs

# construction: no store, no load
grep -A5 'pub const fn new( cursor' ring_consume/src/lib.rs

# no destructor, no reset, anywhere
grep 'impl Drop\|fn reset' ring_consume/src/lib.rs || echo "  none"

# the sweep that needs a fresh consumer AND a fresh cursor per case
command grep -m1 -A24 -F '  let published = published_at( 6 );' ring_consume/tests/consume_test.rs | tail -n 23

# why § N6 exists -- bounded short of its own nested fence, which a bare
# ``` closer inside a grep capture would prematurely end this file's own
# quoted Live output at (recipes.py's fence scan is line-anchored, not
# fence-depth-aware)
grep -A6 'N6' ring_consume/tests/manual/readme.md

# the wrap assumption, and where its consequence is visible
grep -vE "^[[:space:]]*//" ring_types/src/id.rs | grep -A4 'fn advanced_by\|fn distance_to'
```

Live output:

```
  pub fn available( &self ) -> Available
  {
    let position = self.position();
    let readable = self
      .barrier
      .frontier()
      .map_or( 0, | frontier | ring_seqno::pending( frontier, position ) );

    Available::new( position, readable )
  }

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
  pub const fn new( cursor : &'a PaddedCursor, barrier : Barrier< 'a > ) -> Self
  {
    Self { cursor, barrier }
  }

  /// The cursor this consumer reports into — the one a producer gates on.
  none
  let consumer = Consumer::new( &position, Barrier::over( &published ) );
  consumer.commit( Seq( 3 ) ).unwrap();

  assert_eq!( consumer.commit( Seq( 3 ) ), Ok( Seq( 3 ) ) );
  assert_eq!( consumer.position(), Seq( 3 ) );
}

#[ test ]
fn the_accepted_commits_are_exactly_the_available_range_inclusive_of_both_ends()
{
  // The full boundary map, in one sweep: for every published frontier and every
  // consumer position, exactly the values in `position..=frontier` are accepted.
  const FRONTIER : u64 = 8;
  let published = published_at( FRONTIER );

  for position in 0..=FRONTIER
  {
    for candidate in 0..FRONTIER + 4
    {
      // A fresh consumer per case rather than one per row: an accepted commit
      // moves the position, and reusing the consumer would silently change
      // which case the next iteration is actually testing.
      let position_cursor = PaddedCursor::default();
## N6 — the exhaustive sweep builds a fresh consumer per case

The sweep in `the_accepted_commits_are_exactly_the_available_range_inclusive_of_both_ends`
is the crate's strongest test and was, in its first draft, self-defeating: it reused one
consumer across the inner loop, and since an accepted commit *moves the
position*, later iterations were silently testing a different case than the one
they asserted.
--
| 2026-08-28 | N1–N6 | 6/6 as expected — N2 measured 2 stores, N5 clean, N6 confirmed fresh-per-case after the first-draft defect was fixed |
  pub const fn advanced_by( self, n : u64 ) -> Self
  {
    Self( self.0 + n )
  }

--
  pub const fn distance_to( self, later : Self ) -> u64
  {
    later.0.saturating_sub( self.0 )
  }
}
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CN30 | `ring_consume` | n/a — doc gap | Three of four transitions are observable and the load-bearing one is not; every hazard in the crate is that gap, framed once in the module doc and referenced by neither commit function |
| CN31 | `ring_consume` | n/a — doc gap | Commit is irreversible, and the easier of the two calls is the one that discards more; nothing in either doc says so |
| CN32 | `ring_consume` | n/a — observation | A `Consumer` is disposable, the 900-case sweep structurally relies on it, and the property is stated as a property nowhere |
| CN33 | family | n/a — observation | The wrap point is ~585 years away and at least four crates assume it away; the one place its consequence is visible describes it as a signedness choice |
