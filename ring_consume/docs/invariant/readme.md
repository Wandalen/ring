# invariant

Two properties, one for each direction the cursor could go wrong. A consumer
must never be told about a sequence the producer has not published, and the
consumer cursor must never move backwards. Both hold. Neither is enforced where
a reader would look for it.

The first lives in a single `map_or` and in the fact that `slowest` returns a
minimum — nine words of code across two crates, with no assertion anywhere
stating that this is where the property is kept. The second is enforced by
`commit`'s lower bound, which is one of two doors into the cursor; the other is
`cursor()`, which hands out a `&PaddedCursor` that a caller may store to
directly.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Never Reads Past What Was Published](001_never_reads_past_what_was_published.md) | CN10, CN11 — the invariant's true location across two crates, and the one threaded test that asserts it |
| 002 | [The Cursor Only Moves Forward](002_the_cursor_only_moves_forward.md) | CN12, CN13 — a guard on one of two doors, and a manual check naming a test that does not exist |

### The Two Configurations

| | I1 — never past the frontier | I2 — never backwards |
|--|------------------------------|----------------------|
| Held by | `frontier().map_or( 0, pending )` | `commit`'s `through < run.start()` refusal |
| Located in | `ring_consume` + `ring_cursor` + `ring_seqno` | `ring_consume` alone |
| Bypassable via | nothing — the arithmetic is total | `cursor()`, in one line of safe code |
| Asserted by | one `thread::scope` test | the 900-case exhaustive sweep at `consume_test.rs:261` |
| Manual check | `§ N4` | `§ N6` — which names a test that does not exist |

### Why the First Is Harder to Locate Than to Verify

`available()` reads:

```rust
self.barrier.frontier().map_or( 0, | frontier | ring_seqno::pending( frontier, position ) )
```

`pending` is `consumer.distance_to( producer )`, which saturates. `frontier` is
`slowest`, which is `.min()`. So the guarantee that no unpublished sequence is
ever offered comes from three facts in three crates: the minimum over
dependencies, the saturation on subtraction, and the `map_or`'s zero default for
an empty barrier.

Delete any one and the invariant breaks in a different way. Nothing names them
together, and the tests assert the *result* rather than the mechanism, which is
correct testing practice and leaves the mechanism undocumented.

### The Second's Open Door

`commit` refuses `through < run.start()`. That is the whole enforcement, and it
is complete for callers who use `commit`. `cursor()` returns `&'a PaddedCursor`,
`PaddedCursor` implements `SeqCell`, and `SeqCell::store` takes `&self` — so:

```rust
consumer.cursor().store( Seq( 0 ), Ordering::Release );
```

…is safe code that moves the cursor backwards past every guard. The accessor
must exist ([`item/002`](../item/002_the_eight_of_a_consumer.md) CN28); the
finding is that the invariant document and the accessor's own doc comment both
decline to mention it.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order

# where I1 actually lives — three crates, three lines
grep 'frontier()' ring_consume/src/lib.rs
grep -vE "^[[:space:]]*//" ring_cursor/src/lib.rs | grep -A4 'pub fn slowest'
grep -vE "^[[:space:]]*//" ring_seqno/src/lib.rs | grep -A3 'pub fn slowest\|pub fn pending'
grep -vE "^[[:space:]]*//" ring_types/src/id.rs | grep -A4 'fn distance_to'

# I2's only guard, and the door beside it
grep -A8 'pub fn commit(' ring_consume/src/lib.rs
grep -E '^\s*(pub )?fn (store|load|fetch_add|compare_exchange)' ring_cursor/src/lib.rs

# what asserts each
command grep 'thread::scope' ring_consume/tests/*.rs
grep 'fn the_accepted_commits\|fn every_sequence' ring_consume/tests/consume_test.rs

# the stale reference in the manual plan
grep 'commit_accepts_exactly_the_reachable_range' ring_consume/tests/manual/readme.md
grep -r 'commit_accepts_exactly_the_reachable_range' ring_consume/tests/*.rs || echo "  no such test"
```

Live output:

```
      .frontier()
pub fn slowest( cursors : &[ PaddedCursor ] ) -> Option< Seq >
{
  cursors.iter().map( | c | c.load( GATING ) ).min()
}

pub fn pending( producer : Seq, consumer : Seq ) -> u64
{
  consumer.distance_to( producer )
}
--
pub fn slowest( cursors : &[ Seq ] ) -> Option< Seq >
{
  cursors.iter().copied().min()
}
  pub const fn distance_to( self, later : Self ) -> u64
  {
    later.0.saturating_sub( self.0 )
  }
}
  pub fn commit( &self, through : Seq ) -> Result< Seq, RingError >
  {
    let run = self.available();
    if through < run.start() || through > run.end()
    {
      return Err( RingError::Empty );
    }

    self.cursor.store( through, COMMIT );
  fn load( &self, order : Ordering ) -> Seq
  fn store( &self, value : Seq, order : Ordering )
  fn fetch_add( &self, n : u64, order : Ordering ) -> Seq
  fn compare_exchange( &self, current : Seq, new : Seq, success : Ordering, failure : Ordering )
ring_consume/tests/consume_test.rs:  std::thread::scope( | scope |
ring_consume/tests/consume_test.rs:  let seen = std::thread::scope( | scope | -> Vec< Seq >
fn the_accepted_commits_are_exactly_the_available_range_inclusive_of_both_ends()
fn every_sequence_is_offered_exactly_once_across_a_full_drain()
  no such test
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CN10 | `ring_consume` | n/a — unenforced | I1 rests on one `map_or` and on `slowest` being a minimum, spread across three crates and named together nowhere |
| CN11 | `ring_consume` | n/a — coverage | One `thread::scope` test asserts I1, and it is the only thing that does |
| CN12 | `ring_consume` | **latent hazard** | `commit`'s lower bound guards one of two doors into the cursor; `cursor()` is the other and is unguarded |
| CN13 | `ring_consume` | **wrong doc** | `tests/manual/readme.md § N6` names `commit_accepts_exactly_the_reachable_range`, which does not exist; the real test is `the_accepted_commits_are_exactly_the_available_range_inclusive_of_both_ends` at `:261` |
