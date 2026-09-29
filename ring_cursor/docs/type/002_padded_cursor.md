# Type: `PaddedCursor`

### Scope

- **Purpose**: State what a consumer may rely on from `PaddedCursor`, what the type system already guarantees without help, and what the 64 bytes do *not* promise.
- **Responsibility**: Give the declaration, the two derives and the four absent ones, the promises with their enforcing tests, and the boundary.
- **In Scope**: `PaddedCursor` at `src/lib.rs:143-144`; its derives; the `SeqCell` impl at `src/lib.rs:199-221`.
- **Out of Scope**: The declaration-level reference — signatures, attributes, caller tree — which is [`item/001`](../item/001_padded_cursor_and_its_functions.md); the layout arithmetic, which is [`data_structure/001`](../data_structure/001_the_padded_cursor.md).

### The Declaration

```rust
#[ derive( Debug, Default ) ]
pub struct PaddedCursor( CacheAligned< AtomicSeq > );
```

One field, two derives. Everything the type promises comes from
[`ring_align::CacheAligned`](../../../ring_align/docs/type/002_cache_aligned.md)'s
`#[ repr( align( 64 ) ) ]` and from `ring_atomic::AtomicSeq`'s behaviour; this
declaration adds neither state nor logic. `tests/manual/readme.md` M2 is the
check that it stays that way, because a field added inside the existing padding
would not change `size_of`.

### What a Consumer May Rely On

| # | Promise | Enforced by |
|---|---------|-------------|
| P1 | `size_of == align_of == ring_align::CACHE_LINE` | `a_padded_cursor_occupies_exactly_one_cache_line` — three assertions, the third pinning `64` to `CACHE_LINE` rather than to a literal |
| P2 | In an array, consecutive cursors are exactly one line apart | `an_array_of_cursors_gives_each_its_own_line` — the stride, which `align_of` alone would not give |
| P3 | The value is the only state — 64 bytes of which 8 carry a sequence | `a_padded_cursor_is_its_atomic_and_nothing_else`, plus M2 read by hand |
| P4 | Two `&PaddedCursor` to one cursor see each other's writes | The type system — see below |
| P5 | Every `SeqCell` method forwards the caller's ordering unchanged | `padding_does_not_change_what_the_cell_does` |
| P6 | `default()` is a cursor at `Seq::ZERO` | `a_cursor_holds_the_sequence_it_was_built_with` |

**P1 and P2 are separate promises and both are needed.** A type of size 8 with
`align_of == 64` satisfies P1's first half and packs two neighbours 8 bytes apart
in an array; a type of size 64 with `align_of == 8` can start at offset 8 and
straddle two lines. Only the conjunction says "one per line", which is why the
reached-test has three clauses rather than one.

### The Derives, and the Four That Are Absent

| Derive | Present | Why |
|--------|:-------:|-----|
| `Debug` | yes | Required by `CursorPair`'s own `#[ derive( Debug ) ]`, and used by `ring_spsc`'s hand-written `Debug` impl |
| `Default` | yes | A cursor at zero is the only sensible default, and `CursorPair::new` does not use it — it calls `PaddedCursor::new( Seq::ZERO )` explicitly so the constructor can be `const` |
| `Clone` | **no** | Cloning a cursor produces a second, independent cursor — the exact confusion P4 exists to prevent |
| `Copy` | **no** | Same, silently: a `Copy` cursor would be duplicated by every read of a field |
| `PartialEq` | **no** | Comparing two cursors compares two atomics at unspecified moments; `load` then compare is the honest form |
| `Eq` | **no** | Follows `PartialEq` |

`CacheAligned< T >` derives all six, conditionally on `T`. `AtomicSeq` is not
`Copy` — no atomic is — so `CacheAligned< AtomicSeq >` is not `Copy` either, and
`PaddedCursor` could not have been `Copy` even if the declaration had asked.

**Which makes P4's test a documentation test rather than a guard.**
`a_cursor_is_shared_by_reference_not_by_copy` reasons:

> If the padding wrapper had made the type `Copy`, a caller could hold two
> independent cursors while believing it held one.

The wrapper could not have. Two `&T` to one `T` seeing each other's writes is
true of every non-`Copy` type in Rust and cannot fail here. The test records the
intent and would catch a redesign that replaced the atomic with a plain value —
it does not catch the failure its comment describes, because the compiler
already forbids it.

That is worth knowing rather than fixing: a test that cannot fail still earns its
place if the reader is told which of the two it is.

### What the 64 Bytes Do Not Promise

| # | Not promised | Consequence |
|---|--------------|-------------|
| N1 | That the atomic sits at offset 0 within the cursor | `#[ repr( align ) ]` does not imply `repr( C )` or `repr( transparent )`; field placement stays unspecified. Nothing depends on it — every access goes through `CacheAligned::get` |
| N2 | That `addr()` is the atomic's address | It is the *cursor's* address, which is what the line arithmetic needs. `PaddedCursor::addr`'s own documentation says so |
| N3 | That the padding stays empty | `size_of` cannot see a field added inside it. M2 is the instrument, and it is a source reading |
| N4 | That two cursors in *different allocations* are on different lines | They may share one by accident and the arithmetic would still be correct — separation is a property of a `CursorPair`'s layout, not of the type |

**N4 is the one a reader is most likely to over-read.** `PaddedCursor` guarantees
that a cursor *occupies* a whole line, so nothing else can be placed inside it.
It does not guarantee two arbitrary cursors are apart, because two allocations
can land anywhere. `CursorPair::on_distinct_lines` exists to check the case that
matters, on real addresses.

### The `SeqCell` Impl Forwards, It Does Not Decide

```rust
fn load( &self, order : Ordering ) -> Seq { self.0.get().load( order ) }
```

Four methods, each one line, each passing the caller's `order` through unchanged.
This is deliberately the opposite policy from
[`GATING`](001_gating.md) — argued in
[`decisions/001`](../decisions/001_gating_is_fixed_not_a_parameter.md) — and the
two live in the same crate. A `PaddedCursor` on its own is a cell and lets the
caller choose; a `CursorPair`'s readings are a gate and do not.

### What Would Break If This Changes

| Change | Who breaks | How loudly |
|--------|-----------|------------|
| A second field added | Nobody, until the padding fills | **Silently.** `size_of` is unchanged until the type exceeds 64 bytes. M2 is the only check |
| `#[ repr( align( 64 ) ) ]` written here instead of via `CacheAligned` | Nobody | **Silently**, and it forks the family's cache-line constant — see [`pitfall/001`](../pitfall/001_the_obvious_implementation_forks_the_constant.md) |
| `Copy` added | Every caller holding a cursor by value | Compile-time for the impl, silent for correctness — but the atomic forbids it |
| The wrapper removed, leaving a bare `AtomicSeq` | The reached-test's three clauses | Loudly — sizes and gaps both change |

### CU47 — Five Exported Lines, Two Owned Types

```
69:pub use ring_atomic::SeqCell;
89:pub const GATING : Ordering = Ordering::Acquire;
120:pub fn slowest( cursors : &[ PaddedCursor ] ) -> Option< Seq >
144:pub struct PaddedCursor( CacheAligned< AtomicSeq > );
247:pub struct CursorPair
```

One re-export, one constant, one function, two structs.

**Finding.** A definition named `type/` covers two structures and documents a
constant whose type came from `core` and a trait that came from `ring_atomic`.
That is not a mis-scoping — the constant and the trait are part of what a caller
must know to use the two types — but it does mean this definition's subject is
the crate's vocabulary, of which only two-fifths is types it defines.

---

### CU48 — Every Crate in the Family Cites the Same File, and It Is a Directory

```
docs/workstream/008_ring_write_path.md    MISSING
docs/workstream/008_ring_write_path       DIRECTORY
crates carrying the citation: 33
```

`docs/workstream/008_ring_write_path` exists. The `.md` does not. All 33 `ring_*`
crates name the `.md` form on line 4 of their module documentation.

**Finding.** This is not a typo in one crate — it is the family's single outward
pointer, written once and copied 33 times, resolving to nothing in every copy. A
reader following it from any crate gets a missing-path error rather than the
directory sitting beside it. The repair is one character in 33 files, and the
reason it has survived is that no gate checks a path cited from inside a doc
comment: `citations.py` reads Markdown links between corpus documents, not
`//!` prose.

```sh
cd "$(git rev-parse --show-toplevel)"
ls docs/workstream/008_ring_write_path/readme.md
command grep -rl 'docs/workstream/008_ring_write_path/readme\.md' ring_*/src/lib.rs | wc -l
```

Live output:

```
docs/workstream/008_ring_write_path/readme.md
33
```

**Disposition:** applied — corrected the module doc comment's citation in all
33 `ring_*` crates (`src/lib.rs` line 4, or line 5 in `ring_cursor` and
`ring_align`) from the missing `.md` file — and, in `ring_batch`'s case, a bare
directory reference with no filename at all — to the `readme.md` that actually
exists in the directory. One crate, `ring_align`, already cited the directory's `readme.md` correctly
before this fix and was left untouched. Verified with `cargo check --workspace`
(clean) and `cargo test --release -p ring_cursor -p ring_batch` (all passing).
Now prints: `docs/workstream/008_ring_write_path/readme.md`

---

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_padded_cursor.md](../data_structure/001_the_padded_cursor.md) | The layout the promises are measured against |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_surface_that_forwards.md](../api/001_the_surface_that_forwards.md) | The four `SeqCell` methods, as a surface |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_one_cursor_one_line.md](../invariant/001_one_cursor_one_line.md) | P1 and P2 as a standing restriction rather than as a promise |
| [../invariant/002_the_number_64_never_appears_here.md](../invariant/002_the_number_64_never_appears_here.md) | Why the alignment is inherited rather than declared |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_padded_cursor_and_its_functions.md](../item/001_padded_cursor_and_its_functions.md) | Signatures, attributes, and the measured caller tree |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_the_forwarding_newtype.md](../pattern/001_the_forwarding_newtype.md) | The shape this type instantiates |

### Types

| File | Relationship |
|------|--------------|
| [001_gating.md](001_gating.md) | The opposite ordering policy, in the same crate |

### Sources

| File | Relationship |
|------|--------------|
| `ring_cursor/src/lib.rs:125-144` | The declaration and its documentation |
| `ring_cursor/src/lib.rs:199-221` | The `SeqCell` impl |
| `ring_align/src/lib.rs` | `CacheAligned` and its six conditional derives |
| `ring_atomic/src/lib.rs:108-151` | The `SeqCell` trait being implemented |

### Tests

| File | Relationship |
|------|--------------|
| `tests/cursor_test.rs:54-72` | P1 — three clauses, the third pinning `64` to `CACHE_LINE` |
| `tests/cursor_test.rs:113-126` | P2 — the array stride |
| `tests/cursor_test.rs:399-408` | P3 — 64 bytes of which 8 carry a sequence |
| `tests/cursor_test.rs:162-173` | P4 — the documentation test the compiler already guarantees |
| `tests/cursor_test.rs:137-160` | P5 — every method behaves as the unpadded cell does |
| `tests/manual/readme.md` M2 | N3 — the only instrument that can see a field hidden in the padding |
