# Item: `CacheAligned` and Its Associated Functions

### Scope

- **Purpose**: Give the exact declaration of `CacheAligned<T>` and its four associated functions, with every attribute and derive, so that a change to any of them can be checked against one place.
- **Responsibility**: The declarations as written, the derive list with what each forwards to, the `const` status of each function, the callers, and the tests that would fail.
- **In Scope**: `CacheAligned<T>`, `new`, `get`, `get_mut`, `into_inner`.
- **Out of Scope**: Why the type exists, which is [`type/002`](../type/002_cache_aligned.md); the layout it produces, which is [`data_structure/001`](../data_structure/001_the_cache_aligned_wrapper.md).

### Declaration

```rust
// ring_align/src/lib.rs:67-69
#[ derive( Debug, Clone, Copy, Default, PartialEq, Eq ) ]
#[ repr( align( 64 ) ) ]
pub struct CacheAligned< T >( T );
```

One generic parameter, no bounds, one private tuple field. **The absence of
bounds is deliberate**: bounding `T` would restrict what can be padded for no
reason the padding cares about, and each derive carries its own bound
implicitly.

### Attributes

| Attribute | Effect | If removed |
|-----------|--------|------------|
| `#[ repr( align( 64 ) ) ]` | Sets alignment to 64 and rounds size up to a multiple of it | The crate does nothing. Every size assertion in `tests/align_test.rs` fails |
| `#[ derive( … ) ]` | Six traits, below | Callers lose `Debug` (which the workspace lint `missing_debug_implementations` requires) and `Default` (which `PaddedCursor::default()` uses) |

**The alignment literal is 64 rather than `CACHE_LINE`** because
`#[ repr( align( CACHE_LINE ) ) ]` does not compile (`E0693`,
→ [`type/001`](../type/001_cache_line.md)). This is the crate's own copy of the
number, in a crate that exists to prevent copies of the number.

### Derives

| Derive | Bound on `T` | What it acts on |
|--------|--------------|-----------------|
| `Debug` | `T : Debug` | The payload. Required by the workspace's `missing_debug_implementations = "warn"` |
| `Clone` | `T : Clone` | The payload; the clone is independently 64-aligned |
| `Copy` | `T : Copy` | The payload. A 64-byte `memcpy` for an 8-byte value — the padding is copied with it |
| `Default` | `T : Default` | The payload. `PaddedCursor::default()` reaches through this |
| `PartialEq` | `T : PartialEq` | **The payload only.** The padding bytes are not compared |
| `Eq` | `T : Eq` | Same |

**`PartialEq` going through the payload rather than the bytes is the derive
that matters.** The trailing padding is uninitialised, so a byte-wise
comparison would give two equal values an unpredictable answer. The derive does
the right thing automatically, which is precisely why hand-writing the padding
as a `[ u8; 56 ]` field would have been worse than the attribute — that field
*would* participate in the derive
(→ [`data_structure/001`](../data_structure/001_the_cache_aligned_wrapper.md)).

Not derived: `Hash`, `PartialOrd`, `Ord`. Nothing in the family needs them, and
`Capacity` in `ring_types` derives all three — so their absence here is a scope
decision rather than an oversight.

### Associated Functions

```rust
// ring_align/src/lib.rs:71-119
impl< T > CacheAligned< T >
{
  pub const fn new( value : T ) -> Self         { Self( value ) }
  pub const fn get( &self ) -> &T               { &self.0 }
  pub const fn get_mut( &mut self ) -> &mut T   { &mut self.0 }
  pub fn into_inner( self ) -> T                { self.0 }
}
```

| Fn | `const` | `#[ must_use ]` | Receiver | Body |
|----|:-------:|:---------------:|----------|------|
| `new` | **yes** | no | — | `Self( value )` |
| `get` | **yes** | no | `&self` | `&self.0` |
| `get_mut` | **yes** | no | `&mut self` | `&mut self.0` |
| `into_inner` | **no** | no | `self` | `self.0` |

**`into_inner` is the only non-`const` one, and not by choice.** Marking it
`const fn` fails with `E0493`: the destructor of `CacheAligned<T>` cannot be
evaluated at compile time. Moving the payload out of a struct in a `const`
context requires dropping the remainder, which const-eval does not do for a
generic `T` (→ [`type/002`](../type/002_cache_aligned.md)).

**None carries `#[ must_use ]`**, and only `into_inner` has a case for one —
discarding its result silently drops the payload. It is a small gap; the
failure mode that actually matters is *using* the result, which no attribute
catches (→ [`lifecycle/001`](../lifecycle/001_the_wrapped_values_arc.md) § The
Exit Is Silent).

### Callers

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell's ugrep shim, whose hit order varies run
# to run; `sort` pins the rest. No `-n`: the census is of sites, and printed
# line numbers go stale on every edit to the files being cited — twice
# already, when `align_test.rs` grew above the assertions quoted here.
command grep -r 'CacheAligned' --include=*.rs . | LC_ALL=C sort
```

Live output:

```
ring_align/src/lib.rs:  /// assert_eq!( *CacheAligned::new( 1u32 ).get(), 1 );
ring_align/src/lib.rs:  /// assert_eq!( CacheAligned::new( 3u16 ).into_inner(), 3 );
ring_align/src/lib.rs:  /// let a = CacheAligned::new( 5u8 );
ring_align/src/lib.rs:  /// let mut a = CacheAligned::new( 5u8 );
ring_align/src/lib.rs:  /// use ring_align::CacheAligned;
ring_align/src/lib.rs:  /// use ring_align::CacheAligned;
ring_align/src/lib.rs:  /// use ring_align::CacheAligned;
ring_align/src/lib.rs:  /// use ring_align::CacheAligned;
ring_align/src/lib.rs:/// The observable form of what [`CacheAligned`] buys: a test asserts this over
ring_align/src/lib.rs:/// The padding is the point: `size_of::<CacheAligned<T>>()` is a multiple of
ring_align/src/lib.rs:/// assert_eq!( core::mem::align_of::< CacheAligned< u64 > >(), CACHE_LINE );
ring_align/src/lib.rs:/// assert_eq!( core::mem::size_of::< CacheAligned< u64 > >(), CACHE_LINE );
ring_align/src/lib.rs:/// let padded = CacheAligned::new( 7u64 );
ring_align/src/lib.rs:/// use ring_align::{ CacheAligned, CACHE_LINE };
ring_align/src/lib.rs:impl< T > CacheAligned< T >
ring_align/src/lib.rs:pub struct CacheAligned< T >( T );
ring_align/tests/align_test.rs:    consumer : CacheAligned::new( 0 ),
ring_align/tests/align_test.rs:    consumer : CacheAligned< u64 >,
ring_align/tests/align_test.rs:    producer : CacheAligned::new( 0 ),
ring_align/tests/align_test.rs:    producer : CacheAligned< u64 >,
ring_align/tests/align_test.rs:  assert_eq!( CacheAligned::< u32 >::default().into_inner(), 0 );
ring_align/tests/align_test.rs:  assert_eq!( CacheAligned::new( 3u16 ), CacheAligned::new( 3u16 ) );
ring_align/tests/align_test.rs:  assert_eq!( core::mem::align_of::< CacheAligned< u64 > >(), CACHE_LINE );
ring_align/tests/align_test.rs:  assert_eq!( core::mem::align_of::< CacheAligned< u8 > >(), CACHE_LINE );
ring_align/tests/align_test.rs:  assert_eq!( core::mem::size_of::< CacheAligned< [ u8; 63 ] > >(), CACHE_LINE );
ring_align/tests/align_test.rs:  assert_eq!( core::mem::size_of::< CacheAligned< u64 > >(), CACHE_LINE );
ring_align/tests/align_test.rs:  assert_eq!( core::mem::size_of::< CacheAligned< u8 > >(), CACHE_LINE );
ring_align/tests/align_test.rs:  assert_ne!( CacheAligned::new( 3u16 ), CacheAligned::new( 4u16 ) );
ring_align/tests/align_test.rs:  let a = CacheAligned::new( 1u64 );
ring_align/tests/align_test.rs:  let mut wrapped = CacheAligned::new( 7u64 );
ring_align/tests/align_test.rs:  let size = core::mem::size_of::< CacheAligned< [ u8; 65 ] > >();
ring_align/tests/align_test.rs:use ring_align::{ on_distinct_lines, CacheAligned, CACHE_LINE };
ring_cursor/src/lib.rs:    Self( CacheAligned::new( AtomicSeq::new( value ) ) )
ring_cursor/src/lib.rs:    Self( CacheAligned::new( AtomicSeq::new( value ) ) )
ring_cursor/src/lib.rs:// Each forward below assumes `CacheAligned::get` stays a free, no-op
ring_cursor/src/lib.rs://! [`ring_align::CacheAligned`] and nothing else — no field of its own, no
ring_cursor/src/lib.rs:pub struct PaddedCursor( CacheAligned< AtomicSeq > );
ring_cursor/src/lib.rs:use ring_align::{ on_distinct_lines, CacheAligned };
```

Six hits outside this crate, all in `ring_cursor`:

| Site | Uses |
|------|------|
| `ring_cursor/src/lib.rs` — module prose | doc prose naming the type |
| `ring_cursor/src/lib.rs` — import | `use ring_align::{ on_distinct_lines, CacheAligned };` |
| `ring_cursor/src/lib.rs` — struct field | `pub struct PaddedCursor( CacheAligned< AtomicSeq > );` — the only type-level use in the workspace |
| `ring_cursor/src/lib.rs` — `PaddedCursor::new`, `#[ cfg( not( loom ) ) ]` | `CacheAligned::new( … )`, itself `const fn` |
| `ring_cursor/src/lib.rs` — `PaddedCursor::new`, `#[ cfg( loom ) ]` | the same call, in the arm that is not `const` — byte-identical to the row above, which is why the census prints it twice |
| `ring_cursor/src/lib.rs` — forwarding comment | names `CacheAligned::get` — not a use, an assumption about its cost |
| `ring_align/tests/align_test.rs` | Every function, plus the two-field struct |
| doctests in `ring_align/src/lib.rs` | `new`, `get`, `get_mut`, `into_inner`, one each |

**One production consumer, two constructions — the same call under each `loom`
arm.** `PaddedCursor` wraps `AtomicSeq` and nothing else in the family wraps
anything (→ [`integration/001`](../integration/001_one_dependency_one_consumer.md)).

**`new`'s `const` is load-bearing and the non-`loom` arm is the proof.**
`PaddedCursor::new` is declared `pub const fn`, which it can only be because
`CacheAligned::new` is `const`. Under `--cfg loom` the same function drops
`const` — for `AtomicSeq`'s sake, not this crate's, but it makes the dependency
visible: dropping `const` from `CacheAligned::new` would break the non-`loom`
build of `ring_cursor` outright.

The derive list has a wider blast radius than the type itself, since
`PaddedCursor::default()` reaches through `Default` here.

### What Breaks If This Changes

| Change | Detected by |
|--------|-------------|
| `repr` removed or its literal lowered | `a_wrapped_value_occupies_exactly_one_line` — both `size_of` and `align_of` assertions |
| `repr` literal raised without `CACHE_LINE` | Same test, since it compares against the constant, not against 64 |
| A derive dropped | Compile error in `ring_cursor` for `Default`; nothing for `PartialEq`/`Eq`, which no caller uses today |
| `new` losing `const` | Compile error — `PaddedCursor::new` is `const fn` in the non-`loom` build |
| The field made public | Nothing. No test names `.0`, and every access route already exists (→ [`pattern/001`](../pattern/001_the_newtype_as_layout_carrier.md)) |
| `get` losing `const` | Nothing in this workspace — no caller uses it in a `const` context |

**The last two rows detect nothing**, and both are the same shape: a property
the crate provides that no consumer currently exercises. Contrast the `new` row
directly above them, which is the same kind of guarantee and *is* load-bearing.
That is not an argument for removing the unexercised ones; it is an argument
for knowing which guarantees are load-bearing today and which are merely
available.

### AL25 — Every Trait the Type Has Belongs to Its Payload

```
51:#[ derive( Debug, Clone, Copy, Default, PartialEq, Eq ) ]
52:#[ repr( align( 64 ) ) ]
53:pub struct CacheAligned< T >( T );
```

Six derives and one `repr`. All six derives forward to `T` and are conditional
on `T` having the property; the `repr` is the only attribute that contributes
anything of the type's own.

**Finding.** The type's entire contribution is layout, and its entire interface
is borrowed. That is the strongest possible statement of what a layout carrier
is (→ [`pattern/001`](../pattern/001_the_newtype_as_layout_carrier.md)), and it
means removing any derive changes only what the payload can be used for, while
removing the `repr` deletes the crate.

---

### AL26 — `Copy` Is the One Derive With a Cost

Deriving `Copy` on a 64-byte type means a move copies a full cache line to
carry an eight-byte payload. Every other derive forwards and costs nothing.

**Finding.** It is the right choice — a cursor read must not borrow — and it is
the only derive where the wrapper's own size, rather than the payload's, decides
what happens at a call site. A reader auditing the derive list for cost will
find exactly one entry, and it is the one that looks most innocuous.

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_wrapper_surface.md](../api/002_the_wrapper_surface.md) | The same four functions read as a surface, with the caller obligations they carry |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_cache_aligned_wrapper.md](../data_structure/001_the_cache_aligned_wrapper.md) | The layout the `repr` produces, and why there is no padding field for the derives to reach |

### Items

| File | Relationship |
|------|--------------|
| [002_on_distinct_lines.md](002_on_distinct_lines.md) | The crate's other declaration |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_the_wrapped_values_arc.md](../lifecycle/001_the_wrapped_values_arc.md) | Each function as a transition |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_the_newtype_as_layout_carrier.md](../pattern/001_the_newtype_as_layout_carrier.md) | Why a public field would change nothing, and what the privacy actually buys |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_cache_aligned.md](../type/002_cache_aligned.md) | The same declaration as a contract — why it exists rather than what it says |

### Sources

| File | Relationship |
|------|--------------|
| `ring_align/src/lib.rs:53-119` | The declaration and all four functions |
| `ring_cursor/src/lib.rs:162` | The one production construction |

### Tests

| File | Relationship |
|------|--------------|
| `tests/align_test.rs` | Size, alignment, address separation, and payload transparency through every accessor |
| `src/lib.rs` doctests | One per function, which is what makes the `const` claims and the signatures executable documentation |
