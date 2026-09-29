# Item: The Four Accessors and the Limit

### Scope

- **Purpose**: Catalogue the eight items that are not gating readings — the constructor, the four accessors over the `Vec`, the capacity, and the two position-valued methods.
- **Responsibility**: Give each item's signature and contract, and record the one input `new` accepts that nothing tests.
- **In Scope**: `new`, `len`, `is_empty`, `cursor`, `cursors`, `capacity`, `slowest`, `limit`.
- **Out of Scope**: `headroom`, `admits`, `check` — see [`001`](001_the_three_gating_readings.md).

### `GatingSet::new` — the only constructor

```rust
#[ must_use ]
pub fn new( capacity : Capacity, consumers : usize ) -> Self     // :93
{
  let mut cursors = Vec::with_capacity( consumers );
  cursors.resize_with( consumers, PaddedCursor::default );
  Self { cursors, capacity }
}
```

> `consumers` of zero is legal and means ungated — see the module documentation
> for why that is not the same as one consumer at zero.

| Input | Accepted | Meaning |
|-------|:--------:|---------|
| `consumers == 0` | ✅ | Ungated. Documented, tested, and the subject of [`decisions/001`](../decisions/001_capacity_for_an_empty_set.md) |
| `consumers == 1` | ✅ | The only value any production code passes |
| `consumers > 1` | ✅ | The multi-consumer path — [`data_structure/001`](../data_structure/001_the_set_that_cannot_grow.md) |
| `consumers > capacity` | ✅ | **G11** — see below |

`Capacity` does the only validation there is, and it does it before `new` is
reached: it is `ring_types`' newtype refusing zero and non-powers-of-two, which
is why `new` cannot fail and returns `Self` rather than a `Result`.

#### G11 — More Consumers Than Slots

Nothing rejects `GatingSet::new( cap( 4 ), 8 )`, and nothing tests it. The
highest ratio anywhere in the workspace is exactly 1:1:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -rh 'GatingSet::new( cap(' ring_*/tests/*.rs
# the largest consumer count against any capacity is cap( 4 ), 4
```

Live output:

```
  let set = GatingSet::new( cap( 4 ), 1 );
  let empty = GatingSet::new( cap( 8 ), 0 );
  let set = GatingSet::new( cap( 8 ), 2 );
  let gated = GatingSet::new( cap( 1024 ), 1 );
  let tight = GatingSet::new( cap( 4 ), 1 );
  let ungated = GatingSet::new( cap( 64 ), 0 );
  let consumers = GatingSet::new( cap( 64 ), 1 );
  let consumers = GatingSet::new( cap( CAPACITY ), 1 );
  let consumers = GatingSet::new( cap( 4 ), 1 );
  let consumers = GatingSet::new( cap( 4 ), 1 );
  let consumers = GatingSet::new( cap( 4 ), 1 );
  let consumers = GatingSet::new( cap( 4 ), 1 );
  let consumers = GatingSet::new( cap( 4 ), 1 );
  let consumers = GatingSet::new( cap( 8 ), 1 );
  let consumers = GatingSet::new( cap( 4 ), 1 );
  let consumers = GatingSet::new( cap( 4 ), 1 );
  let consumers = GatingSet::new( cap( 8 ), 1 );
  let consumers = GatingSet::new( cap( 8 ), 2 );
  let consumers = GatingSet::new( cap( 8 ), 1 );
  let consumers = GatingSet::new( cap( 16_384 ), 0 );
  let consumers = GatingSet::new( cap( 16_384 ), 0 );
  let consumers = GatingSet::new( cap( 16_384 ), 0 );
  let consumers = GatingSet::new( cap( 8_192 ), 0 );
  let consumers = GatingSet::new( cap( CAPACITY ), 1 );
  let consumers = GatingSet::new( cap( 4 ), 1 );
  let consumers = GatingSet::new( cap( 8 ), 1 );
  let set = GatingSet::new( cap( capacity ), positions.len() );
  let ungated = GatingSet::new( cap( 4 ), 0 );
  let ungated = GatingSet::new( cap( 4 ), 0 );
  let set = GatingSet::new( cap( 8 ), 1 );
    let set = GatingSet::new( cap( 4 ), consumers );
  let set = GatingSet::new( cap( 4 ), 3 );
  let set = GatingSet::new( cap( 4 ), 4 );
  assert_eq!( GatingSet::new( cap( 32 ), 1 ).capacity().get(), 32 );
  let set = GatingSet::new( cap( CAPACITY ), 1 );
    let consumers = GatingSet::new( cap( 8 ), 1 );
    let consumers = GatingSet::new( cap( 8 ), 1 );
    let consumers = GatingSet::new( cap( 4 ), 1 );
```

Accepting it is correct. Capacity bounds *slots in flight*, not readers — eight
consumers each reading the same 4-slot stream is a legitimate fan-out, and the
gate's answer is well-defined: all eight cursors start at zero, `slowest` is
`Some( Seq::ZERO )`, and `headroom` behaves exactly as it would with one.

What is missing is the assertion that it *is* well-defined. The cost of adding it
is one line inside the existing sweep at `tests/gating_test.rs:315-327`, which
already loops `for consumers in 0..5` against `cap( 4 )` — raising the bound to
`0..8` would cover it. Recorded rather than changed: a test edit belongs to a run
with its own verification.

### The Four Accessors

| Item | Signature | Body | Contract |
|------|-----------|------|----------|
| `len` | `( &self ) -> usize` | `self.cursors.len()` | The consumer count, fixed for the value's life |
| `is_empty` | `( &self ) -> bool` | `self.cursors.is_empty()` | `len() == 0`; means *ungated*, not *idle* |
| `cursor` | `( &self, index ) -> Option< &PaddedCursor >` | `self.cursors.get( index )` | `None` past the end; the borrow permits stores |
| `cursors` | `( &self ) -> &[ PaddedCursor ]` | `&self.cursors` | The whole slice, for a `Barrier` to borrow |

All four are `#[ must_use ]`, none is `const`, and the reasons differ — `len` and
`is_empty` could be, `cursor` and `cursors` cannot
([`api/001`](../api/001_eleven_methods_over_one_owned_vec.md) § G5).

`is_empty`'s contract is the one worth stating carefully. It reports that the set
has *no consumers*, which is a permanent structural fact. It says nothing about
whether the ring has data. A reader who takes it for "nothing to read" has the
[`pitfall/001`](../pitfall/001_unwrapping_the_empty_set_to_zero.md) mistake.

`cursors` is the item the family composes through: `Barrier::over( set.cursors() )`
is how a chained consumer waits on the same cursors a producer gates on, and it
is the only reason the accessor returns a slice rather than an iterator.

### `GatingSet::capacity`

```rust
#[ must_use ]
pub const fn capacity( &self ) -> Capacity     // :168
```

The crate's only `const fn`. Returns the `Capacity` newtype, not a `usize` — so a
caller that wants the number writes `.capacity().get()`, and cannot accidentally
pass a raw slot count where a validated capacity is required.

`ring_claim` calls it twice, both times to build the `BatchTooLarge` error it
constructs instead of calling `check`
([`decisions/002`](../decisions/002_a_result_rather_than_a_bool.md)).

### The Two Positions

```rust
#[ must_use ]
pub fn slowest( &self ) -> Option< Seq >     // :197
{
  ring_cursor::slowest( &self.cursors )
}

#[ must_use ]
pub fn limit( &self ) -> Option< Seq >       // :321
{
  self.slowest().map( | s | s.advanced_by( self.capacity.get() as u64 ) )
}
```

| | `slowest` | `limit` |
|---|---|---|
| Answers | Where the rearmost consumer is | Where the producer will block |
| Empty set | `None` | `None` |
| Relationship | — | `slowest + capacity` |
| Overflow | Impossible | `advanced_by` is plain addition — see [`type/001`](../type/001_a_usize_headroom_and_a_u64_limit.md) |

`slowest`'s doc is the longest in the crate, and none of it is about `slowest`.
It is about the ordering the fold uses, which lives in `ring_cursor`:

> Every cursor is read there at `ring_cursor::GATING`, which is the family's one
> statement of the ordering a gating read uses, for the reason argued there: the
> caller is about to overwrite a slot on the answer, and a `Relaxed` load would
> let it act on a barrier the consumer has already moved past — or, worse, one it
> has not yet reached.

That is the right place for it. The crate that *names* no ordering
([`invariant/002`](../invariant/002_this_crate_names_no_ordering.md)) still has
to tell its readers which one they are getting, and pointing at the one
definition is how it does that without acquiring a second copy.

`limit`'s doctest shows the arithmetic directly:

```rust
let set = GatingSet::new( Capacity::new( 8 ).unwrap(), 1 );
assert_eq!( set.limit(), Some( Seq( 8 ) ) );

set.cursor( 0 ).unwrap().store( Seq( 5 ), Ordering::Release );
assert_eq!( set.limit(), Some( Seq( 13 ) ) );
```

`0 + 8`, then `5 + 8`. Two assertions, and the second is what distinguishes
`limit` from a constant.

### GT29 — More Consumers Than Slots Is Legal and Untested

```
93:  pub fn new( capacity : Capacity, consumers : usize ) -> Self
      ...no relation asserted between the two arguments
set_at( cap, positions ) calls in the suite : 16
      with len( positions ) >= cap          :  0
```

A set with more consumers than the ring has slots gates harder than one with
fewer, which is arithmetically fine. Whether it is intended is undocumented, and
no test visits the region.

**Finding.** It accepts any `consumers` count including one larger than the ring's capacity, which is constructible, harmless, and untested — all sixteen `set_at` calls in the suite build fewer consumers than the capacity, none at or above it

---

### GT30 — The Accessors Do Not Restrict by Index

```
142:  pub fn cursor( &self, index : usize ) -> Option< &PaddedCursor >
155:  pub fn cursors( &self ) -> &[ PaddedCursor ]
      both &self; neither takes any token identifying the caller
```

The set stores one cursor per consumer and therefore knows which index belongs
to whom. Nothing in either accessor uses that.

**Finding.** One hands out a single `&PaddedCursor`, the other the whole slice, and both go through `&self`. Neither signature distinguishes a consumer advancing its own position from a producer advancing someone else's: the set knows which index belongs to which consumer and never uses that knowledge to restrict access

**Disposition:** declined — closing this requires an API-surface redesign
(an index-scoped capability or token distinguishing "the consumer this index
belongs to" from any other holder of `&GatingSet`), not a test or doc fix
inside this crate. It would change `cursor`/`cursors`' signatures and ripple
into every caller across the family that holds a `&GatingSet` today
(`ring_barrier`, `ring_mpsc`, `ring_batch`, and the crate's own tests) —
out of scope for a documentation disposition pass. Unlike GT26 and this
file's own G11/GT29, no "(to create)" test is named anywhere in this
instance, because there is no cheap assertion that would close the gap
without the redesign.

---


### APIs

| File | Relationship |
|------|--------------|
| [../api/001_eleven_methods_over_one_owned_vec.md](../api/001_eleven_methods_over_one_owned_vec.md) | The `const fn` and `#[ must_use ]` census |
| [../api/002_the_reading_that_returns_a_position.md](../api/002_the_reading_that_returns_a_position.md) | Why `limit` propagates its `Option` |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_set_that_cannot_grow.md](../data_structure/001_the_set_that_cannot_grow.md) | What `new`'s second argument fixes forever |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_this_crate_names_no_ordering.md](../invariant/002_this_crate_names_no_ordering.md) | Why `slowest`'s doc talks about a crate this file does not import |

### Items

| File | Relationship |
|------|--------------|
| [001_the_three_gating_readings.md](001_the_three_gating_readings.md) | The three readings that consume these eight |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_unwrapping_the_empty_set_to_zero.md](../pitfall/001_unwrapping_the_empty_set_to_zero.md) | Misreading `is_empty` and `slowest() == None` |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_a_usize_headroom_and_a_u64_limit.md](../type/001_a_usize_headroom_and_a_u64_limit.md) | `limit`'s width, and its one arithmetic hazard |

### Sources

| File | Relationship |
|------|--------------|
| `ring_gating/src/lib.rs:78-98` | `new`, its doc and the two-line body |
| `ring_gating/src/lib.rs:100-171` | The four accessors and `capacity` |
| `ring_gating/src/lib.rs:173-200` | `slowest`, and the ordering note |
| `ring_gating/src/lib.rs:296-324` | `limit` |

### Tests

| File | Relationship |
|------|--------------|
| `tests/gating_test.rs:315-327` | The four accessors, swept over 0–4 consumers |
| `tests/gating_test.rs:329-339` | Every cursor starts at zero |
| `tests/gating_test.rs:341-353` | `cursors()` returns cache-line-separated elements |
| `tests/gating_test.rs:355-359` | `capacity` round-trips |
| `tests/gating_test.rs:291-311` | `limit`, and its correspondence with `headroom` |
