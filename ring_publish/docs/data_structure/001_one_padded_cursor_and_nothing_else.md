# Data Structure: One Padded Cursor and Nothing Else

### Scope

- **Purpose**: Specify `Publisher`'s layout — one private field, 64 bytes, eight of them state — and what each layer of that field contributes.
- **Responsibility**: Trace the type from `Publisher` down to the `AtomicU64`, account for the 56 bytes of padding, and record the one `const fn` the crate has and the one it does not.
- **In Scope**: `Publisher` (`src/lib.rs:84-88`) and the type stack beneath its field.
- **Out of Scope**: The other three cursors a publication is observed through — see [`data_structure/002`](002_the_four_cursors_of_the_handshake.md).

### The Whole Structure

```rust
#[ derive( Debug, Default ) ]
pub struct Publisher
{
  cursor : PaddedCursor,
}
```

One field, private, no generics, no lifetime, no `PhantomData`. There is no
capacity, no mask, no buffer, no record of what was claimed, and no reference to
any other cursor. Everything the crate knows is one `Seq` in one cell.

The type stack under that field, and what each layer adds:

| Layer | Crate | Adds |
|-------|-------|------|
| `PaddedCursor( CacheAligned< AtomicSeq > )` | `ring_cursor:143` | the `SeqCell` implementation, and `addr()` for checking the padding against reality |
| `CacheAligned< T >( T )`, `#[ repr( align( 64 ) ) ]` | `ring_align:68-69` | a whole cache line to itself |
| `AtomicSeq( AtomicU64 )` | `ring_atomic:166` | the explicit-ordering surface, and the `--cfg loom` seam |
| `AtomicU64` | `core` or `loom` | the state |

Four crates deep for eight bytes of state, and each layer is load-bearing:
strip `CacheAligned` and this cursor shares a line with whatever a consumer put
beside it; strip `AtomicSeq` and the orderings stop being named
([`pattern/002`](../pattern/002_the_named_ordering_constant.md)) and the `loom`
seam has nowhere to live
([`workaround/001`](../workaround/001_the_loom_seam_and_its_only_user.md)).

### PB13 — Eight Bytes of State in Sixty-Four Bytes of Struct

`ring_cursor`'s own doctest asserts the size (`:139-140`):

```rust
assert_eq!( core::mem::size_of::< PaddedCursor >(), 64 );
assert_eq!( core::mem::align_of::< PaddedCursor >(), 64 );
```

`Publisher` is a struct with exactly one field of that type, so it inherits both:
64 bytes, 64-aligned, of which eight are the `AtomicU64` and **fifty-six are
padding**. `ring_align:53-55` states the reason as a guarantee rather than a
side effect:

> The padding is the point: `size_of::<CacheAligned<T>>()` is a multiple of
> [`CACHE_LINE`] for any `T` that fits, so two of them in one struct are
> guaranteed to land on different lines.

Which raises the obvious question for a struct with *one* of them: padding
against what? Nothing inside `Publisher` can share its line, because nothing else
is inside `Publisher`. The answer is that the sharing this prevents is with
whatever the *caller* places next to it, and in the handshake that is always
another hot cursor. `tests/handshake_test.rs` puts a `Publisher`, a `GatingSet`
and a `Claimer` in scope together; without the padding, a producer's
compare-exchange on the published cursor would invalidate the line holding the
consumer's cursor, and every gating read would miss.

The cost is real and worth naming: at 64 bytes for 8 bytes of state, a hypothetical
ring holding many publishers would spend 87.5% of that array on padding. That is
the same arithmetic that ruled out `PaddedCursor` for `ring_mpsc`'s per-slot
stamps — a per-slot stamp must not be 64-byte aligned, since `PaddedCursor`
would make the stamp array 64× the payload array — and it is why that crate
reached for the unpadded `AtomicSeq` instead. One publisher per ring makes the trade obviously
right here and obviously wrong there, on identical types.

### PB14 — One `const fn`, and It Is Not the Constructor

```sh
cd "$(git rev-parse --show-toplevel)"
grep -E '^\s*pub (const )?fn ' ring_publish/src/lib.rs
```

Live output:

```
  pub fn new() -> Self
  pub const fn cursor( &self ) -> &PaddedCursor
  pub fn published( &self ) -> Seq
  pub fn try_publish( &self, start : Seq, len : usize ) -> Result< Seq, Seq >
  pub fn publish( &self, start : Seq, len : usize ) -> Seq
  pub fn is_published( &self, seq : Seq ) -> bool
```

Six methods; exactly one is `const`, and it is `cursor` (`:117`), an accessor.
`new` (`:100`) is not:

```rust
pub fn new() -> Self
{
  Self::default()
}
```

`Default::default()` is not const-callable, so the body forces the signature.
Written instead as `Self { cursor : PaddedCursor::new( Seq::ZERO ) }` it *would*
be const — `PaddedCursor::new` is `const fn` at `ring_cursor:159` — but only in
an ordinary build. Under `--cfg loom` that constructor is the non-`const` variant
at `:167`, because loom's atomics carry per-execution model state and have no
`const` constructor (`ring_atomic:48-51`). A `const` `Publisher::new` would
therefore have to carry `ring_cursor`'s `#[ cfg( not( loom ) ) ]` /
`#[ cfg( loom ) ]` duplication down into this crate — two constructors, one
comment explaining the split, in a crate that currently has neither.

So the crate pays one non-`const` constructor to keep the `loom` seam invisible,
which is the stated design goal of the seam itself (`ring_atomic:44-46`: *"no
other crate needs to know the seam exists"*). It is a real trade and it is
undocumented in the source — this is the record of it.

`tests/publish_test.rs:36-43` guards the delegation from the other side, and says
why it is asserted rather than assumed:

> `new` is `Self::default()` today. Asserting the equivalence rather than
> trusting it means a future `new` that grew a parameter or a non-zero start has
> to notice this test rather than silently diverging from `Default`.

### The Field Is Private and Handed Out By Reference

`cursor` is not `pub`. The only way to it is `cursor( &self ) -> &PaddedCursor`,
and the reference — not a copy — is the whole contract:

| If `cursor()` returned | Then |
|------------------------|------|
| `&PaddedCursor` (actual) | a consumer's `Barrier` observes every advance |
| `PaddedCursor` (a copy) | every consumer's frontier is frozen at construction — a ring that compiles, runs, and never delivers |
| `Seq` (a snapshot) | that is `published()`, which exists separately and answers a different question |

`tests/publish_test.rs:45-58` asserts the reference identity with
`core::ptr::eq` — the suite's only address-level assertion — after checking that
a borrow taken *before* a publication sees the advance.

The private field is also what makes
[`invariant/001`](../invariant/001_the_frontier_moves_only_by_compare_exchange.md)
structural rather than conventional: `&PaddedCursor` exposes `SeqCell`, which has
`store` and `fetch_add` on it, so a holder of the reference *can* move the
frontier arbitrarily. What the privacy buys is that nothing inside this crate
does, and `tests/manual/readme.md § P2` is the grep that keeps it so.

### What `Publisher` Derives, and What It Cannot

`#[ derive( Debug, Default ) ]` — two, and no more:

| Trait | Status | Why |
|-------|--------|-----|
| `Debug` | derived | via `PaddedCursor: Debug`, which is derived over `CacheAligned: Debug` |
| `Default` | derived | the frontier starts at `Seq::ZERO`, which is the only sensible start |
| `Clone` / `Copy` | impossible | `AtomicU64` is neither; a copied publisher would be a second, silently divergent frontier |
| `PartialEq` / `Eq` | absent | comparing two publishers is comparing two frontiers at two unsynchronised moments |
| `Send` / `Sync` | automatic | `AtomicU64` is both; this is what lets `&Publisher` cross a `thread::scope` boundary in every threaded test |

The last row is the one the tests depend on and nothing states: every threaded
test shares one `&Publisher` across several producer threads, which works because
the auto-traits hold, and would stop compiling the moment a non-`Sync` field were
added. See
[`type/002`](../type/002_two_derives_and_the_ones_that_are_absent.md).

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_six_methods_and_no_caller.md](../api/001_six_methods_and_no_caller.md) | The six methods over this one field |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_the_compare_exchange_that_refuses.md](../algorithm/001_the_compare_exchange_that_refuses.md) | The only operation that writes this cell |

### Data Structures

| File | Relationship |
|------|--------------|
| [002_the_four_cursors_of_the_handshake.md](002_the_four_cursors_of_the_handshake.md) | The three other cursors a publication is observed against |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_ten_crates_name_it_and_none_depends_on_it.md](../integration/001_ten_crates_name_it_and_none_depends_on_it.md) | `ring_cursor`, the dependency this field is |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_the_frontier_moves_only_by_compare_exchange.md](../invariant/001_the_frontier_moves_only_by_compare_exchange.md) | What the privacy of this field establishes |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_three_readings_of_the_cursor.md](../item/001_the_three_readings_of_the_cursor.md) | The three ways out to this one value |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_two_derives_and_the_ones_that_are_absent.md](../type/002_two_derives_and_the_ones_that_are_absent.md) | The derive set, and the four traits that are absent |

### Workarounds

| File | Relationship |
|------|--------------|
| [../workaround/001_the_loom_seam_and_its_only_user.md](../workaround/001_the_loom_seam_and_its_only_user.md) | The seam three layers down, and what it costs `new` |

### Sources

| File | Relationship |
|------|--------------|
| `ring_publish/src/lib.rs:69-120` | The struct, its doctest, and the two accessors |
| `ring_cursor/src/lib.rs:125-190` | `PaddedCursor`, its asserted size and alignment, and the `const`/non-`const` constructor split |
| `ring_align/src/lib.rs:28-53` | `CACHE_LINE`, and the padding guarantee stated as the point |
| `ring_atomic/src/lib.rs:45-57,32` | The `loom` seam and its stated `const` cost; `AtomicSeq` |

### Tests

| File | Relationship |
|------|--------------|
| `tests/publish_test.rs:26-34` | A new publisher has published nothing, read three ways |
| `tests/publish_test.rs:36-43` | `new` and `default` agree, asserted rather than assumed |
| `tests/publish_test.rs:45-58` | The cursor handed out is the one publication moves — `core::ptr::eq` |
| `tests/manual/readme.md § P2` | Nothing in the crate writes the cell except the one compare-exchange |
