# API: The Borrow Is the Whole Type

### Scope

- **Purpose**: Show that `Barrier` is a slice reference with nine methods hung off it, and work out what that identity gives a caller and what it takes away.
- **Responsibility**: Give the size, the auto-traits, the `Copy` consequence, and the one place the `'a` lifetime is visible in a return type.
- **In Scope**: The type's identity as seen from outside.
- **Out of Scope**: The field's layout — see [`data_structure/001`](../data_structure/001_one_field_and_a_sixteen_byte_view.md).

### BR7 — The Type Is Its Field

```
size_of::< Barrier >()          = 16
size_of::< &[ PaddedCursor ] >() = 16
align_of::< Barrier >()         = 8
Barrier : Send + Sync + Copy
```

Sixteen bytes — a pointer and a length, and nothing added. Every property below
follows from that and none of them was written down:

| Property | Source | Consequence |
|----------|--------|-------------|
| `Copy` | `#[ derive( … Copy ) ]` on a `Copy` field | A barrier can be handed to a thread and still used |
| `Send` | `&T : Send` where `T : Sync`, and `PaddedCursor` is `Sync` | It crosses a thread boundary as a plain value |
| `Sync` | `&T : Sync` where `T : Sync` | Two threads may share one |
| No `Drop` | Nothing owned | Releasing it is not an event |
| Sixteen bytes | One slice reference | Passing by value costs a register pair |

`Copy` is load-bearing rather than convenient. `a_consumer_waiting_on_a_producer_thread_makes_progress`
moves a barrier into a spawned thread and keeps using it on the main one; with
`Clone` alone that test needs a `.clone()` at the boundary, and with neither it
needs a scoped thread or an `Arc`. The derive is four characters and it is the
reason the crate's one concurrency test reads like straight-line code.

The compiler has an opinion about how thoroughly this is a view. `drop( barrier )`
warns:

```
warning: calls to `std::mem::drop` with a value that implements `Copy`
   = note: `#[warn(dropping_copy_types)]` on by default
```

There is no way to release a barrier, because there is nothing to release. See
[`lifecycle/001`](../lifecycle/001_a_barrier_from_over_to_the_end_of_a_borrow.md).

### BR8 — `cursor` Returns `&'a`, Not `&self`

Two crates expose the same accessor over the same element type and they differ
in one lifetime:

```rust
// ring_barrier/src/lib.rs:164
pub fn cursor( &self, index : usize ) -> Option< &'a PaddedCursor >

// ring_gating/src/lib.rs:142
pub fn cursor( &self, index : usize ) -> Option< &PaddedCursor >
```

The second elides to `&self`'s lifetime. The first names the slice's, and the
consequence is that a reference obtained through a barrier is **not** bounded by
the barrier:

```rust
let borrowed : Option< &PaddedCursor > =
{
  let barrier = Barrier::over( &cursors );
  let c = barrier.cursor( 0 );
  drop( barrier );
  c                                        // compiles
};
// prints: outlived barrier, reads Some( Seq( 7 ) )
```

The identical shape against `GatingSet` does not compile:

```
error[E0597]: `set` does not live long enough
11 |     set.cursor( 0 )
   |     ^^^ borrowed value does not live long enough
12 |   };
   |   - `set` dropped here while still borrowed
```

That is not two styles. It is the ownership difference showing up in the type
system exactly where it should: a `GatingSet` *owns* its cursors, so a reference
into it cannot outlive it; a `Barrier` borrows cursors that already outlive it,
so a reference through it is a reference to somebody else's data and the barrier
is not in the picture at all.

| | `Barrier::cursor` | `GatingSet::cursor` |
|--|-------------------|---------------------|
| Returned lifetime | The dependencies' | The set's |
| Reference may outlive the receiver | Yes | No — `E0597` |
| Because | The cursors are somebody else's | The cursors are the set's |

Getting this backwards is not a compile error either — `Option< &PaddedCursor >`
on a `Barrier` compiles fine and is simply more restrictive than necessary. It
would be a silent narrowing that only bites the one caller that needs the wider
form, which is exactly the class of change nothing in the suite would catch.

### What the Borrow Costs

The type is 16 bytes and carries no capacity, no ownership, and no identity —
so three things a caller might reasonably expect are absent:

| Expectation | Reality | Where discussed |
|-------------|---------|-----------------|
| "Two barriers over the same set are the same barrier" | No `PartialEq` — there is nothing to compare but a pointer | [`type/002`](../type/002_the_traits_derived_and_the_traits_absent.md) |
| "Dropping the barrier releases the dependency" | Dropping it is a no-op the compiler warns about | [`lifecycle/001`](../lifecycle/001_a_barrier_from_over_to_the_end_of_a_borrow.md) |
| "The barrier knows how big the ring is" | It never has and cannot | [`invariant/002`](../invariant/002_capacity_never_enters_the_arithmetic.md) |

`two_barriers_over_one_set_agree` (`tests/barrier_test.rs:402-417`) asserts the
first of those behaviourally — two independently constructed barriers over one
slice report the same frontier — which is the useful half of an equality nobody
can write.

### BR26 — Two Accessors Hand Out the Barrier's Lifetime, Not Their Own

`cursor` and `dependencies` both return `'a`, not `&self` — a reference
borrowed from the slice the barrier was built over rather than from the barrier.
So both outlive the `Barrier` they came from, and a caller can drop the barrier
and keep reading its dependencies.

That is deliberate and it is what makes `Consumer::barrier()` able to return a
`Barrier<'a>` by value at all. But it means the type has two lifetimes a reader
must keep separate — the borrow of the slice, which is `'a` and long, and the
borrow of the barrier itself, which is whatever `&self` happens to be and is
never what any accessor returns. Nine methods take `&self`; not one returns a
reference tied to it.

```sh
cd "$(git rev-parse --show-toplevel)"
grep -nE "pub (const )?fn .*-> .*'a|pub (const )?fn .*&self" ring_barrier/src/lib.rs \
  | sed 's/^\([0-9]*\):  */\1  /' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
```

Live output:

```
119  pub const fn dependencies( &self ) -> &'a [ PaddedCursor ]
135  pub const fn len( &self ) -> usize
147  pub const fn is_empty( &self ) -> bool
164  pub fn cursor( &self, index : usize ) -> Option< &'a PaddedCursor >
191  pub fn frontier( &self ) -> Option< Seq >
216  pub fn available( &self, from : Seq ) -> u64
238  pub fn admits( &self, from : Seq, count : u64 ) -> bool
282  pub fn wait_for( &self, from : Seq, count : u64, kind : WaitKind, spins : usize )
```

### APIs

| File | Relationship |
|------|--------------|
| [001_nine_methods_over_one_borrowed_slice.md](001_nine_methods_over_one_borrowed_slice.md) | The nine signatures this lifetime runs through |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_one_field_and_a_sixteen_byte_view.md](../data_structure/001_one_field_and_a_sixteen_byte_view.md) | The field, and the padding it points at |
| [../data_structure/002_the_slices_three_provenances.md](../data_structure/002_the_slices_three_provenances.md) | Where borrowed cursors come from |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_a_slice_rather_than_an_aggregate.md](../decisions/002_a_slice_rather_than_an_aggregate.md) | Why the type borrows at all |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_a_barrier_from_over_to_the_end_of_a_borrow.md](../lifecycle/001_a_barrier_from_over_to_the_end_of_a_borrow.md) | A life with no destructor and no release |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_the_borrowed_view_and_the_owned_set.md](../pattern/001_the_borrowed_view_and_the_owned_set.md) | The same contrast at the level of the two crates |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_the_traits_derived_and_the_traits_absent.md](../type/002_the_traits_derived_and_the_traits_absent.md) | The three derives, and the auto-traits nobody wrote |

### Sources

| File | Relationship |
|------|--------------|
| `ring_barrier/src/lib.rs:80-84` | The derive and the field |
| `ring_barrier/src/lib.rs:164-167` | The `&'a` return |
| `ring_gating/src/lib.rs:142-145` | The `&self` return, for contrast |

### Tests

| File | Relationship |
|------|--------------|
| `tests/barrier_test.rs:341-370` | `Copy` across a thread boundary |
| `tests/barrier_test.rs:402-417` | Two barriers over one set agreeing |
| `tests/barrier_test.rs:419-434` | `dependencies()` returning the slice it was given |
