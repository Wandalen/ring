# data_structure

Two structures, four fields between them, and neither is the size a reader
summing the fields would predict. `Claim` is sixteen bytes and exactly its
contents. `Claimer` is seventy-two bytes of fields in a hundred and twenty-eight
byte type, because a single 8-byte reference added to a cache-line-aligned
struct costs a whole cache line.

The two files split memory from consequence. The first measures — sizes,
alignments, padding, which of two cache lines is contended — and finds the
crate's cache-line test asserting the wrong attribute of the right property. The
second follows the same 8-byte field outward, into the one crate that builds a
ring on this one, where it is named as the cause of a public type's existence in
three separate doc comments.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Sixteen Bytes and One Hundred Twenty-Eight](001_sixteen_bytes_and_one_hundred_twenty_eight.md) | CL15, CL16 — both layouts measured from a running program, the 56 padding bytes, the two cache lines and which is written, and the three gaps in the test named for that property |
| 002 | [The Borrow That Is Half the Type](002_the_borrow_that_is_half_the_type.md) | CL17, CL18 — why the gating set cannot be owned, the type it forced into existence in `ring_mpsc`, and the four lifetime-carrying types with no type parameter |

### Both Layouts

| Type | Fields | Field bytes | Size | Align | Padding |
|------|--------|------------:|-----:|------:|--------:|
| `Claim` | `start : Seq`, `len : usize` | 16 | 16 | 8 | 0 |
| `Claimer< 'a >` | `cursor : PaddedCursor`, `consumers : &'a GatingSet` | 72 | **128** | **64** | **56** |

`Claim` is `Copy` and `Claimer` is not, and that is the same fact as the table:
copying two integers shares nothing, while copying a cursor forks it — two
producers advancing private cursors from the same start, granting the same
sequences, with no `unsafe` and no race to blame.

### Where the Bytes Go

| Byte range | Contents | Written after construction | Read by |
|-----------|----------|----------------------------|---------|
| 0–63 | `cursor` | every successful claim | every gate re-read |
| 64–71 | `consumers` | never | every `headroom` call |
| 72–127 | tail padding | never | never |

The padding is not waste in the usual sense — it is what keeps anything else
from landing on the second line and invalidating a reference every producer
reads. It costs 56 bytes exactly once per ring.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order

# both structures and their fields
sed -n '/^#\[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]$/,/^}$/p;/^\/\/\/ let claimer = Claimer::new( &consumers );$/,/^}$/p' ring_claim/src/lib.rs

# every public type in the family carrying a lifetime
command grep -r "^pub struct [A-Za-z]*< *'a" ring_*/src/*.rs

# …and the four with no type parameter alongside it
command grep -r "^pub struct [A-Za-z]*< *'a *>" ring_*/src/*.rs

# the three Ends types, and which one names a cause
command grep -r -B4 '^pub struct Ends' ring_*/src/*.rs

# what the test named for cache-line isolation actually asserts
command grep -m1 -A9 -F 'fn the_producer_cursor_is_readable_and_starts_at_zero()' ring_claim/tests/claim_test.rs | tail -n 9
```

Live output:

```
#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
#[ must_use = "a claimed range that is never published strands its slots and stalls every consumer" ]
pub struct Claim
{
  start : Seq,
  len : usize,
}
/// let claimer = Claimer::new( &consumers );
///
/// let claim = claimer.claim( 3 ).expect( "an empty ring has room" );
/// assert_eq!( claim.start(), Seq::ZERO );
/// assert_eq!( claimer.claimed(), Seq( 3 ) );
/// ```
#[ derive( Debug ) ]
pub struct Claimer< 'a >
{
  cursor : PaddedCursor,
  consumers : &'a GatingSet,
}
ring_barrier/src/lib.rs:pub struct Barrier< 'a >
ring_claim/src/lib.rs:pub struct Claimer< 'a >
ring_consume/src/lib.rs:pub struct Consumer< 'a >
ring_core/src/lib.rs:pub struct Ends< 'a, T >
ring_core/src/lib.rs:pub struct Producer< 'a, T >
ring_core/src/lib.rs:pub struct Consumer< 'a, T >
ring_flush/src/lib.rs:pub struct Flusher< 'a, T >
ring_handle/src/lib.rs:pub struct Ends< 'a, T >
ring_handle/src/lib.rs:pub struct Producer< 'a, T >
ring_handle/src/lib.rs:pub struct Consumer< 'a, T >
ring_mpsc/src/lib.rs:pub struct Ends< 'a, S >
ring_mpsc/src/lib.rs:pub struct Producer< 'a, S >
ring_mpsc/src/lib.rs:pub struct Reserved< 'a, S >
ring_mpsc/src/lib.rs:pub struct Consumer< 'a, S >
ring_mpsc/src/lib.rs:pub struct Batch< 'a, S >
ring_shutdown/src/lib.rs:pub struct Stopped< 'a >
ring_shutdown/src/lib.rs:pub struct Guarded< 'a, T >
ring_spsc/src/lib.rs:pub struct Producer< 'a, S >
ring_spsc/src/lib.rs:pub struct Reservation< 'a, S >
ring_spsc/src/lib.rs:pub struct Consumer< 'a, S >
ring_spsc/src/lib.rs:pub struct Batch< 'a, S >
ring_tls/src/lib.rs:pub struct Flush< 'a, T >
ring_barrier/src/lib.rs:pub struct Barrier< 'a >
ring_claim/src/lib.rs:pub struct Claimer< 'a >
ring_consume/src/lib.rs:pub struct Consumer< 'a >
ring_shutdown/src/lib.rs:pub struct Stopped< 'a >
ring_core/src/lib.rs-}
ring_core/src/lib.rs-
ring_core/src/lib.rs-/// A borrow of a [`Ring`], from which the two ends are taken.
ring_core/src/lib.rs-#[ derive( Debug ) ]
ring_core/src/lib.rs:pub struct Ends< 'a, T >
--
ring_handle/src/lib.rs-}
ring_handle/src/lib.rs-
ring_handle/src/lib.rs-/// The two ends, before they are separated.
ring_handle/src/lib.rs-#[ derive( Debug ) ]
ring_handle/src/lib.rs:pub struct Ends< 'a, T >
--
ring_mpsc/src/lib.rs-/// Holds the [`Claimer`] — and therefore the claim cursor — that every producer
ring_mpsc/src/lib.rs-/// shares. It exists as a separate type only because `Claimer` borrows the
ring_mpsc/src/lib.rs-/// [`GatingSet`] inside the ring; see [`Ring::ends`].
ring_mpsc/src/lib.rs-#[ derive( Debug ) ]
ring_mpsc/src/lib.rs:pub struct Ends< 'a, S >
{
  let consumers = GatingSet::new( cap( 8 ), 1 );
  let claimer = Claimer::new( &consumers );

  assert_eq!( claimer.cursor().load( Ordering::Acquire ), Seq::ZERO );
  let _claim = claimer.claim( 3 ).unwrap();
  assert_eq!( claimer.cursor().load( Ordering::Acquire ), Seq( 3 ) );
}
```

Sizes come from a scratch binary rather than a command — see
[`001`](001_sixteen_bytes_and_one_hundred_twenty_eight.md) for the ten-line
`main.rs` that prints them.

| | Value |
|--|------:|
| `Claim` size / align | 16 / 8 |
| `Claimer` size / align | 128 / 64 |
| …live field bytes | 72 |
| …tail padding | **56** — 44% of the type |
| Cost of adding one 8-byte field to a 64-aligned struct | **64 bytes** |
| `PaddedCursor` size / align | 64 / 64 |
| `GatingSet` size / align | 32 / 8 |
| `RingError` size | 24 |
| `Result< Claim, RingError >` size | **24** — the success arm is free |
| Cache lines a `Claimer` spans | 2 |
| …written after construction | **1** |
| Derives on `Claim` | 5 |
| Derives on `Claimer` | **1** — `Debug` |
| Public types in the family carrying a lifetime | 23 |
| …that also carry a type parameter | 19 |
| …with no type parameter | **4** |
| Crates with an `Ends` type | 3 |
| …whose doc names a cause | **1** |
| `ring_mpsc` doc comments naming this crate's borrow | 3 |

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CL15 | `ring_claim` | n/a — observation | `Claimer` is 128 bytes holding 72 bytes of fields; adding one 8-byte reference to a 64-byte cache-line-aligned struct costs 64 bytes, not 8, and 56 of them are dead |
| CL16 | `ring_claim` | n/a — observation | A `Claimer` spans two cache lines and only the first is ever written; the second holds a reference assigned once, so it stays shared-clean in every core's cache, and the padding is what guarantees nothing else lands there |
| CL17 | `ring_mpsc` | n/a — observation | The borrow forced `ring_mpsc::Ends` into existence; of the family's three `Ends` types it is the only one whose doc states a cause, and that cause is this crate's second field, named in three separate doc comments |
| CL18 | family | n/a — observation | Four of the family's 23 lifetime-carrying public types have no type parameter, and they are exactly the four operating on sequences rather than payloads — the Tier 5 boundary drawn in the type system |

Supporting observations recorded alongside those findings, carrying no ID of their own:

| Note | Where |
|------|-------|
| Acceptable here only because there is exactly one `Claimer` per ring — the same field on a per-producer type would multiply | [001](001_sixteen_bytes_and_one_hundred_twenty_eight.md) |
| `the_producer_cursor_occupies_its_own_cache_line` asserts `size_of::< PaddedCursor >() == 64` and nothing else — it never names `Claimer`, never checks `align_of`, and would pass unchanged if the field were deleted or the alignment dropped to 8 | [001](001_sixteen_bytes_and_one_hundred_twenty_eight.md) |
| `Result< Claim, RingError >` is 24 bytes and so is `RingError` alone, so the success arm is free — an accident of two independent widths that a third `Claim` field would silently end | [001](001_sixteen_bytes_and_one_hundred_twenty_eight.md) |
| The alternative to borrowing is not owning, it is a self-referential ring: an owned `GatingSet` would give consumers and producers two different sets, and back-pressure would stop working entirely | [002](002_the_borrow_that_is_half_the_type.md) |
| `Claimer` is the only one of the four a ring wants to *keep*, which is precisely when a borrow stops being a call-site concern and becomes a structural one | [002](002_the_borrow_that_is_half_the_type.md) |

