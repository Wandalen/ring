# API: The Registry Surface

### Scope

- **Purpose**: Define the seven operations, and record the two signature decisions that are not obvious from the names.
- **Responsibility**: Signatures, guarantees, and what each one costs the caller.
- **In Scope**: `Registry::{ new, register, get_mut, remove, contains, len, is_empty, names }`.
- **Out of Scope**: What the registry owns and when it lets go (→ [`lifecycle/001`](../lifecycle/001_a_ring_from_registration_to_drop.md)); the error value (→ [`type/001`](../type/001_registry_error.md)).

### Abstract

A name-to-ring map with eight operations, and two signature choices that carry
the whole design: there is **no immutable `get`**, and `register` hands the
rejected ring *back* rather than dropping it.

Both fall out of what the registry actually stores. A `Split< T >` is a pair of
handles whose useful operations all take `&mut self`, so lending one immutably
would lend something nothing could be done with; and a refused registration that
consumed its argument would destroy a ring for the crime of colliding on a name.

### Operations

| Operation | Signature | Returns |
|---|---|---|
| `new` / `default` | `fn() -> Self` | An empty registry |
| `register` | `fn( &mut self, impl Into< String >, Split< T > ) -> Result< (), ( RegistryError, Split< T > ) >` | Nothing, or the error **and the rejected ring**; an owned `String` avoids the name allocation |
| `get_mut` | `fn( &mut self, &str ) -> Option< &mut Split< T > >` | A mutable borrow |
| `remove` | `fn( &mut self, &str ) -> Option< Split< T > >` | Ownership |
| `contains` | `fn( &self, &str ) -> bool` | — |
| `len` / `is_empty` | `fn( &self ) -> usize` / `bool` | — |
| `names` | `fn( &self ) -> impl Iterator< Item = &str >` | Live names, unordered |

#### There is no immutable `get`, and that is not an omission

`ring_handle::Split< T >` has exactly two methods: `new`, and `ends( &mut self )`.
So a `&Split< T >` permits **no operation at all** — a caller holding one can
neither read the ring, nor write to it, nor ask it anything.

An immutable `get` would therefore be a method that compiles, returns something
non-`None`, and is useless. [`Self::contains`] is the immutable query that is
actually answerable, and `len`/`is_empty`/`names` are the rest of what can be
known without borrowing mutably.

**This is a fact about `ring_handle`, not a preference.** If `Split` ever grew a
`&self` accessor — a capacity reading, say — an immutable `get` would become
worth having, and this paragraph is the record of why it was not there before.

#### `register` returns the ring it refused

The signature is `Result< (), ( RegistryError, Split< T > ) >` rather than
`Result< (), RegistryError >`, because a caller whose name collided still owns a
perfectly good ring — one that may already hold records. Swallowing it would
make "you chose a name that was taken" destroy data, which is a wildly
disproportionate consequence for a naming mistake.

**It costs the caller a `Debug` bound at the `.unwrap()`/`.expect()` sites**,
and a second time, independently: `Registry< T >` derives `Debug` on its own
struct, so a `T` that never calls `register` still owes the bound to print the
registry or hold it in a struct that derives `Debug`. Three tests failed to
compile until their record type derived `Debug` — `Result::expect` requires it.

**It also costs a `Result` 448 bytes wide, and a suppressed lint.** That number
is not an estimate — `clippy::result_large_err` measured it and failed the build
under `-D warnings`. The lint's two remedies are both refused, and the refusals
are the same argument as the table below one level down: shrinking the payload
means dropping the ring, which is [`pitfall/001`](../pitfall/001_insert_would_have_replaced_silently.md)
displaced from a successful registration to a failed one; boxing it allocates on
the failure path to narrow a `Result` whose width the caller already pays on the
way *in*, since `register` takes the same `Split< T >` by value. The width lands
on the `Ok` path too, which is the honest cost — acceptable only because
`register` is a setup-time call, once per ring, never in a loop. The suppression
is `#[ allow( clippy::result_large_err, reason = .. ) ]` on `register` itself,
not a crate-level allow, so a *different* oversized `Result` appearing later
still fails the build.

| Alternative | Why not |
|---|---|
| `Result< (), RegistryError >` | Drops the rejected ring, and its records, as a side effect of a name collision |
| Replace silently | Same, but for the ring already registered — worse, because it is not the one the caller was thinking about |
| `contains` first, then an infallible `register` | Two calls where one belongs, and no compiler pressure to make the first one |
| `Result< (), Box< ( RegistryError, Split< T > ) > >` | Silences the lint by allocating on the failure path, and makes the error awkward to destructure, to narrow a width the `register` argument already carries |
| **Current** | One call; the ring comes back whole, the name is consumed and reported by copy; a `Debug` bound at `.expect()` sites, a 448-byte `Result`, and one scoped `allow` |

### Error Handling

One variant, [`RegistryError::NameTaken`](../type/001_registry_error.md).
`get_mut` and `remove` return `Option` rather than `Result` — an absent name is
an ordinary answer, not a failure.

### Compatibility Guarantees

| # | Guarantee | Evidence |
|---|---|---|
| A1 | A registered ring is retrievable by its own name and by no other | `a_registered_ring_is_retrievable_by_its_name_and_by_no_other` |
| A2 | Retrieval lends the same ring, not a copy or a rebuild | `a_retrieved_ring_keeps_what_was_written_to_it` |
| A3 | A refused registration changes nothing | `a_second_registration_under_a_live_name_is_refused`, `a_refused_registration_does_not_drop_the_ring_already_there` |
| A4 | A refused ring comes back usable | `a_refused_registration_hands_the_ring_back` |
| A5 | `remove` frees the name for reuse | `removing_a_name_frees_it_for_reuse` |
| A6 | Any string is a valid name | `unusual_names_are_ordinary_names` |

**A3 is the one with a specific failure behind it.** The natural implementation
is `HashMap::insert`, which *replaces* and returns the displaced value — ignore
that return and a name collision silently destroys the ring already registered,
along with everything unread in it. The implementation uses `Entry` for exactly
this reason, and the drop-counter test is what would notice if it stopped.

**A6 is pinned rather than assumed.** `register` takes `impl Into< String >`, so
the empty string, a string with spaces, and one with a newline are all ordinary
keys. A later validation pass rejecting them would be a behaviour change, and
`unusual_names_are_ordinary_names` is the test that would say so.

### Preconditions

| # | Precondition | On whom | If violated |
|---|---|---|---|
| B1 | One `T` per registry | The type system | Does not compile — see [`decisions/readme.md`](../decisions/readme.md) Pending 1 |
| B2 | `names`' order is not relied on | The caller | A test that passes on one hasher seed and fails on another |

**B2 is stated because it is easy to violate by accident.** `HashMap` iteration
order is unspecified and varies; the crate's own test sorts before comparing, and
a caller that does not will write something that looks correct and is not.

---

## What the Two Stated Costs Actually Reach

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order. Line numbers are
# omitted for this file alone: the findings below are appended to it, so any
# number printed here would be stale by the time it is read. The patterns are
# anchored for the same reason — unanchored, they would match this command and
# its own quoted output, both of which now live in the file being searched.
a=ring_registry/docs/api/001_the_registry_surface.md
echo '  -- where this file prices the Debug bound, and to what --'
command grep -h '^\*\*It costs the caller a\|^| \*\*Current\*\* | One call' "$a" | cut -c1-96 | sed 's/^/    /'
echo '  -- the two arguments register takes by value --'
command grep -m1 -A1 -F '    name : impl Into< String >,' ring_registry/src/lib.rs | sed 's/^/    /'
echo '  -- what happens to each of them on the refusal path --'
command grep -m1 -A5 -F '    match self.rings.entry( name )' ring_registry/src/lib.rs | sed 's/^/    /'
echo '  -- every derive that puts T under a Debug bound --'
command grep '^#\[ derive' ring_registry/src/lib.rs | sed 's/^/    ring_registry:/'
command grep '^#\[ derive' ring_handle/src/lib.rs | head -2 | sed 's/^/    ring_handle:/'
echo '  -- and every place the tests get hold of a RegistryError --'
command grep 'expect_err\|RegistryError::NameTaken { name :' ring_registry/tests/registry_test.rs | cut -c1-88 | sed 's/^/    /'
echo '  -- the second, independent source of the Debug bound --'
command grep -h '^and a second time, independently' "$a" | sed 's/^/    /'
echo '  -- what an owned String buys register, named in the operations table --'
command grep -h '^| .register. | .fn( &mut self' "$a" | sed 's/^/    /'
```

Live output:

```
  -- where this file prices the Debug bound, and to what --
    **It costs the caller a `Debug` bound at the `.unwrap()`/`.expect()` sites**,
    | **Current** | One call; the ring comes back whole, the name is consumed and reported by copy; 
  -- the two arguments register takes by value --
        name : impl Into< String >,
        ring : Split< T >,
  -- what happens to each of them on the refusal path --
        match self.rings.entry( name )
        {
          Entry::Occupied( occupied ) =>
          {
            let name = occupied.key().clone();
            Err( ( RegistryError::NameTaken { name }, ring ) )
  -- every derive that puts T under a Debug bound --
    ring_registry:#[ derive( Debug, Clone, PartialEq, Eq ) ]
    ring_registry:#[ derive( Debug ) ]
    ring_handle:#[ derive( Debug ) ]
    ring_handle:#[ derive( Debug ) ]
  -- and every place the tests get hold of a RegistryError --
        .expect_err( "a live name was taken twice" );
      assert_eq!( error, RegistryError::NameTaken { name : "events".to_string() } );
        .expect_err( "refused" );
        .expect_err( "refused" );
      let error = RegistryError::NameTaken { name : "events".to_string() };
        Err( Box::new( RegistryError::NameTaken { name : "x".to_string() } ) )
  -- the second, independent source of the Debug bound --
    and a second time, independently: `Registry< T >` derives `Debug` on its own
  -- what an owned String buys register, named in the operations table --
    | `register` | `fn( &mut self, impl Into< String >, Split< T > ) -> Result< (), ( RegistryError, Split< T > ) >` | Nothing, or the error **and the rejected ring**; an owned `String` avoids the name allocation |
```

### Every Way a Non-`Debug` Record Type Fails to Compile

```rust
// compile/-debug_bound.rs
pub struct NoDebug;

// (a) the documented cost: `.expect()` on the register result.
pub fn at_expect( r : &mut Registry< NoDebug >, s : Split< NoDebug > )
{
  r.register( "a", s ).expect( "a free name" );
}

// (b) the undocumented one: printing a registry, or embedding it in a struct
// that derives Debug. Neither goes near `register`.
pub fn at_print( r : &Registry< NoDebug > ) { println!( "{r:?}" ); }

#[ derive( Debug ) ]
pub struct Holder { rings : Registry< NoDebug > }
```

```
error[E0277]: `NoDebug` doesn't implement `Debug`
   |                        ^^^^^^ the trait `Debug` is not implemented for `NoDebug`
   = note: required for `ring_handle::Split<NoDebug>` to implement `Debug`
   = note: required for `(RegistryError, ring_handle::Split<NoDebug>)` to implement `Debug`
note: required by a bound in `Result::<T, E>::expect`
error[E0277]: `NoDebug` doesn't implement `Debug`
help: the trait `Debug` is implemented for `Registry<T>`
   = note: required for `Registry<NoDebug>` to implement `Debug`
   = note: required for `&Registry<NoDebug>` to implement `Debug`
error[E0277]: `NoDebug` doesn't implement `Debug`
   |   ^^^^^^^^^^^^^^^^^^^^^^^^^^^ the trait `Debug` is not implemented for `NoDebug`
help: the trait `Debug` is implemented for `Registry<T>`
error: aborting due to 3 previous errors
```

### What Becomes of the Name

Every ring is built before the measurement window opens — `Ring::new` allocates,
and counting that would put the ring's cost on the name's bill. The caller's
`String` is over-reserved so a copy of it is distinguishable from the original:

```rust
// src/bin/name_identity.rs
let mut mine = String::with_capacity( 64 );
mine.push_str( "events" );
let mine_cap = mine.capacity();
reset();
let refused = registry.register( mine, spare.pop().unwrap() );
```

```
    a String the caller already owns, into a taken name:
      allocations 1, deallocations 1
      the caller handed over a String of capacity 64
      the name in the error has capacity 6
      the two are equal as strings: true
    a &str into a taken name:
      allocations 2, deallocations 1   refused true
    a String into a free name:
      allocations 0, deallocations 0   accepted true
```

---

### RG5 — The `Debug` Bound Is Charged to the Error Payload and Imposed Twice More by a Derive

This file states the cost precisely and attributes it to one place: "**It costs
the caller a `Debug` bound at the `.unwrap()`/`.expect()` sites**", repeated in
the alternatives table as "a `Debug` bound at `.expect()` sites". The mechanism
given is right — `Result::expect` requires `E : Debug`, and `E` is
`( RegistryError, Split< T > )`, which is `Debug` only when `T` is.

The bound has a second source that has nothing to do with `register`.
`#[ derive( Debug ) ]` on `Registry< T >` at `src/lib.rs:87` generates
`impl< T : Debug > Debug for Registry< T >`, so a `Registry< NoDebug >` is simply
not printable. Compiled, a non-`Debug` record type produces **three** errors, not
one: the documented `.expect()` site, `println!( "{r:?}" )` on a registry, and a
struct that holds a registry and derives `Debug` itself. The last two are reached
by callers who never call `register` and never see a `Result`.

What makes this worth recording rather than shrugging at is that the crate
already knows the move. [RG47](../type/002_what_registry_derives_and_what_it_never_asserts.md)
shows `Default` was hand-written specifically to avoid a spurious `T : Default`
bound that `#[ derive( Default ) ]` would have added — the identical hazard, on
the identical struct, two lines apart. One derive was reasoned about and replaced;
the other was left, and its cost was then attributed to a different part of the
API entirely.

**Finding.** Recorded as an understated cost with an inconsistency behind it. The
sentence is not wrong; it is one of three. A clause — that `Registry< T >` is
`Debug` only when `T` is, independently of `register` — puts the cost where the
callers who pay it will look. Whether to also hand-write `Debug`, the way
`Default` already is, is a real question this file is the right place to ask: the
answer is probably no, since a registry that cannot be printed is worse than one
that requires `T : Debug` to be printed, but "probably no" written down beats an
asymmetry nobody noticed.

**Disposition:** applied — the Debug-bound paragraph (`:56-60`) now names the
second, independent source: "`Registry< T >` derives `Debug` on its own struct,
so a `T` that never calls `register` still owes the bound to print the registry
or hold it in a struct that derives `Debug`". Now prints: `on its own`

---

### RG6 — The Refusal Hands Back One of `register`'s Two By-Value Arguments, and Nothing Says Which

`register` takes two things by value: `name : impl Into< String >` and
`ring : Split< T >`. The document's longest argument is about what happens to the
second on a refusal — it comes back, because "swallowing it would make 'you chose
a name that was taken' destroy data". The first is never mentioned.

It does not come back. `self.rings.entry( name )` consumes the name; on the
occupied path it is dropped, and the error is built from
`occupied.key().clone()` — a fresh copy of the key the *registry* holds. Measured
with a caller's `String` of capacity 64: one allocation, one deallocation, and
the name in the error has capacity 6. The two are equal as strings, so nothing
observable goes wrong today; they are equal because no normalization exists
between what a caller passes and what is stored. The tests cannot see the
difference either — every `register` call in the suite passes a literal identical
to the one already registered, so `assert_eq!( error, NameTaken { name : "events" } )`
at `tests/registry_test.rs:125` passes whichever of the two the error reports.

The same measurement prices a choice the operations table does not mention.
`impl Into< String >` makes the call's cost depend on what the caller has: an
owned `String` into a free name is **zero** allocations, the same `String` into a
taken name is one allocation and one deallocation, and a `&str` — which is what
every test, every doc example and the readme use — is two and one. The cheapest
shape the API offers is the one nothing demonstrates.

**Finding.** Recorded as an asymmetry in the crate's central contract, and a
cost table with a row missing. Two repairs, both small. This file's "nothing
destroyed" row is true of the ring and false of the name; saying so — the ring
comes back, the name is consumed and reported by copy — completes the sentence
that the rest of the crate is built around. And one line in the operations table
noting that passing an owned `String` avoids the allocation makes the generic
argument a documented lever rather than an incidental convenience. The clone
itself is not a defect; it is forced
([RG49](../workaround/001_a_key_bought_back_from_the_map.md)), which is exactly
why the asymmetry deserves a sentence instead of a fix.

**Disposition:** applied — the "Current" row (`:82`) now says "the ring comes
back whole, the name is consumed and reported by copy", and the `register` row
in the Operations table (`:26`) now notes an owned `String` avoids the name
allocation. Now prints: `avoids the name allocation`

---

### Types

| File | Relationship |
|------|--------------|
| [../type/001_registry_error.md](../type/001_registry_error.md) | The error, and why it carries the name |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_one_name_one_ring.md](../invariant/001_one_name_one_ring.md) | A1 and A3 as standing properties rather than as guarantees of a call |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_a_ring_from_registration_to_drop.md](../lifecycle/001_a_ring_from_registration_to_drop.md) | A4 and A5 — where ownership sits at each moment |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_insert_would_have_replaced_silently.md](../pitfall/001_insert_would_have_replaced_silently.md) | A3's specific failure, and why `Entry` rather than `insert` |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_one_declared_edge_of_three.md](../integration/001_one_declared_edge_of_three.md) | Why `Split< T >` and not `Ring< T >` is what gets stored |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | The surface |
| [`ring_handle/src/lib.rs`](../../../ring_handle/src/lib.rs) | `Split`'s two methods — the fact behind "no immutable `get`" |

### Tests

| File | Relationship |
|------|--------------|
| `tests/registry_test.rs` | 14 tests; A1–A6 each named above |
