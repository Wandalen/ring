# Pattern: The Borrowed View and the Owned Set

### Scope

- **Purpose**: Name the ownership pattern this crate is the family's borrowing half of, and show that the choice follows from who creates the cursors rather than from taste.
- **Responsibility**: State the two shapes, the rule that selects between them, what each buys, and the one case where the rule was discovered rather than applied.
- **In Scope**: Ownership of the cursors a type reasons about.
- **Out of Scope**: The decision as this crate made it — see [`decisions/002`](../decisions/002_a_slice_rather_than_an_aggregate.md).

### The Two Shapes

| | Owned Set | **Borrowed View** |
|--|-----------|-------------------|
| Instance | `ring_gating::GatingSet` | **`ring_barrier::Barrier`** |
| Holds | `Vec< PaddedCursor >` | `&'a [ PaddedCursor ]` |
| Creates its cursors | ✔ | ✘ |
| `size_of` | 32 + heap | 16 |
| `Copy` | ✘ | ✔ |
| Destructor | frees the `Vec` | none |
| Construction cost | 1 allocation (`consumers > 0`) | 0 |
| A handed-out reference is bounded by | the set | **the cursors, not the view** |

Both hold the same element type and take the same minimum over it. Everything
else about them differs, and all of it follows from one question.

### The Rule

> **The type that creates the cursors owns them. Every other type borrows.**

A producer's gating set *creates* the consumer cursors — that is what
`GatingSet::new( capacity, consumers )` does, and it is why a set of `n`
consumers is the thing consumers are handed cursors *from*. A consumer's barrier
creates nothing: it points at cursors a publisher or another set already owns
and already writes to.

The failure mode of getting it backwards is not a compile error either way:

| Mistake | Compiles | Passes the sequential suite | Actually wrong because |
|---------|:--------:|:---------------------------:|------------------------|
| A barrier owning a `Vec< PaddedCursor >` | ✔ | ✔ | It reads a snapshot taken at construction while the producer gates on the live cursors |
| A gating set borrowing cursors | ✔ | ✔ | Nobody creates them, so every consumer must be handed one from somewhere that does |

The first is what manual check B2 exists to catch, and its note says exactly why
the sequential suite is no help: a snapshot and a live cursor agree until
something advances.

### The Rule Was Discovered, Not Applied

This crate had the other shape first. `Barrier::over` took a `&GatingSet` — the
symmetric-looking choice, given that `ring_barrier` and `ring_gating` are two
halves of one thing —
and the design held until a test could not be written:

> That second mistake is not hypothetical — it is what this crate did until
> `ring_publish/tests/handshake_test.rs` could not be written, which is the
> staged-validation loop working as intended: build the machinery, let it tell
> you the design is wrong.
>
> — `tests/manual/readme.md` § B2

A `Publisher` owns its published cursor. There is no operation that moves an
existing `PaddedCursor` into a `GatingSet`'s `Vec`, and there should not be —
the publisher is still writing to it. So the aggregate signature made the
family's most common shape, one consumer behind one producer, unexpressible.

That is the useful part of the pattern: the rule is not an aesthetic preference
that happened to be right. It was recovered from a concrete failure, and the
failure was a test that would not compile.

### What Borrowing Buys, Beyond the Handshake

| Property | Consequence |
|----------|-------------|
| `Copy` | A barrier crosses a thread boundary by value; the concurrency test needs no `Arc`, no `clone`, no scoped-borrow gymnastics — [`api/002`](../api/002_the_borrow_is_the_whole_type.md) |
| No destructor | There is no release, so no ordering question about *when* a dependency stops being depended on — [`lifecycle/001`](../lifecycle/001_a_barrier_from_over_to_the_end_of_a_borrow.md) |
| Zero-cost construction | A barrier can be built per call site rather than stored; `handshake_test.rs` builds ten |
| Three provenances, one parameter | A publisher's cursor, a gating set's slice, and a bare array all fit — [`data_structure/002`](../data_structure/002_the_slices_three_provenances.md) |

The third row is worth noticing. Because construction is free and the type is
16 bytes, `Barrier::over( … )` is closer to a cast than to an object — which is
why nothing in the family stores one except `ring_consume`, and why the 63
construction sites are almost all inline in the expression that reads them.

### What It Costs

| Cost | Detail |
|------|--------|
| Contiguity | Dependencies must live in one slice; three cursors owned by three different things need three barriers or a caller-built array |
| No compile-time pairing | Nothing in the type says these are `ring_gating`'s other half — `ring_gating` is only a dev-dependency, [`workaround/002`](../workaround/002_ring_gating_as_a_dev_dependency.md) |
| A wrong slice compiles | A barrier over a cursor nobody advances waits forever, and no signature here can tell — [`decisions/002`](../decisions/002_a_slice_rather_than_an_aggregate.md) |

### BR44 — The Two Halves of One Feature Share No Type, No Trait, and No Function Signature

This crate and `ring_gating` read the same cursors, take
the same fold, and answer questions that are mirror images. They have no common
trait, no shared return type, and no function either could pass to the other:
`Barrier::available` returns `u64`, `GatingSet::headroom` returns `usize`;
`available` takes a `Seq`, `headroom` takes a `Seq` and closes over a capacity.

The shared part was factored out — `ring_cursor::slowest`, which both call and
neither owns. Everything above it diverged, and the module documentation of both
crates argues that this is correct. What no document states is that the two
crates are, at the type level, unrelated.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -E "pub fn (available|headroom|admits|check)" \
  ring_barrier/src/lib.rs ring_gating/src/lib.rs | sed 's|ring/||'
# the one thing they share
command grep -r "ring_cursor::slowest" --include=*.rs ring_*/src/ | sed 's|ring/||'
```

Live output:

```
ring_barrier/src/lib.rs:  pub fn available( &self, from : Seq ) -> u64
ring_barrier/src/lib.rs:  pub fn admits( &self, from : Seq, count : u64 ) -> bool
ring_gating/src/lib.rs:  pub fn headroom( &self, producer : Seq ) -> usize
ring_gating/src/lib.rs:  pub fn admits( &self, producer : Seq, count : usize ) -> bool
ring_gating/src/lib.rs:  pub fn check( &self, producer : Seq, count : usize ) -> Result< (), RingError >
ring_barrier/src/lib.rs://! [`ring_cursor::slowest`] and lives in neither of them.
ring_barrier/src/lib.rs:    ring_cursor::slowest( self.dependencies )
ring_cursor/src/lib.rs:/// assert_eq!( ring_cursor::slowest( &[] ), None );
ring_cursor/src/lib.rs:/// assert_eq!( ring_cursor::slowest( &cursors ), Some( Seq( 4 ) ) );
ring_cursor/src/lib.rs:/// assert_eq!( ring_cursor::slowest( &cursors ), Some( Seq( 9 ) ) );
ring_gating/src/lib.rs://! `ring_cursor::slowest` returns `None` for an empty set rather than `Seq::ZERO`,
ring_gating/src/lib.rs:  /// The fold itself is [`ring_cursor::slowest`], shared with `ring_barrier`,
ring_gating/src/lib.rs:    ring_cursor::slowest( &self.cursors )
```

### Patterns

| File | Relationship |
|------|--------------|
| [002_the_quantity_the_predicate_and_the_wait.md](002_the_quantity_the_predicate_and_the_wait.md) | The API shape both halves build on top of these two |
| [`ring_gating`'s pattern/001](../../../ring_gating/docs/pattern/001_the_owned_set_with_shared_readers.md) | The owning half of this pattern |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_borrow_is_the_whole_type.md](../api/002_the_borrow_is_the_whole_type.md) | The lifetime that makes the view a view |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_one_field_and_a_sixteen_byte_view.md](../data_structure/001_one_field_and_a_sixteen_byte_view.md) | The size comparison |
| [../data_structure/002_the_slices_three_provenances.md](../data_structure/002_the_slices_three_provenances.md) | The owners a view points at |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_a_slice_rather_than_an_aggregate.md](../decisions/002_a_slice_rather_than_an_aggregate.md) | The decision, with its alternatives priced |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_a_barrier_from_over_to_the_end_of_a_borrow.md](../lifecycle/001_a_barrier_from_over_to_the_end_of_a_borrow.md) | What a view has instead of a lifecycle |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_the_traits_derived_and_the_traits_absent.md](../type/002_the_traits_derived_and_the_traits_absent.md) | The traits borrowing makes free |

### Workarounds

| File | Relationship |
|------|--------------|
| [../workaround/002_ring_gating_as_a_dev_dependency.md](../workaround/002_ring_gating_as_a_dev_dependency.md) | The missing compile-time pairing |

### Sources

| File | Relationship |
|------|--------------|
| `ring_barrier/src/lib.rs:30-44` | The module's argument for the borrow |
| `ring_gating/src/lib.rs` | `GatingSet`, the owning half |
| `ring_publish/src/lib.rs:117` | The cursor that cannot be moved |
| `tests/manual/readme.md` § B2 | The check, and how the rule was found |

### Tests

| File | Relationship |
|------|--------------|
| `tests/barrier_test.rs:436-449` | A view over an owned set — both shapes at once |
| `tests/barrier_test.rs:341-370` | `Copy` across threads |
| `ring_publish/tests/handshake_test.rs` | The test that the aggregate signature made unwriteable |
