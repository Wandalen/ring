# Type: `Send` and `Sync` Without `unsafe`

### Scope

- **Purpose**: Trace `GatingSet`'s thread-safety to its source, and establish that not one line of the chain is an author's assertion.
- **Responsibility**: Give the four-wrapper derivation, prove no `unsafe` appears in any crate along it, and identify what the compiler would reject if a field changed.
- **In Scope**: Why `&GatingSet` may cross a thread boundary.
- **Out of Scope**: The memory ordering the cursors are read at — see [`invariant/002`](../invariant/002_this_crate_names_no_ordering.md).

### The Chain

```
GatingSet { cursors : Vec< PaddedCursor >, capacity : Capacity }   ring_gating
  └─ PaddedCursor( CacheAligned< AtomicSeq > )                     ring_cursor
       └─ CacheAligned< T >( T )   #[ repr( align( 64 ) ) ]        ring_align
            └─ AtomicSeq( AtomicU64 )                              ring_atomic
                 └─ AtomicU64                                      core
```

`AtomicU64 : Sync` is the standard library's own `unsafe impl`, and it is the
only one in the chain. Every layer above it is a newtype over a `Sync` field,
so `Sync` propagates automatically:

| Layer | Gets `Sync` | Written where |
|-------|-------------|---------------|
| `AtomicU64` | asserted | `core` |
| `AtomicSeq` | inferred | — |
| `CacheAligned< AtomicSeq >` | inferred | — |
| `PaddedCursor` | inferred | — |
| `Vec< PaddedCursor >` | inferred | `std`'s conditional impl |
| `GatingSet` | inferred | — |

`Capacity( usize )` is `Sync` for the same reason every plain integer is.

### No `unsafe` Anywhere Along It

```sh
cd "$(git rev-parse --show-toplevel)"
for f in ring_{align,atomic,cursor,gating}/src/lib.rs; do
  echo "$f: $( grep -vE "^[[:space:]]*(///|//!)" "$f" | grep -c 'unsafe' )"
done
# all four report 0
```

Live output:

```
ring_align/src/lib.rs: 0
ring_atomic/src/lib.rs: 0
ring_cursor/src/lib.rs: 0
ring_gating/src/lib.rs: 0
```

Four crates, zero `unsafe` in code. A bare `grep unsafe` reports 4 hits each in
`ring_align` and `ring_atomic`, and **all eight are prose** — both crates
explicitly document that they need none:

> No `unsafe` is needed for any of it — `#[ repr( align( 64 ) ) ]` is a safe
> attribute
>
> — `ring_align/src/lib.rs:15`

> No `unsafe`: `AtomicU64` is a safe abstraction, so nothing here needs to opt
> out of the workspace-wide `unsafe-code = "deny"`. This crate used to hold an
> entry in `bench_harness/gate/declared/ring/unsafe_allowlist.txt`
>
> — `ring_atomic/src/lib.rs:35-37`

Both record a *removed* allowlist entry. So the absence is a state that was
reached, not a state that was always true — the same shape of history as
[`invariant/002`](../invariant/002_this_crate_names_no_ordering.md)'s ordering
absence.

Workspace-wide, exactly two crates opt back in, and neither is on this chain:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r 'allow( *unsafe_code' ring_*/src/*.rs
# ring_mpsc:191, ring_spsc:169 — the two ring backends
```

Live output:

```
ring_mpsc/src/lib.rs:#![allow(unsafe_code)]
ring_spsc/src/lib.rs:#![allow(unsafe_code)]
```

### What This Buys

**`GatingSet`'s thread-safety is a compiler conclusion, and could not have been
an author's assertion.** The workspace sets `unsafe-code = "deny"` and this crate
does not opt out, so `unsafe impl Sync for GatingSet {}` would not compile here
even if someone wanted it.

The consequence is sharper than tidiness. If a field were replaced with something
not `Sync` — a `Cell`, an `Rc`, a raw pointer — the derivation would simply stop,
and the crate's own concurrent test would fail to compile:

```rust
// tests/gating_test.rs:405-427, as originally written
std::thread::scope( | scope |
{
  scope.spawn( ||
  {
    set.cursor( 0 ).unwrap().store( Seq( position ), Ordering::Release );
  } );

  for _ in 0..10_000
  {
    let headroom = set.headroom( producer );
    …
  }
} );
```

`&set` is captured by the spawned closure while the main thread reads through
the same reference. That requires `GatingSet : Sync`, and there is no `unsafe
impl` available to paper over its loss.

| Field change | Result |
|--------------|--------|
| `Vec< PaddedCursor >` → `Vec< Cell< Seq > >` | Test stops compiling; no way to force it |
| `Capacity` → `Rc< Capacity >` | Same |
| Adding `unsafe impl Send`/`Sync` | Rejected by `unsafe-code = "deny"` |

That last row is what makes the other two safe. A crate that *could* write
`unsafe impl` would be one edit away from turning a compile error into a data
race; this one is not.

### The One Property That Is Not Free

`Sync` comes for free. **Interior mutability does not** — it is why the whole
`&self` surface works, and it comes from `AtomicU64` specifically rather than
from the wrappers.

`cursor()` returns `&PaddedCursor`, and a caller stores through it. A shared
reference to an ordinary type would not allow that; a shared reference to one
wrapping an atomic does. So the four wrappers are transparent in both directions:
they neither add thread-safety nor take away the mutability the atomic provides.

That transparency is the reason `CacheAligned` can be a `#[ repr( align( 64 ) ) ]`
newtype rather than something cleverer — see
[`api/001`](../api/001_eleven_methods_over_one_owned_vec.md) § G1 for what it
means for the surface.

### What the Alignment Is Doing

`CacheAligned` is on the chain for a reason unrelated to `Sync`: it makes
`size_of::< PaddedCursor >()` and `align_of::< PaddedCursor >()` both 64, so a
`Vec< PaddedCursor >` gives every consumer's cursor its own cache line. A test in
this crate checks the consequence directly:

```rust
// tests/gating_test.rs:341-353
for window in set.cursors().windows( 2 )
{
  assert_eq!( window[ 1 ].addr() - window[ 0 ].addr(), 64 );
}
```

Its comment names why the check belongs here rather than in `ring_align`: *"A
`GatingSet` is where several consumers' cursors are most likely to end up
adjacent, so it is where false sharing would actually bite."* That is
leaf-proximate placement working — the property is `ring_align`'s, the *stake* in
it is this crate's.

### GT53 — Four Crates, Four Wrappers, No `unsafe`

```
GatingSet -> Vec< PaddedCursor >     ( ring_gating )
          -> PaddedCursor             ( ring_cursor )
          -> CacheAligned< AtomicSeq > ( ring_align )
          -> AtomicSeq -> AtomicU64    ( ring_atomic )
unsafe in ring_gating/src/lib.rs : 0
```

The auto traits arrive by inference through every wrapper in the chain. Two of
the crates in it document why they no longer need an `unsafe impl` to say so.

**Finding.** It crosses four crates and four wrapper types without one line of `unsafe` — and two of those crates document why they no longer need any

---

### GT54 — Nothing Asserts Either Trait

```
occurrences of 'Send' in ring_gating/ : 0
occurrences of 'Sync' in ring_gating/ : 0
      ( src and tests both )
```

Inheriting a property is not the same as pinning it. A `fn assert_sync< T : Sync >()`
call needs no `unsafe` and no allowlist entry, so the absence here is a gap
rather than something the crate is prevented from doing.

**Finding.** Nothing. Neither trait is named anywhere in this crate or its tests, and both are inherited from `AtomicU64` three crates down. A static assertion would need no `unsafe` and no allowlist entry, so the absence is a gap rather than a constraint the crate is under

---


### APIs

| File | Relationship |
|------|--------------|
| [../api/001_eleven_methods_over_one_owned_vec.md](../api/001_eleven_methods_over_one_owned_vec.md) | G1 — the `&self`-only surface interior mutability makes possible |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_owning_the_cursors_rather_than_borrowing_them.md](../data_structure/002_owning_the_cursors_rather_than_borrowing_them.md) | The two fields this derivation reads |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_this_crate_names_no_ordering.md](../invariant/002_this_crate_names_no_ordering.md) | The absence this one is the mechanism behind |

### Types

| File | Relationship |
|------|--------------|
| [001_a_usize_headroom_and_a_u64_limit.md](001_a_usize_headroom_and_a_u64_limit.md) | The widths the same field types decide |

### Sources

| File | Relationship |
|------|--------------|
| `ring_cursor/src/lib.rs:143-144` | `PaddedCursor( CacheAligned< AtomicSeq > )` |
| `ring_align/src/lib.rs:67-69` | `CacheAligned< T >`, `#[ repr( align( 64 ) ) ]` |
| `ring_align/src/lib.rs:14-19` | Why it needs no `unsafe`, and the allowlist entry it dropped |
| `ring_atomic/src/lib.rs:166` | `AtomicSeq( AtomicU64 )` |
| `ring_atomic/src/lib.rs:35-41` | The same history, from the atomic's side |
| `Cargo.toml` § `[workspace.lints.rust]` | `unsafe-code = "deny"` |

### Tests

| File | Relationship |
|------|--------------|
| `tests/gating_test.rs:394-428` | Compiles only because the derivation holds |
| `tests/gating_test.rs:341-353` | The 64-byte stride the alignment wrapper buys |
| `tests/gating_test.rs:329-339` | Stores and loads through `&self`, single-threaded |
