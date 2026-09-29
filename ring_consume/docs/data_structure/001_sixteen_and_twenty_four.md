# Data Structure: Sixteen and Twenty-Four

### Scope

**Purpose:** Establish what the crate's two types cost in memory, and why the
read half of the handshake is five times smaller than the write half.

**Responsibility:** `Available` and `Consumer< 'a >` — size, alignment, field
layout, and the comparison against `ring_claim`'s equivalents.

**In Scope:** `size_of` and `align_of` for both types and their neighbours,
measured; the field lists that explain the numbers.

**Out of Scope:** What the types commit to in their signatures — that is
[`type/001`](../type/001_availables_const_surface.md) and
[`type/002`](../type/002_the_lifetime_on_consumer.md). Cache behaviour under
contention, which a single-consumer crate does not have.

---

## Measured

```sh
cd "$(git rev-parse --show-toplevel)"

# a scratch binary outside the workspace. `mkdir`, not `cargo new` — inside a
# workspace `cargo new` appends the new directory to the root manifest's
# `members`, which is a tracked file.
mkdir -p ./-cn_size/src
cat > ./-cn_size/Cargo.toml <<'TOML'
[workspace]

[package]
name = "cn_size"
version = "0.1.0"
edition = "2024"

[dependencies]
ring_consume = { path = "../ring_consume" }
ring_barrier = { path = "../ring_barrier" }
ring_claim   = { path = "../ring_claim" }
ring_cursor  = { path = "../ring_cursor" }
ring_types   = { path = "../ring_types" }
TOML

cat > ./-cn_size/src/main.rs <<'RS'
use core::mem::{ align_of, size_of };
macro_rules! p { ( $t:ty ) => {
  println!( "  {:<44} {:>4} / {:>2}", stringify!( $t ), size_of::<$t>(), align_of::<$t>() );
} }
fn main()
{
  use ring_barrier::Barrier;
  use ring_claim::{ Claim, Claimer };
  use ring_consume::{ Available, Consumer };
  use ring_cursor::PaddedCursor;
  use ring_types::{ RingError, Seq };

  println!( "this crate:" );
  p!( Available );
  p!( Consumer<'static> );
  p!( Result< Seq, RingError > );
  println!( "for contrast:" );
  p!( Claim );
  p!( Claimer<'static> );
  p!( Barrier<'static> );
  p!( PaddedCursor );
  p!( Seq );
  p!( Option< Seq > );
}
RS

cargo run --release --quiet --manifest-path ./-cn_size/Cargo.toml
rm -rf -- ./-cn_size
```

Live output:

```
this crate:
  Available                                      16 /  8
  Consumer<'static>                              24 /  8
  Result< Seq, RingError >                       24 /  8
for contrast:
  Claim                                          16 /  8
  Claimer<'static>                              128 / 64
  Barrier<'static>                               16 /  8
  PaddedCursor                                   64 / 64
  Seq                                             8 /  8
  Option< Seq >                                  16 /  8
```

---

### CN22 — `Available` Is the Third Sixteen-Byte Range in the Family

```rust
pub struct Available
{
  start : Seq,
  len   : u64,
}
```

A `Seq` and a `u64`, sixteen bytes, alignment eight — the same size and shape as
`ring_claim`'s `Claim` and `ring_batch`'s `BatchClaim`. Three crates
independently arrived at "a start and a length, sixteen bytes, `Copy`" as the
representation of a sequence range:

| Type | Crate | Fields | Means |
|------|-------|--------|-------|
| `Claim` | `ring_claim` | `start : Seq`, `len : usize` | a range this producer has been granted and must publish |
| `BatchClaim` | `ring_batch` | `start : Seq`, `count : usize` | the same, for several sequences at once |
| `Available` | `ring_consume` | `start : Seq`, `len : u64` | a range that has been published and may be read |

Two things follow, and the second is the one worth carrying.

**The convergence is real but the length types diverge.** Two crates chose
`usize` and this one chose `u64`. On any 64-bit target the sizes coincide, which
is why all three measure 16 bytes; the types do not coincide, so
`Available::len()` returns a `u64` where `Claim::len()` returns a `usize` and
code composing the two halves of the handshake needs a cast. Nothing documents
which is intended as the family's length type. `u64` is arguably the better
choice here — a sequence distance is a `u64` throughout `ring_seqno`, so
`Available` matches its own arithmetic — but that argument is not written down,
and the two older crates make the opposite choice.

**The three are structurally interchangeable and semantically not.** A `Claim`
carries an obligation; an `Available` carries a permission. Dropping a `Claim`
strands a slot forever (`ring_claim` CL43); dropping an `Available` costs
nothing at all. The type system distinguishes them; the layout does not, and a
copy-paste between the two crates compiles more readily than it should because
the shapes agree.

The corresponding `must_use` treatment is the visible signal of the difference,
and [`type/001`](../type/001_availables_const_surface.md) covers it: `Claim`
carries the most severe `must_use` message in the family; `Available`'s
`must_use` is unmessaged, because there is nothing to warn about.

**Cost:** none today. Recorded because three identical layouts with opposite
drop semantics is a fact a reader of any one of them should know.

---

### CN23 — A `Consumer` Is a Fifth the Size of a `Claimer`, Because It Owns Nothing

```rust
pub struct Consumer< 'a >
{
  cursor  : &'a PaddedCursor,
  barrier : Barrier< 'a >,
}
```

Twenty-four bytes: an eight-byte reference plus a sixteen-byte `Barrier`, which
is itself a slice reference (pointer plus length). The `Consumer` owns no
state at all — every field is a borrow, and the whole struct is a pair of
pointers with a length.

The contrast that makes this legible:

| | `Claimer< 'a >` | `Consumer< 'a >` |
|--|---------------:|-----------------:|
| Size | 128 | **24** |
| Alignment | 64 | 8 |
| Cache lines occupied | 2 | 1 — and shares it |
| Owns a cursor | ✔ — a `PaddedCursor`, 64 bytes | ✘ — borrows one |

`Claimer` is 128 bytes because it *owns* a `PaddedCursor`, which is padded to a
64-byte cache line specifically so that a producer advancing its own cursor
does not invalidate the line a different producer is reading. That padding is
the entire point of `PaddedCursor` and it costs 64 bytes plus alignment.

A `Consumer` needs none of it, for a reason that is structural rather than
incidental: **the crate is single-consumer.** There is exactly one `Consumer`
per read cursor, so there is no second consumer whose cursor could share a line
and no false sharing to pad against. The cursor it borrows is padded — that is
`ring_cursor`'s doing, protecting it from the *producers* that gate on it — but
the `Consumer` itself is a view, and views do not need padding.

The consequence for callers: a `Consumer` is cheap to construct, cheap to copy
around by value, and cheap to build ad hoc for a single call. Nothing is saved
by holding one long-term, and `Consumer::new` is `const`
([`type/002`](../type/002_the_lifetime_on_consumer.md)), so one can be built in
a `const` context. A `Claimer` is none of those things and is meant to be held.

That difference is not stated anywhere in either crate. Both are documented as
handles; only one is a handle you would think twice about creating.

**Cost:** none. It is a fact about the shape of the two halves that neither
crate's documentation makes available.

---

## What the Numbers Do Not Show

| | Why |
|--|-----|
| `Barrier`'s 16 bytes are a slice ref | it points at the dependency cursors; the cursors themselves are 64 bytes each and belong to the ring |
| `Result< Seq, RingError >` is 24, not 16 | `Seq` has no niche (`pub struct Seq( pub u64 )`), so the discriminant costs a word — see [`type/001`](../type/001_availables_const_surface.md) |
| `Option< Seq >` is 16, not 8 | the same absent niche; every `Barrier::frontier()` moves 16 bytes to carry 8 |
| a `Consumer` is one cache line | but not a dedicated one — it will share with whatever the caller puts near it, which is correct for a borrowed view |

---

## Cross-References

| Definition | Instance | Relationship |
|------------|----------|--------------|
| data_structure | [002](002_two_borrows_and_no_owned_state.md) | what having no owned state means for construction and threading |
| type | [001](../type/001_availables_const_surface.md) | `Seq`'s missing niche, and the `must_use` asymmetry |
| type | [002](../type/002_the_lifetime_on_consumer.md) | the single lifetime both borrows share |
| pattern | [001](../pattern/001_the_half_open_range_as_a_value.md) | the shape `Available` shares with `Claim` and `BatchClaim` |

### Sources

| What | Where |
|------|-------|
| `Available`'s fields | `ring_consume/src/lib.rs:98-103` |
| `Consumer`'s fields | `ring_consume/src/lib.rs:209-214` |
| `PaddedCursor`'s padding | `ring_cursor/src/lib.rs` |
| `Seq`'s definition | `ring_types/src/id.rs:25` |

### Tests

| Claim | Verified by |
|-------|-------------|
| `Available` is 16/8 | the `size_of`/`align_of` probe above |
| `Consumer` is 24/8 | the same probe |
| `Claimer` is 128/64 | the same probe |
| `Option< Seq >` is 16 | the same probe — no niche in `Seq` |
