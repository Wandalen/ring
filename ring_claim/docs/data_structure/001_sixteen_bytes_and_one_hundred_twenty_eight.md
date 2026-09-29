# Data Structure: Sixteen Bytes and One Hundred Twenty-Eight

### Scope

- **Purpose**: Measure both structures, account for the 56 bytes of tail padding in `Claimer`, and record that an 8-byte reference costs 64 bytes when added to a cache-line-aligned struct.
- **Responsibility**: Report sizes and alignments from a running program rather than by inference, decompose `Claimer`'s layout field by field, and state which of the two cache lines it spans is the contended one.
- **In Scope**: `Claim`, `Claimer`, and the two types they are built from.
- **Out of Scope**: What the borrow does to callers' APIs — see [`data_structure/002`](002_the_borrow_that_is_half_the_type.md).

### Measured, Not Inferred

```sh
cd "$(git rev-parse --show-toplevel)"

# a scratch binary outside the workspace, depending on the four crates.
# `mkdir`, not `cargo new` — inside a workspace `cargo new` appends the
# new directory to the root manifest's `members`, which is a tracked file.
mkdir -p ./-size_probe/src
cat > ./-size_probe/Cargo.toml <<'TOML'
[workspace]

[package]
name = "size_probe"
version = "0.1.0"
edition = "2024"

[dependencies]
ring_claim  = { path = "../ring_claim" }
ring_cursor = { path = "../ring_cursor" }
ring_gating = { path = "../ring_gating" }
ring_types  = { path = "../ring_types" }
TOML

cat > ./-size_probe/src/main.rs <<'RS'
use core::mem::{ align_of, size_of };
use ring_claim::{ Claim, Claimer };
fn main()
{
  println!( "Claim        size {:3}  align {:3}", size_of::< Claim >(), align_of::< Claim >() );
  println!( "Claimer      size {:3}  align {:3}", size_of::< Claimer >(), align_of::< Claimer >() );
  println!( "PaddedCursor size {:3}  align {:3}", size_of::< ring_cursor::PaddedCursor >(), align_of::< ring_cursor::PaddedCursor >() );
  println!( "GatingSet    size {:3}  align {:3}", size_of::< ring_gating::GatingSet >(), align_of::< ring_gating::GatingSet >() );
  println!( "Seq          size {:3}", size_of::< ring_types::Seq >() );
  println!( "Result<Claim,RingError> size {:3}", size_of::< Result< Claim, ring_types::RingError > >() );
}
RS

cargo run --quiet --manifest-path ./-size_probe/Cargo.toml
rm -rf -- ./-size_probe
```

Live output:

```
Claim        size  16  align   8
Claimer      size 128  align  64
PaddedCursor size  64  align  64
GatingSet    size  32  align   8
Seq          size   8
Result<Claim,RingError> size  24
```

| Type | Size | Align |
|------|-----:|------:|
| `Seq` | 8 | 8 |
| `Claim` | **16** | 8 |
| `Result< Claim, RingError >` | 24 | 8 |
| `GatingSet` | 32 | 8 |
| `PaddedCursor` | 64 | 64 |
| `Claimer` | **128** | **64** |

The two numbers worth explaining are the last row's, and they are not what a
reader summing the fields would predict.

### CL15 — `Claimer` Is 128 Bytes, of Which 56 Are Padding

```rust
pub struct Claimer< 'a >     // :250
{
  cursor : PaddedCursor,     // 64 bytes, align 64
  consumers : &'a GatingSet, //  8 bytes, align  8
}
```

Seventy-two bytes of fields. The struct is 128.

| Region | Bytes | What it is |
|--------|------:|------------|
| `cursor` | 0–63 | the contended cache line |
| `consumers` | 64–71 | an 8-byte reference |
| tail padding | 72–127 | **56 bytes**, so `size` is a multiple of `align` |

The padding exists because Rust requires `size_of::< T >()` to be a multiple of
`align_of::< T >()` — otherwise `[ Claimer; 2 ]` could not keep every element
aligned. `PaddedCursor` sets the alignment to 64, so the struct rounds up to the
next multiple of 64, and 72 rounds to 128.

The consequence is worth stating as a rule of thumb because it is
counter-intuitive and it is measurable in one command: **adding one 8-byte field
to a 64-byte cache-line-aligned struct costs 64 bytes, not 8.** A `Claimer`
carrying no reference at all would be 64 bytes. The gate reference doubles it,
and 56 of the 64 added bytes are dead.

That is 44% of the type by size, and it is entirely acceptable here for a reason
the arithmetic makes obvious: there is one `Claimer` per ring. `ring_mpsc`
constructs exactly one, inside `Ends`, and every producer shares it. Fifty-six
bytes once is not a cost worth a layout change. The number is recorded because
the same field added to a per-producer type would multiply.

### CL16 — Two Cache Lines, and Only the First Is Contended

`PaddedCursor`'s whole purpose is to occupy a cache line alone, so that a
producer's `compare_exchange` does not invalidate a line some other thread is
reading. `Claimer` spans two lines:

| Line | Holds | Written by | Read by |
|------|-------|-----------|---------|
| first (0–63) | `cursor` | every producer, on every successful claim | every producer, on every gate re-read |
| second (64–127) | `consumers`, then padding | **never after construction** | every producer, on every `headroom` call |

The second line is read-only for the life of the `Claimer` — a reference set
once by `new` and never reassigned. So it is shared-clean in every core's cache
and never invalidated, which is the best case for a shared read. The 56 padding
bytes are what guarantees no *other* data lands on that line and starts
invalidating it.

`tests/claim_test.rs:309` — `the_producer_cursor_occupies_its_own_cache_line` —
is the test named for the first line's isolation. Its entire assertion is:

```rust
assert_eq!( core::mem::size_of::< ring_cursor::PaddedCursor >(), 64 );
```

The name claims a property of *this crate's* type; the assertion is about
another crate's type and mentions neither `Claimer` nor alignment. Three gaps
follow, and all three are reachable without the assertion ever failing:

| If this changed | Would the test notice |
|-----------------|-----------------------|
| `Claimer`'s `cursor` field deleted entirely | **no** — it never names `Claimer` |
| `PaddedCursor`'s alignment dropped to 8, size still 64 | **no** — it asserts size, not `align_of` |
| a 16-byte field inserted *before* `cursor` | **no** — same reason |

Only the second and third would actually break the isolation, and size alone
cannot see either: a 64-byte type at alignment 8 shares lines routinely.
`align_of::< PaddedCursor >() == 64` and
`align_of::< Claimer >() == 64` are the two assertions that would close the gap,
and both are one line. This is `ring_cursor`'s invariant to hold, so the test's
existence here is a *dependent's* check on an upstream property — a reasonable
thing to want, testing the wrong attribute of it.

Nothing asserts the second line's read-only status, and nothing needs to: it is
a consequence of `consumers` being a `&`-reference assigned once in `new`, which
the type system already enforces.

The gate data itself is *not* on either line. `consumers` is a reference, so
`headroom()` dereferences into a `GatingSet` allocated wherever the caller put
it — and that 32-byte structure in turn holds the consumer cursors that
producers must read and consumers write. The genuinely contended traffic in a
claim is therefore three hops: this crate's cursor, the reference, and the
consumer cursors behind it
([`non_functional_requirement/001`](../non_functional_requirement/002_what_contention_costs.md)).

### `Claim`, by Contrast

```rust
pub struct Claim     // :96
{
  start : Seq,       // 8
  len : usize,       // 8
}
```

Sixteen bytes, align 8, no padding, `Copy`. It fits in two registers, so
returning one costs nothing on any target this family builds for.

The `Result` is the better number. `RingError` alone is 24 bytes — its widest
variant is `BatchTooLarge { requested : usize, capacity : usize }`, 16 bytes plus
a discriminant word — and `Result< Claim, RingError >` is **also 24**. The
success arm is free: `Claim`'s 16 bytes fit inside the error's existing layout
and the `Ok`/`Err` discriminant fits in a niche, so wrapping the claim in a
`Result` costs nothing over returning the error type by itself.

That is a happy accident of two independent choices — `Claim` being two words,
`RingError`'s widest variant being two words plus a tag — and it is worth
recording precisely because it could silently stop being true. A third field on
`Claim`, or a wider `RingError` variant, would grow every successful claim's
return value with no diagnostic and no test to notice.

### The Two Types' Derives

```sh
cd "$(git rev-parse --show-toplevel)"
grep -B1 'pub struct Claim\b\|pub struct Claimer' ring_claim/src/lib.rs
```

Live output:

```
#[ must_use = "a claimed range that is never published strands its slots and stalls every consumer" ]
pub struct Claim
--
#[ derive( Debug ) ]
pub struct Claimer< 'a >
```

| Type | Derives | Why not more |
|------|---------|--------------|
| `Claim` | `Debug, Clone, Copy, PartialEq, Eq` | `Ord` would imply a total order on ranges, which overlapping ranges do not have |
| `Claimer` | `Debug` | `Clone` would fork the cursor; `Copy` doubly so |

`Claimer`'s single derive is the load-bearing one. A `Clone` impl would produce
two claimers each advancing a private cursor from the same starting point, both
gated by the same `GatingSet`, both granting the same sequences to different
producers — the exact failure multi-producer exclusivity forbids, reachable
without any `unsafe` and without any race, from a single-threaded program.
Nothing in the crate would detect it.

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_two_constructors_of_a_range.md](../api/002_the_two_constructors_of_a_range.md) | The `Result` whose 24 bytes are measured here |

### Data Structures

| File | Relationship |
|------|--------------|
| [002_the_borrow_that_is_half_the_type.md](002_the_borrow_that_is_half_the_type.md) | What the 8-byte field costs callers, as opposed to memory |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_no_two_producers_hold_one_sequence.md](../invariant/001_no_two_producers_hold_one_sequence.md) | The property a `Clone` derive would break without a race |

### Non Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_what_contention_costs.md](../non_functional_requirement/002_what_contention_costs.md) | The three hops a claim touches |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_the_lifetime_on_the_claimer.md](../type/002_the_lifetime_on_the_claimer.md) | The `'a` that makes the second field a reference rather than a value |

### Sources

| File | Relationship |
|------|--------------|
| `ring_claim/src/lib.rs:94-100` | `Claim`, its derives and its two fields |
| `ring_claim/src/lib.rs:256-261` | `Claimer`, its single derive and its two fields |
| `ring_cursor/src/lib.rs` | `PaddedCursor`, and the 64-byte alignment that propagates |
| `ring_gating/src/lib.rs` | `GatingSet`, the 32 bytes behind the reference |
| `ring_types/src/error.rs:63-71` | `BatchTooLarge`, the widest variant |

### Tests

| File | Relationship |
|------|--------------|
| `tests/claim_test.rs:309` — `the_producer_cursor_occupies_its_own_cache_line` | The first line's isolation, asserted |
| `tests/manual/readme.md § C4` | No `Drop`, and the derives unchanged |
