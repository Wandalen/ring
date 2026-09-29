# API: The Reading Surface

### Scope

- **Purpose**: Document the half of the crate that grants no capability — a number to compare against and a predicate to ask a question with.
- **Responsibility**: State what a caller consults these for, what neither can tell it, and why the predicate is the family's shared vocabulary rather than a private helper.
- **In Scope**: [`CACHE_LINE`](../type/001_cache_line.md) as consumed; `on_distinct_lines` as called.
- **Out of Scope**: Producing a separated layout, which is [`api/002`](002_the_wrapper_surface.md); the arithmetic inside the predicate, which is [`algorithm/001`](../algorithm/001_deciding_line_membership_by_division.md).

### Surface

```rust
pub const CACHE_LINE : usize;
pub const fn on_distinct_lines( a : usize, b : usize ) -> bool;
```

Two names, both `const`, neither of which allocates, borrows, or can fail.
**Importing this half acquires no capability** — a caller gains the ability to
*check* something about a layout it already produced by other means. That is
the opposite of [`api/002`](002_the_wrapper_surface.md), which produces the
layout and grants nothing to inspect it.

### What a Caller Consults Them For

| Use | Which name | Example |
|-----|-----------|---------|
| Assert a type is exactly one line | `CACHE_LINE` | `assert_eq!( size_of::< PaddedCursor >(), CACHE_LINE )` |
| Assert a pair costs no more than expected | `CACHE_LINE` | `assert!( size <= 3 * CACHE_LINE, "capacity must not cost a whole extra line" )` |
| Assert two live fields do not contend | `on_distinct_lines` | `on_distinct_lines( self.producer.addr(), self.consumer.addr() )` |

The first two are compile-time-ish facts a test asserts once. **The third is
the only one that reads real addresses**, and it is the reason the predicate is
public rather than a test helper: `ring_cursor` exposes it as a method on
`CursorPair`, so the check survives into consumers that never depend on this
crate.

### What Neither Can Tell a Caller

- **Whether the constant is right for the running machine.** `CACHE_LINE` is
  64 because the family's targets use 64; nothing here reads the host. On a
  128-byte-line machine both the constant and the predicate agree with each
  other and are jointly wrong (→ [`pitfall/001`](../pitfall/001_a_constant_too_small_buys_nothing.md)).
  The manual plan's M1 is the only check, and it is a human one.
- **Whether separation was *achieved* or merely *observed*.** `on_distinct_lines( 63, 64 )`
  is `true`, and 63 and 64 are one byte apart. A caller verifying its own
  padding wants the stronger distance claim, not this one
  (→ [`algorithm/001`](../algorithm/001_deciding_line_membership_by_division.md) § Why Both Forms Appear).
- **Whether the addresses passed in are still valid.** They are integers. A
  caller that computes an address, drops the value, and then asks gets a
  confident answer about memory that no longer holds what it thinks
  (→ [`decisions/002`](../decisions/002_the_predicate_takes_integers.md)).

### The Name Travelled Further Than the Function

`on_distinct_lines` is called — as a function — in exactly one place outside
this crate. But **four crates expose a name spelled `on_distinct_lines`**:

| Crate | Form | Reaches this function? |
|-------|------|------------------------|
| `ring_align` | `pub const fn on_distinct_lines( a, b )` | is it |
| [`ring_cursor`](../../../ring_cursor/readme.md) | `CursorPair::on_distinct_lines( &self )` | **yes** — the only caller |
| [`ring_spsc`](../../../ring_spsc/readme.md) | `Ring::on_distinct_lines( &self )` | transitively — delegates to `CursorPair`'s |
| [`ring_mpsc`](../../../ring_mpsc/readme.md) | its own `on_distinct_lines( &self )` | **no — see below** |

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -r 'on_distinct_lines' --include=*.rs . \
  | command grep -v '^ring_align/' \
  | sed -E 's/:/: /' | LC_ALL=C sort -u
```

Live output:

```
ring_cursor/src/lib.rs:     on_distinct_lines( self.producer.addr(), self.consumer.addr() )
ring_cursor/src/lib.rs:   /// The input to [`CursorPair::on_distinct_lines`], and the only way to check
ring_cursor/src/lib.rs:   /// assert!( pair.on_distinct_lines() );
ring_cursor/src/lib.rs:   pub fn on_distinct_lines( &self ) -> bool
ring_cursor/src/lib.rs: //! [`CursorPair::on_distinct_lines`] is the third assertion, and the only one
ring_cursor/src/lib.rs: use ring_align::{ on_distinct_lines, CacheAligned };
ring_cursor/tests/cursor_test.rs:     pair.on_distinct_lines(),
ring_cursor/tests/cursor_test.rs:   assert!( boxed.on_distinct_lines(), "still separated after a move onto the heap" );
ring_mpsc/src/lib.rs:   /// assert!( producer.on_distinct_lines() );
ring_mpsc/src/lib.rs:   pub fn on_distinct_lines( &self ) -> bool
ring_mpsc/tests/mpsc_test.rs:     assert!( producer.on_distinct_lines() );
ring_spsc/src/lib.rs:     self.cursors.on_distinct_lines()
ring_spsc/src/lib.rs:   /// Delegated to [`CursorPair::on_distinct_lines`], and re-exposed here so a
ring_spsc/src/lib.rs:   /// assert!( ring.on_distinct_lines() );
ring_spsc/src/lib.rs:   pub fn on_distinct_lines( &self ) -> bool
ring_spsc/src/lib.rs: /// assert!( ring.on_distinct_lines(), "the two cursors do not share a line" );
ring_spsc/tests/spsc_test.rs:     assert!( ring.on_distinct_lines() );
```

**This is the crate's real reach, and it is not a dependency edge.** Two crates
re-expose the question under the same name so a consumer can ask it about the
thing it actually holds — `ring_spsc`'s own doc gives the reason: "re-exposed
here so a test can assert the property about the ring it is measuring rather
than about a type it happens to contain." The vocabulary propagated where the
dependency did not, which is what naming a predicate well buys and what a
`pub(crate)` helper would have forfeited.

It also means **a rename here is a four-crate change** that the compiler will
only catch in one of them. The other three keep compiling with a method whose
name no longer matches the concept it came from.

**And in one of them the name already does not match.** `ring_mpsc`'s method
does not call this function, does not delegate to one that does, and does not
depend on this crate at all — it recomputes the question from a literal
(→ [`pitfall/001`](../pitfall/001_a_constant_too_small_buys_nothing.md) § The
Duplicate Already Exists). Same name, different predicate, second copy of the
constant.

### AL7 — Importing This Half Grants No Capability

`CACHE_LINE` is a number and `on_distinct_lines` is a pure predicate over two
integers. Neither allocates, neither mutates, neither produces a layout. A
caller that imports both can *ask* whether two addresses it already holds are on
one line, and can do nothing it could not do before beyond asking that question
correctly.

**Finding.** The crate's entire effect on a running program comes from the other
half — from what `#[ repr( align( 64 ) ) ]` instructs the compiler to do at a
consumer's field. This half is diagnostic, which is why a consumer can adopt it
without changing a single byte of layout, and why the two halves are documented
as separate APIs rather than one surface.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_deciding_line_membership_by_division.md](../algorithm/001_deciding_line_membership_by_division.md) | What the predicate computes, and the subtraction form callers sometimes want instead |

### APIs

| File | Relationship |
|------|--------------|
| [002_the_wrapper_surface.md](002_the_wrapper_surface.md) | The producing half — grants a capability, offers nothing to inspect |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_the_predicate_takes_integers.md](../decisions/002_the_predicate_takes_integers.md) | Why `usize` rather than `&T`, and the validity obligation that pushes onto the caller |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_one_dependency_one_consumer.md](../integration/001_one_dependency_one_consumer.md) | One declared consumer against four crates carrying the name — the gap this instance measures |

### Items

| File | Relationship |
|------|--------------|
| [../item/002_on_distinct_lines.md](../item/002_on_distinct_lines.md) | The declaration, its Caller Tree, and the grep behind the four-crate table |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_a_constant_too_small_buys_nothing.md](../pitfall/001_a_constant_too_small_buys_nothing.md) | The failure both names share and neither can detect |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_cache_line.md](../type/001_cache_line.md) | The constant as a design decision rather than as a value consulted |

### Sources

| File | Relationship |
|------|--------------|
| [`../invariant/001_two_wrapped_fields_never_share_a_line.md`](../invariant/001_two_wrapped_fields_never_share_a_line.md) | The invariant whose padding half this crate delivers; the reading surface is how a consumer confirms it |

### Tests

| File | Relationship |
|------|--------------|
| `tests/align_test.rs` | `cache_line_is_sixty_four` and `distinct_lines_follows_boundaries_not_distance` cover both names directly |
| `tests/manual/readme.md` | M1 covers what neither name can answer: whether 64 matches the host |
