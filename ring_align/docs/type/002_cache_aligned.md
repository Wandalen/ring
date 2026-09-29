# Type: `CacheAligned<T>`

### Scope

- **Purpose**: Document the wrapper that carries no data of its own — where the padding *is* the type — and the transparency contract that makes it usable as a field rather than an obstacle.
- **Responsibility**: State the representation, the derives and why each is needed, the accessor surface, and what the type deliberately does not do.
- **In Scope**: The newtype's shape, its trait derivations, its four associated functions.
- **Out of Scope**: The layout rule that produces the size, which is [`algorithm/002`](../algorithm/002_rounding_a_payload_up_to_whole_lines.md); the pair-level guarantee, which is [`invariant/001`](../invariant/001_two_wrapped_fields_never_share_a_line.md).

### Representation

```rust
#[ derive( Debug, Clone, Copy, Default, PartialEq, Eq ) ]
#[ repr( align( 64 ) ) ]
pub struct CacheAligned< T >( T );
```

A tuple newtype with one private field. **There is no padding field**, no
`[ u8; N ]` filler, and no `PhantomData` — the alignment attribute produces the
size, and the crate writes no arithmetic to achieve it
(→ [`algorithm/002`](../algorithm/002_rounding_a_payload_up_to_whole_lines.md)).

The field is private, which is the difference between a wrapper and a
transparent alias. A public field would let a caller take `&pair.0` and pass it
somewhere that copies the payload out of its line, and the type would still
report the right `size_of`. Keeping it private means every route to the value
goes through the four accessors below.

### Kind

A generic struct — one type parameter, no bounds. **No bounds is the decision
here.** `T : Copy` or `T : Default` would each be locally convenient and would
each disqualify a payload the family has no reason to exclude; the derives
below acquire those properties conditionally instead, which is what `derive`
does by default.

### The Derives, and Why Each

| Derive | Why |
|--------|-----|
| `Debug` | The workspace lints `missing_debug_implementations = "warn"`; a type in a public surface without it is a warning, and a padded cursor that cannot be printed is unpleasant to debug |
| `Clone`, `Copy` | So a cursor read does not have to borrow. Conditional on `T : Copy`, which is what the family's payloads are — `AtomicSeq` in [`ring_cursor`](../../../ring_cursor/readme.md) |
| `Default` | So a padded field can appear in a `#[ derive( Default ) ]` struct without a manual impl at every consumer |
| `PartialEq`, `Eq` | So two wrapped values compare by payload. Without it, equality would have to unwrap first at every call site |

**All six are conditional on the payload having the same property**, which is
the correct default and worth stating because the alternative — implementing
them unconditionally by hand — would make `CacheAligned< NotEq >` compare equal
to itself, which is a lie the compiler would not catch.

`Copy` in particular is asserted rather than assumed:

```rust
let a = CacheAligned::new( 1u64 );
let b = a;
assert_eq!( *a.get(), *b.get() );   // `a` still usable — it was copied, not moved
```

That test would fail to *compile* rather than fail to pass if `Copy` were
dropped, which makes it a compile-time assertion wearing a runtime test's
clothes. It is in the suite deliberately: a derive silently removed during a
refactor is exactly the kind of change that breaks a consumer far away.

### Accessor Surface

| Function | Signature | Const | Purpose |
|----------|-----------|:-----:|---------|
| `new` | `( value : T ) -> Self` | yes | Wrap |
| `get` | `( &self ) -> &T` | yes | Borrow |
| `get_mut` | `( &mut self ) -> &mut T` | yes | Borrow mutably |
| `into_inner` | `( self ) -> T` | **no** | Unwrap, discarding the padding |

**Three of the four are `const fn`, and the fourth cannot be.** Marking
`into_inner` const is rejected with `E0493` — *"destructor of `CacheAligned<T>`
cannot be evaluated at compile-time"*: taking `self` by value drops the wrapper
at the end of the body, and for an unbounded generic `T` the compiler must
assume that destructor exists. Check it directly:

```sh
printf '#[ repr( align( 64 ) ) ]\npub struct CacheAligned< T >( T );\nimpl< T > CacheAligned< T > { pub const fn into_inner( self ) -> T { self.0 } }\nfn main() {}\n' > ./-probe.rs
rustc --crate-name probe --edition 2021 -o /dev/null ./-probe.rs   # E0493
rm -f ./-probe.rs
```

Live output:

```
error[E0493]: destructor of `CacheAligned<T>` cannot be evaluated at compile-time
 --> ./-probe.rs:3:56
  |
3 | impl< T > CacheAligned< T > { pub const fn into_inner( self ) -> T { self.0 } }
  |                                                        ^^^^                 - value is dropped here
  |                                                        |
  |                                                        the destructor for this type cannot be evaluated in constant functions

error: aborting due to 1 previous error

For more information about this error, try `rustc --explain E0493`.
```

The asymmetry is not an oversight and is worth recording, because a reader who
notices it will otherwise assume one was forgotten.

`get` and `get_mut` returning references rather than values is what keeps the
type usable for atomics: `AtomicSeq` is not `Copy`-through-read, and a
by-value accessor would be useless for the crate's actual consumer
(→ [`api/002`](../api/002_the_wrapper_surface.md)).

### What the Type Does Not Do

- **It does not implement `Deref`.** That would make the padding invisible at
  the call site, which is convenient and wrong: the whole point is that a
  reader can see the value has been given a line. Explicit `.get()` is the
  cost, and it is deliberate.
- **It does not check that its payload is small.** A payload larger than a line
  gets whole lines, silently. That is correct behaviour and a real memory cost
  (→ [`algorithm/002`](../algorithm/002_rounding_a_payload_up_to_whole_lines.md) § The Cost, Stated).
- **It does not know about cursors.** The wrapper is generic and this crate has
  no dependency on anything that uses it. Directionality of the edge is the
  subject of [`integration/002`](../integration/002_why_the_constant_lives_here.md).

### Usage

| Crate | Uses it as |
|-------|------------|
| [`ring_cursor`](../../../ring_cursor/readme.md) | `pub struct PaddedCursor( CacheAligned< AtomicSeq > );` — the crate's entire representation, as its own module doc states: "`CacheAligned` and nothing else — no field of its own" |

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r "CacheAligned" --include=*.rs . | grep -v '^ring_align/'
```

Live output:

```
ring_cursor/src/lib.rs://! [`ring_align::CacheAligned`] and nothing else — no field of its own, no
ring_cursor/src/lib.rs:use ring_align::{ on_distinct_lines, CacheAligned };
ring_cursor/src/lib.rs:pub struct PaddedCursor( CacheAligned< AtomicSeq > );
ring_cursor/src/lib.rs:    Self( CacheAligned::new( AtomicSeq::new( value ) ) )
ring_cursor/src/lib.rs:    Self( CacheAligned::new( AtomicSeq::new( value ) ) )
ring_cursor/src/lib.rs:// Each forward below assumes `CacheAligned::get` stays a free, no-op
```

### AL46 — The Payload Is Unbounded Because the Bound That Would Help Is Not Expressible

One `impl` block, no bound on `T`. Every payload is accepted, including the
zero-sized one whose behaviour contradicts the module doc — recorded next door
as [`data_structure/001`](../data_structure/001_the_cache_aligned_wrapper.md)'s
AL9, where `CacheAligned< () >` is 64 bytes of nothing.

**Finding.** The obvious repair — reject zero-sized payloads at the type level —
is not available: there is no `T : !ZeroSized` bound, and `size_of::< T >() > 0`
is not a where-clause the language accepts. So the gap is the language's rather
than the design's, and that is precisely why AL9 is filed as a **misleading doc**
against the module comment instead of as a defect against this type. The
correction belongs in prose because prose is the only place it can be made.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_rounding_a_payload_up_to_whole_lines.md](../algorithm/002_rounding_a_payload_up_to_whole_lines.md) | The rule the attribute invokes, and the two cases it produces |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_wrapper_surface.md](../api/002_the_wrapper_surface.md) | The accessor surface as a caller meets it, including the `const` asymmetry |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_cache_aligned_wrapper.md](../data_structure/001_the_cache_aligned_wrapper.md) | The same declaration read as a layout rather than as a name |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_why_the_constant_lives_here.md](../integration/002_why_the_constant_lives_here.md) | Why this type knows nothing about its only consumer |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_cache_aligned_and_its_associated_functions.md](../item/001_cache_aligned_and_its_associated_functions.md) | Declaration sites and grep-verified usage for the struct and its four associated functions |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_the_wrapped_values_arc.md](../lifecycle/001_the_wrapped_values_arc.md) | Wrap, borrow, unwrap — and the state the padding does not survive |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_the_newtype_as_layout_carrier.md](../pattern/001_the_newtype_as_layout_carrier.md) | The general form: a type whose only content is its representation |

### Types

| File | Relationship |
|------|--------------|
| [001_cache_line.md](001_cache_line.md) | The number the attribute restates as a literal, and why the language forbids sharing it |

### Sources

| File | Relationship |
|------|--------------|
| [`../invariant/001_two_wrapped_fields_never_share_a_line.md`](../invariant/001_two_wrapped_fields_never_share_a_line.md) | "Padded so that it occupies a cache line by itself" — this type is that padding, without the cursor |

### Tests

| File | Relationship |
|------|--------------|
| `tests/align_test.rs` | `the_wrapper_round_trips_its_payload` covers all four accessors plus `Default` and `PartialEq`; `the_wrapper_is_copy` is the compile-time assertion described above |
