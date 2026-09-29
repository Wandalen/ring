# Type: The Traits Derived and the Traits Absent

### Scope

- **Purpose**: Account for every trait `Barrier` has and every trait it does not, separating the ones that are impossible from the one that is merely declined.
- **Responsibility**: List the three derives, the two auto traits, and the five absences — with a probe result for each claim rather than an argument.
- **In Scope**: `Barrier`'s trait surface.
- **Out of Scope**: What `Copy` buys the concurrency test — see [`api/002`](../api/002_the_borrow_is_the_whole_type.md).

### The Whole Declaration

```rust
// ring_barrier/src/lib.rs:80-84
#[ derive( Debug, Clone, Copy ) ]
pub struct Barrier< 'a >
{
  dependencies : &'a [ PaddedCursor ],
}
```

Three derives, one field, and no hand-written `impl` of anything —
`grep -c 'impl' ring_barrier/src/lib.rs` finds only the inherent block.

### Everything the Type Has

| Trait | How | Probe |
|-------|-----|-------|
| `Debug` | derived | `Barrier { dependencies: [PaddedCursor(CacheAligned(AtomicSeq(3)))] }` |
| `Clone` | derived, implied by `Copy` | — |
| `Copy` | derived | `assert_copy::< Barrier >()` compiles |
| `Send` | **auto** — `&T : Send` where `T : Sync` | `assert_send::< Barrier< 'static > >()` compiles |
| `Sync` | **auto** — `&T : Sync` where `T : Sync` | `assert_sync::< Barrier< 'static > >()` compiles |

`Send` and `Sync` are the two that matter and neither is written down anywhere.
They hold because `PaddedCursor` wraps an `AtomicSeq`, which is `Sync`; a shared
reference to a `Sync` type is both. So the type that crosses a thread boundary
in every concurrent test in the crate does so on a property nothing in the
source states — which is why `tests/barrier_test.rs:341-370` moving a `Barrier`
into a spawned closure is load-bearing documentation, not just a test.

The `Debug` output is worth reading once: it prints the *cursors*, three
newtypes deep, not a summary. A barrier over 64 dependencies logs 64 cache-line
structs. That is the derive being honest about what the type is — a view of
other people's state ([`data_structure/001`](../data_structure/001_one_field_and_a_sixteen_byte_view.md)).

### Everything the Type Lacks

`Seq` derives nine traits (`ring_types/src/id.rs:24`); `Barrier` derives
three.
The six-trait gap splits into two groups, and the split is the finding:

| Trait | Absent because | Verified by |
|-------|----------------|-------------|
| `PartialEq` | **impossible** — `PaddedCursor` does not implement it | `error[E0369]` (below) |
| `Eq` | **impossible** — requires `PartialEq` | same |
| `PartialOrd` | **impossible** — requires `PartialEq` | same |
| `Ord` | **impossible** — requires `Eq` + `PartialOrd` | same |
| `Hash` | **impossible** — requires the field to be `Hash`, and an atomic is not | same |
| `Default` | **possible, and declined** | a probe that compiles |

The first five are one fact, not five. `PaddedCursor` derives exactly
`Debug, Default` (`ring_cursor:143`), and adding `PartialEq` there would mean
comparing two atomics — a read of each, at some ordering, at two different
instants. The compiler's refusal is the right answer:

```
error[E0369]: binary operation `==` cannot be applied to type `&[PaddedCursor]`
  = note: `PaddedCursor` does not implement `PartialEq`
```

So there is no version of this type that compares. Two barriers over the same
slice are indistinguishable by construction and comparable by nothing — which
also means no test can assert `barrier_a == barrier_b`, and none tries.

### `Default` Is the Interesting Absence

`&[T]` *does* implement `Default`, returning the empty slice. So
`#[ derive( Debug, Clone, Copy, Default ) ]` would compile here, and
`Barrier::default()` would be exactly `Barrier::over( &[] )`:

```rust
// probe: the same field, with Default added
#[ derive( Debug, Default ) ]
struct DefaultProbe< 'a >{ dependencies : &'a [ PaddedCursor ] }
// prints: Default over &[PaddedCursor] derives; len = 0
```

It compiles. It is not there. And the reason is the whole of
[`pitfall/001`](../pitfall/001_the_two_empty_answers_look_like_a_bug.md):

| | |
|--|--|
| What `Barrier::default()` would mean | a barrier over nothing |
| What a barrier over nothing answers | `frontier() == None`, `available( … ) == 0`, `admits( …, 1 ) == false` |
| So a defaulted barrier is | a consumer that can never read |

A `Default` that silently produces the *most restrictive possible value* is a
trap: `..Default::default()` in a struct-update expression, or a `#[derive(Default)]`
on some future config that embeds one, would yield a barrier that blocks
forever and reports no error. `GatingSet` has the mirror-image hazard — its
empty set means *unbounded* — and the two would fail in opposite, equally
silent directions.

Declining a derive that would compile is the kind of decision that leaves no
trace in the source, so it is recorded here rather than inferred from the
absence.

### What This Costs

| Wanted | Available instead |
|--------|-------------------|
| `assert_eq!( barrier_a, barrier_b )` | compare `len()`, or `frontier()` — both `PartialEq` |
| A `Barrier` in a `HashMap` key | none; no `Hash`, and none possible |
| `Barrier::default()` | `Barrier::over( &[] )`, which says what it means |
| Sorting barriers | none; and the ordering would be over addresses, not state |

Only the first is a real inconvenience, and the tests route around it by
asserting on the readings rather than the view — which is the right thing to
assert on anyway, since two barriers over different slices holding equal
positions *should* compare equal to a consumer and would not compare equal to a
derived `PartialEq`.

### BR49 — No `Default`, and the Empty Barrier Is Reached by a Literal Instead

`Barrier` derives `Debug`, `Clone` and `Copy` and implements nothing else. In
particular there is no `Default`, so the empty barrier — the input class the
crate documents most and tests four ways — is constructed as
`Barrier::over( &[] )` every time it is needed.

A `Default` returning that would be sound and would read better at every call
site. It is absent for a reason the type states without saying: `Default` on a
borrowing type must produce a `'static` borrow, and while `&[]` satisfies that,
the impl would advertise a constructor whose lifetime behaviour is unlike every
other way of building the type. The literal is the honest form, and it is
written out four times — three tests and one doctest, all of them here.

```sh
cd "$(git rev-parse --show-toplevel)"
grep "derive\|^impl" ring_barrier/src/lib.rs | grep -v "^.*://"
grep -c "over( &\[\] )" ring_barrier/tests/barrier_test.rs ring_barrier/src/lib.rs
```

Live output:

```
#[ derive( Debug, Clone, Copy ) ]
impl< 'a > Barrier< 'a >
ring_barrier/tests/barrier_test.rs:4
ring_barrier/src/lib.rs:1
```

### Types

| File | Relationship |
|------|--------------|
| [001_a_u64_distance_and_a_usize_headroom.md](001_a_u64_distance_and_a_usize_headroom.md) | The widths in the signatures these traits wrap |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_borrow_is_the_whole_type.md](../api/002_the_borrow_is_the_whole_type.md) | What `Copy` and the lifetime do together |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_one_field_and_a_sixteen_byte_view.md](../data_structure/001_one_field_and_a_sixteen_byte_view.md) | The one field, and what `Debug` prints of it |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_a_barrier_from_over_to_the_end_of_a_borrow.md](../lifecycle/001_a_barrier_from_over_to_the_end_of_a_borrow.md) | Why there is no `Drop` either |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_the_borrowed_view_and_the_owned_set.md](../pattern/001_the_borrowed_view_and_the_owned_set.md) | The traits borrowing makes free, and the ones it forecloses |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_the_two_empty_answers_look_like_a_bug.md](../pitfall/001_the_two_empty_answers_look_like_a_bug.md) | Why the empty barrier is a bad default |

### Sources

| File | Relationship |
|------|--------------|
| `ring_barrier/src/lib.rs:80-84,86` | The derive, the field, and the only `impl` |
| `ring_cursor/src/lib.rs:143-144` | `PaddedCursor`'s two derives |
| `ring_types/src/id.rs:24-25` | `Seq`'s nine, for contrast |

### Tests

| File | Relationship |
|------|--------------|
| `tests/barrier_test.rs:341-370` | `Send` + `Copy` across a thread boundary, unstated in the source |
| `tests/barrier_test.rs:234-246` | The readings a test asserts on instead of comparing views |
