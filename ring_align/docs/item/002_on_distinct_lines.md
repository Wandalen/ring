# Item: `on_distinct_lines`

### Scope

- **Purpose**: Give the exact declaration of the crate's one free function, its attributes, and a caller tree measured across all 33 crates — including the three re-declarations of the same name that this function never reaches.
- **Responsibility**: The declaration, the `#[ must_use ]`, the caller tree with the grep that produces it, and the tests that would fail.
- **In Scope**: `on_distinct_lines` as declared and as called.
- **Out of Scope**: The arithmetic and its alternatives, which is [`algorithm/001`](../algorithm/001_deciding_line_membership_by_division.md); why the parameters are integers, which is [`decisions/002`](../decisions/002_the_predicate_takes_integers.md).

### Declaration

```rust
// ring_align/src/lib.rs:137-141
#[ must_use ]
pub const fn on_distinct_lines( a : usize, b : usize ) -> bool
{
  a / CACHE_LINE != b / CACHE_LINE
}
```

Four lines including the signature. **The only item in the crate that computes
anything.**

| Property | Value | Note |
|----------|-------|------|
| `const` | yes | Verified evaluable at compile time — a `const SEPARATED : bool = on_distinct_lines( 63, 64 );` binding compiles (→ [`non_functional_requirement/002`](../non_functional_requirement/002_the_crate_costs_nothing_at_runtime.md)) |
| `#[ must_use ]` | yes | **The crate's only one.** Calling it and discarding the result is always a mistake — it has no side effect, so the result is the entire point |
| Parameters | `usize`, `usize` | Addresses as integers, not references (→ [`decisions/002`](../decisions/002_the_predicate_takes_integers.md)) |
| Panics | none | Integer division by `CACHE_LINE`, a non-zero constant |
| Generic | no | The only non-generic item besides the constant |

**`#[ must_use ]` is worth noting for what it does not cover.** It catches
`on_distinct_lines( a, b );` as a statement. It does not catch passing a length
where an address was meant, which is the failure this signature actually
exposes, and no attribute can.

### Caller Tree

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rn 'on_distinct_lines' --include=*.rs . \
  | sed -E 's/:[0-9]+:/: /' | LC_ALL=C sort -u
```

Live output:

```
ring_align/src/lib.rs:   "CACHE_LINE must be a power of two: on_distinct_lines divides by it to compute a line index"
ring_align/src/lib.rs: // `on_distinct_lines` below computes `a / CACHE_LINE` as a line index. That
ring_align/src/lib.rs: /// assert!( !on_distinct_lines( 0, 63 ) );    // both in line 0
ring_align/src/lib.rs: /// assert!( !on_distinct_lines( 128, 130 ) ); // both in line 2
ring_align/src/lib.rs: /// assert!( on_distinct_lines( 63, 64 ) );    // straddling the boundary
ring_align/src/lib.rs: /// use ring_align::on_distinct_lines;
ring_align/src/lib.rs: pub const fn on_distinct_lines( a : usize, b : usize ) -> bool
ring_align/tests/align_test.rs:   assert!( !on_distinct_lines( 0, 63 ), "0 and 63 are both in line 0" );
ring_align/tests/align_test.rs:   assert!( !on_distinct_lines( 100, 100 ) );
ring_align/tests/align_test.rs:   assert!( !on_distinct_lines( 64, 127 ) );
ring_align/tests/align_test.rs:   assert!( !on_distinct_lines( a, b ), "the unpadded pair should share a line" );
ring_align/tests/align_test.rs:   assert!( on_distinct_lines( 0, 64 ) );
ring_align/tests/align_test.rs:   assert!( on_distinct_lines( 127, 128 ) );
ring_align/tests/align_test.rs:   assert!( on_distinct_lines( 63, 64 ), "63 and 64 straddle the boundary" );
ring_align/tests/align_test.rs:   assert!( on_distinct_lines( a, b ) );
ring_align/tests/align_test.rs: /// `on_distinct_lines` reads line boundaries, not raw distance: two addresses
ring_align/tests/align_test.rs: /// crate, because `on_distinct_lines` stays calibrated to the same wrong
ring_align/tests/align_test.rs: use ring_align::{ on_distinct_lines, CacheAligned, CACHE_LINE };
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

| Site | Kind | Reaches this function |
|------|------|:---------------------:|
| `ring_align/src/lib.rs:133-135` | 3 doctest assertions | direct |
| `ring_align/tests/align_test.rs:93` | `two_wrapped_fields_land_on_different_lines` | direct |
| `ring_align/tests/align_test.rs:113` | `two_unwrapped_fields_share_a_line` — the negative control | direct |
| `ring_align/tests/align_test.rs:121-126` | `distinct_lines_follows_boundaries_not_distance`, 6 assertions | direct |
| `ring_cursor/src/lib.rs:440` | `CursorPair::on_distinct_lines` — `on_distinct_lines( self.producer.addr(), self.consumer.addr() )` | **direct — the only production call in the workspace** |
| `ring_cursor/tests/cursor_test.rs:85, 99` | via `CursorPair`'s method | transitively |
| `ring_spsc/src/lib.rs:364` | `Ring::on_distinct_lines` — `self.cursors.on_distinct_lines()` | transitively, via `CursorPair` |
| `ring_spsc/tests/spsc_test.rs:217` | via `Ring`'s method | transitively |
| `ring_mpsc/src/lib.rs`, `Producer::on_distinct_lines` | a method of the same name | **no** — computes `abs_diff( … ) >= 64` from a literal |
| `ring_mpsc/tests/mpsc_test.rs:288` | via `ring_mpsc`'s own method | **no** |

**One production call.** Everything else is a test, a doctest, or a re-export
of the *question* under the same name in a crate that holds the values
(→ [`api/001`](../api/001_the_reading_surface.md) § The Name Travelled Further
Than the Function).

**The last two rows are the finding.** `ring_mpsc` declares a method with this
exact name that does not call this function, does not delegate to one that
does, and does not depend on this crate — and its test asserts on it. Nothing
about that is a type error (→ [`pitfall/001`](../pitfall/001_a_constant_too_small_buys_nothing.md)
§ The Duplicate Already Exists).

### Rename Blast Radius

Renaming this function is a **four-crate change that the compiler reports in
one crate**:

| Crate | Compiler catches it? |
|-------|:--------------------:|
| `ring_align` | yes — the declaration and its own tests |
| `ring_cursor` | yes — the `use` and the call at `:440` |
| `ring_spsc` | **no** — its method's *name* matches by convention, not by reference |
| `ring_mpsc` | **no** — no reference to this crate at all |

After a rename, `ring_spsc` and `ring_mpsc` keep compiling with a method named
after a concept whose canonical name has changed. That is a Term Collision
waiting on the next rename rather than a defect today, and it is the concrete
cost of the vocabulary having spread further than the dependency edge.

### What Breaks If This Changes

| Change | Detected by |
|--------|-------------|
| Division swapped for `abs_diff >= CACHE_LINE` | `distinct_lines_follows_boundaries_not_distance` — its six cases exist precisely to pin the boundary semantics (`63, 64` and `100, 100` are the discriminating pair) |
| `CACHE_LINE` raised without updating the doctest cases | The `63, 64` and `0, 64` assertions fail — both become `false` at 128 (→ [`lifecycle/002`](../lifecycle/002_the_constant_across_a_platform_port.md) site 5) |
| `const` dropped | Nothing in this workspace — no caller uses it in a `const` context today |
| `#[ must_use ]` dropped | Nothing. No test asserts on the lint |
| Made private | Compile error in `ring_cursor`; `ring_spsc` and `ring_mpsc` unaffected |

### AL27 — The Whole Family-Facing Use of This Crate Passes Through One Consumer

```
    CACHE_LINE         ring_align ring_cursor
    CacheAligned       ring_align ring_cursor
    on_distinct_lines  ring_align ring_cursor ring_mpsc ring_spsc
```

Two of the three exported names are known to exactly two crates. The third
appears in four, and two of those four are the spelling collision recorded at
[`integration/001`](../integration/001_one_dependency_one_consumer.md) AL24
rather than four users.

**Finding.** Read correctly, all three rows say the same thing: `ring_cursor` is
the only consumer. The `on_distinct_lines` row is the one that would mislead a
reader running a single grep, and it is the row that looks most like evidence of
reach.

---

### AL28 — The Annotation Is on the Obvious Case Rather Than the Costly One

```
121-#[ must_use ]
122:pub const fn on_distinct_lines( a : usize, b : usize ) -> bool
```

This is the crate's one `#[ must_use ]`, on the item where ignoring the result
is merely useless. `into_inner` — where ignoring the result destroys the value —
has none
(→ [`api/002`](../api/002_the_wrapper_surface.md) AL5).

**Finding.** A predicate is the case a reader thinks of when they think of
`must_use`, so the annotation landed where the idiom points rather than where
the comparison would have put it. The evidence that no comparison happened is
that the crate has five entry points and one annotation.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_deciding_line_membership_by_division.md](../algorithm/001_deciding_line_membership_by_division.md) | The one line of arithmetic, and the subtraction form it is not |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_reading_surface.md](../api/001_the_reading_surface.md) | The same function as half of the reading surface, with the four-crate name table |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_the_predicate_takes_integers.md](../decisions/002_the_predicate_takes_integers.md) | Why the parameters are `usize`, and the `E0015` bind behind it |

### Items

| File | Relationship |
|------|--------------|
| [001_cache_aligned_and_its_associated_functions.md](001_cache_aligned_and_its_associated_functions.md) | The crate's other declaration |

### Non Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_the_crate_costs_nothing_at_runtime.md](../non_functional_requirement/002_the_crate_costs_nothing_at_runtime.md) | The `const`-evaluability probe, and the single division as the crate's whole instruction cost |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_a_constant_too_small_buys_nothing.md](../pitfall/001_a_constant_too_small_buys_nothing.md) | The two rows in the caller tree that do not reach this function |

### Sources

| File | Relationship |
|------|--------------|
| `ring_align/src/lib.rs:121-141` | The declaration and its doc comment |
| `ring_cursor/src/lib.rs:440` | The one production call |
| `ring_mpsc/src/lib.rs:857-866` | The same name, a different predicate, a private constant |

### Tests

| File | Relationship |
|------|--------------|
| `tests/align_test.rs` | 9 direct assertions across three tests, including the negative control |
| `src/lib.rs` doctests | 3 assertions on integer literals — the form the signature makes possible |
| `tests/manual/readme.md` | M4 records why the doctest uses literals rather than stack locals |
