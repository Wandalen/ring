# Item: The Seven of the Claimer

### Scope

- **Purpose**: Take `Claimer`'s seven public items one at a time, sort them by what they cost at the hardware level, and record where each one is actually called from.
- **Responsibility**: Tabulate the seven, show the three-way split by atomic cost, and record that the accessor with the most specific documentation is used for none of the two things that documentation names.
- **In Scope**: `Claimer::new`, `cursor`, `consumers`, `claimed`, `headroom`, `claim`, `claim_up_to`.
- **Out of Scope**: `Claim`'s eight — see [`item/001`](001_the_eight_readings_of_a_range.md). The loop inside the last two is [`algorithm/001`](../algorithm/001_the_gate_inside_the_retry.md).

### The Seven

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A205 -F 'impl< '"'"'a > Claimer< '"'"'a >' ring_claim/src/lib.rs
```

Live output:

```
impl< 'a > Claimer< 'a >
{
  /// A claimer starting at sequence zero, gated by `consumers`.
  ///
  /// ```
  /// use ring_claim::Claimer;
  /// use ring_gating::GatingSet;
  /// use ring_types::{ Capacity, Seq };
  ///
  /// let consumers = GatingSet::new( Capacity::new( 8 ).unwrap(), 1 );
  /// assert_eq!( Claimer::new( &consumers ).claimed(), Seq::ZERO );
  /// ```
  #[ must_use ]
  pub fn new( consumers : &'a GatingSet ) -> Self
  {
    Self { cursor : PaddedCursor::default(), consumers }
  }

  /// The producer cursor, for `ring_publish` to read and for a gating set on
  /// the other side of the ring to be built against.
  ///
  /// # This is a convention, not encapsulation
  ///
  /// `PaddedCursor` implements the public `SeqCell` trait, and every one of
  /// its methods takes `&self` — so a `&PaddedCursor` is enough to `store` the
  /// cursor backwards or `fetch_add` it past the gate. The second is exactly
  /// the design this crate's module documentation rejects, reachable from
  /// outside in one line of safe code, and
  /// `writing_through_the_cursor_accessor_defeats_the_gate` in
  /// `tests/claim_test.rs` demonstrates both failure modes it produces:
  /// a grant past a gate that had just returned `Full`, and two live `Claim`s
  /// that `Claim::overlaps` reports as covering the same sequences.
  ///
  /// Monotonicity is therefore a property of the *methods* — `claim`,
  /// `claim_up_to`, `claimed` — and not of the cursor itself. Read it, hand it
  /// to `ring_publish`, take its address for a layout assertion; do not write
  /// through it. The crate's manual `§ C2` check greps this crate's own source
  /// for `store` and `fetch_add` and passes, which is correct and says nothing
  /// about callers.
  ///
  /// ```
  /// use ring_claim::Claimer;
  /// use ring_cursor::SeqCell;
  /// use ring_gating::GatingSet;
  /// use ring_types::{ Capacity, Seq };
  /// use core::sync::atomic::Ordering;
  ///
  /// let consumers = GatingSet::new( Capacity::new( 8 ).unwrap(), 1 );
  /// let claimer = Claimer::new( &consumers );
  /// assert_eq!( claimer.cursor().load( Ordering::Acquire ), Seq::ZERO );
  /// ```
  #[ must_use ]
  pub const fn cursor( &self ) -> &PaddedCursor
  {
    &self.cursor
  }

  /// The gating set this claimer respects.
  ///
  /// ```
  /// use ring_claim::Claimer;
  /// use ring_gating::GatingSet;
  /// use ring_types::Capacity;
  ///
  /// let consumers = GatingSet::new( Capacity::new( 8 ).unwrap(), 2 );
  /// assert_eq!( Claimer::new( &consumers ).consumers().len(), 2 );
  /// ```
  #[ must_use ]
  pub const fn consumers( &self ) -> &'a GatingSet
  {
    self.consumers
  }

  /// How far claiming has advanced — one past the last sequence handed out.
  ///
  /// Note this is *claimed*, not published: a slot counted here may still be
  /// mid-write. `ring_publish` tracks the published frontier separately, and
  /// conflating the two is exactly what
  /// `docs/feature/170_claim_publish_available_commit_handshake.md` forbids.
  ///
  /// ```
  /// use ring_claim::Claimer;
  /// use ring_gating::GatingSet;
  /// use ring_types::{ Capacity, Seq };
  ///
  /// let consumers = GatingSet::new( Capacity::new( 8 ).unwrap(), 1 );
  /// let claimer = Claimer::new( &consumers );
  /// let _claim = claimer.claim( 2 ).unwrap();
  /// assert_eq!( claimer.claimed(), Seq( 2 ) );
  /// ```
  #[ must_use ]
  pub fn claimed( &self ) -> Seq
  {
    self.cursor.load( GATING )
  }

  /// How many slots could be claimed right now.
  ///
  /// A hint only. Between this reading and a [`claim`] another producer may
  /// take the space — which is why `claim` re-checks rather than trusting a
  /// prior `headroom`.
  ///
  /// Costs one load plus one more per registered consumer — cheap for a
  /// diagnostic, worth avoiding in a hot loop with many consumers.
  ///
  /// [`claim`]: Self::claim
  ///
  /// ```
  /// use ring_claim::Claimer;
  /// use ring_gating::GatingSet;
  ///
  /// let consumers = GatingSet::new( ring_types::Capacity::new( 4 ).unwrap(), 1 );
  /// let claimer = Claimer::new( &consumers );
  /// assert_eq!( claimer.headroom(), 4 );
  ///
  /// let _claim = claimer.claim( 3 ).unwrap();
  /// assert_eq!( claimer.headroom(), 1 );
  /// ```
  #[ must_use ]
  pub fn headroom( &self ) -> usize
  {
    self.consumers.headroom( self.claimed() )
  }

  /// Claim exactly `count` contiguous sequences, or fail.
  ///
  /// Never waits and never claims fewer than asked — see
  /// [`claim_up_to`] for the partial variant.
  ///
  /// [`claim_up_to`]: Self::claim_up_to
  ///
  /// # Errors
  ///
  /// [`RingError::BatchTooLarge`] when `count` exceeds the ring's capacity: a
  /// configuration error no consumer's progress can fix, so a retry loop must
  /// stop. [`RingError::Full`] when the space is not available *right now* —
  /// back-pressure, so a retry loop should keep going.
  ///
  /// A `count` of zero always succeeds, even on a full ring — there is
  /// nothing for back-pressure to block. [`claim_up_to`] treats a zero grant
  /// as `Full` instead; the two functions disagree here on purpose.
  ///
  /// ```
  /// use core::sync::atomic::Ordering;
  /// use ring_claim::Claimer;
  /// use ring_cursor::SeqCell;
  /// use ring_gating::GatingSet;
  /// use ring_types::{ Capacity, RingError, Seq };
  ///
  /// let consumers = GatingSet::new( Capacity::new( 4 ).unwrap(), 1 );
  /// let claimer = Claimer::new( &consumers );
  ///
  /// assert_eq!( claimer.claim( 4 ).unwrap().start(), Seq::ZERO );
  /// assert_eq!( claimer.claim( 1 ), Err( RingError::Full ) );
  ///
  /// consumers.cursor( 0 ).unwrap().store( Seq( 2 ), Ordering::Release );
  /// assert_eq!( claimer.claim( 2 ).unwrap().start(), Seq( 4 ) );
  /// ```
  pub fn claim( &self, count : usize ) -> Result< Claim, RingError >
  {
    if count > self.consumers.capacity().get()
    {
      return Err( RingError::BatchTooLarge
      {
        requested : count,
        capacity : self.consumers.capacity().get(),
      } );
    }

    // The gate is the loop condition, and is therefore re-read on every
    // iteration: on a failed exchange another producer moved the cursor, so
    // the headroom computed against the old value is stale and granting on it
    // would overlap that producer's range.
    let mut current = self.claimed();
    while count <= self.consumers.headroom( current )
    {
      let next = current.advanced_by( count as u64 );
      match self.cursor.compare_exchange( current, next, CLAIM_SUCCESS, GATING )
      {
        Ok( _ ) => return Ok( Claim::new( current, count ) ),
        Err( actual ) => current = actual,
      }
    }

    Err( RingError::Full )
  }

  /// Claim as many of `max` sequences as are available, down to one.
  ///
  /// For a batching producer that would rather write four items now than wait
  /// for room for eight.
  ///
  /// # Errors
  ///
  /// [`RingError::Full`] when not even one slot is free. Never
  /// `BatchTooLarge` — a `max` wider than the ring is not an error here, it is
  /// simply more than will be granted.
  ///
  /// A `max` of zero is also `Full`, since there is no partial success at
  /// zero to report — this differs from [`claim`], which treats a `count`
  /// of zero as always satisfiable.
  ///
  /// [`claim`]: Self::claim
  ///
  /// ```
  /// use core::sync::atomic::Ordering;
```

| Item | Line | `const` | `must_use` | Atomic cost | Can fail |
|------|-----:|:-------:|:----------:|-------------|:--------:|
| `new( consumers )` | 269 | ✘ | ✔ | **none** | no |
| `cursor()` | 289 | ✔ | ✔ | **none** — hands out the cell | no |
| `consumers()` | 305 | ✔ | ✔ | **none** | no |
| `claimed()` | 328 | ✘ | ✔ | one `Acquire` load | no |
| `headroom()` | 353 | ✘ | ✔ | one load + one per consumer | no |
| `claim( count )` | 388 | ✘ | — (`Result`'s) | ≥1 `AcqRel` compare-exchange | **yes** |
| `claim_up_to( max )` | 442 | ✘ | — (`Result`'s) | ≥1 `AcqRel` compare-exchange | **yes** |

Every one takes `&self` — including the two that mutate, which is the whole
point of the type: claiming is a shared-reference operation so several producers
can hold `&Claimer` at once. `Claim`'s eight take `self` by value without
exception ([`item/001`](001_the_eight_readings_of_a_range.md)); these seven take
`&self` without exception. Neither type has a `&mut self` method anywhere.

### CL29 — Three Free, Two Reading, Two Writing

The seven sort cleanly into three tiers, and the tiers are not documented
anywhere in the source — they have to be read off the bodies:

| Tier | Items | What it costs | Safe to call in a loop |
|------|-------|---------------|:----------------------:|
| **Free** | `new`, `cursor`, `consumers` | no atomic instruction at all | ✔ |
| **Reading** | `claimed`, `headroom` | one `Acquire` load; `headroom` adds one per registered consumer | ✔ — but the answer is stale on return |
| **Writing** | `claim`, `claim_up_to` | a compare-exchange per attempt, unbounded attempts | ✔ — that is their contract |

The middle tier is the one that misleads, and `headroom`'s own doc says so
(`:335-337`):

> A hint only. Between this reading and a [`claim`] another producer may take
> the space — which is why `claim` re-checks rather than trusting a prior
> `headroom`.

That sentence is the crate's founding decision written as an API note. A caller
who reads `headroom()` and then calls `claim()` with the number it returned has
reconstructed, at the call site, exactly the check-then-act shape that
[`decisions/001`](../decisions/001_compare_exchange_rather_than_fetch_add.md)
rejects inside the loop — except now it is unfixable, because the crate cannot
see it.

The saving grace is that being wrong about it is harmless here: the worst
outcome is `Err( Full )`, because `claim` re-checks. The rejected design's
failure was silent; this one is a returned error. That asymmetry is why
`headroom` can be public at all.

`headroom`'s cost also scales, which nothing states: it forwards to
`GatingSet::headroom`, which calls `slowest()` and reads **every** registered
consumer cursor. On a one-consumer ring that is two loads; on an eight-consumer
ring it is nine. Fine for a diagnostic, worth knowing before putting it in a hot
loop.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -A1 -F 'Costs one load plus one more per registered consumer' ring_claim/src/lib.rs
```

Live output:

```
    /// Costs one load plus one more per registered consumer — cheap for a
    /// diagnostic, worth avoiding in a hot loop with many consumers.
```

**Disposition:** applied — `headroom`'s doc comment now states its
per-consumer cost scaling directly, so the tier table's "Fine for a
diagnostic, worth knowing before putting it in a hot loop" observation is
no longer something a reader has to derive from the source body. Now
prints: `worth avoiding in a hot loop with many consumers`

### CL30 — `cursor()` Names Two Purposes and Is Used for a Third

`cursor()` has the most specific doc comment of the seven (`:274-275`):

> The producer cursor, for `ring_publish` to read and for a gating set on the
> other side of the ring to be built against.

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
command grep -r 'claimer\.cursor()' */src/*.rs */tests/*.rs
grep 'cursor' ring_publish/src/lib.rs | head
```

Live output:

```
ring_claim/src/lib.rs:  /// assert_eq!( claimer.cursor().load( Ordering::Acquire ), Seq::ZERO );
ring_mpsc/src/lib.rs:    let claim = self.claimer.cursor().addr();
ring_claim/tests/claim_test.rs:  assert_eq!( claimer.cursor().load( Ordering::Acquire ), Seq::ZERO );
ring_claim/tests/claim_test.rs:  assert_eq!( claimer.cursor().load( Ordering::Acquire ), Seq( 3 ) );
ring_claim/tests/claim_test.rs:  let stolen = claimer.cursor().fetch_add( 4, Ordering::AcqRel );
ring_claim/tests/claim_test.rs:  claimer.cursor().store( Seq::ZERO, Ordering::Release );
//! Depends on `ring_types`, `ring_cursor`.
//! implementation existed and is not among them. Publishing is a cursor
//! ## The published cursor is not the claimed cursor
//! `ring_claim` advances a cursor when a producer *takes* a range;
//! Conflating the two cursors is not a subtle bug. It publishes uninitialised
//! [`Publisher::try_publish`] advances only when the published cursor is
use ring_cursor::{ PaddedCursor, SeqCell, GATING };
/// `Release`, paired with the consumer's `Acquire` read of the same cursor:
/// Separate from `ring_claim::Claimer`'s cursor on purpose — see the module
  cursor : PaddedCursor,
```

Both named purposes turn out not to happen, and the first one is a thing the
named crate's own documentation forbids:

| Named purpose | Reality |
|---------------|---------|
| "for `ring_publish` to read" | `ring_publish` owns its **own** `PaddedCursor` (`:87`) with its own `cursor()` (`:117`) and never takes this one |
| "for a gating set … to be built against" | in `ring_mpsc` the `GatingSet` is built first and the `Claimer` is constructed *from* it (`:540`) — the dependency runs the other way |

The first row is the sharper one. `ring_publish`'s module documentation opens
with a section titled **"The published cursor is not the claimed cursor"**
(`:18-25`):

> `ring_claim` advances a cursor when a producer *takes* a range; […]
> Conflating the two cursors is not a subtle bug. It publishes uninitialised
> [data]

So `Claimer::cursor()` invites the reader toward precisely the mistake the
sibling crate exists to prevent, in a sentence naming that sibling crate. Not a
defect in behaviour — the accessor returns a cursor and the caller decides what
to do with it — but a defect in the doc, and a well-aimed one: it points a reader
at the one integration that must never be built.

What the accessor is *actually* for shows up in its single external caller,
`ring_mpsc::Producer::on_distinct_lines` (`:806`):

```rust
let claim = self.claimer.cursor().addr();
let consume = self.ring.consumer_cursor().addr();

claim.abs_diff( consume ) >= 64
```

It never loads a sequence. It takes `addr()` — the cursor's *address* — to check
that the producer and consumer cursors sit on different cache lines. The
accessor's real job is to expose the cell's identity, not its value, and that is
a third purpose neither half of the doc mentions.

The correction is one sentence: `cursor()` exposes the cell so its address and
alignment can be inspected, and `claimed()` is how the value should be read.
That is also the safer framing, because `claimed()` pins the ordering to
`GATING` (`Acquire`) while `cursor()` hands out a cell any caller may `load`
with `Relaxed`.

### The Two That Can Fail

`claim` and `claim_up_to` are the only items in either type that return a
`Result`, and they are the only two without an explicit `#[ must_use ]` — for
the same reason as `Claim::new` and `Claim::sequences`, that `Result` is already
`#[ must_use ]` in `core` and the `Claim` inside it carries the messaged one.

Their `# Errors` sections split by what a caller should do, which is the more
useful axis than what went wrong (`:365-370`):

| Error | Meaning | A retry loop should |
|-------|---------|---------------------|
| `BatchTooLarge` | `count` exceeds capacity — a configuration error | **stop** |
| `Full` | the space is not free *right now* — back-pressure | **keep going** |

`claim_up_to` can only ever return the second, since it asks for at most what is
there — which is why the crate has a test named
`claim_up_to_never_reports_batch_too_large`, and why the two functions disagree
at zero ([`algorithm/002`](../algorithm/002_two_loops_that_disagree_at_zero.md)).

### Where the Seven Are Called From

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
command grep -r 'claimer\.\(claim\|claim_up_to\|claimed\|headroom\|cursor\|consumers\)(' \
  */src/*.rs | grep -v '^ring_claim/'
```

Live output:

```
ring_mpsc/src/lib.rs:        let claim = self.claimer.claim(1)?;
ring_mpsc/src/lib.rs:        self.claimer.headroom()
ring_mpsc/src/lib.rs:        self.claimer.claimed()
ring_mpsc/src/lib.rs:        let claim = self.claimer.cursor().addr();
```

| Item | External callers | Where |
|------|-----------------:|-------|
| `claim` | 1 | `ring_mpsc:773` — inside `Producer::claim` |
| `headroom` | 1 | `ring_mpsc:805` — as `Producer::free_capacity` |
| `claimed` | 1 | `ring_mpsc:832` — as `Producer::claimed` |
| `cursor` | 1 | `ring_mpsc:862` — as `Producer::on_distinct_lines`, via `addr()` |
| `consumers` | 0 | — |
| `claim_up_to` | **0** | — |
| `new` | 1 | `ring_mpsc:577` — building `Ends` |

One consumer crate, five of seven items used, each exactly once, each wrapped in
a method with a different name. `ring_mpsc::Producer` is a renaming layer over
this type, and the renames are improvements —
`free_capacity` says what `headroom` means to a caller who has never read this
crate.

`claim_up_to` having no caller is worth flagging: it is the more forgiving of
the two entry points, it is the one omitted from the module header's list of the
tests carrying the crate
([`invariant/001`](../invariant/001_no_two_producers_hold_one_sequence.md)), and
nothing above this crate has adopted it. Its only appearance anywhere outside
`ring_claim` is a **comment**:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r 'claim_up_to' */src/*.rs */tests/*.rs | grep -v '^ring_claim/'
# → ring_publish/tests/publish_test.rs:115:
#     // `claim_up_to` can legitimately grant a shorter range than asked for.
```

Live output:

```
ring_publish/tests/publish_test.rs:    // `claim_up_to` can legitimately grant a shorter range than asked for.
```

A sibling crate's test explains what it does, in prose, without calling it. That
is the whole external footprint of the function.

### Items

| File | Relationship |
|------|--------------|
| [001_the_eight_readings_of_a_range.md](001_the_eight_readings_of_a_range.md) | The other type's eight, taking `self` by value throughout |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_the_gate_inside_the_retry.md](../algorithm/001_the_gate_inside_the_retry.md) | What the two writing items do |
| [../algorithm/002_two_loops_that_disagree_at_zero.md](../algorithm/002_two_loops_that_disagree_at_zero.md) | Where `claim` and `claim_up_to` part |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_four_predicates_and_the_one_that_is_called.md](../integration/002_four_predicates_and_the_one_that_is_called.md) | `headroom` is the one predicate this crate calls |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_borrow_that_is_half_the_type.md](../data_structure/002_the_borrow_that_is_half_the_type.md) | Why every item takes `&self` |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_compare_exchange_rather_than_fetch_add.md](../decisions/001_compare_exchange_rather_than_fetch_add.md) | The shape `headroom` lets a caller rebuild outside the crate |

### Sources

| File | Relationship |
|------|--------------|
| `ring_claim/src/lib.rs:263-500` | The seven, in full |
| `ring_claim/src/lib.rs:281-283` | `cursor()`'s two named purposes |
| `ring_publish/src/lib.rs:18-25,87,117` | The crate that owns its own cursor, and says why |
| `ring_cursor/src/lib.rs:186-189` | `addr()`, what `cursor()` is actually used for |
| `ring_mpsc/src/lib.rs:577, 773, 805, 832, 862` | Every external call, all five |

### Tests

| File | Relationship |
|------|--------------|
| `tests/claim_test.rs:274` — `headroom_tracks_what_claiming_consumed` | The reading tier, against a shrinking gate |
| `tests/claim_test.rs:288` — `the_claimer_exposes_the_gate_it_was_built_over` | `consumers()`, which nothing outside the crate calls |
| `tests/claim_test.rs:298` — `the_producer_cursor_is_readable_and_starts_at_zero` | `cursor()` read as a value |
| `tests/claim_test.rs:309` — `the_producer_cursor_occupies_its_own_cache_line` | `cursor()` read as a layout fact |
| `tests/claim_test.rs:249` — `claim_up_to_never_reports_batch_too_large` | The variant that cannot return the first error |
