# Type: `Available`'s Const Surface

### Scope

**Purpose:** Record which of `Available`'s items are `const fn`, establish that
the boundary is exactly where it must be, and record what `Seq`'s own type
choice costs the crate that builds on it.

**Responsibility:** `const` on `Available` and on the crate as a whole; `Seq`'s
representation as it reaches this crate.

**In Scope:** The six items of `Available`; the eight of `Consumer`; `Seq`'s
derives, field visibility, and niche.

**Out of Scope:** What each item *does* — that is
[`item/001`](../item/001_the_six_of_a_run.md). The `ring_seqno` functions this
crate calls but does not own, which are
[`workaround/001`](../workaround/001_five_functions_none_const.md).

---

## The Boundary

```sh
cd "$(git rev-parse --show-toplevel)"
grep -E '^\s*pub (const )?fn' ring_consume/src/lib.rs
```

Live output:

```
  pub const fn new( start : Seq, len : u64 ) -> Self
  pub const fn start( self ) -> Seq
  pub const fn end( self ) -> Seq
  pub const fn len( self ) -> u64
  pub const fn is_empty( self ) -> bool
  pub fn sequences( self ) -> impl Iterator< Item = Seq >
  pub const fn new( cursor : &'a PaddedCursor, barrier : Barrier< 'a > ) -> Self
  pub const fn cursor( &self ) -> &'a PaddedCursor
  pub const fn barrier( &self ) -> Barrier< 'a >
  pub fn position( &self ) -> Seq
  pub fn available( &self ) -> Available
  pub fn available_up_to( &self, max : u64 ) -> Available
  pub fn commit( &self, through : Seq ) -> Result< Seq, RingError >
  pub fn commit_available( &self ) -> Seq
```

Live output, all fourteen public functions:

```
101:  pub const fn new( start : Seq, len : u64 ) -> Self
114:  pub const fn start( self ) -> Seq
127:  pub const fn end( self ) -> Seq
140:  pub const fn len( self ) -> u64
154:  pub const fn is_empty( self ) -> bool
168:  pub fn sequences( self ) -> impl Iterator< Item = Seq >
223:  pub const fn new( cursor : &'a PaddedCursor, barrier : Barrier< 'a > ) -> Self
250:  pub const fn cursor( &self ) -> &'a PaddedCursor
268:  pub const fn barrier( &self ) -> Barrier< 'a >
287:  pub fn position( &self ) -> Seq
316:  pub fn available( &self ) -> Available
345:  pub fn available_up_to( &self, max : u64 ) -> Available
383:  pub fn commit( &self, through : Seq ) -> Result< Seq, RingError >
412:  pub fn commit_available( &self ) -> Seq
```

Eight of fourteen. Five of `Available`'s six, three of `Consumer`'s eight.

### CN47 — Every Item That Could Be `const` Is, and the One Exception Is Provable

Two different reasons produce the six non-`const` items, and neither is an
oversight:

| Item | Why not `const` |
|------|-----------------|
| `Available::sequences` | returns an iterator; `Iterator::map` is not a `const` method |
| `Consumer::position` | atomic load |
| `Consumer::available` | atomic loads |
| `Consumer::available_up_to` | atomic loads |
| `Consumer::commit` | atomic loads and a store |
| `Consumer::commit_available` | atomic loads and a store |

Five of the six are the atomic boundary, which is the same line as the `const`
boundary without exception ([`item/002`](../item/002_the_eight_of_a_consumer.md)).
The sixth is `sequences`, and it is the only one where "could this have been
`const`?" is a real question rather than a settled one.

It could not. Compiling its body verbatim with `const` added:

```
error[E0015]: cannot call non-const method
  `<std::ops::Range<u64> as Iterator>::map::<Seq, fn(u64) -> Seq {Seq}>`
  in constant functions
```

So the crate's `const` surface is maximal: every item that the language permits
to be `const` is `const`, and the two that are not are blocked by the atomic and
by the iterator respectively. That is a stronger statement than "mostly const"
and it is worth having on record, because the same claim is *false* one crate
down — [`workaround/001`](../workaround/001_five_functions_none_const.md) shows
`ring_seqno` leaving four const-able functions non-`const`.

**Cost:** none. Recorded because it is a property the crate has and does not
claim, and because the identical audit fails for its own dependency.

---

### CN48 — `Seq( pub u64 )` Forecloses the Niche, and the Crate Pays for It in the Return Type Nobody Sees

```sh
cd "$(git rev-parse --show-toplevel)"
grep -B1 'pub struct Seq' ring_types/src/id.rs
```

Live output:

```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Seq(pub u64);
```

```
24:#[ derive( Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default ) ]
25:pub struct Seq( pub u64 );
```

A newtype over a plain `u64` with a public field. Measured:

| Type | Size |
|------|-----:|
| `Seq` | 8 |
| `Option< Seq >` | **16** |
| `Option< NonZeroU64 >` | 8 |
| `Option< Option< Seq > >` | 16 |

`Option< Seq >` doubles because `Seq` has no invalid bit pattern to use as a
niche. `Option< NonZeroU64 >` is the control: same width, same `Option`, half the
size, because one value is reserved.

This lands on `ring_consume` at exactly one point, and it is on the hot path:

```rust
let readable = self
  .barrier
  .frontier()                                          // -> Option< Seq >, 16 bytes
  .map_or( 0, | frontier | ring_seqno::pending( frontier, position ) );
```

`Barrier::frontier` returns `Option< Seq >`, and `available()` calls it on every
poll ([`algorithm/001`](../algorithm/001_position_frontier_pending.md)). This
section used to dismiss the eight wasted bytes by pointing at the heap
allocation sitting in the same call: a much larger cost adjacent to it, so why
measure the smaller one. `b7e075ca` removed that allocation
([`non_functional_requirement/001`](../non_functional_requirement/001_what_the_read_path_costs.md)),
which takes the argument away rather than the cost — the eight bytes are now the
only overhead left on that return, so they are more visible than they were, not
less. They still do not reach a measurement: a sixteen-byte return is two
registers where one would do, on a call whose other work is `1 + n` atomic
loads.

What is worth recording is that the decision is not revisitable. `Seq`'s field is
`pub`, so:

- a niche would require making it private, which is a breaking change across the
  15 source files that use `.0`, and
- `Seq::ZERO` is a legitimate, heavily-used value, so `NonZeroU64` is not the
  available shape anyway — a niche would have to reserve `u64::MAX`, which the
  wrap discussion in
  [`lifecycle/002`](../lifecycle/002_the_consumer_over_a_rings_life.md) treats as
  unreachable and therefore reservable.

So there is a coherent design available — private field, `u64::MAX` reserved,
`Option< Seq >` at eight bytes — and the public field is what took it off the
table. That same public field is the crate's only escape hatch for iteration
([`workaround/001`](../workaround/001_five_functions_none_const.md) CN52), so the
two consequences trade against each other and neither is documented.

**Cost:** not reachable as a performance defect. Recorded because one decision in
a Tier 0 crate produces both an escape hatch this crate uses and a foreclosed
optimisation this crate pays for, and neither direction is written down.

---

## What `Available` Derives

```sh
cd "$(git rev-parse --show-toplevel)"
grep -B1 'pub struct Available' ring_consume/src/lib.rs
```

Live output:

```
#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
pub struct Available
```

```
84:#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
85:pub struct Available
```

Notably **not** `PartialOrd`/`Ord`, which `Seq` does derive. That is correct: two
runs are equal or they are not, and there is no total order on ranges that anyone
would want by default. `Seq`'s ordering is the one that matters and `Available`
never needs its own.

`Consumer`, by contrast, derives `Debug` and nothing else — including no `Copy`,
though every one of its fields is `Copy`. That withholding is deliberate and is
[`002`](002_the_lifetime_on_consumer.md) CN50.

---

## Cross-References

| Definition | Instance | Relationship |
|------------|----------|--------------|
| type | [002](002_the_lifetime_on_consumer.md) | the other type-level parameter |
| item | [001](../item/001_the_six_of_a_run.md) | what each of the six does |
| workaround | [001](../workaround/001_five_functions_none_const.md) | the same audit, failing, one crate down |
| algorithm | [001](../algorithm/001_position_frontier_pending.md) | where `Option< Seq >` reaches this crate |
| data_structure | [001](../data_structure/001_sixteen_and_twenty_four.md) | the sizes CN48 extends |

### Sources

| What | Where |
|------|-------|
| The fourteen signatures | `ring_consume/src/lib.rs` |
| `Seq`'s declaration | `ring_types/src/id.rs:24-25` |
| `frontier`'s `Option` return | `ring_barrier/src/lib.rs`, `fn frontier` |
| `.0` escapes across the family | 69 occurrences in 15 source files |

### Tests

| Claim | Verified by |
|-------|-------------|
| 8 of 14 are `const` | the grep above, counted |
| `sequences` cannot be `const` | the E0015 compile error quoted above |
| `Option< Seq >` is 16 bytes | the `size_of` probe, against `Option< NonZeroU64 >` at 8 |
| `Seq`'s field is public | `id.rs:25` |
| `Available` does not derive `Ord` | its derive list |
