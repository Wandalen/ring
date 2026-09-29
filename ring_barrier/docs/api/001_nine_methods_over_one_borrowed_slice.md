# API: Nine Methods Over One Borrowed Slice

### Scope

- **Purpose**: Give the crate's whole public surface in one table, and read what its shape commits to.
- **Responsibility**: Enumerate the nine methods, their three tiers, the four that are `const`, the eight that are `#[ must_use ]`, and the one that is neither.
- **In Scope**: Signatures and the obligations they place on a caller.
- **Out of Scope**: The borrow itself — see [`002`](002_the_borrow_is_the_whole_type.md).

### The Surface

One type, nine methods, no free functions, no traits implemented by hand.

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'pub const fn\|pub fn' ring_barrier/src/lib.rs
grep -c '#\[ must_use \]' ring_barrier/src/lib.rs   # 8
```

Live output:

```
  pub const fn over( dependencies : &'a [ PaddedCursor ] ) -> Self
  pub const fn dependencies( &self ) -> &'a [ PaddedCursor ]
  pub const fn len( &self ) -> usize
  pub const fn is_empty( &self ) -> bool
  pub fn cursor( &self, index : usize ) -> Option< &'a PaddedCursor >
  pub fn frontier( &self ) -> Option< Seq >
  pub fn available( &self, from : Seq ) -> u64
  pub fn admits( &self, from : Seq, count : u64 ) -> bool
  pub fn wait_for( &self, from : Seq, count : u64, kind : WaitKind, spins : usize )
8
```

| Line | Method | Tier | `const` | `#[ must_use ]` | Returns |
|-----:|--------|------|:-------:|:---------------:|---------|
| 104 | `over( &'a [ PaddedCursor ] )` | shape | ✔ | ✔ | `Self` |
| 119 | `dependencies( &self )` | shape | ✔ | ✔ | `&'a [ PaddedCursor ]` |
| 135 | `len( &self )` | shape | ✔ | ✔ | `usize` |
| 147 | `is_empty( &self )` | shape | ✔ | ✔ | `bool` |
| 164 | `cursor( &self, index )` | shape | — | ✔ | `Option< &'a PaddedCursor >` |
| 191 | `frontier( &self )` | reading | — | ✔ | `Option< Seq >` |
| 216 | `available( &self, from )` | reading | — | ✔ | `u64` |
| 238 | `admits( &self, from, count )` | reading | — | ✔ | `bool` |
| 282 | `wait_for( &self, from, count, kind, spins )` | waiting | — | — | `Result< Seq, RingError >` |

The three tiers are the same three the family uses everywhere — a quantity, a
predicate over it, and a wait on the predicate. That shape and its three
inconsistent namings are
[`pattern/002`](../pattern/002_the_quantity_the_predicate_and_the_wait.md).

### BR9 — The Four `const` Methods, and Where the Line Falls

Four of nine are `const fn`, against one of eleven in `ring_gating`. The line is
not a policy either crate chose; it is exactly where the first non-`const`
operation appears:

| Method | `const`? | Because |
|--------|:--------:|---------|
| `over` | ✔ | A struct literal |
| `dependencies` | ✔ | A field read |
| `len` / `is_empty` | ✔ | `<[T]>::len` and `is_empty` are `const` |
| `cursor` | — | `<[T]>::get` is not `const` on stable |
| `frontier` | — | An atomic load, a `Vec`, and a fold |
| `available` / `admits` | — | Both reach `frontier` |
| `wait_for` | — | A closure and a `?` |

So the boundary lands between `is_empty` and `cursor` for a reason that has
nothing to do with this crate: one stable-`const` slice method and one that is
not. **BR9** is that comparison stated as a count — `ring_gating` reaches its
first non-`const` operation one method in, this crate reaches it five methods
in, and the difference is entirely that this crate has more methods that only
look at the slice's shape.

Nothing in the family evaluates a `Barrier` at compile time, so the four
`const`s buy nothing today. They cost nothing either, and they are the kind of
thing that is far easier to add before a caller exists than after.

### The One Method Without `#[ must_use ]`

`wait_for` is the exception, and it is not an omission: `Result` is
`#[ must_use ]` in `core`, so the attribute would be redundant and the lint
would fire on the attribute rather than on a caller. Every method that returns a
plain value carries it explicitly.

That matters more here than in most crates, because **every one of the eight is
a pure read.** A discarded `barrier.available( from )` is not a partial effect
that happened and was ignored — it is a heap allocation, a fold across every
dependency, and no effect at all
([`non_functional_requirement/001`](../non_functional_requirement/001_every_frontier_read_allocates_nothing.md)).
The attribute is what turns that from a silent waste into a compile-time
warning.

### What the Signatures Commit To

| Commitment | Where it is visible | Cost of changing it |
|------------|--------------------|---------------------|
| A borrow, not an aggregate | `over( &'a [ PaddedCursor ] )` | The handshake in `ring_publish/tests/handshake_test.rs` becomes unwireable — see [`data_structure/002`](../data_structure/002_the_slices_three_provenances.md) |
| Capacity is absent | No `Capacity` anywhere in nine signatures | The crate becomes `ring_gating` with a sign flipped — [`invariant/002`](../invariant/002_capacity_never_enters_the_arithmetic.md) |
| The empty case is representable | `frontier` returns `Option`, not `Seq` | The empty barrier collapses into "dependencies at zero" — [`decisions/001`](../decisions/001_zero_for_a_barrier_over_nothing.md) |
| Distances are `u64`, positions are `Seq` | `available` returns `u64` | Callers begin comparing a count against a position — [`type/001`](../type/001_a_u64_distance_and_a_usize_headroom.md) |
| The wait is bounded and the budget is the caller's | `spins : usize` | The crate acquires a blocking path it cannot bound |

### BR1 — What Nothing Calls

The surface is complete and almost none of it is used outside tests.

```sh
cd "$(git rev-parse --show-toplevel)"
for f in ring_*/src/*.rs ring_*/tests/*.rs; do
  n=$( grep -vE "^[[:space:]]*(///|//!)" "$f" | grep -c 'Barrier::over' )
  [ "$n" != 0 ] && printf '%-46s %s\n' "$f" "$n" || true
done
```

Live output:

```
ring_barrier/tests/allocation_test.rs     5
ring_barrier/tests/barrier_test.rs        29
ring_consume/tests/allocation_test.rs     2
ring_consume/tests/consume_test.rs        18
ring_publish/tests/handshake_test.rs      10
```

| Site | `Barrier::over` |
|------|----------------:|
| `ring_barrier/tests/barrier_test.rs` | 27 |
| `ring_consume/tests/consume_test.rs` | 18 |
| `ring_publish/tests/handshake_test.rs` | 10 |
| `ring_barrier/tests/allocation_test.rs` | 5 |
| `ring_consume/tests/allocation_test.rs` | 2 |
| **Any `src/`** | **0** |

**BR1** — sixty-two constructions, none in a library. The one library that holds
a `Barrier` (`ring_consume`) receives it as a constructor argument and never
builds one. That is not a defect: a barrier is a *view*, and a view is
constructed by whoever knows which cursors to point it at, which is by
definition not the crate that reads through it. It does mean every guarantee in
the surface above is currently exercised by tests alone.

The narrower version of the same reading, per method, is
[`item/002`](../item/002_the_five_accessors_and_the_wait.md).

### BR25 — The One Method Without `must_use` Is the One That Does Not Need It

Eight of the nine methods carry an explicit `#[ must_use ]`. `wait_for` does
not — and it is the only one returning a `Result`, which `std` already marks
`#[ must_use ]` at the type. The annotation is present on exactly the eight that
would otherwise be silently discardable and absent from the one that gets the
lint for free.

That is the correct allocation, and it is worth recording because the crate this
one waits through does the opposite: `ring_wait::escalation_hint` carries an
explicit `must_use` on an `Option` return while `pause` returns a bare `bool`
carrying the non-blocking guarantee with no annotation at all. Two crates, one
ruling, opposite outcomes — and nothing in the family checks either.

```sh
cd "$(git rev-parse --show-toplevel)"
awk '/#\[ must_use \]/{m=1;next} /pub (const )?fn /{ printf "%-10s %s\n", (m?"must_use":"BARE"), $0; m=0 }' \
  ring_barrier/src/lib.rs | sed 's/( *&self.*//;s/pub //'
```

Live output:

```
must_use     const fn over( dependencies : &'a [ PaddedCursor ] ) -> Self
must_use     const fn dependencies
must_use     const fn len
must_use     const fn is_empty
must_use     fn cursor
must_use     fn frontier
must_use     fn available
must_use     fn admits
BARE         fn wait_for
```

### APIs

| File | Relationship |
|------|--------------|
| [002_the_borrow_is_the_whole_type.md](002_the_borrow_is_the_whole_type.md) | What `&'a` in five of these signatures does |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_wait_for_asks_twice.md](../algorithm/002_wait_for_asks_twice.md) | The one method whose return slot is already spent |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_one_field_and_a_sixteen_byte_view.md](../data_structure/001_one_field_and_a_sixteen_byte_view.md) | The single field all nine methods read |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_zero_for_a_barrier_over_nothing.md](../decisions/001_zero_for_a_barrier_over_nothing.md) | The `Option` in `frontier`'s return type |
| [../decisions/002_a_slice_rather_than_an_aggregate.md](../decisions/002_a_slice_rather_than_an_aggregate.md) | The parameter type of `over` |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_three_barrier_readings.md](../item/001_the_three_barrier_readings.md) | The reading tier, method by method |
| [../item/002_the_five_accessors_and_the_wait.md](../item/002_the_five_accessors_and_the_wait.md) | The shape tier and `wait_for`, method by method |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_the_quantity_the_predicate_and_the_wait.md](../pattern/002_the_quantity_the_predicate_and_the_wait.md) | The three tiers as a family shape |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_a_u64_distance_and_a_usize_headroom.md](../type/001_a_u64_distance_and_a_usize_headroom.md) | Why `available` is `u64` and `headroom` is `usize` |

### Sources

| File | Relationship |
|------|--------------|
| `ring_barrier/src/lib.rs:86-288` | The whole `impl` block |
| `ring_gating/src/lib.rs` | The eleven-method comparison |

### Tests

| File | Relationship |
|------|--------------|
| `tests/barrier_test.rs` | 22 tests over the nine methods |
| `src/lib.rs` | 10 doctests — one on the struct and one on every method |
