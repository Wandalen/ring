# data_structure

Sixteen bytes and twenty-four, and neither number is interesting on its own. What
makes them worth recording is the comparison: a `Consumer` is 24 bytes against a
`Claimer`'s 128, and the entire difference is that the producer owns its cursor
and the consumer borrows one.

Both instances follow from that. The first measures the three sixteen-byte range
types the family has independently arrived at. The second takes the borrow apart
— why the cursor cannot be owned, what that buys, and what auto-derived `Send`
and `Sync` are quietly carrying on top of it.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Sixteen and Twenty-Four](001_sixteen_and_twenty_four.md) | CN22, CN23 — the third 16-byte range in the family, and a consumer a fifth the size of a claimer |
| 002 | [Two Borrows and No Owned State](002_two_borrows_and_no_owned_state.md) | CN24, CN25 — the borrow as the mechanism rather than an optimisation, and auto-derived `Send`/`Sync` nobody prices |

### Every Measurement

| Type | Size | Align |
|------|-----:|------:|
| `Seq` | 8 | 8 |
| `Option< Seq >` | 16 | 8 |
| `Available` | 16 | 8 |
| `Claim` | 16 | 8 |
| `Barrier< 'a >` | 16 | 8 |
| `Consumer< 'a >` | 24 | 8 |
| `PaddedCursor` | 64 | 64 |
| `Claimer< 'a >` | 128 | 64 |

`Consumer` is a reference (8) plus a `Barrier` (16). `Claimer` is a
cache-line-aligned `PaddedCursor` (64) plus a reference, rounded to alignment —
128. The 5× difference is one field's ownership.

### Why the Cursor Cannot Be Owned

The consumer's cursor is what the producer gates on. `ring_gating` reads it,
`Barrier::over` collects references to cursors of exactly this kind, and
`ring_publish/tests/handshake_test.rs` asserts with `ptr::eq` that the cursor the
`Consumer` holds *is* the one in the producer's gating set.

An owned cursor would be a different address, so the producer would gate on a
cursor nobody advances and the ring would fill and stop. The borrow is therefore
not a size optimisation — it is the wiring. The module documentation says so, in
the only place in the family where a borrow is argued for on mechanism grounds
rather than cost grounds.

### The Auto-Derived Half

`Consumer` is `Send` and `Sync` by auto-derivation — no `unsafe impl`, no
explicit bound, no mention anywhere in the crate. Both hold because
`&PaddedCursor` and `Barrier< 'a >` are, and both are relied on by the crate's
`thread::scope` tests, which would not compile without them.

Zero occurrences of `Send`, `Sync`, or `unsafe impl` in a crate whose entire
purpose is to be read from a different thread than the one publishing. The
property is real, load-bearing, and asserted only by the fact that some tests
compile.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order

# both structures and their fields
grep -vE "^[[:space:]]*//" ring_consume/src/lib.rs | grep -A5 'pub struct'

# the sibling range types, for the three-way comparison
grep -A5 'pub struct Claim$'     ring_claim/src/lib.rs
grep -A5 'pub struct BatchClaim' ring_batch/src/lib.rs

# the producer's equivalent, and why it is 128
grep -vE "^[[:space:]]*//" ring_claim/src/lib.rs | grep -A5 'pub struct Claimer'
grep -B3 'pub struct PaddedCursor' ring_cursor/src/lib.rs

# the borrow argued as mechanism, not cost
grep 'borrow\|gating set' ring_consume/src/lib.rs | head

# Send/Sync: expect nothing
grep -E 'Send|Sync|unsafe impl' ring_consume/src/lib.rs || echo "  none"
command grep 'thread::scope' ring_consume/tests/*.rs
```

Live output:

```
pub struct Available
{
  start : Seq,
  len : u64,
}

--
pub struct Consumer< 'a >
{
  cursor : &'a PaddedCursor,
  barrier : Barrier< 'a >,
}

pub struct Claim
{
  start : Seq,
  len : usize,
}

pub struct BatchClaim
{
  start : Seq,
  count : usize,
}

pub struct Claimer< 'a >
{
  cursor : PaddedCursor,
  consumers : &'a GatingSet,
}

/// assert_eq!( core::mem::align_of::< PaddedCursor >(), 64 );
/// ```
#[ derive( Debug, Default ) ]
pub struct PaddedCursor( CacheAligned< AtomicSeq > );
//! corruption the gating set exists to prevent, reintroduced above it.
//! the two calls is a read of borrowed slots.
//! matter of giving each its own cursor in the gating set — which is what
//! ## Why the cursor is borrowed rather than owned
/// // In a wired ring this cursor comes from the producer's gating set.
  none
ring_consume/tests/consume_test.rs:  std::thread::scope( | scope |
ring_consume/tests/consume_test.rs:  let seen = std::thread::scope( | scope | -> Vec< Seq >
```

Sizes come from a `size_of`/`align_of` probe over the eight types above; the
figures are quoted in [001](001_sixteen_and_twenty_four.md) at the point they
are used.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CN22 | family | n/a — duplication | `Available` is the third independently-written 16-byte half-open range in the family, agreeing on layout and diverging on length type |
| CN23 | family | n/a — observation | A `Consumer` is 24 bytes against a `Claimer`'s 128, and the whole difference is that one owns its cursor and the other borrows |
| CN24 | `ring_consume` | n/a — observation | The borrow is the wiring, not an optimisation — an owned cursor would leave the producer gating on an address nobody advances |
| CN25 | `ring_consume` | n/a — unenforced | `Send` and `Sync` are auto-derived, load-bearing, relied on by the threaded tests, and named nowhere in the crate |
