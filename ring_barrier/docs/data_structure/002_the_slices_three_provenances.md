# Data Structure: The Slice's Three Provenances

### Scope

- **Purpose**: Enumerate where a barrier's dependency slice actually comes from, and show that the three sources have no common supertype other than "a slice".
- **Responsibility**: Census every construction in the family, group the arguments by origin, and argue what each origin would cost under a narrower parameter type.
- **In Scope**: The provenance of `&'a [ PaddedCursor ]`.
- **Out of Scope**: Why the parameter is a slice at all — see [`decisions/002`](../decisions/002_a_slice_rather_than_an_aggregate.md).

### The Census

```sh
cd "$(git rev-parse --show-toplevel)"
grep -hoE 'Barrier::over\([^)]*\)' ring_*/tests/*.rs \
  | sed 's/Barrier::over( *//; s/ *)$//' | sort | uniq -c | sort -rn
```

Live output:

```
     19 &published
     15 &deps
     10 core::slice::from_ref( publisher.cursor(
      6 &[]
      2 set.cursors(
      2 &one
      1 &three
      1 &nothing_published
      1 &none
      1 empty.cursors(
      1 &deps_at( &[ 9, 3 ]
      1 &deps_at( &[ 8, 9, 2 ]
      1 &deps_at( &[ 8, 2, 9 ]
      1 &deps_at( &[ 3, 9 ]
      1 &deps_at( &[ 2, 8, 9 ]
```

| Provenance | Argument forms | Sites |
|------------|----------------|------:|
| An array or `Vec` the caller made | `&published`, `&deps`, `&nothing_published`, `&deps_at( … )`, `&one`, `&three`, `&none` | 44 |
| One cursor, widened | `core::slice::from_ref( publisher.cursor() )` | 10 |
| Another crate's owned storage | `set.cursors()`, `empty.cursors()` | 3 |
| The empty literal | `&[]` | 6 |
| | | **63** |

Sixty-three constructions, none in a library ([`api/001`](../api/001_nine_methods_over_one_borrowed_slice.md) § BR1).

### The Three That Are Not the Same Thing

Strip the test scaffolding and three genuinely different origins remain, each
with a different owner and a different lifetime story:

| | `Publisher::cursor()` | `GatingSet::cursors()` | A bare array |
|--|----------------------|------------------------|--------------|
| Owned by | A publisher, one cursor | A gating set, a `Vec` | The caller's stack or heap |
| Yields | `&PaddedCursor` | `&[ PaddedCursor ]` | `&[ PaddedCursor; N ]` |
| Count | Always 1 | 0 or more | Any |
| Signature | `pub const fn cursor( &self ) -> &PaddedCursor` | `pub fn cursors( &self ) -> &[ PaddedCursor ]` | — |
| Means | *A consumer reading behind a producer* | *A consumer chained behind other consumers* | *A test, or a caller assembling dependencies by hand* |

The middle column is where `Barrier` and `GatingSet` meet: the cursors a
producer gates on through a `GatingSet` are the same objects a downstream
barrier waits on. `a_barrier_over_a_gating_set_reads_that_set_and_not_a_copy`
(`tests/barrier_test.rs:436-449`) is that composition asserted directly —
storing through `set.cursor( 0 )` changes what `barrier.frontier()` reports,
and `barrier.frontier() == set.slowest()` on the same instant.

### Why `from_ref` Is the Interesting One

The single-cursor case is the family's most common real shape — one producer,
one consumer reading behind it — and it is the one a narrower parameter type
would have broken:

```rust
// ring_publish/tests/handshake_test.rs:119
let barrier = Barrier::over( core::slice::from_ref( publisher.cursor() ) );
```

`core::slice::from_ref` is a `const fn` that reinterprets a `&T` as a
`&[ T; 1 ]` — no allocation, no copy, no move. A `Publisher` owns its cursor
and hands out a shared borrow of it; there is no way to move that cursor into an
aggregate that owns its own, so a `Barrier::over( &GatingSet )` signature would
have left this shape unwireable. The module documentation says so in as many
words:

> Taking the aggregate instead would have meant a publisher's cursor could never
> be depended on at all, since there is no way to move an existing cursor into a
> set that owns its own. That is not a hypothetical: it is what made the
> four-operation handshake in `ring_publish/tests/handshake_test.rs` unwireable
> until this signature changed.
>
> — `ring_barrier/src/lib.rs:40-44`

Ten of the sixty-three sites are that handshake, which is the strongest evidence
in the crate that the slice was forced by a caller rather than chosen for taste.

### The Fourth Row Is Not a Provenance

`&[]` is a slice with no origin — the empty case, six sites, all of them
deliberate:

| Site | Asserts |
|------|---------|
| `tests/barrier_test.rs:229` | A zero-length request is admitted even with no dependencies at all |
| `tests/barrier_test.rs:237` | No frontier, nothing available |
| `tests/barrier_test.rs:269` | Waiting fails rather than hanging |
| `tests/barrier_test.rs:283` | Admits a zero-length request but still refuses to wait for it |
| `ring_consume/tests/allocation_test.rs:119` | An idle consumer over an empty barrier, part of the zero-allocation measurement |
| `ring_consume/tests/consume_test.rs:158` | A consumer behind nothing has nothing to consume |

That the empty slice is constructible at all — and cheap, and `const` — is what
lets the empty case be a *tested input* rather than a special constructor. The
answer it produces is
[`decisions/001`](../decisions/001_zero_for_a_barrier_over_nothing.md).

### BR29 — The Const Chain Is Complete and Nothing Uses It

`PaddedCursor::new` is `const`. `Barrier::over` is `const`.
`ring_consume::Consumer::new` is `const` and takes a `Barrier<'a>`. So a
consumer bounded by a barrier over a static array of cursors can be built
entirely at compile time — the chain has no gap in it.

No `const` or `static` of either type exists anywhere in the family. The
const-ness was carried across three crates and two API boundaries for a
construction nobody performs, which is a different thing from const-ness that
cannot be used: this one works, and the cost of keeping it working is a
constraint on every future change to three constructors.

```sh
cd "$(git rev-parse --show-toplevel)"
grep "pub const fn new\|pub const fn over" \
  ring_cursor/src/lib.rs ring_barrier/src/lib.rs ring_consume/src/lib.rs
# any const or static of either type, family-wide
grep -rE "(const|static) [A-Z_]+ *: *(Barrier|Consumer)" --include=*.rs ring_*/ \
  || echo '(no const or static of either type exists)'
# control: the identical expression for the one const the family does declare
grep -rE "(const|static) [A-Z_]+ *: *Self" --include=*.rs ring_types/src/ | head -2
```

Live output:

```
ring_cursor/src/lib.rs:  pub const fn new( value : Seq ) -> Self
ring_cursor/src/lib.rs:  pub const fn new( capacity : Capacity ) -> Self
ring_barrier/src/lib.rs:  pub const fn over( dependencies : &'a [ PaddedCursor ] ) -> Self
ring_consume/src/lib.rs:  pub const fn new( start : Seq, len : u64 ) -> Self
ring_consume/src/lib.rs:  pub const fn new( cursor : &'a PaddedCursor, barrier : Barrier< 'a > ) -> Self
(no const or static of either type exists)
ring_types/src/id.rs:  pub const ZERO : Self = Self( 0 );
```

### Data Structures

| File | Relationship |
|------|--------------|
| [001_one_field_and_a_sixteen_byte_view.md](001_one_field_and_a_sixteen_byte_view.md) | The field these slices land in |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_nine_methods_over_one_borrowed_slice.md](../api/001_nine_methods_over_one_borrowed_slice.md) | The BR1 census these sixty-three sites are |
| [../api/002_the_borrow_is_the_whole_type.md](../api/002_the_borrow_is_the_whole_type.md) | The lifetime each provenance supplies |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_zero_for_a_barrier_over_nothing.md](../decisions/001_zero_for_a_barrier_over_nothing.md) | What the fourth row resolves to |
| [../decisions/002_a_slice_rather_than_an_aggregate.md](../decisions/002_a_slice_rather_than_an_aggregate.md) | The decision these provenances forced |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_three_dependencies_and_one_dependent.md](../integration/001_three_dependencies_and_one_dependent.md) | `ring_publish` and `ring_gating` as sources, without being dependencies |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_a_consumer_draining_behind_a_producer.md](../lifecycle/002_a_consumer_draining_behind_a_producer.md) | The `from_ref` provenance in motion |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_the_borrowed_view_and_the_owned_set.md](../pattern/001_the_borrowed_view_and_the_owned_set.md) | Borrowing as what makes three provenances one parameter |

### Sources

| File | Relationship |
|------|--------------|
| `ring_barrier/src/lib.rs:30-44` | The module's own argument for the slice |
| `ring_publish/src/lib.rs:117` | `Publisher::cursor` |
| `ring_gating/src/lib.rs:155` | `GatingSet::cursors` |

### Tests

| File | Relationship |
|------|--------------|
| `tests/barrier_test.rs:436-449` | A barrier over a `GatingSet`'s cursors |
| `tests/barrier_test.rs:419-434` | A barrier over a bare array |
| `ring_publish/tests/handshake_test.rs` | Ten `from_ref` sites, the handshake this signature exists for |
