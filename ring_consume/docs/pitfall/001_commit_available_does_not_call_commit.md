# Pitfall: `commit_available` Does Not Call `commit`

### Scope

**Purpose:** Record that the crate's two commit operations share a contract and
not an implementation, and what that costs a future change to either.

**Responsibility:** `Consumer::commit` and `Consumer::commit_available` — their
bodies, the guard one has and the other does not, and the store both perform.

**In Scope:** `Consumer::commit` and `Consumer::commit_available` in
`ring_consume/src/lib.rs`; the tests covering both; what a plausible
future guard would and would not reach.

**Out of Scope:** Whether the guard's conditions are right — they are, and
[`api/002`](../api/002_the_two_commits.md) covers the contract. The memory
ordering on the store, which is
[`non_functional_requirement/002`](../non_functional_requirement/002_eleven_constants_and_the_one_that_is_shared.md).

---

## Two Bodies, One Store Each

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A35 -F '  pub fn commit( &self, through : Seq ) -> Result< Seq, RingError >' ring_consume/src/lib.rs
echo '  -- every store outside a doctest --'
command grep 'store(' ring_consume/src/lib.rs | command grep -v '///'
```

Live output:

```
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
  -- every store outside a doctest --
    self.cursor.store( through, COMMIT );
      self.cursor.store( end, COMMIT );
```

Two stores in the whole crate, one per body. The two functions, side by side:

```rust
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

pub fn commit_available( &self ) -> Seq
{
  let end = self.available().end();
  self.cursor.store( end, COMMIT );
  end
}
```

`commit_available` is `commit( self.available().end() )` in everything but
implementation. It computes the same run, arrives at a value that is by
construction inside that run, and then performs the store itself rather than
handing the value to `commit`.

### CN43 — The Convenience Form Duplicates the Store Rather Than Delegating

Today the two are exactly equivalent, and provably so: `commit_available`'s
argument is `run.end()`, and `commit`'s guard rejects only values `< run.start()`
or `> run.end()`, so `commit( run.end() )` can never take the error branch. The
duplication is currently free.

What it costs is any future change to what committing *means*. Four plausible
additions, and where each would land:

| A change to `commit` adding… | Reaches `commit_available` |
|------------------------------|:--------------------------:|
| a debug assertion on cursor monotonicity | ✘ |
| a statistics counter for committed sequences | ✘ |
| a `ring_trace` hook on the commit event | ✘ |
| a stricter guard (e.g. refusing while a read is outstanding) | ✘ |

Each of those would be written into `commit`, tested through `commit`, and
silently skipped by every caller that used the convenience form. The failure
mode is the family's characteristic one, already recorded four times in
`ring_claim`'s corpus: nothing fires at the point of the mistake. The author
adds a guard, the test for the guard passes, and a second entry point with the
same name-stem and the same documented meaning walks straight past it.

The fix is one line — `commit_available` becoming

```rust
pub fn commit_available( &self ) -> Seq
{
  let end = self.available().end();
  self.commit( end ).unwrap_or( end )
}
```

— except that the `unwrap_or` is itself a smell, because the `Err` arm is
unreachable and the compiler cannot know it. That is the real reason the
duplication exists: making `commit_available` delegate requires either
discarding an impossible error or changing `commit_available`'s return type to
`Result`, and neither is obviously better than two stores. The finding is not
that the current shape is wrong; it is that the shape is a tradeoff nobody has
written down, and the cost of the tradeoff falls entirely on a future change.

**Cost:** reachable, and it is latent rather than present. Nothing is wrong
today. Everything added to `commit` tomorrow is wrong by default.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A2 -F '  // Duplicates `commit`'"'"'s store rather than delegating to it' ring_consume/src/lib.rs
```

Live output:

```
    // Duplicates `commit`'s store rather than delegating to it: `run.end()` is
    // always inside `commit`'s accepted range, so delegating would mean either
    // discarding an unreachable `Err` (`unwrap_or`, itself a smell) or changing
```

**Disposition:** applied — the tradeoff nobody had written down is now a
comment directly above `commit_available`, stating why it duplicates the
store rather than delegating (delegating would mean discarding an unreachable
`Err` or making the return type `Result` for an error that cannot occur) and
naming the cost: anything added to `commit` later must be added here too, by
hand. The crate's 22 tests (1 `allocation_test.rs` + 21 `consume_test.rs`)
plus 17 doctests re-verified passing (`cargo test --all-features`,
2026-09-04). Now prints:
`so delegating would mean either`

---

### CN44 — An Idle Poll Used to Write

`commit_available` stored unconditionally. It did not check whether the run it
computed was empty:

```rust
let end = self.available().end();
self.cursor.store( end, COMMIT );
```

When nothing had been published since the last call, `end` equalled the cursor's
current value, and the crate performed a `Release` store of a value the cursor
already held. Measured — the probe in
[`non_functional_requirement/001`](../non_functional_requirement/001_what_the_read_path_costs.md)
called `commit_available()` a thousand times against a barrier that had 300
sequences available on the first call and nothing on the remaining 999:

```
  Consumer::commit_available()     1000
  ...cursor is now Seq(400), and it allocated on all 1000
```

So a consumer polling an idle ring paid, per poll:

| Per idle poll | Cost, as measured | Today |
|---------------|-------------------|-------|
| Heap allocations | 1 — `ring_cursor::slowest`'s `Vec` (CN34) | 0 — the `Vec` is gone (`b7e075ca`) |
| Atomic `Release` stores | 1 — of the value already there | 0 — this finding's guard |
| Atomic loads | 1 + one per barrier dependency | unchanged |

None of that was incorrect. A redundant release store of an unchanged value is
harmless to the invariant, and on x86 it is a plain `mov`. On a weaker ordering
model it is a real store-release with a real fence, and it dirties a cache line
that other threads are reading — the consumer's cursor is precisely the line
every producer gates on, so an idle consumer polling in a tight loop invalidated
that line on every iteration for every producer.

`ring_cursor`'s `PaddedCursor` exists to stop exactly this kind of interference
between *different* cursors. It could not help here, because the contention was
on the one line that genuinely is shared.

A single `if end != run.start()` — or `if !run.is_empty()` — around the store
removes it, and that is what the crate now does.
`commit_available_takes_everything_and_reports_where_it_reached` already covered
the case where it matters, asserting that a second `commit_available()` on an
already-drained run returns the same sequence; it passes unchanged.

**Cost:** was reachable, and only under polling. A consumer that called
`commit_available` after doing work never saw it; a consumer that spun on an
empty ring paid it on every iteration, and the guard against it was one
comparison.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A9 -F '  pub fn commit_available( &self ) -> Seq' ring_consume/src/lib.rs
```

Live output:

```
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
```

The one comparison this section asked for.
`commit_available_takes_everything_and_reports_where_it_reached`'s idempotence
assertion — a second `commit_available()` on an already-drained run returns the
same sequence — still passes with the guard in place, run via `cargo test
--release -p ring_consume` against an isolated `CARGO_TARGET_DIR` (21/21
integration tests, 17/17 doctests, 0 failed).

**Disposition:** applied — `commit_available` now stores only when the run it
computed is non-empty, so an idle poll performs zero atomic stores instead of
one redundant `Release` store of the value already there. Now prints: `if end != run.start()`

---

## What Is Correctly Absent

| Not present | Correctly so |
|-------------|--------------|
| a `commit_available` that returns `Result` | it cannot fail; the `Result` would be noise at every call site |
| a guard against committing while a read borrow is outstanding | the crate hands out no borrows — `Available` is `Copy` and owns nothing |
| a `debug_assert!` on monotonicity | `commit`'s guard already refuses backwards commits, and it is a real check rather than a debug one |

---

## Cross-References

| Definition | Instance | Relationship |
|------------|----------|--------------|
| pitfall | [002](002_the_empty_barrier_is_the_case_nobody_measured.md) | the other place the empty run behaves differently than it reads |
| api | [002](../api/002_the_two_commits.md) | the contract the two share |
| non_functional_requirement | [001](../non_functional_requirement/001_what_the_read_path_costs.md) | the allocation CN44's poll also pays |
| algorithm | [002](../algorithm/002_the_two_sided_guard.md) | the guard `commit_available` does not run |
| decisions | [002](../decisions/002_plain_stores_rather_than_compare_exchange.md) | why the store is a store and not a compare-exchange |

### Sources

| What | Where |
|------|-------|
| `commit` and its guard | `ring_consume/src/lib.rs`, `pub fn commit( &self, through : Seq )` |
| `commit_available` | `ring_consume/src/lib.rs`, `pub fn commit_available( &self )` |
| The two store sites | `ring_consume/src/lib.rs`, the two `self.cursor.store(` outside doctests |
| The idempotence test | `ring_consume/tests/consume_test.rs`, `commit_available_takes_everything_and_reports_where_it_reached` |

### Tests

| Claim | Verified by |
|-------|-------------|
| `commit_available` does not call `commit` | reading both bodies; two distinct `store` sites, no `self.commit(` anywhere |
| The two are equivalent today | `commit`'s guard cannot reject `run.end()` |
| The store was unconditional | the 1000/1000 measurement with 999 empty runs, taken before the guard |
| A second call is a no-op that no longer stores | `commit_available_takes_everything_and_reports_where_it_reached` asserts the return; the guard skips the store on an empty run |
